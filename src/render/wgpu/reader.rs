use crate::formats::pointxyzrgba::PointXyzRgba;
use crate::formats::PointCloud;
use crate::pcd::{read_pcd_file, PointCloudData};
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use tokio::sync::mpsc::UnboundedSender;
use wgpu_glyph::ab_glyph::Point;

use super::renderable::Renderable;

pub trait RenderReader<T: Renderable> {
    fn start(&mut self) -> Option<T>;
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
    points: Vec<PointCloudData>,
}

impl PcdMemoryReader {
    pub fn from_vec(points: Vec<PointCloudData>) -> Self {
        Self { points }
    }
}

impl RenderReader<PointCloud<PointXyzRgba>> for PcdMemoryReader {
    fn start(&mut self) -> Option<PointCloud<PointXyzRgba>> {
        self.get_at(0)
    }

    fn get_at(&mut self, index: usize) -> Option<PointCloud<PointXyzRgba>> {
        self.points.get(index).map(|p| PointCloud::from(p.clone()))
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
    rx: Receiver<PointCloud<PointXyzRgba>>,
    tx: UnboundedSender<FrameRequest>,
}

#[cfg(feature = "dash")]
#[derive(Debug)]
pub struct FrameRequest {
    pub object_id: u8,
    pub quality: u8,
    pub frame_offset: u64,
}

#[cfg(feature = "dash")]
impl PcdAsyncReader {
    pub fn new(rx: Receiver<PointCloud<PointXyzRgba>>, tx: UnboundedSender<FrameRequest>) -> Self {
        Self {
            current_frame: 0,
            next_to_get: 0,
            rx,
            tx,
            total_frames: 30, // default number of frames
        }
    }
}

#[cfg(feature = "dash")]
impl RenderReader<PointCloud<PointXyzRgba>> for PcdAsyncReader {
    fn start(&mut self) -> Option<PointCloud<PointXyzRgba>> {
        for i in 0..9 {
            self.tx
                .send(FrameRequest {
                    object_id: 0,
                    quality: 0,
                    frame_offset: i,
                })
                .unwrap();
        }
        self.next_to_get = 9;
        loop {
            if let Some(data) = self.get_at(0) {
                self.current_frame += 1;
                break Some(data);
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }

    fn get_at(&mut self, index: usize) -> Option<PointCloud<PointXyzRgba>> {
        println!("get_at called with {}", index);

        if self.next_to_get - self.current_frame < 10 {
            self.tx
                .send(FrameRequest {
                    object_id: 0u8,
                    quality: 0u8,
                    frame_offset: self.next_to_get as u64,
                })
                .unwrap();
            self.next_to_get = (self.next_to_get + 1) % (self.len() as u64);
        }

        println!("get_at returned... {}", index);
        if let Ok(data) = self.rx.try_recv() {
            self.current_frame = (self.current_frame + 1) % (self.len() as u64);
            Some(data)
        } else {
            None
        }
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
