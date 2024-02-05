use crate::{
    formats::{bounds::Bounds, pointxyzrgba::PointXyzRgba, PointCloud},
    utils::get_pc_bound,
};

use rand::prelude::*;
use std::iter::zip;

pub fn subsample(
    points: &PointCloud<PointXyzRgba>,
    proportions: Vec<usize>,
    points_per_voxel_threshold: usize,
) -> Vec<PointCloud<PointXyzRgba>> {
    if points.points.is_empty() {
        return vec![];
    } else {
        let bound = get_pc_bound(&points);

        let points = random_sumsample(
            points.points.clone(),
            &proportions,
            bound,
            points_per_voxel_threshold,
        );

        let mut point_clouds = vec![];
        for points in points {
            point_clouds.push(PointCloud::new(points.len(), points));
        }

        point_clouds
    }
}

fn random_sumsample(
    mut points: Vec<PointXyzRgba>,
    proportions: &Vec<usize>,
    bounds: Bounds,
    points_per_voxel_threshold: usize,
) -> Vec<Vec<PointXyzRgba>> {
    if points.len() <= points_per_voxel_threshold {
        let mut rng = rand::thread_rng();
        points.shuffle(&mut rng);

        let mut points = points.into_iter();
        let mut result = vec![];

        let factor = points.len() as f32 / proportions.iter().sum::<usize>() as f32;
        for proportion in proportions {
            let next = (*proportion as f32 * factor).ceil() as usize;
            result.push(points.by_ref().take(next).collect::<Vec<PointXyzRgba>>());
        }

        return result;
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

    zip(voxels, split_bounds).fold(vec![], |acc, (voxel, bounds)| {
        let points = random_sumsample(voxel, proportions, bounds, points_per_voxel_threshold);

        if acc.is_empty() {
            points
        } else {
            acc.into_iter()
                .zip(points.into_iter())
                .map(|(mut acc, points)| {
                    acc.extend(points);
                    acc
                })
                .collect()
        }
    })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    // test whether the subsamples contain all the points
    fn test_subsample() {
        let mut rng = rand::thread_rng();
        let mut points = vec![];
        for _ in 0..100 {
            points.push(PointXyzRgba {
                x: rng.gen(),
                y: rng.gen(),
                z: rng.gen(),
                r: rng.gen(),
                g: rng.gen(),
                b: rng.gen(),
                a: rng.gen(),
            });
        }

        let proportions = vec![7, 1, 1, 1];
        let points_per_voxel_threshold = 20;
        let subsamples = subsample(
            &PointCloud::new(points.len(), points),
            proportions,
            points_per_voxel_threshold,
        );

        let mut subsample_points = vec![];
        for subsample in subsamples {
            println!("subsample: {:?}", subsample.points.len());
            subsample_points.extend(subsample.points);
        }

        assert_eq!(100, subsample_points.len());
    }
}
