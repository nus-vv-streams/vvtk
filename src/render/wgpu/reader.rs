use crate::formats::pointxyzrgba::PointXyzRgba;
use crate::formats::PointCloud;
use crate::pcd::read_pcd_file;
use crate::BufMsg;

use log::debug;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use tokio::sync::mpsc::UnboundedSender;

use super::camera::CameraPosition;
use super::renderable::Renderable;

pub trait RenderReader<T: Renderable> {
    /// Initialize the input reader for our renderer. Returns the first frame, if any.
    fn start(&mut self) -> (Option<CameraPosition>, Option<T>);
    /// Returns the optional new camera position requested by the player backend and the `index`-th frame given the current camera position
    fn get_at(
        &mut self,
        index: usize,
        camera_pos: Option<CameraPosition>,
    ) -> (Option<CameraPosition>, Option<T>);
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
    fn start(&mut self) -> (Option<CameraPosition>, Option<PointCloud<PointXyzRgba>>) {
        self.get_at(0, None)
    }

    fn get_at(
        &mut self,
        index: usize,
        _camera_pos: Option<CameraPosition>,
    ) -> (Option<CameraPosition>, Option<PointCloud<PointXyzRgba>>) {
        (
            None,
            self.files
                .get(index)
                .and_then(|f| read_pcd_file(f).ok())
                .map(PointCloud::from),
        )
    }

    fn len(&self) -> usize {
        self.files.len()
    }

    fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    fn set_len(&mut self, _len: usize) {}
}

#[cfg(feature = "dash")]
pub struct PcdAsyncReader {
    total_frames: u64,
    rx: Receiver<(FrameRequest, PointCloud<PointXyzRgba>)>,
    tx: UnboundedSender<BufMsg>,
}

#[cfg(feature = "dash")]
#[derive(Debug, Clone, Copy)]
/// A request to the player backend for a frame to be displayed by the renderer.
pub struct FrameRequest {
    pub object_id: u8,
    /// Frame offset from the start of the video.
    ///
    /// To get the frame number, add the offset to the frame number of the first frame in the video.
    pub frame_offset: u64,
    /// The camera position when the frame was requested.
    pub camera_pos: Option<CameraPosition>,
}

impl PartialEq for FrameRequest {
    fn eq(&self, other: &Self) -> bool {
        self.object_id == other.object_id && self.frame_offset == other.frame_offset
    }
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
            rx,
            tx,
            // buffer_size,
            // cache: HashMap::with_capacity(buffer_size as usize),
            total_frames: 30, // default number of frames. Use `set_len` to overwrite this value
        }
    }
}

#[cfg(feature = "dash")]
impl RenderReader<PointCloud<PointXyzRgba>> for PcdAsyncReader {
    fn start(&mut self) -> (Option<CameraPosition>, Option<PointCloud<PointXyzRgba>>) {
        self.get_at(0, None)
    }

    fn get_at(
        &mut self,
        index: usize,
        camera_pos: Option<CameraPosition>,
    ) -> (Option<CameraPosition>, Option<PointCloud<PointXyzRgba>>) {
        let index = index as u64;
        _ = self.tx.send(BufMsg::FrameRequest(FrameRequest {
            object_id: 0,
            frame_offset: index % self.total_frames,
            camera_pos,
        }));
        if let Ok((frame_req, pc)) = self.rx.recv() {
            (frame_req.camera_pos, Some(pc))
        } else {
            (None, None)
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
