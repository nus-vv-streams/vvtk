mod psnr;

use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader, Read, Write},
};

use kiddo::KdTree;

use crate::formats::{pointxyzrgba::PointXyzRgba, PointCloud};

use self::psnr::Psnr;

pub struct Metrics(BTreeMap<String, String>);

impl Metrics {
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    pub fn insert(&mut self, key: String, value: String) {
        self.0.insert(key, value);
    }

    pub fn metrics(&self) -> Vec<(String, String)> {
        self.0
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    pub fn from_reader<R>(read: &mut R) -> Self
    where
        R: Read,
    {
        let mut map = BTreeMap::new();
        let mut s = String::new();
        let mut buf = BufReader::new(read);
        while let Ok(size) = buf.read_line(&mut s) {
            if size == 0 {
                break;
            }
            let mut split = s.split(",");
            map.insert(
                split.next().expect("Metric name expected").to_string(),
                split.next().expect("Metric value expected").to_string(),
            );
            s.clear();
        }
        Self(map)
    }
    pub fn write_to<W>(&self, writer: &mut W) -> std::io::Result<()>
    where
        W: Write,
    {
        for (key, val) in self.0.iter() {
            writeln!(writer, "{},{}", key, val)?;
        }
        Ok(())
    }
}

pub fn calculate_metrics(
    original: &PointCloud<PointXyzRgba>,
    reconstructed: &PointCloud<PointXyzRgba>,
) -> Metrics {
    let mut original_tree = KdTree::new();
    for (i, pt) in original.points.iter().enumerate() {
        original_tree
            .add(&[pt.x, pt.y, pt.z], i)
            .expect("Failed to add to original tree");
    }
    let mut reconstructed_tree = KdTree::new();
    for (i, pt) in reconstructed.points.iter().enumerate() {
        reconstructed_tree
            .add(&[pt.x, pt.y, pt.z], i)
            .expect("Failed to add to original tree");
    }
    let mut metrics = Metrics::new();
    Psnr::calculate_metric(
        &original.points,
        &original_tree,
        &reconstructed.points,
        &reconstructed_tree,
        &mut metrics,
    );

    metrics
}
