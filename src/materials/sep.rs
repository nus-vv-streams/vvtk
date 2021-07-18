use crate::points::{ Points, Point };
use crate::tool::renderer::Renderer;

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

    pub fn render_with_method<F: Fn(&mut Renderer, &Point), U: Fn(&mut Renderer, &Point)>(&self, method1: F, method2: U) {
        let mut renderer = Renderer::new();
        while renderer.rendering() {
            renderer.render_frame_with_method(&self.first_half, &method1);
            renderer.render_frame_with_method(&self.second_half, &method2);
        }
    }
}