use crate::pointcloud::PointCloud;
use std::path::PathBuf;

pub struct Ply {
    title: Option<PathBuf>,
    points: PointCloud,
}

impl Ply {
    pub fn of(title: Option<PathBuf>, points: PointCloud) -> Self {
        Ply { title, points }
    }

    pub fn get_points(self) -> PointCloud {
        self.points
    }

    pub fn get_points_as_ref(&self) -> &PointCloud {
        &self.points
    }

    pub fn get_title(&self) -> Option<&str> {
        self.title
            .as_ref()
            .map(|title| title.file_name().unwrap().to_str())
            .flatten()
    }

    pub fn nothing() -> Self {
        Ply {
            title: None,
            points: PointCloud::default(),
        }
    }
}
