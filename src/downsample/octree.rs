use crate::formats::{pointxyzrgba::PointXyzRgba, PointCloud};

use std::iter::zip;

const DELTA: f32 = 1e-4;

pub fn downsample(
    points: PointCloud<PointXyzRgba>,
    points_per_voxel: usize,
) -> PointCloud<PointXyzRgba> {
    if points.points.is_empty() {
        points
    } else {
        let first_point = points.points[0];
        let mut min_x = first_point.x;
        let mut max_x = first_point.x;
        let mut min_y = first_point.y;
        let mut max_y = first_point.y;
        let mut min_z = first_point.z;
        let mut max_z = first_point.z;

        for &point in &points.points {
            min_x = min_x.min(point.x);
            max_x = max_x.max(point.x);
            min_y = min_y.min(point.y);
            max_y = max_y.max(point.y);
            min_z = min_z.min(point.z);
            max_z = max_z.max(point.z);
        }

        let points = octree_downsample(
            points.points,
            Bounds {
                min_x,
                max_x,
                min_y,
                max_y,
                min_z,
                max_z,
            },
            points_per_voxel,
        );
        println!("len {}", points.len());
        PointCloud {
            number_of_points: points.len(),
            points,
        }
    }
}

struct Bounds {
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
    min_z: f32,
    max_z: f32,
}

impl Bounds {
    fn new(min_x: f32, max_x: f32, min_y: f32, max_y: f32, min_z: f32, max_z: f32) -> Self {
        Self {
            min_x,
            max_x,
            min_y,
            max_y,
            min_z,
            max_z,
        }
    }

    fn split(&self) -> Vec<Bounds> {
        let &Bounds {
            min_x,
            max_x,
            min_y,
            max_y,
            min_z,
            max_z,
        } = self;

        let bisect_x = (max_x + min_x) / 2f32;
        let bisect_y = (max_y + min_y) / 2f32;
        let bisect_z = (max_z + min_z) / 2f32;

        vec![
            Bounds::new(min_x, bisect_x, min_y, bisect_y, min_z, bisect_z),
            Bounds::new(min_x, bisect_x, min_y, bisect_y, bisect_z + DELTA, max_z),
            Bounds::new(min_x, bisect_x, bisect_y + DELTA, max_y, min_z, bisect_z),
            Bounds::new(
                min_x,
                bisect_x,
                bisect_y + DELTA,
                max_y,
                bisect_z + DELTA,
                max_z,
            ),
            Bounds::new(bisect_x + DELTA, max_x, min_y, bisect_y, min_z, bisect_z),
            Bounds::new(
                bisect_x + DELTA,
                max_x,
                min_y,
                bisect_y,
                bisect_z + DELTA,
                max_z,
            ),
            Bounds::new(
                bisect_x + DELTA,
                max_x,
                bisect_y + DELTA,
                max_y,
                min_z,
                bisect_z,
            ),
            Bounds::new(
                bisect_x + DELTA,
                max_x,
                bisect_y + DELTA,
                max_y,
                bisect_z + DELTA,
                max_z,
            ),
        ]
    }

    fn contains(&self, point: &PointXyzRgba) -> bool {
        point.x >= self.min_x
            && point.x <= self.max_x
            && point.y >= self.min_y
            && point.y <= self.max_y
            && point.z >= self.min_z
            && point.z <= self.max_z
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

    zip(voxels, split_bounds)
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
