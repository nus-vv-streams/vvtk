use crate::interpolate_controller::*;
use crate::params::Params;
use crate::point::Point;
use crate::points::Points;
use std::sync::Arc;

use crate::Instant;

/// Computes Chebyshev Distance for 2 given points
///
/// # Arguments
/// * `a` - the first point
/// * `b` - the second point
///
pub fn inf_norm(a: &[f32; 3], b: &[f32; 3]) -> f32 {
    let max: f32;
    let dx = (a[0] - b[0]).abs();
    let dy = (a[1] - b[1]).abs();
    let dz = (a[2] - b[2]).abs();
    if dx > dy {
        max = dx;
    } else {
        max = dy;
    }
    if max > dz {
        max
    } else {
        dz
    }
}

/// Point to point interolation method
pub fn closest_with_ratio_average_points_recovery(
    mut prev_points: Points,
    next_points: Points,
    params: Params,
    exists_output_dir: bool,
) -> (Points, Points, Points) {
    //start time
    let now = Instant::now();
    prev_points.reference_frame = next_points.data.clone();
    // println!("ref frame cloning: {}", now.elapsed().as_millis());
    let kd_tree = next_points.clone().to_kdtree();

    //    println!("kd tree constrcution: {}", now.elapsed().as_millis());

    // let mutex_tree = Mutex::new(kd_tree);
    let arc_tree = Arc::new(kd_tree);
    // let kd = 'static kd_tree;
    let arc_next_points = Arc::new(next_points);
    let arc_params = Arc::new(params);
    // println!("arc cloning: {}", now.elapsed().as_millis());
    let data_copy = prev_points.data.clone();
    let mut interpolated_points: Vec<Point> = Vec::new();

    if !data_copy.is_empty() {
        interpolated_points = parallel_query_closests(
            &data_copy,
            &arc_tree,
            arc_params.threads,
            arc_params.options_for_nearest,
            arc_next_points,
            &arc_params,
            &mut prev_points.reference_frame,
        );
    }

    // No parallelization interpolation
    // let mut interpolated_points: Vec<Point> = Vec::with_capacity(100);
    // for s in data_copy {
    //     let nearests = s.method_of_neighbour_query(&arc_tree, arc_params.options_for_nearest, params.radius);
    //     let p = s.get_average_closest(&arc_next_points, &nearests, &mut self.reference_frame, &arc_params);
    //     interpolated_points.push(p);
    // }

    if exists_output_dir {
        println!("interpolation time: {}", now.elapsed().as_millis());
    }

    let mut point_data = Points::of(None, interpolated_points);
    if arc_params.compute_frame_delta {
        prev_points.frame_delta(point_data.clone());
    }

    if arc_params.show_unmapped_points {
        prev_points.mark_unmapped_points(arc_tree, exists_output_dir);
    }

    /////////////
    //point_data.render(); //original interpolated frame
    /////////////

    if arc_params.resize_near_cracks {
        point_data.adjust_point_sizes(arc_params.density_radius);
    }

    let mut marked_interpolated_frame = Points::new();
    if arc_params.resize_near_cracks && arc_params.mark_enlarged {
        marked_interpolated_frame =
            prev_points.mark_points_near_cracks(&point_data, exists_output_dir);
    }

    (
        point_data,
        Points::of(None, prev_points.reference_frame.clone()),
        marked_interpolated_frame,
    )
}
