mod psnr;

use kiddo::KdTree;

use crate::formats::{pointxyzrgba::PointXyzRgba, PointCloud};

use self::psnr::Psnr;

pub struct Metrics {
    drms: f64,
    psnr: f64,
}

impl Metrics {
    pub fn to_bytes(&self) -> Vec<u8> {
        format!("drms,{}\npsnr,{}", self.drms, self.psnr)
            .as_bytes()
            .to_owned()
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
    let (drms, psnr) = Psnr::calculate_metric(
        &original.points,
        &original_tree,
        &reconstructed.points,
        &reconstructed_tree,
    );

    Metrics { drms, psnr }
}
