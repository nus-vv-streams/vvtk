use crate::formats::metadata::MetaData;
use crate::formats::pointxyzrgba::PointXyzRgba;
use crate::formats::PointCloud;
use crate::pcd::{read_pcd_header, PCDHeader};
use std::marker::PhantomData;
use std::path::Path;
use std::process::exit;

use super::camera::CameraState;
use super::reader::{PointCloudFileReader, RenderReader};
use super::renderable::Renderable;
use super::resolution_controller::ResolutionController;

pub trait RenderManager<T: Renderable> {
    fn start(&mut self) -> Option<T>;
    fn get_at(&mut self, index: usize) -> Option<T>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn set_len(&mut self, len: usize);
    fn set_camera_state(&mut self, camera_state: Option<CameraState>);
}

pub struct AdaptiveManager {
    base_reader: PointCloudFileReader,
    // each additional reader handles a different segment
    additional_readers: Option<Vec<PointCloudFileReader>>,
    camera_state: Option<CameraState>,
    resolution_controller: Option<ResolutionController>,
}

fn infer_format(src: &String) -> String {
    let choices = ["pcd", "ply", "bin", "http"];
    const PCD: usize = 0;
    const PLY: usize = 1;
    const BIN: usize = 2;

    if choices.contains(&src.as_str()) {
        return src.clone();
    }

    let path = Path::new(src);
    // infer by counting extension numbers (pcd ply and bin)

    let mut choice_count = [0, 0, 0];
    for file_entry in path.read_dir().unwrap() {
        match file_entry {
            Ok(entry) => {
                if let Some(ext) = entry.path().extension() {
                    if ext.eq("pcd") {
                        choice_count[PCD] += 1;
                    } else if ext.eq("ply") {
                        choice_count[PLY] += 1;
                    } else if ext.eq("bin") {
                        choice_count[BIN] += 1;
                    }
                }
            }
            Err(e) => {
                eprintln!("{e}")
            }
        }
    }

    let max_index = choice_count
        .iter()
        .enumerate()
        .max_by_key(|(_, &item)| item)
        .map(|(index, _)| index);
    choices[max_index.unwrap()].to_string()
}

impl AdaptiveManager {
    pub fn new(src: &String, lod: bool) -> Self {
        let base_path = if lod {
            src.clone() + "/base"
        } else {
            src.clone()
        };

        let play_format = infer_format(&base_path);
        let base_path = Path::new(&base_path);
        let mut base_reader = PointCloudFileReader::from_directory(base_path, &play_format);

        if base_reader.is_empty() {
            eprintln!("Must provide at least one file!");
            exit(1);
        }

        if lod {
            let metadata_path = Path::new(&src).join("metadata.json");
            let metadata: MetaData = if metadata_path.exists() {
                let data = std::fs::read_to_string(metadata_path).unwrap();
                serde_json::from_str(&data).unwrap()
            } else {
                eprintln!("Must provide metafile for LOD mode!");
                exit(1);
            };

            let additional_readers =
                (0..metadata.partitions.0 * metadata.partitions.1 * metadata.partitions.2)
                    .map(|i| {
                        let path = Path::new(&src).join(i.to_string());
                        PointCloudFileReader::from_directory(&path, &play_format)
                    })
                    .collect::<Vec<_>>();

            let len = base_reader.len();
            for reader in additional_readers.iter() {
                if reader.len() != len {
                    eprintln!("All readers must have the same length");
                    exit(1);
                }
            }

            let anchor_point_cloud = base_reader.start().unwrap();
            let resolution_controller = ResolutionController::new(
                &anchor_point_cloud.points,
                Some(metadata),
                anchor_point_cloud.antialias(),
            );

            Self {
                base_reader,
                additional_readers: Some(additional_readers),
                camera_state: None,
                resolution_controller: Some(resolution_controller),
            }
        } else {
            Self {
                base_reader,
                additional_readers: None,
                camera_state: None,
                resolution_controller: None,
            }
        }
    }

    pub fn get_desired_point_cloud(&mut self, index: usize) -> Option<PointCloud<PointXyzRgba>> {
        let base_pc = self.base_reader.get_at(index).unwrap();

        if self.additional_readers.is_none()
            || self.camera_state.is_none()
            || self.resolution_controller.is_none()
            || self.additional_readers.is_none()
        {
            return Some(base_pc);
        }

        let additional_num_points_desired = self
            .resolution_controller
            .as_mut()
            .unwrap()
            .get_desired_num_points(index, self.camera_state.as_ref().unwrap(), true);

        let mut header = read_pcd_header(self.base_reader.get_path_at(index).unwrap()).unwrap();
        let additional_points_required = additional_num_points_desired
            .iter()
            .enumerate()
            .map(|(segment, &num)| self.read_more_points(index, &mut header, num, segment))
            .collect::<Vec<_>>()
            .concat();

        let new_pc = base_pc.merge_points(additional_points_required);

        Some(new_pc)
    }

    pub fn read_more_points(
        &self,
        index: usize,
        header: &mut PCDHeader,
        num_of_points: usize,
        segment: usize,
    ) -> Vec<PointXyzRgba> {
        if num_of_points <= 0 {
            vec![]
        } else {
            header.set_points(num_of_points as u64);

            let pc = self
                .additional_readers
                .as_ref()
                .unwrap()
                .get(segment)
                .unwrap()
                .get_with_header_at(index, header.clone())
                .unwrap();

            pc.points
        }
    }

    pub fn len(&self) -> usize {
        self.base_reader.len()
    }
}

impl RenderManager<PointCloud<PointXyzRgba>> for AdaptiveManager {
    fn start(&mut self) -> Option<PointCloud<PointXyzRgba>> {
        self.get_desired_point_cloud(0)
    }

    fn get_at(&mut self, index: usize) -> Option<PointCloud<PointXyzRgba>> {
        self.get_desired_point_cloud(index)
    }
    fn len(&self) -> usize {
        self.base_reader.len()
    }

    fn is_empty(&self) -> bool {
        self.base_reader.is_empty()
    }

    fn set_len(&mut self, len: usize) {
        self.base_reader.set_len(len);
    }

    fn set_camera_state(&mut self, camera_state: Option<CameraState>) {
        self.camera_state = camera_state;
    }
}

/// Dummy wrapper for RenderReader
pub struct RenderReaderWrapper<T, U>
where
    T: RenderReader<U>,
    U: Renderable,
{
    reader: T,
    _data: PhantomData<U>,
}

impl<T, U> RenderReaderWrapper<T, U>
where
    T: RenderReader<U>,
    U: Renderable,
{
    pub fn new(reader: T) -> Self {
        Self {
            reader,
            _data: PhantomData,
        }
    }
}

impl<T, U> RenderManager<U> for RenderReaderWrapper<T, U>
where
    T: RenderReader<U>,
    U: Renderable,
{
    fn start(&mut self) -> Option<U> {
        self.reader.start()
    }

    fn get_at(&mut self, index: usize) -> Option<U> {
        self.reader.get_at(index)
    }

    fn len(&self) -> usize {
        self.reader.len()
    }

    fn is_empty(&self) -> bool {
        self.reader.is_empty()
    }

    fn set_len(&mut self, len: usize) {
        self.reader.set_len(len);
    }

    fn set_camera_state(&mut self, _camera_state: Option<CameraState>) {}
}
