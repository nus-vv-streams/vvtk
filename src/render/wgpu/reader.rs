use std::path::{Path, PathBuf};
use crate::formats::PointCloud;
use crate::formats::pointxyzrgba::PointXyzRgba;
use crate::pcd::{PointCloudData, read_pcd_file};
use crate::render::wgpu::AntiAlias;
use crate::render::wgpu::renderable::Renderable;

pub trait RenderReader<T: Renderable> {
    fn get_at(&self, index: usize) -> Option<T>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn antialias(&self) -> AntiAlias {
        AntiAlias::new(1.0, 1.0, 1.0)
    }
}

pub struct PcdFileReader {
    files: Vec<PathBuf>
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
                },
                Err(e) => {
                    eprintln!("{e}")
                }
            }
        }
        files.sort();
        Self {
            files
        }
    }
}

impl RenderReader<PointCloudData> for PcdFileReader {
    fn get_at(&self, index: usize) -> Option<PointCloudData> {
        self.files.get(index)
            .and_then(|f| read_pcd_file(f).ok())
    }

    fn len(&self) -> usize {
        self.files.len()
    }

    fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    fn antialias(&self) -> AntiAlias {
        let pcd = self.get_at(0).unwrap();
        let pointcloud: PointCloud<PointXyzRgba> = pcd.into();
        let first_point = pointcloud.points.get(0).unwrap();
        let mut max_x = first_point.x;
        let mut max_y = first_point.y;
        let mut max_z = first_point.z;

        for point in pointcloud.points {
            max_x = max_x.max(point.x.abs());
            max_y = max_y.max(point.y.abs());
            max_z = max_z.max(point.z.abs());
        }
        let max = max_x.max(max_y).max(max_z);
        AntiAlias::new(max, max, max)
    }
}