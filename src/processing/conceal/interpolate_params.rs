// use kiddo::distance::squared_euclidean;
// use kiddo::KdTree;
use crate::processing::conceal::interpolate::inf_norm;

#[derive(Clone)]
/// Struct containing all settings needed to execute interpolation
pub struct InterpolateParams {
    /// Weightage to penalize coordinate delta
    pub penalize_coor: f32,
    /// Weightage to penalize colour delta
    pub penalize_col: f32,
    /// Weightage to penalize pre-mapped points
    pub penalize_mapped: f32,
    /// Radius to determine point density and potentially query nearest neighbours
    pub density_radius: f32,
    /// Number of neighbours to query
    pub neighborhood_size: usize,
    /// Flag to trigger highlighting of points with a mapping count of 0
    pub show_unmapped_points: bool,
    /// Flag to resize points in close range to cracks
    pub resize_near_cracks: bool,
    /// Flag to highlight points that were enlarged due to proximity with cracks
    pub mark_enlarged: bool,
    /// Flag to compute coordinate and colour delta between the prev and next frames
    pub compute_frame_delta: bool,
    /// Number of threads to use for the interpolation process
    pub threads: usize,
    /// Scale factor to constrain coordinate delta between [0, 1]
    pub scale_coor_delta: f32,
    /// Scale factor to constrain color delta between [0, 1]
    pub scale_col_delta: f32,
    /// Weight for previous frame when averaging points to get interpolated point
    pub prev_weight: f32,
    /// Weight for next frame when averaging points to get interpolated point
    pub next_weight: f32,
    /// Distance function for use in interpolation
    pub dist_func: fn(&[f32; 3], &[f32; 3]) -> f32,
}

impl InterpolateParams {
    /// Create a new instance of type Params
    pub fn new() -> Self {
        InterpolateParams {
            penalize_coor: 0.0,
            penalize_col: 0.0,
            penalize_mapped: 0.0,
            density_radius: 0.0,
            neighborhood_size: 0,
            show_unmapped_points: false,
            resize_near_cracks: false,
            mark_enlarged: false,
            compute_frame_delta: false,
            threads: 1,
            scale_coor_delta: 1.0,
            scale_col_delta: 1.0,
            prev_weight: 0.5,
            next_weight: 0.5,
            dist_func: inf_norm,
        }
    }
}

impl Default for InterpolateParams {
    fn default() -> Self {
        Self::new()
    }
}
