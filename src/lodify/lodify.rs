use crate::formats::{pointxyzrgba::PointXyzRgba, PointCloud};
use crate::subsample::random_sampler::subsample;

const DELTA: f32 = 1e-4;

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

    let x_step = (max_x - min_x + DELTA) / partitions.0 as f32;
    let y_step = (max_y - min_y + DELTA) / partitions.1 as f32;
    let z_step = (max_z - min_z + DELTA) / partitions.2 as f32;

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

#[cfg(test)]
mod test {
    use super::*;

    #[test]

    fn test_partition() {
        let points = vec![
            PointXyzRgba {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            },
            PointXyzRgba {
                x: 1.0,
                y: 1.0,
                z: 1.0,
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            },
            PointXyzRgba {
                x: 2.0,
                y: 2.0,
                z: 2.0,
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            },
            PointXyzRgba {
                x: 3.0,
                y: 3.0,
                z: 3.0,
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            },
        ];

        let pc = PointCloud::new(4, points);

        let result = partition(&pc, (2, 2, 2));

        assert_eq!(result.points.len(), 4);
        assert_eq!(result.segments.len(), 8);
        assert_eq!(result.segments[0].points.len(), 2);
        assert_eq!(result.segments[1].points.len(), 0);
        assert_eq!(result.segments[2].points.len(), 0);
        assert_eq!(result.segments[3].points.len(), 0);
        assert_eq!(result.segments[4].points.len(), 0);
        assert_eq!(result.segments[5].points.len(), 0);
        assert_eq!(result.segments[6].points.len(), 0);
        assert_eq!(result.segments[7].points.len(), 2);
    }
}
