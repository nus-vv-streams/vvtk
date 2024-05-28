use crate::formats::pointxyzrgba::PointXyzRgba;
use crate::formats::PointCloud;
use crate::pcd::read_pcd_file;
use crate::utils::{read_file_to_point_cloud, read_files_to_point_cloud};
use crate::BufMsg;

// use crate::utils::read_file_to_point_cloud;
use std::collections::VecDeque;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::sync::mpsc::Receiver;
use tokio::sync::mpsc::UnboundedSender;

use super::camera::{CameraPosition, CameraState};
use super::renderable::Renderable;

// RenderReader for the original RenderReader used by vvplay
pub trait RenderReader<T: Renderable> {
    fn start(&mut self) -> Option<T>;
    fn get_at(&mut self, index: usize) -> Option<T>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn set_len(&mut self, len: usize);
}
// RenderReaderCameraPos for the one with CameraPosition
pub trait RenderReaderCameraPos<T: Renderable> {
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
    fn set_cache_size(&mut self, size: usize);
    fn set_camera_state(&mut self, camera_state: Option<CameraState>);
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
    fn start(&mut self) -> Option<PointCloud<PointXyzRgba>> {
        RenderReader::get_at(self, 0)
    }

    fn get_at(&mut self, index: usize) -> Option<PointCloud<PointXyzRgba>> {
        let file_path = self.files.get(index)?;
        read_file_to_point_cloud(file_path)
    }

    fn len(&self) -> usize {
        self.files.len()
    }

    fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    fn set_len(&mut self, _len: usize) {}
}

impl RenderReaderCameraPos<PointCloud<PointXyzRgba>> for PointCloudFileReader {
    fn start(&mut self) -> (Option<CameraPosition>, Option<PointCloud<PointXyzRgba>>) {
        RenderReaderCameraPos::get_at(self, 0, Some(CameraPosition::default()))
    }

    fn get_at(
        &mut self,
        index: usize,
        _camera_pos: Option<CameraPosition>,
    ) -> (Option<CameraPosition>, Option<PointCloud<PointXyzRgba>>) {
        let file_path = self.files.get(index).unwrap();
        (
            Some(CameraPosition::default()),
            read_file_to_point_cloud(file_path),
        )
    }

    fn len(&self) -> usize {
        self.files.len()
    }

    fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    fn set_len(&mut self, _len: usize) {}

    fn set_cache_size(&mut self, _size: usize) {}

    fn set_camera_state(&mut self, _camera_state: Option<CameraState>) {}
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

pub struct LODFileReader {
    base_files: Vec<PathBuf>,
    additional_files: Option<Vec<Vec<PathBuf>>>,
}

impl LODFileReader {
    pub fn new(base_dir: &Path, additional_dirs: Option<Vec<&Path>>, file_type: &str) -> Self {
        let base_files = Self::from_directory(base_dir, file_type);

        if additional_dirs.is_none() {
            return Self {
                base_files,
                additional_files: None,
            };
        }

        let additional_files = additional_dirs
            .unwrap()
            .iter()
            .map(|dir| Self::from_directory(dir, file_type))
            .collect::<Vec<_>>();

        let len = base_files.len();
        for reader in additional_files.iter() {
            if reader.len() != len {
                eprintln!("All readers must have the same length");
                exit(1);
            }
        }

        Self {
            base_files,
            additional_files: Some(additional_files),
        }
    }

    fn from_directory(directory: &Path, file_type: &str) -> Vec<PathBuf> {
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
        files
    }

    /// Get the point point cloud at the given index with additional points at the given indices.
    pub fn get_with_additional_at(
        &self,
        index: usize,
        additional_points: &Vec<usize>,
    ) -> Option<PointCloud<PointXyzRgba>> {
        let base_file = self.base_files.get(index)?;
        let additional_files = self
            .additional_files
            .as_ref()?
            .iter()
            .map(|reader| reader.get(index).unwrap())
            .collect::<Vec<_>>();
        read_files_to_point_cloud(base_file, &additional_files, additional_points)
    }
}

impl RenderReader<PointCloud<PointXyzRgba>> for LODFileReader {
    fn start(&mut self) -> Option<PointCloud<PointXyzRgba>> {
        self.get_at(0)
    }

