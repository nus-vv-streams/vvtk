use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use wgpu_glyph::ab_glyph::Point;

use cgmath::*;
use crate::formats::metadata::MetaData;
use crate::formats::pointxyzrgba::PointXyzRgba;
use crate::formats::PointCloud;
use crate::render::wgpu::antialias;
use std::marker::PhantomData;
use std::path::Path;
use std::process::exit;
use std::time::{Duration, Instant};

use super::camera::CameraState;
use super::reader::{LODFileReader, RenderReader};
use super::renderable::Renderable;
use super::resolution_controller::ResolutionController;
use super::upsampler::Upsampler;

pub trait RenderManager<T: Renderable> {
    fn start(&mut self) -> Option<T>;
    fn get_at(&mut self, index: usize) -> Option<T>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn set_len(&mut self, len: usize);
    fn set_camera_state(&mut self, camera_state: Option<CameraState>);
    fn should_redraw(&mut self, camera_state: &CameraState) -> bool;
    fn get_visible_points(&self, point_cloud: PointCloud<PointXyzRgba>) -> PointCloud<PointXyzRgba>;
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

    // For upsampling
    upsampler: Upsampler,
    pc: Option<PointCloud<PointXyzRgba>>,

    total_latency: Duration,
    sample_size: i32,
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
                pc: None,
                upsampler: Upsampler {  },
                reader,
                camera_state: None,
                resolution_controller: Some(resolution_controller),
                metadata: Some(metadata),
                current_index: usize::MAX, // no point cloud loaded yet
                additional_points_loaded,
                total_latency: Duration::new(0, 0),
                sample_size: 0,
                
            }
        } else {
            let reader = LODFileReader::new(base_path, None, &play_format);

            if reader.is_empty() {
                eprintln!("Must provide at least one file!");
                exit(1);
            }

            Self {
                pc: None,
                upsampler: Upsampler {  },
                reader,
                camera_state: None,
                resolution_controller: None,
                metadata: None,
                current_index: usize::MAX,
                additional_points_loaded: vec![],
                total_latency: Duration::new(0, 0),
                sample_size: 0,
            }
        }
    }

    pub fn get_desired_point_cloud(&mut self, index: usize) -> Option<PointCloud<PointXyzRgba>> {
        // let now = std::time::Instant::now();

        if self.metadata.is_none() {
            // println!("get base pc: {:?}", now.elapsed());
            let pc = self.reader.get_at(index).unwrap();
            return Some(pc);
        }

        let metadata = self.metadata.as_ref().unwrap();
        let base_point_num = metadata.base_point_num.get(index).unwrap();
        let bound = metadata.bounds.get(index).unwrap().clone();

        if self.camera_state.is_none() || self.resolution_controller.is_none() {
            let mut pc = self.reader.get_at(index).unwrap();
            pc.self_segment(base_point_num, &bound.partition(metadata.partitions));
            return Some(pc);
        }

        let additional_num_points_desired = self
            .resolution_controller
            .as_mut()
            .unwrap()
            .get_desired_num_points(index, self.camera_state.as_ref().unwrap());

        self.current_index = index;
        self.additional_points_loaded = additional_num_points_desired;

        let extra_point_num = metadata.additional_point_num.get(index).unwrap();
        let to_load = self
            .additional_points_loaded
            .iter()
            .enumerate()
            .map(|(segment, &num)| (num - base_point_num[segment]).min(extra_point_num[segment]))
            .collect::<Vec<_>>();

        let mut pc = self.reader.get_with_additional_at(index, &to_load).unwrap();

        let mut offsets = base_point_num.clone();
        offsets.extend(&to_load);

        let mut bound_indices = (0..base_point_num.len()).collect::<Vec<_>>();
        bound_indices.extend((0..to_load.len()).collect::<Vec<_>>());

        pc.self_segment_with_bound_indices(
            &offsets,
            &bound_indices,
            &bound.partition(metadata.partitions),
        );

        Some(pc)
    }

    fn should_load_more_points(&mut self, camera_state: &CameraState) -> bool {
        if self.metadata.is_none()
            || self.camera_state.is_none()
            || self.resolution_controller.is_none()
        {
            return false;
        }

        if self.current_index > self.reader.len() {
            return true;
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
        // println!("RenderManager get_at: {:?}", index);

        if index != self.current_index || self.pc.is_none() {
            // println!("Loading point cloud at index: {:?}, {:?} {:?}", index, self.current_index, self.pc.is_none());
            self.pc = Some(self.get_desired_point_cloud(index)?);
            self.current_index = index;
        }

        let pc = self.pc.as_ref().unwrap();
        let start = Instant::now();
        let mut visible_pc = self.get_visible_points(pc.clone());
        // let visibility_elasped = start.elapsed();
        // println!("Total points {:?}, Visible points {:?}, took {:?}", pc.points.len(), visible_pc.points.len(), visibility_elasped);

        let should_upsample = self.upsampler.should_upsample(&visible_pc, &self.camera_state.as_ref().unwrap());

        if should_upsample {
            let init_len = visible_pc.points.len();

            let upsampled_points = self.upsampler.upsample_grid(&visible_pc, 7);
            let upsampled_pc = PointCloud::new(upsampled_points.len(), upsampled_points.clone());
            self.pc.as_mut().unwrap().combine(&upsampled_pc);

            visible_pc.combine(&upsampled_pc);

            let upsample_elasped: Duration = start.elapsed();

            println!("Upsampled points from {:?} to {:?} in {:?}", init_len, visible_pc.points.len(), upsample_elasped);
        }
        Some(visible_pc)
        // println!("Point visibility took: {:?}", start.elapsed());
        // self.total_latency += start.elapsed();
        // self.sample_size += 1;
        // println!("Average Point visibility took: {:?}", self.total_latency / self.sample_size.try_into().unwrap());

    }

    fn get_visible_points(&self, point_cloud: PointCloud<PointXyzRgba>) -> PointCloud<PointXyzRgba> {
        // println!("Number of points total: {:?}", point_cloud.points.len());
        let view_proj_matrix = Matrix4::from(self.camera_state.as_ref().unwrap().camera_uniform.view_proj);
        let antialias = point_cloud.antialias();
        let visible_points = point_cloud.points.into_par_iter().filter(|point| {
            let point_vec = Point3::new(point.x - antialias.x, point.y - antialias.y, point.z - antialias.z) / antialias.scale;
            let point_in_view = view_proj_matrix.transform_point(point_vec);

            point_in_view.x.abs() <= 1.0 &&
            point_in_view.y.abs() <= 1.0 &&
            point_in_view.z.abs() <= 1.0
        }).collect::<Vec<_>>();
        // println!("Number of points visible: {:?}", visible_points.len());
        PointCloud::new(visible_points.len(), visible_points)
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
        true
        // self.should_load_more_points(camera_state)
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

    fn get_visible_points(&self, point_cloud: PointCloud<PointXyzRgba>) -> PointCloud<PointXyzRgba> {
        PointCloud::new(0, vec![])
    }
}
