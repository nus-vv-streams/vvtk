use crate::points::{ Points };
use crate::tool::renderer;

pub struct SepPoints {
    first_half: Points,
    second_half: Points
}

impl SepPoints {
    pub fn of(first_half: Points, second_half: Points) -> Self {
        SepPoints {
            first_half,
            second_half
        }
    }

    pub fn render(&self) {
        let mut renderer = renderer::Renderer::new();
        while renderer.rendering() {
            renderer.render_in_green(&self.first_half);
            renderer.set_point_size(1.0);
            renderer.render_frame(&self.second_half);
        }
    }
}