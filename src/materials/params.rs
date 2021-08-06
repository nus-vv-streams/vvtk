pub struct Params {
    pub penalize_coor: f32,
    pub penalize_col: f32,
    pub penalize_mapped: f32,
    pub radius: f32,
    pub options_for_nearest: usize,
    pub show_unmapped_points: bool,
    pub resize_near_cracks: bool,
    pub mark_enlarged: bool,
    pub compute_frame_delta: bool,
}

impl Params {
    pub fn new() -> Self {
        Params{
            penalize_coor: 0.0,
            penalize_col: 0.0,
            penalize_mapped: 0.0,
            radius: 0.0,
            options_for_nearest: 0,
            show_unmapped_points: false,
            resize_near_cracks: false,
            mark_enlarged: false,
            compute_frame_delta: false,

        }
       
    }
}

impl Default for Params {
    fn default() -> Self {
        Self::new()
    }
}
    