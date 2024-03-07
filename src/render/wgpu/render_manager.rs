use crate::formats::metadata::MetaData;
use crate::formats::pointxyzrgba::PointXyzRgba;
use crate::formats::PointCloud;
use std::marker::PhantomData;
use std::path::Path;
use std::process::exit;

use super::camera::CameraState;
use super::reader::{LODFileReader, RenderReader};
use super::renderable::Renderable;
use super::resolution_controller::ResolutionController;

pub trait RenderManager<T: Renderable> {
    fn start(&mut self) -> Option<T>;
    fn get_at(&mut self, index: usize) -> Option<T>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn set_len(&mut self, len: usize);
    fn set_camera_state(&mut self, camera_state: Option<CameraState>);
    fn should_redraw(&mut self, camera_state: &CameraState) -> bool;
}

pub struct AdaptiveManager {
    reader: LODFileReader,

    // For adaptive loading
    camera_state: Option<CameraState>,
    resolution_controller: Option<ResolutionController>,

    // For segmentation
    metadata: Option<MetaData>,

    // As the temporary cache
    current_index: usize,
    additional_points_loaded: Vec<usize>,
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

        if lod {
            let metadata_path = Path::new(&src).join("metadata.json");
            let metadata: MetaData = if metadata_path.exists() {
                let data = std::fs::read_to_string(metadata_path).unwrap();
                serde_json::from_str(&data).unwrap()
            } else {
                eprintln!("Must provide metafile for LOD mode!");
                exit(1);
            };

            let add_paths =
                (0..metadata.partitions.0 * metadata.partitions.1 * metadata.partitions.2)
                    .map(|i| format!("{}/{}", src, i))
                    .collect::<Vec<_>>();

            let add_dirs = add_paths.iter().map(|s| Path::new(s)).collect::<Vec<_>>();

            let mut reader = LODFileReader::new(base_path, Some(add_dirs), &play_format);

            if reader.is_empty() {
                eprintln!("Must provide at least one file!");
                exit(1);
            }

            let anchor_point_cloud = reader.start().unwrap();
            let resolution_controller = ResolutionController::new(
                &anchor_point_cloud.points,
                Some(metadata.clone()),
                anchor_point_cloud.antialias(),
            );

            // no additional points loaded yet
            let additional_points_loaded = vec![0; reader.len()];

            Self {
                reader,
                camera_state: None,
                resolution_controller: Some(resolution_controller),
                metadata: Some(metadata),
                current_index: usize::MAX, // no point cloud loaded yet
                additional_points_loaded,
            }
        } else {
            let reader = LODFileReader::new(base_path, None, &play_format);

            if reader.is_empty() {
                eprintln!("Must provide at least one file!");
                exit(1);
            }

            Self {
                reader,
                camera_state: None,
                resolution_controller: None,
                metadata: None,
                current_index: usize::MAX,
                additional_points_loaded: vec![],
            }
        }
    }

    pub fn get_desired_point_cloud(&mut self, index: usize) -> Option<PointCloud<PointXyzRgba>> {
        // let now = std::time::Instant::now();
        let mut base_pc = self.reader.get_at(index).unwrap();

        if self.metadata.is_none() {
            // println!("get base pc: {:?}", now.elapsed());
            return Some(base_pc);
        }

        let metadata = self.metadata.as_ref().unwrap();
        let base_point_num = metadata.base_point_num.get(index).unwrap();
        let bound = metadata.bounds.get(index).unwrap().clone();

        base_pc.self_segment(base_point_num, &bound.partition(metadata.partitions));
        // println!("get base pc: {:?}", now.elapsed());

        self.current_index = index;

        if self.camera_state.is_none() || self.resolution_controller.is_none() {
            return Some(base_pc);
        }

        let metadata = self.metadata.as_ref().unwrap();

        let base_point_num = metadata.base_point_num.get(index).unwrap();
        let extra_point_num = metadata.additional_point_num.get(index).unwrap();

        let additional_num_points_desired = self
            .resolution_controller
            .as_mut()
            .unwrap()
            .get_desired_num_points(index, self.camera_state.as_ref().unwrap());

        self.additional_points_loaded = additional_num_points_desired;

        let to_load = self
            .additional_points_loaded
            .iter()
            .enumerate()
            .map(|(segment, &num)| (num - base_point_num[segment]).min(extra_point_num[segment]))
            .collect::<Vec<_>>();

        // println!("to load now: {:?}", now.elapsed());
        // total to be added
        base_pc.prepare_for_addition(&to_load);

        // let mut header = read_pcd_header(self.base_reader.get_path_at(index).unwrap()).unwrap();
        // // println!("original base points: {}", base_pc.number_of_points);
        // // println!("read header: {:?}", now.elapsed());

        // to_load
        //     .iter()
        //     .zip(self.additional_readers.as_ref().unwrap())
        //     .enumerate()
        //     .for_each(|(segment, (&to_read, reader))| {
        //         if to_read > 0 {
        //             header.set_points(to_read as u64);
        //             let pc = reader.get_with_header_at(index, header.clone()).unwrap();
        //             // println!(
        //             //     "Read {} points for segment {} in {:?}",
        //             //     to_read,
        //             //     segment,
        //             //     now.elapsed()
        //             // );

        //             // let now = std::time::Instant::now();

        //             base_pc.add_points(pc.points, segment);
        //             // println!("add points for segment {} in {:?}", segment, now.elapsed());
        //         }
        //     });

        // println!("total points: {}", base_pc.number_of_points);
        // println!("get desired pc: {:?}", now.elapsed());

        Some(base_pc)
    }

    fn should_load_more_points(&mut self, camera_state: &CameraState) -> bool {
        if self.metadata.is_none()
            || self.camera_state.is_none()
            || self.resolution_controller.is_none()
        {
            return false;
        }

        let additional_num_points_desired = self
            .resolution_controller
            .as_mut()
            .unwrap()
            .get_desired_num_points(self.current_index, camera_state);

        // should load more if any of the segments need more points
        additional_num_points_desired
            .iter()
            .enumerate()
            .any(|(segment, &num)| num > self.additional_points_loaded[segment])
    }

    pub fn len(&self) -> usize {
        self.reader.len()
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
        self.reader.len()
    }

    fn is_empty(&self) -> bool {
        self.reader.is_empty()
    }

    fn set_len(&mut self, len: usize) {
        self.reader.set_len(len);
    }

    fn set_camera_state(&mut self, camera_state: Option<CameraState>) {
        self.camera_state = camera_state;
    }

    fn should_redraw(&mut self, camera_state: &CameraState) -> bool {
        self.should_load_more_points(camera_state)
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

    fn should_redraw(&mut self, _camera_state: &CameraState) -> bool {
        false
    }
}
