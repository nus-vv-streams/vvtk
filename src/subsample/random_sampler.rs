use crate::formats::{pointxyzrgba::PointXyzRgba, PointCloud};

use rand::prelude::*;
use std::iter::zip;

const DELTA: f32 = 1e-4;

pub fn subsample(
    points: PointCloud<PointXyzRgba>,
    proportions: Vec<usize>,
    points_per_voxel_threshold: usize,
) -> Vec<PointCloud<PointXyzRgba>> {
    if points.points.is_empty() {
        vec![points]
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

        let points = random_sumsample(
            points.points,
            &proportions,
            Bounds {
                min_x,
                max_x,
                min_y,
                max_y,
                min_z,
                max_z,
            },
            points_per_voxel_threshold,
        );

        let mut point_clouds = vec![];

        for points in points {
            point_clouds.push(PointCloud {
                number_of_points: points.len(),
                points,
            });
        }

        point_clouds
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
            PointCloud {
                number_of_points: points.len(),
                points,
            },
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
