use crate::{
    formats::{bounds::Bounds, pointxyzrgba::PointXyzRgba, PointCloud},
    utils::get_pc_bound,
};

use rayon::prelude::*;

pub fn downsample(
    points: PointCloud<PointXyzRgba>,
    points_per_voxel: usize,
) -> PointCloud<PointXyzRgba> {
    if points.points.is_empty() {
        points
    } else {
        let bound = get_pc_bound(&points);
        let points = octree_downsample(points.points, bound, points_per_voxel);
        PointCloud::new(points.len(), points)
    }
}

fn octree_downsample(
    points: Vec<PointXyzRgba>,
    bounds: Bounds,
    points_per_voxel: usize,
) -> Vec<PointXyzRgba> {
    if points.is_empty() {
        return vec![];
    }

    if points.len() <= points_per_voxel {
        return vec![centroid(points)];
    }

    let mut voxels = vec![vec![]; 8];
    let split_bounds = bounds.split();
    for point in points {
        for i in 0..8 {
            if split_bounds[i].contains(&point) {
                voxels[i].push(point);
                break;
            }
        }
    }

    voxels
        .into_par_iter()
        .zip(split_bounds.into_par_iter())
        .flat_map(|(p, b)| octree_downsample(p, b, points_per_voxel))
        .collect()
}

fn centroid(points: Vec<PointXyzRgba>) -> PointXyzRgba {
    let mut x = 0f64;
    let mut y = 0f64;
    let mut z = 0f64;
    let mut r = 0usize;
    let mut g = 0usize;
    let mut b = 0usize;
    let mut a = 00usize;

    let size = points.len();
    for point in points {
        x += point.x as f64;
        y += point.y as f64;
        z += point.z as f64;
        r += point.r as usize;
        g += point.g as usize;
        b += point.b as usize;
        a += point.a as usize;
    }

    PointXyzRgba {
        x: (x / size as f64) as f32,
        y: (y / size as f64) as f32,
        z: (z / size as f64) as f32,
        r: (r / size) as u8,
        g: (g / size) as u8,
        b: (b / size) as u8,
        a: (a / size) as u8,
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        pcd::{create_pcd, write_pcd_file},
        utils::read_file_to_point_cloud,
    };

    use std::path::PathBuf;
    #[test]
    fn test_downsample() {
        let pcd_path =
            PathBuf::from("./test_files/pcd_ascii/longdress_vox10_1213_short_upsampled.pcd");
        let pcd = read_file_to_point_cloud(&pcd_path).unwrap();
        println!("before: {:?}", pcd);
        let downsampled = downsample(pcd, 2);
        println!("Downsampled: {:?}", downsampled);
        let pcd = create_pcd(&downsampled);
        let outpath =
            PathBuf::from("./test_files/pcd_ascii/longdress_vox10_1213_short_up_downsampled.pcd");
        write_pcd_file(&pcd, crate::pcd::PCDDataType::Ascii, &outpath).unwrap();
    }
}
