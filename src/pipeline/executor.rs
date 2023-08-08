use super::{
    channel::Channel, subcommands::Subcommand, PipelineMessage, Progress, SubcommandCreator,
};
use crossbeam_channel::{unbounded, Receiver};
use std::collections::HashSet;

pub struct Executor {
    name: String,
    input_stream_names: Vec<String>,
    output_name: String,
    inputs: Vec<Receiver<PipelineMessage>>,
    channel: Channel,
    handler: Box<dyn Subcommand>,
}

pub struct ExecutorBuilder {
    output_stream_names: HashSet<String>,
}

impl ExecutorBuilder {
    pub fn new() -> Self {
        ExecutorBuilder {
            output_stream_names: HashSet::new(),
        }
    }

    pub fn create(
        &mut self,
        args: Vec<String>,
        creator: SubcommandCreator,
    ) -> Result<(Executor, Receiver<Progress>), String> {
        let name = match args.first() {
            Some(command_name) => command_name.clone(),
            None => return Err("Should have command name".to_string()),
        };

        let mut inner_args = Vec::new();
        let mut input_stream_names = Vec::new();
        let mut output_name = "".to_string();

        let cmd = args[0].clone();

        let mut has_input = false;
        let mut has_help = false;
        // println!("args: {:?}", args);
        for arg in args {
            if arg.eq("--help") || arg.eq("-h") {
                has_help = true;
            }

            if arg.starts_with("+input") || arg.starts_with("+in") {
                let input_streams = match arg.split("=").nth(1) {
                    Some(input_streams) => input_streams,
                    None => return Err("Expected name of input stream".to_string()),
                };

                for input_name in input_streams.split(',') {
                    // check if input stream name is in the set, panic if not
                    if !self.output_stream_names.contains(input_name) {
                        // get the existing output stream names, concat them with ", "
                        let existing_output_stream_names = self
                            .output_stream_names
                            .iter()
                            .map(|s| format!("`{}`", s))
                            .collect::<Vec<String>>()
                            .join(", ");

                        return Err(format!(
                            "No output stream with name `{}` found, existing outputs are {}",
                            input_streams, existing_output_stream_names
                        ));
                    } else {
                        input_stream_names.push(input_name.to_string());
                    }
                }
                has_input = true;
            } else if arg.starts_with("+output") || arg.starts_with("+out") {
                output_name = match arg.split('=').nth(1) {
                    Some(output_name) => output_name.to_string(),
                    None => return Err("Expected name of output stream".to_string()),
                };

                self.output_stream_names.insert(output_name.clone());
            } else {
                inner_args.push(arg);
            }
        }

        if has_input
            || cmd.as_str() == "read"
            || cmd.as_str() == "convert"
            || cmd.as_str() == "info"
            || cmd.as_str() == "dash"
            || has_help
        {
        } else {
            return Err(format!(
                "`{}` needs to consume an input, but no named input is found, specify it using `+input=input_name`",
                cmd.as_str()
            ));
        }

        let handler = creator(inner_args);

        let (progress_tx, progress_rx) = unbounded();
        let channel = Channel::new(progress_tx);
        let executor = Executor {
            name,
            input_stream_names,
            output_name,
            inputs: vec![],
            channel,
            handler,
        };
        Ok((executor, progress_rx))
    }
}

unsafe impl Send for Executor {}

impl Executor {
    #[allow(dead_code)]
    pub fn create(args: Vec<String>, creator: SubcommandCreator) -> (Self, Receiver<Progress>) {
        let name = args.first().expect("Should have command name").clone();
        let mut inner_args = Vec::new();
        let mut input_stream_names = Vec::new();
        let mut output_name = "".to_string();
        for arg in args {
            if arg.starts_with("+input") || arg.starts_with("+in") {
                let input_streams = arg
                    .split('=')
                    .nth(1)
                    .expect("Expected name of input stream");
                for input_name in input_streams.split(',') {
                    input_stream_names.push(input_name.to_string());
                }
            } else if arg.starts_with("+output") || arg.starts_with("+out") {
                output_name = arg
                    .split('=')
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
