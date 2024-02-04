use crate::formats::{pointxyzrgba::PointXyzRgba, PointCloud};
use crate::subsample::random_sampler::subsample;

pub fn lodify(
    points: PointCloud<PointXyzRgba>,
    partitions: (usize, usize, usize),
    proportions: Vec<usize>,
    points_per_voxel_threshold: usize,
) -> Vec<PointCloud<PointXyzRgba>> {
    if points.points.is_empty() {
        return vec![];
    } else {
        let points = subsample(points, proportions, points_per_voxel_threshold);

        if points.len() == 1 {
            return points;
        }

        let base_pc = points[0].clone();
        let additional_pcs = points[1..].to_vec();

        let mut result = vec![base_pc];

        for pc in additional_pcs {
            let partitioned_pc = partition(&pc, partitions);
            result.push(partitioned_pc);
        }

        result
    }
}

fn partition(
    pc: &PointCloud<PointXyzRgba>,
    partitions: (usize, usize, usize),
) -> PointCloud<PointXyzRgba> {
    let first_point = pc.points[0];
    let mut min_x = first_point.x;
    let mut max_x = first_point.x;
    let mut min_y = first_point.y;
    let mut max_y = first_point.y;
    let mut min_z = first_point.z;
    let mut max_z = first_point.z;

    for &point in &pc.points {
        min_x = min_x.min(point.x);
        max_x = max_x.max(point.x);
        min_y = min_y.min(point.y);
        max_y = max_y.max(point.y);
        min_z = min_z.min(point.z);
        max_z = max_z.max(point.z);
    }

    let x_step = (max_x - min_x) / partitions.0 as f32;
    let y_step = (max_y - min_y) / partitions.1 as f32;
    let z_step = (max_z - min_z) / partitions.2 as f32;

    let mut partitioned_points = vec![vec![]; partitions.0 * partitions.1 * partitions.2];

    for point in &pc.points {
        let x = ((point.x - min_x) / x_step).floor() as usize;
        let y = ((point.y - min_y) / y_step).floor() as usize;
        let z = ((point.z - min_z) / z_step).floor() as usize;

        let index = x + y * partitions.0 + z * partitions.0 * partitions.1;

        partitioned_points[index].push(*point);
    }

    PointCloud::new_with_segments(partitioned_points)
}
