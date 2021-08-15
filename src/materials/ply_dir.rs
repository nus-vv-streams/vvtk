use nalgebra::Point3;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::Arc;

use crate::tool::{reader, renderer};

/// Structure representing a directory containing ply files
pub struct PlyDir {
    paths: Vec<PathBuf>,
}

impl PlyDir {
    /// Creating a new `PlyDir`
    pub fn new(path: &str) -> Self {
        let mut entries = std::fs::read_dir(path)
            .unwrap()
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, std::io::Error>>()
            .unwrap();

        entries.sort();

        PlyDir { paths: entries }
    }

    /// Return number of ply files
    pub fn count(&self) -> usize {
        self.paths.len()
    }

    /// Open the window and play 3D video
    pub fn play(self) {
        self.play_with_camera(None, None);
    }

    /// Open the window and play 3D video with specific camera
    pub fn play_with_camera(self, eye: Option<Point3<f32>>, at: Option<Point3<f32>>) {
        let len = self.count();
        let paths = Arc::new(self.paths);

        let (tx, rx) = channel();
        let (paths_clone, tx) = (paths, tx);

        std::thread::spawn(move || {
            let mut index: usize = 0;
            loop {
                index += 1;
                let frame = reader::read(paths_clone[index % len].as_path().to_str());
                tx.send(frame).unwrap();
            }
        });

        let mut renderer = renderer::Renderer::new(None);

        renderer.config_camera(eye, at);

        let mut frame;

        while renderer.render() {
            frame = rx.recv().unwrap();
            renderer.render_frame(&frame.expect("Hasagi"));
        }
    }

    // pub fn write_to_ble(self, new_dir: &str) {
    //     std::fs::create_dir(new_dir).unwrap();

    //     for entry in self.paths {
    //         PlyFile::write_to_ble(entry, new_dir).unwrap();
    //     }
    // }
}
