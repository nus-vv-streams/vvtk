use std::path::{Path, PathBuf};
use crate::pcd::{PointCloudData, read_pcd_file};
use crate::render::wgpu::renderer::Renderable;

pub trait RenderReader<T: Renderable> {
    fn get_at(&self, index: usize) -> Option<T>;
    fn len(&self) -> usize;
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
                    println!("{e}")
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
}