    fn get_at(&mut self, index: usize) -> Option<PointCloud<PointXyzRgba>> {
        let file_path = self.base_files.get(index)?;
        read_file_to_point_cloud(file_path)
    }

    fn len(&self) -> usize {
        self.base_files.len()
    }

    fn is_empty(&self) -> bool {
        self.base_files.is_empty()
    }

    fn set_len(&mut self, _len: usize) {}
}

pub struct PcdAsyncReader {
    total_frames: u64,
    rx: Receiver<(FrameRequest, PointCloud<PointXyzRgba>)>,
    //playback cache
    cache: VecDeque<(u64, PointCloud<PointXyzRgba>)>,
    cache_size: usize,
    tx: UnboundedSender<BufMsg>,
}

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
            cache: VecDeque::new(),
            cache_size: 10, //default number of size, Use `set_cache_size` to overwrite this value
            total_frames: 30, // default number of frames. Use `set_len` to overwrite this value
        }
    }
}

impl RenderReaderCameraPos<PointCloud<PointXyzRgba>> for PcdAsyncReader {
    fn start(&mut self) -> (Option<CameraPosition>, Option<PointCloud<PointXyzRgba>>) {
        RenderReaderCameraPos::get_at(self, 0, Some(CameraPosition::default()))
    }

    fn get_at(
        &mut self,
        index: usize,
        camera_pos: Option<CameraPosition>,
    ) -> (Option<CameraPosition>, Option<PointCloud<PointXyzRgba>>) {
        println!("----------------------------------");
        println! {"get at request index: {}", index};
        let index = index as u64;
        if let Some(&ref result) = self.cache.iter().find(|&i| i.0 == index) {
            //if the result is already inside the cache, just return
            //can improve this O(size) find algorithm
            return (camera_pos, Some(result.1.clone()));
        }
        _ = self.tx.send(BufMsg::FrameRequest(FrameRequest {
            object_id: 0,
            frame_offset: index % self.total_frames,
            camera_pos,
        }));
        if let Ok((frame_req, pc)) = self.rx.recv() {
            if self.cache.len() >= self.cache_size {
                self.cache.pop_front();
            }
            //println!(
            //    "one frame is added to the point cloud cache: index:{}",
            //    index
            //);
            self.cache.push_back((index, pc.clone()));
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

    fn set_cache_size(&mut self, size: usize) {
        self.cache_size = size;
    }

    fn set_camera_state(&mut self, _camera_state: Option<CameraState>) {}
}

// This is used by vvplay_async
impl RenderReader<PointCloud<PointXyzRgba>> for PcdAsyncReader {
    fn start(&mut self) -> Option<PointCloud<PointXyzRgba>> {
        RenderReader::get_at(self, 0)
    }

    fn get_at(&mut self, index: usize) -> Option<PointCloud<PointXyzRgba>> {
        let index = index as u64;
        // Everytime a request is made, find it from the playback cache first
        if let Some(&ref result) = self.cache.iter().find(|&i| i.0 == index) {
            // Enhancement: can improve this O(n) find algorithm in future
            return Some(result.1.clone());
        }
        // Send request to prepare for the frame
        _ = self.tx.send(BufMsg::FrameRequest(FrameRequest {
            object_id: 0,
            frame_offset: index % self.total_frames,
            camera_pos: Some(CameraPosition::default()),
        }));
        // Wait for the point cloud to be ready, cache it then return
        if let Ok((_frame_req, pc)) = self.rx.recv() {
            if self.cache.len() >= 10 {
                self.cache.pop_front();
            }
            println!(
                "one frame is added to the point cloud cache: index:{}",
                index
            );
            self.cache.push_back((index, pc.clone()));
            Some(pc)
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

// !! BufRenderReader is not used and comments are deleted.
