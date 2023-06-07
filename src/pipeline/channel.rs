use crossbeam_channel::{bounded, Receiver, Sender};

use super::{PipelineMessage, Progress};

const MAX_MESSAGES: usize = 30;

pub struct Channel {
    progress_tx: Sender<Progress>,
    listeners: Vec<Sender<PipelineMessage>>,
}

impl Channel {
    pub fn new(progress_tx: Sender<Progress>) -> Self {
        Self {
            progress_tx,
            listeners: vec![],
        }
    }

    pub fn send(&self, message: PipelineMessage) -> Vec<()> {
        match &message {
            PipelineMessage::End => self.progress_tx.send(Progress::Completed),
            _ => self.progress_tx.send(Progress::Incr),
        }
        .expect("Should be able to send progress");

        self.listeners
            .iter()
            .map(|sender| {
                sender
                    .send(message.clone())
                    .expect("Should be able to send a message through the channel")
            })
            .collect()
    }

    pub fn subscribe(&mut self) -> Receiver<PipelineMessage> {
        let (tx, rx) = bounded(MAX_MESSAGES);
        self.listeners.push(tx);
        rx
    }
}
