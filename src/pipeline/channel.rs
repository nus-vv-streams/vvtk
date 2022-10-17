use std::sync::mpsc::{channel, Receiver, SendError, Sender};

use super::PipelineMessage;

pub struct Channel {
    listeners: Vec<Sender<PipelineMessage>>,
}

impl Channel {
    pub fn new() -> Self {
        Self { listeners: vec![] }
    }

    pub fn send(&self, message: PipelineMessage) -> Result<Vec<()>, SendError<PipelineMessage>> {
        self.listeners
            .iter()
            .map(|sender| sender.send(message.clone()))
            .collect()
    }

    pub fn subscribe(&mut self) -> Receiver<PipelineMessage> {
        let (tx, rx) = channel();
        self.listeners.push(tx);
        rx
    }
}
