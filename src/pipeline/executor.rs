use std::sync::mpsc::{Receiver, Sender};

use super::{subcommands::Subcommand, PipelineMessage, Progress, SubcommandCreator};

pub struct Executor {
    name: String,
    input: Option<Receiver<PipelineMessage>>,
    output: Sender<PipelineMessage>,
    progress: Sender<Progress>,
    handler: Box<dyn Subcommand>,
}

unsafe impl Send for Executor {}

impl Executor {
    pub fn create(
        args: Vec<String>,
        creator: SubcommandCreator,
    ) -> (Self, Receiver<PipelineMessage>, Receiver<Progress>) {
        let name = args.first().expect("Should have command name").clone();
        let handler = creator(args);
        let (pipeline_tx, pipeline_rx) = std::sync::mpsc::channel();
        let (progress_tx, progress_rx) = std::sync::mpsc::channel();
        let executor = Self {
            name,
            input: None,
            output: pipeline_tx,
            progress: progress_tx,
            handler,
        };
        (executor, pipeline_rx, progress_rx)
    }

    pub fn set_input(&mut self, recv: Receiver<PipelineMessage>) {
        self.input = Some(recv);
    }

    pub fn run(self) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || self.start())
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    fn start(mut self) {
        if self.input.is_none() {
            self.handler
                .handle(PipelineMessage::End, &self.output, &self.progress);
            return;
        }
        let input = self.input.unwrap();
        while let Ok(message) = input.recv() {
            let should_break = if let PipelineMessage::End = message {
                true
            } else {
                false
            };

            self.handler.handle(message, &self.output, &self.progress);

            if should_break {
                break;
            }
        }
    }
}
