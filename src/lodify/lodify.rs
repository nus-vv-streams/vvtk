use crate::formats::{pointxyzrgba::PointXyzRgba, PointCloud};
use crate::subsample::random_sampler::subsample;
use crate::utils::get_pc_bound;

pub fn lodify(
    points: &PointCloud<PointXyzRgba>,
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

pub fn partition(
    pc: &PointCloud<PointXyzRgba>,
    partitions: (usize, usize, usize),
) -> PointCloud<PointXyzRgba> {
    let bound = get_pc_bound(&pc);
    let mut partitioned_points = vec![vec![]; partitions.0 * partitions.1 * partitions.2];

    for point in &pc.points {
        let index = bound.get_bound_index(point, partitions);
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
