// use kiddo::distance::squared_euclidean;
// use kiddo::KdTree;
// use crate::points::inf_norm;

#[derive(Debug, Clone)]
/// Struct containing all settings needed to execute interpolation
pub struct Params {
    /// Weightage to penalize coordinate delta
    pub penalize_coor: f32,
    /// Weightage to penalize colour delta
    pub penalize_col: f32,
    /// Weightage to penalize pre-mapped points
    pub penalize_mapped: f32,
    /// Radius to determine point density and potentially query nearest neighbours
    pub density_radius: f32,
    /// Number of neighbours to query
    pub options_for_nearest: usize,
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
    pub scale_col_delta: f32
}

impl Params {
    /// Create a new instance of type Params
    pub fn new() -> Self {
        Params {
            penalize_coor: 0.0,
            penalize_col: 0.0,
            penalize_mapped: 0.0,
            density_radius: 0.0,
            options_for_nearest: 0,
            show_unmapped_points: false,
            resize_near_cracks: false,
            mark_enlarged: false,
            compute_frame_delta: false,
            threads: 1,
            scale_coor_delta: 1.0,
            scale_col_delta: 1.0
        }
    }
}

impl Default for Params {
    fn default() -> Self {
        Self::new()
    }
}
