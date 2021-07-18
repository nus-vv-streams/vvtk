use std::sync::{ Arc };
use std::sync::mpsc::channel;
use std::path::PathBuf;

use crate::renderer;
use crate::ply_file::PlyFile;


pub struct PlyDir {
    paths: Vec<PathBuf>
}

impl PlyDir {
    pub fn new(path: &str) -> Self {
        let mut entries = std::fs::read_dir(path).unwrap()
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, std::io::Error>>().unwrap();

        entries.sort();
        
        PlyDir { paths: entries }
    }

    pub fn count(&self) -> usize {
        self.paths.len()
    }

    pub fn play(self) {
        let len = self.count();
        let paths = Arc::new(self.paths);
        
        let (tx, rx) = channel();
        let (paths_clone, tx) = (paths.clone(), tx.clone());
        
        std::thread::spawn(move || {
            let mut index: usize = 0;
            loop {
                index += 1;
                let frame = PlyFile::new(paths_clone[index % len].as_path().to_str().unwrap()).unwrap().read();
                tx.send(frame).unwrap();
            }
        });


        let mut renderer = renderer::Renderer::new();

        let mut frame;

        while renderer.rendering() {
            frame = rx.recv().unwrap();
            renderer.render_frame(&frame);
        }
    }

    // pub fn write_to_ble(self, new_dir: &str) {
    //     std::fs::create_dir(new_dir).unwrap();

    //     for entry in self.paths {
    //         PlyFile::write_to_ble(entry, new_dir).unwrap();
    //     }
    // }
}