use std::fmt::Debug;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
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

pub struct BufRenderReader<U: Renderable + Send> {
    size_tx: Sender<usize>,
    receiver: Receiver<(usize, Option<U>)>,
    length: usize,
    antialias: AntiAlias
}

impl<U> BufRenderReader<U> where U: 'static + Renderable + Send + Debug {
    pub fn new<T: 'static + RenderReader<U> + Send + Sync>(buffer_size: usize, reader: T) -> Self {
        let (size_tx, size_rx) = std::sync::mpsc::channel();
        let (sender, receiver) = std::sync::mpsc::channel();
        let antialias = reader.antialias();
        let length = reader.len();

        let threads = rayon::current_num_threads().saturating_sub(3).min(buffer_size);
        if threads == 0 {
            panic!("Not enough threads!");
        }
        rayon::spawn(move || {
            let mut started = false;
            let max = length;
            let mut current = 0;
            let length = buffer_size;
            let mut next = 0;
            let (range_tx, range_rx) = std::sync::mpsc::channel::<Range<usize>>();
            rayon::spawn(move || {
                loop {
                    if let Ok(range) = range_rx.recv() {
                        range
                            .into_par_iter()
                            .map(|i| (i, reader.get_at(i)))
                            .collect::<Vec<(usize, Option<U>)>>()
                            .into_iter()
                            .for_each(|out| {
                                sender.send(out).unwrap();
                            } );
                    }
                }
            });
            loop {
                if let Ok(pos) = size_rx.try_recv() {
                    if (started && pos <= current) || pos >= next  {
                        next = pos;
                    }
                    started = true;
                    current = pos;
                }
                if (length - (next - current)) >= threads && next != max {
                    let to = (next + threads).min(max).min(current + length);
                    range_tx.send(next..to).expect("Failed to send range to worker");
                    next = to;
                }
            }
        });


        Self {
            size_tx,
            receiver,
            length,
            antialias
        }
    }
}

impl<U> RenderReader<U> for BufRenderReader<U> where U: 'static + Renderable + Send + Debug {
    fn get_at(&self, index: usize) -> Option<U> {
        self.size_tx.send(index).unwrap();
        while let Ok((pos, val)) = self.receiver.recv() {
            if pos == index {
                return val;
            }
        }
        None
    }

    fn len(&self) -> usize {
        self.length
    }

    fn is_empty(&self) -> bool {
        self.length == 0
    }

    fn antialias(&self) -> AntiAlias {
        self.antialias
    }
}