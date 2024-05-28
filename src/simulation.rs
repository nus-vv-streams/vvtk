use std::{
    cell::RefCell,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};

use cgmath::Point3;
use log::warn;

use crate::render::wgpu::camera::CameraPosition;

pub struct CameraTrace {
    data: Vec<CameraPosition>,
    index: RefCell<usize>,
    path: PathBuf,
}

impl CameraTrace {
    /// The network trace file to contain the network bandwidth in Kbps, each line representing 1 bandwidth sample.
    /// # Arguments
    ///
    /// * `path` - The path to the network trace file.
    pub fn new(path: &Path, is_record: bool) -> Self {
        use std::io::BufRead;
        match File::open(path) {
            Err(err) => {
                if !is_record {
                    panic!("Failed to open camera trace file: {err:?}");
                }
                Self {
                    data: Vec::new(),
                    index: RefCell::new(0),
                    path: path.to_path_buf(),
                }
            }
            Ok(file) => {
                if is_record {
                    panic!("Camera trace file already exists: {path:?}");
                }
                let reader = BufReader::new(file);
                let data = reader
                    .lines()
                    .map(|line| {
                        let line = line.unwrap();
                        let mut it = line.trim().split(',').map(|s| s.parse::<f32>().unwrap());
                        let position =
                            Point3::new(it.next().unwrap(), it.next().unwrap(), it.next().unwrap());
                        let pitch = cgmath::Deg(it.next().unwrap()).into();
                        let yaw = cgmath::Deg(it.next().unwrap()).into();
                        CameraPosition {
                            position,
                            pitch,
                            yaw,
                            up: cgmath::Vector3::unit_y(), // todo: trace has up vector?
                        }
                    })
                    .collect();
                Self {
                    data,
                    index: RefCell::new(0),
                    path: path.to_path_buf(),
                }
            }
        }
    }

    /// Get the next bandwidth sample. Used when playing back a camera trace.
    pub fn next(&self) -> CameraPosition {
        let idx = *self.index.borrow();
        let next_idx = (idx + 1) % self.data.len();
        *self.index.borrow_mut() = next_idx;
        self.data[idx]
    }

    /// Add a new position to the trace. Used when recording a camera trace.
    pub fn add(&mut self, pos: CameraPosition) {
        self.data.push(pos);
    }
}

impl Drop for CameraTrace {
    fn drop(&mut self) {
        use std::io::BufWriter;
        use std::io::Write;

        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&self.path)
        {
            Ok(mut file) => {
                let mut writer = BufWriter::new(&mut file);
                for pos in &self.data {
                    writeln!(
                        writer,
                        "{},{},{},{},{},0.0",
                        pos.position.x,
                        pos.position.y,
                        pos.position.z,
                        pos.pitch.0.to_degrees(),
                        pos.yaw.0.to_degrees()
                    )
                    .unwrap();
                }
            }
            Err(_) => {
                warn!("Camera trace file already exists, not writing");
            }
        }
    }
}

pub struct NetworkTrace {
    data: Vec<f64>,
    index: RefCell<usize>,
}

impl NetworkTrace {
    /// The network trace file to contain the network bandwidth in Kbps, each line representing 1 bandwidth sample.
    /// # Arguments
    ///
    /// * `path` - The path to the network trace file.
    pub fn new(path: &Path) -> Self {
        use std::io::BufRead;

        let file = File::open(path).unwrap();
        let reader = BufReader::new(file);
        let data = reader
            .lines()
            .map(|line| line.unwrap().trim().parse::<f64>().unwrap())
            .collect();
        NetworkTrace {
            data,
            index: RefCell::new(0),
        }
    }

    // Get the next bandwidth sample
    pub fn next(&self) -> f64 {
        let idx = *self.index.borrow();
        let next_idx = (idx + 1) % self.data.len();
        *self.index.borrow_mut() = next_idx;
        self.data[idx]
    }
}
