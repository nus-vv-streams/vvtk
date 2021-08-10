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
        Params {
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

    pub fn clone(&self) -> Params {
        Params {
            penalize_coor: self.penalize_coor,
            penalize_col: self.penalize_col,
            penalize_mapped: self.penalize_mapped,
            radius: self.radius,
            options_for_nearest: self.options_for_nearest,
            show_unmapped_points: self.show_unmapped_points,
            resize_near_cracks: self.resize_near_cracks,
            mark_enlarged: self.mark_enlarged,
            compute_frame_delta: self.compute_frame_delta,
        }
    }
}

impl Default for Params {
    fn default() -> Self {
        Self::new()
    }
}
