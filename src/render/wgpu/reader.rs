use crate::formats::pointxyzrgba::PointXyzRgba;
use crate::formats::PointCloud;
use crate::pcd::read_pcd_file;
use log::{debug, trace};
use std::collections::HashMap;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use tokio::sync::mpsc::UnboundedSender;

use super::renderable::Renderable;

pub trait RenderReader<T: Renderable> {
    /// Initialize the input reader for our renderer. Returns the first frame, if any.
    fn start(&mut self) -> Option<T>;
    /// Returns the `index`-th frame
    fn get_at(&mut self, index: usize) -> Option<T>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn set_len(&mut self, len: usize);
}

pub struct PcdFileReader {
    files: Vec<PathBuf>,
}

impl PcdFileReader {
    pub fn from_directory(directory: &Path) -> Self {
        let mut files = vec![];
        for file_entry in directory.read_dir().unwrap() {
            match file_entry {
                Ok(entry) => {
                    if let Some(ext) = entry.path().extension() {
                        if ext.eq("pcd") {
                            files.push(entry.path());
                        }
                    }
                }
                Err(e) => {
                    eprintln!("{e}")
                }
            }
        }
        files.sort();
        Self { files }
    }

    pub fn file_at(&self, index: usize) -> Option<&PathBuf> {
        self.files.get(index)
    }
}

impl RenderReader<PointCloud<PointXyzRgba>> for PcdFileReader {
    fn start(&mut self) -> Option<PointCloud<PointXyzRgba>> {
        self.get_at(0)
    }

    fn get_at(&mut self, index: usize) -> Option<PointCloud<PointXyzRgba>> {
        self.files
            .get(index)
            .and_then(|f| read_pcd_file(f).ok())
            .map(PointCloud::from)
    }

    fn len(&self) -> usize {
        self.files.len()
    }

    fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    fn set_len(&mut self, _len: usize) {}
}

pub struct PcdMemoryReader {
    points: Vec<PointCloud<PointXyzRgba>>,
}

impl PcdMemoryReader {
    pub fn from_vec(points: Vec<PointCloud<PointXyzRgba>>) -> Self {
        Self { points }
    }
}

impl RenderReader<PointCloud<PointXyzRgba>> for PcdMemoryReader {
    fn get_at(&mut self, index: usize) -> Option<PointCloud<PointXyzRgba>> {
        self.points.get(index).map(|pc| pc.clone())
    }

    fn start(&mut self) -> Option<PointCloud<PointXyzRgba>> {
        self.get_at(0)
    }

    fn len(&self) -> usize {
        self.points.len()
    }

    fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    fn set_len(&mut self, _len: usize) {}
}

#[cfg(feature = "dash")]
pub struct PcdAsyncReader {
    current_frame: u64,
    next_to_get: u64,
    total_frames: u64,
    /// PcdAsyncReader tries to maintain this level of buffer occupancy at any time
    // buffer_size: u8,
    // cache: HashMap<FrameRequest, PointCloud<PointXyzRgba>>,
    rx: Receiver<(FrameRequest, PointCloud<PointXyzRgba>)>,
    tx: UnboundedSender<BufMsg>,
}

#[cfg(feature = "dash")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FrameRequest {
    pub object_id: u8,
    // pub quality: u8,
    pub frame_offset: u64,
}

#[derive(Debug, Clone)]
pub enum BufMsg {
    PointCloud((FrameRequest, PointCloud<PointXyzRgba>)),
    FrameRequest(FrameRequest),
}

#[cfg(feature = "dash")]
impl PcdAsyncReader {
    pub fn new(
        rx: Receiver<(FrameRequest, PointCloud<PointXyzRgba>)>,
        tx: UnboundedSender<BufMsg>,
        // buffer_size: Option<u8>,
    ) -> Self {
        // let buffer_size = buffer_size.unwrap_or(1);
        Self {
            current_frame: 0,
            next_to_get: 0,
            rx,
            tx,
            // buffer_size,
            // cache: HashMap::with_capacity(buffer_size as usize),
            total_frames: 30, // default number of frames. Use `set_len` to overwrite this value
        }
    }

    // fn send_next_req(&mut self, object_id: u8) {
    //     assert_eq!(object_id, 0, "only one object supported for now");
    //     while self.next_to_get == 0
    //         || self.next_to_get - self.current_frame <= self.buffer_size as u64 + 1
    //     {
    //         debug!(
    //             "next_to_get {}, current_frame {}, buffer_size {}",
    //             self.next_to_get, self.current_frame, self.buffer_size
    //         );
    //         // FIXME: change the object_id.
    //         self.tx
    //             .send(FrameRequest {
    //                 object_id,
    //                 frame_offset: self.next_to_get as u64,
    //             })
    //             .unwrap();
    //         self.next_to_get = (self.next_to_get + 1) % (self.len() as u64);
    //         // FIXME: THIS IS A HACK to handle edge case and loop over.
    //         if self.next_to_get == 0 {
    //             break;
    //         }
    //     }
    // }
}

