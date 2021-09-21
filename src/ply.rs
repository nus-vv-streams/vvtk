use crate::points::Points;
use std::path::PathBuf;

pub struct Ply {
    title: Option<PathBuf>,
    points: Points,
}

impl Ply {
    pub fn of(title: Option<PathBuf>, points: Points) -> Self {
        Ply { title, points }
    }

    pub fn get_points(self) -> Points {
        self.points
    }

    pub fn get_points_as_ref(&self) -> &Points {
        &self.points
    }

    pub fn get_title(&self) -> Option<&str> {
        self.title
            .as_ref()
            .map(|title| title.file_name().unwrap().to_str())
            .flatten()
    }

    pub fn nothing() -> Self {
        Ply { title: None, points: Points::default()}
    }
}
