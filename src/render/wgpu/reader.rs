use crate::formats::pointxyzrgba::PointXyzRgba;
use crate::formats::PointCloud;
use crate::pcd::read_pcd_file;
use crate::BufMsg;

use crate::utils::read_file_to_point_cloud;
// use std::collections::HashMap;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use tokio::sync::mpsc::UnboundedSender;

use super::camera::CameraPosition;
use super::renderable::Renderable;

pub trait RenderReaderLegacy<T: Renderable> {
    fn start(&mut self) -> Option<T>;
    fn get_at(&mut self, index: usize) -> Option<T>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn set_len(&mut self, len: usize);
}

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
                        //t: only add in .pcd file
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

pub struct PointCloudFileReader {
    files: Vec<PathBuf>,
}

impl PointCloudFileReader {
    pub fn from_directory(directory: &Path, file_type: &str) -> Self {
        let mut files = vec![];
        for file_entry in directory.read_dir().unwrap() {
            match file_entry {
                Ok(entry) => {
                    if let Some(ext) = entry.path().extension() {
                        if ext.eq(file_type) {
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
}

impl RenderReader<PointCloud<PointXyzRgba>> for PointCloudFileReader {
    fn start(&mut self) -> (Option<CameraPosition>, Option<PointCloud<PointXyzRgba>>) {
        self.get_at(0, None)
    }

    fn get_at(
        &mut self,
        index: usize,
        _camera_pos: Option<CameraPosition>,
    ) -> (Option<CameraPosition>, Option<PointCloud<PointXyzRgba>>) {
        let file_path = self.files.get(index).unwrap();
        (None, read_file_to_point_cloud(file_path))
    }

    fn len(&self) -> usize {
        self.files.len()
    }

    fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    fn set_len(&mut self, _len: usize) {}
}

impl RenderReaderLegacy<PointCloud<PointXyzRgba>> for PcdFileReader {
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
    //t: not used yet?
    points: Vec<PointCloud<PointXyzRgba>>,
}

impl PcdMemoryReader {
    pub fn from_vec(points: Vec<PointCloud<PointXyzRgba>>) -> Self {
        Self { points }
    }
}

impl RenderReaderLegacy<PointCloud<PointXyzRgba>> for PcdMemoryReader {
    fn get_at(&mut self, index: usize) -> Option<PointCloud<PointXyzRgba>> {
        self.points.get(index).cloned()
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
    total_frames: u64,
    rx: Receiver<(FrameRequest, PointCloud<PointXyzRgba>)>,
    //t: add cache to store rendered point cloud
    cache: Vec<(u64, PointCloud<PointXyzRgba>)>,
    //t: the request is sent from here all the time
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
        // buffer_size: Option<u8>,rame requst id: {}, offset: {}", new_key.object_id, new_key.frame_offsei
    ) -> Self {
        Self {
            rx,
            tx,
            // buffer_size,
            // cache: HashMap::with_capacity(buffer_size as usize),
            //t: add the cache here, might need to change to a better data structure if needed
            cache: vec![],
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
        println!{"get at request index: {}", index};
        let index = index as u64;
        if let Some(&ref result) = self.cache.iter().find(|&i| i.0 == index) {
            //it: f the result is already inside the cache, just return
            //can improve this find algorithm
            return (camera_pos, Some(result.1.clone()));
        }
        //t: if it is not found, send the frame request to vvplay async there
        _ = self.tx.send(BufMsg::FrameRequest(FrameRequest {
            object_id: 0,
            frame_offset: index % self.total_frames,
            camera_pos,
        }));
        //t: if there is any, send the camera pos and the point cloud, to modify, can manually set up the camera pos here
        //t: if receive something from the rx channel, cache it then return
        if let Ok((frame_req, pc)) = self.rx.recv() {
            //t: cache the result here first
            //t: the cache has size 10 for now
            if self.cache.len() >= 10 {
                self.cache.pop();
            }
            println!("one frame is added to the point cloud cache: index:{}", index);
            self.cache.push((index, pc.clone()));
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

// !! BufRenderReader is not used and comments are deleted.
