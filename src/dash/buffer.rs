use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

/// This is a bounded buffer.
pub struct Buffer<T> {
    tx: mpsc::Sender<T>,
    capacity: usize,
    length: Arc<Mutex<usize>>,
}

impl<T> Buffer<T>
where
    T: Debug,
{
    pub fn new(capacity: usize) -> (Self, Receiver<T>) {
        let (decoder_tx, decoder_rx) = mpsc::channel(capacity);
        let length = Arc::new(Mutex::new(0));
        (
            Self {
                tx: decoder_tx,
                length: length.clone(),
                capacity,
            },
            Receiver {
                chan: decoder_rx,
                length,
            },
        )
    }

    pub async fn push(&self, item: T) {
        *self.length.lock().await += 1;
        self.tx.send(item).await.unwrap();
    }

    pub async fn len_approx(&self) -> usize {
        *self.length.lock().await
    }

    pub async fn slack(&self) -> usize {
        let len = self.len_approx().await;
        // To avoid underflow, as len might be greater than capacity
        std::cmp::max(self.capacity, len) - len
    }
}

pub struct Receiver<T> {
    chan: mpsc::Receiver<T>,
    /// this is a copy of buffer's length field
    length: Arc<Mutex<usize>>,
}

impl<T> Receiver<T>
where
    T: Debug,
{
    pub async fn recv(&mut self) -> T {
        let item = self.chan.recv().await.unwrap();
        *self.length.lock().await -= 1;
        item
    }

    pub async fn try_recv(&mut self) -> Option<T> {
        match self.chan.try_recv().ok() {
            None => None,
            Some(item) => {
                *self.length.lock().await -= 1;
                Some(item)
            }
        }
    }
}
