use crossbeam_channel::{unbounded, Receiver};

use super::{
    channel::Channel, subcommands::Subcommand, PipelineMessage, Progress, SubcommandCreator,
};

pub struct Executor {
    name: String,
    input_stream_names: Vec<String>,
    output_name: String,
    inputs: Vec<Receiver<PipelineMessage>>,
    channel: Channel,
    handler: Box<dyn Subcommand>,
}

unsafe impl Send for Executor {}

impl Executor {
    pub fn create(args: Vec<String>, creator: SubcommandCreator) -> (Self, Receiver<Progress>) {
        let name = args.first().expect("Should have command name").clone();
        let mut inner_args = Vec::new();
        let mut input_stream_names = Vec::new();
        let mut output_name = "".to_string();
        for arg in args {
            if arg.starts_with("+input") {
                let input_streams = arg
                    .split("=")
                    .nth(1)
                    .expect("Expected name of input stream");
                for input_name in input_streams.split(",") {
                    input_stream_names.push(input_name.to_string());
                }
            } else if arg.starts_with("+output") {
                output_name = arg
                    .split("=")
                    .nth(1)
                    .expect("Expected name of output stream")
                    .to_string();
            } else {
                inner_args.push(arg);
            }
        }
        let handler = creator(inner_args);

        let (progress_tx, progress_rx) = unbounded();
        let channel = Channel::new(progress_tx);
        let executor = Self {
            name,
            input_stream_names,
            output_name,
            inputs: vec![],
            channel,
            handler,
        };
        (executor, progress_rx)
    }

    pub fn input_names(&self) -> Vec<String> {
        self.input_stream_names.clone()
    }

    pub fn output_name(&self) -> &str {
        &self.output_name
    }

    pub fn output(&mut self) -> Receiver<PipelineMessage> {
        self.channel.subscribe()
    }

    pub fn set_inputs(&mut self, inputs: Vec<Receiver<PipelineMessage>>) {
        self.inputs = inputs;
    }

    pub fn run(self) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || self.start())
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    fn start(mut self) {
        if self.inputs.is_empty() {
            self.handler.handle(vec![], &self.channel);
            return;
        }
        while let Ok(messages) = self
            .inputs
            .iter()
            .map(|recv| recv.recv())
            .collect::<Result<Vec<PipelineMessage>, _>>()
        {
            let should_break = messages.iter().any(|message| {
                if let PipelineMessage::End = message {
                    true
                } else {
                    false
                }
            });

            self.handler.handle(messages, &self.channel);

            if should_break {
                break;
            }
        }
    }
}
