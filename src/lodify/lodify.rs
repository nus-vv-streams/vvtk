use std::collections::VecDeque;
use std::iter::zip;

use crate::formats::bounds::Bounds;
use crate::formats::{pointxyzrgba::PointXyzRgba, PointCloud};
use crate::utils::get_pc_bound;

pub fn lodify(
    points: &PointCloud<PointXyzRgba>,
    partitions: (usize, usize, usize),
    base_proportion: usize,
    points_per_voxel_threshold: usize,
) -> (
    PointCloud<PointXyzRgba>,
    Vec<PointCloud<PointXyzRgba>>,
    Vec<usize>,
    Vec<usize>,
) {
    if points.points.is_empty() {
        (points.clone(), vec![], vec![], vec![])
    } else {
        let factor = base_proportion as f32 / 100.0;
        let base_point_num = (points.points.len() as f32 * factor).ceil() as usize;

        let (base_pc, additional_pc) = sample(points, base_point_num, points_per_voxel_threshold);

        let partitioned_base_pc = partition(&base_pc, partitions);
        let partitioned_add_pc = partition(&additional_pc, partitions);

        let add_segments = partitioned_add_pc.segments.as_ref().unwrap();
        let pc_by_segment = (0..add_segments.len())
            .map(|segment_id| {
                let points = partitioned_add_pc.get_points_in_segment(segment_id);
                PointCloud::new(add_segments[segment_id].point_indices.len(), points)
            })
            .collect();

        let base_point_nums = partitioned_base_pc
            .segments
            .as_ref()
            .unwrap()
            .iter()
            .map(|segment| segment.point_indices.len())
            .collect();

        let additional_point_nums = partitioned_add_pc
            .segments
            .as_ref()
            .unwrap()
            .iter()
            .map(|segment| segment.point_indices.len())
            .collect();

        (
            partitioned_base_pc,
            pc_by_segment,
            base_point_nums,
            additional_point_nums,
        )
    }
}

fn sample(
    pc: &PointCloud<PointXyzRgba>,
    base_point_num: usize,
    points_per_voxel_threshold: usize,
) -> (PointCloud<PointXyzRgba>, PointCloud<PointXyzRgba>) {
    if pc.points.is_empty() {
        (pc.clone(), PointCloud::new(0, vec![]))
    } else {
        let bound = get_pc_bound(pc);
        let mut points_by_voxel = VecDeque::from(get_points_in_small_enough_voxel(
            pc.points.clone(),
            points_per_voxel_threshold,
            bound,
        ));

        let mut base_pc = vec![];
        let mut additional_pcs = vec![];

        // this attempts to keep the base points as evenly distributed as possible
        while !points_by_voxel.is_empty() {
            let mut points = points_by_voxel.pop_front().unwrap();

            if points.is_empty() {
                continue;
            }

            let popped = points.pop().unwrap();

            if base_pc.len() < base_point_num {
                base_pc.push(popped);
            } else {
                additional_pcs.push(popped);
            }

            if !points.is_empty() {
                points_by_voxel.push_back(points);
            }
        }

        (
            PointCloud::new(base_pc.len(), base_pc),
            PointCloud::new(additional_pcs.len(), additional_pcs),
        )
    }
}

/// obtain points in a voxel that is small enough
fn get_points_in_small_enough_voxel(
    points: Vec<PointXyzRgba>,
    points_per_voxel_threshold: usize,
    bound: Bounds,
) -> Vec<Vec<PointXyzRgba>> {
    if points.len() <= points_per_voxel_threshold {
        return vec![points];
    }

    let mut voxels = vec![vec![]; 8];
    let split_bounds = bound.split();
    for point in points {
        for i in 0..8 {
            if split_bounds[i].contains(&point) {
                voxels[i].push(point);
                break;
            }
        }
    }

    zip(voxels, split_bounds)
        .flat_map(|(p, b)| get_points_in_small_enough_voxel(p, points_per_voxel_threshold, b))
        .collect()
}

fn partition(
    pc: &PointCloud<PointXyzRgba>,
    partitions: (usize, usize, usize),
) -> PointCloud<PointXyzRgba> {
    let pc_bound = get_pc_bound(pc);
    let child_bounds = pc_bound.partition(partitions);

    let num_segments = child_bounds.len();
    let mut partitioned_points = vec![vec![]; num_segments];

    for point in &pc.points {
        for (index, bound) in child_bounds.iter().enumerate() {
            if bound.contains(point) {
                partitioned_points[index].push(*point);
                break;
            }
        }
    }

    let base_point_nums: Vec<usize> = partitioned_points
        .iter()
        .map(|points| points.len())
        .collect();

    // flatten the points
    let points = partitioned_points.into_iter().flatten().collect();
    let mut new_pc = PointCloud::new(pc.number_of_points, points);

    new_pc.self_segment(&base_point_nums, &child_bounds);
    new_pc
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
        let segments = result.segments.unwrap();

        assert_eq!(result.points.len(), 4);
        assert_eq!(segments.len(), 8);
        assert_eq!(segments[0].point_indices.len(), 2);
        assert_eq!(segments[1].point_indices.len(), 0);
        assert_eq!(segments[2].point_indices.len(), 0);
        assert_eq!(segments[3].point_indices.len(), 0);
        assert_eq!(segments[4].point_indices.len(), 0);
        assert_eq!(segments[5].point_indices.len(), 0);
        assert_eq!(segments[6].point_indices.len(), 0);
        assert_eq!(segments[7].point_indices.len(), 2);
    }
}
