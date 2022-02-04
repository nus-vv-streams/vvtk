use crate::processing::conceal::ConcealedPointCloud;
use crate::processing::conceal::interpolate_controller::*;
use crate::processing::conceal::InterpolateParams;
use crate::point::Point;
use std::sync::Arc;

use crate::Instant;

pub fn two_norm(a: &[f32; 3], b: &[f32; 3]) -> f32 {
    let mut sum: f32 = 0.0;
    for i in 0..a.len() {
        sum += (a[i] - b[i]).powi(2);
    }
    sum.sqrt()
}

/// Computes Chebyshev Distance for 2 given points
///
/// # Arguments
/// * `a` - the first point
/// * `b` - the second point
///
pub fn inf_norm(a: &[f32; 3], b: &[f32; 3]) -> f32 {
    let mut max: f32 = f32::MIN;
    for i in 0..a.len() {
        let diff = (a[i] - b[i]).abs();
        if diff > max {
            max = diff;
        }
    }

    max
}

/// Point to point interpolation method
pub fn closest_with_ratio_average_points_recovery(
    prev: ConcealedPointCloud,
    next: ConcealedPointCloud,
    params: InterpolateParams,
) -> (ConcealedPointCloud, ConcealedPointCloud, ConcealedPointCloud) {
    let kd_tree = next.clone().pc.to_kdtree();
    let targets = prev.pc.data.clone();
    let mut interpolated_points: Vec<Point> = Vec::new();

    if !targets.is_empty() {
        let mut slices =
            targets.chunks((targets.len() as f32 / params.threads as f32).ceil() as usize);

        interpolated_points = interpolate_in_parallel(
            params.threads,
            &mut slices,
            Arc::new(kd_tree),
            Arc::new(next.pc.clone()),
            Arc::new(params),
        )
    }
    (
        ConcealedPointCloud::of(interpolated_points),
        prev, next
    )
}