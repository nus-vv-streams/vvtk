use std::sync::mpsc::{Receiver, Sender};

use super::{subcommands::Subcommand, PipelineMessage, SubcommandCreator};

pub struct Executor {
    input: Option<Receiver<PipelineMessage>>,
    output: Sender<PipelineMessage>,
    handler: Box<dyn Subcommand>,
}

unsafe impl Send for Executor {}

impl Executor {
    pub fn create(
        args: Vec<String>,
        creator: SubcommandCreator,
    ) -> (Self, Receiver<PipelineMessage>) {
        let handler = creator(args);
        let (tx, rx) = std::sync::mpsc::channel();
        let executor = Self {
            input: None,
            output: tx,
            handler,
        };
        (executor, rx)
    }

    pub fn set_input(&mut self, recv: Receiver<PipelineMessage>) {
        self.input = Some(recv);
    }

    pub fn run(self) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || self.start())
    }

    fn start(mut self) {
        if self.input.is_none() {
            self.handler.handle(PipelineMessage::End, &self.output);
            return;
        }
        let input = self.input.unwrap();
        while let Ok(message) = input.recv() {
            let should_break = if let PipelineMessage::End = message {
                true
            } else {
                false
            };

            self.handler.handle(message, &self.output);

            if should_break {
                break;
            }
        }
    }
}
