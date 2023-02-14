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
        self.tx.send(item).await.unwrap();
        *self.length.lock().await += 1;
    }

    /// this function might be sometimes off by 1 when used concurrently.
    pub async fn len_approx(&self) -> usize {
        *self.length.lock().await
    }

    pub async fn slack(&self) -> usize {
        self.capacity - self.len_approx().await
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
}
