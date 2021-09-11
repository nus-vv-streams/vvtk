use nalgebra::Point3;
use std::path::Path;
use std::sync::mpsc::channel;
use std::sync::Arc;

use crate::tool::{reader, renderer};

/// Structure representing a directory containing ply files
pub struct PlyDir {
    paths: Vec<Box<Path>>,
}

impl PlyDir {
    /// Creating a new `PlyDir`
    pub fn new(path: &str) -> Self {
        let mut entries = std::fs::read_dir(path)
            .unwrap()
            .map(|res| res.map(|e| e.path().into_boxed_path()))
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
    pub fn play(self) -> Result<(), std::io::Error> {
        self.play_with_camera(None, None, None)
    }

    /// Open the window and play 3D video with specific camera
    pub fn play_with_camera(
        self,
        eye: Option<Point3<f32>>,
        at: Option<Point3<f32>>,
        background_color: Option<Point3<f32>>,
    ) -> Result<(), std::io::Error> {
        let len = self.count();
        let paths = Arc::new(self.paths);

        let (tx, rx) = channel();
        let (paths_clone, tx) = (paths, tx);

        std::thread::spawn(move || {
            let mut index: usize = 0;
            loop {
                index += 1;
                let frame = reader::read(paths_clone[index % len].to_str());
                tx.send(frame).unwrap();
            }
        });

        let mut renderer = renderer::Renderer::new(None);

        renderer.config_camera(eye, at);

        renderer.config_background_color(background_color);

        let mut frame;

        while renderer.render() {
            frame = rx.recv().unwrap();
            match frame {
                Ok(f) => {
                    renderer.render_frame(&f);
                }
                Err(e) => {
                    eprintln!("Problem with reading file:\n    {}", e);
                    continue;
                }
            }
        }

        Ok(())
    }

    // pub fn write_to_ble(self, new_dir: &str) {
    //     std::fs::create_dir(new_dir).unwrap();

    //     for entry in self.paths {
    //         PlyFile::write_to_ble(entry, new_dir).unwrap();
    //     }
    // }
}