#[cfg(feature = "dash")]
impl RenderReader<PointCloud<PointXyzRgba>> for PcdAsyncReader {
    fn start(&mut self) -> Option<PointCloud<PointXyzRgba>> {
        // for i in 0..self.buffer_size {
        //     self.tx
        //         .send(FrameRequest {
        //             object_id: 0,
        //             frame_offset: i as u64,
        //         })
        //         .unwrap();
        // }
        // self.next_to_get = self.buffer_size as u64;
        // loop {
        //     if let Some(data) = self.get_at(0) {
        //         break Some(data);
        //     }
        //     std::thread::sleep(std::time::Duration::from_secs(1));
        // }
        self.get_at(0)
    }

    fn get_at(&mut self, index: usize) -> Option<PointCloud<PointXyzRgba>> {
        // debug!(
        //     "reader::get_at called with {}. buffer occupancy is {}",
        //     index,
        //     self.cache.len()
        // );
        let index = index as u64;
        self.tx
            .send(BufMsg::FrameRequest(FrameRequest {
                object_id: 0,
                frame_offset: index % self.total_frames,
            }))
            .unwrap();
        dbg!("sent request. waiting for result ...");
        self.rx.recv().ok().map(|op| op.1)

        // remove if we have in the buffer.
        // FIXME: change the object_id and quality.
        // if let Some(data) = self.cache.remove(&FrameRequest {
        //     object_id: 0u8,
        //     frame_offset: index,
        // }) {
        //     trace!("get_at returned from buffer ... {}", index);
        //     self.send_next_req(0);
        //     self.current_frame = (self.current_frame + 1) % (self.len() as u64);
        //     return Some(data);
        // }

        // loop {
        //     trace!("{} looping...", index);
        //     if let Ok((req, data)) = self.rx.recv() {
        //         if req.frame_offset == index {
        //             debug!("get_at returned from channel ... {}", req.frame_offset);
        //             self.send_next_req(0);
        //             self.current_frame = (self.current_frame + 1) % (self.len() as u64);
        //             return Some(data);
        //         }

        //         // enqueues the data into our cache and preemptively start the next request.
        //         debug!("get_at buffers... {}", req.frame_offset);
        //         self.cache.insert(req, data);
        //         self.send_next_req(0);
        //     } else {
        //         break None;
        //     }
        // }
    }

    fn len(&self) -> usize {
        self.total_frames as usize
    }

    fn is_empty(&self) -> bool {
        false
    }

    fn set_len(&mut self, len: usize) {
        self.total_frames = len as u64;
    }
}

// pub struct BufRenderReader<U: Renderable + Send> {
//     size_tx: Sender<usize>,
//     receiver: Receiver<(usize, Option<U>)>,
//     length: usize,
// }

// impl<U> BufRenderReader<U>
// where
//     U: 'static + Renderable + Send + Debug,
// {
//     pub fn new<T: 'static + RenderReader<U> + Send + Sync>(buffer_size: usize, reader: T) -> Self {
//         let (size_tx, size_rx) = std::sync::mpsc::channel();
//         let (sender, receiver) = std::sync::mpsc::channel();
//         let length = reader.len();

//         let threads = rayon::current_num_threads()
//             .saturating_sub(3)
//             .min(buffer_size);
//         if threads == 0 {
//             panic!("Not enough threads!");
//         }
//         rayon::spawn(move || {
//             let mut started = false;
//             let max = length;
//             let mut current = 0;
//             let length = buffer_size;
//             let mut next = 0;
//             let (range_tx, range_rx) = std::sync::mpsc::channel::<Range<usize>>();
//             rayon::spawn(move || loop {
//                 if let Ok(range) = range_rx.recv() {
//                     // range
//                     //     .into_par_iter()
//                     //     .map(|i| (i, reader.get_at(i)))
//                     //     .collect::<Vec<(usize, Option<U>)>>()
//                     //     .into_iter()
//                     //     .for_each(|out| {
//                     //         sender.send(out).unwrap();
//                     //     });
//                 }
//             });
//             loop {
//                 if let Ok(pos) = size_rx.try_recv() {
//                     if (started && pos <= current) || pos >= next {
//                         next = pos;
//                     }
//                     started = true;
//                     current = pos;
//                 }
//                 if (length - (next - current)) >= threads && next != max {
//                     let to = (next + threads).min(max).min(current + length);
//                     range_tx
//                         .send(next..to)
//                         .expect("Failed to send range to worker");
//                     next = to;
//                 }
//             }
//         });

//         Self {
//             size_tx,
//             receiver,
//             length,
//         }
//     }
// }

// impl<U> RenderReader<U> for BufRenderReader<U>
// where
//     U: 'static + Renderable + Send + Debug,
// {
//     fn get_at(&mut self, index: usize) -> Option<U> {
//         self.size_tx.send(index).unwrap();
//         while let Ok((pos, val)) = self.receiver.recv() {
//             if pos == index {
//                 return val;
//             }
//         }
//         None
//     }

//     fn len(&self) -> usize {
//         self.length
//     }

//     fn is_empty(&self) -> bool {
//         self.length == 0
//     }
// }
