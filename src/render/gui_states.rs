use kiss3d::conrod;
use kiss3d::conrod::color;

pub enum OpeningState {
    Open,
    Close,
}

impl OpeningState {
    pub fn is_opening(&self) -> bool {
        matches!(*self, OpeningState::Open)
    }
}

pub struct State {
    opening_state: OpeningState,
    info_button_color: conrod::Color,
    canvas_w_h: (conrod::Scalar, conrod::Scalar),
}

impl State {
    pub fn is_opening(&self) -> bool {
        self.opening_state.is_opening()
    }

    pub fn get_info_button_color(&self) -> &conrod::Color {
        &self.info_button_color
    }

    pub fn get_canvas_w_h(&self) -> (conrod::Scalar, conrod::Scalar) {
        self.canvas_w_h
    }

    pub fn switch_state(&mut self) {
        if self.is_opening() {
            *self = close_state()
        } else {
            *self = open_state()
        }
    }
}

pub fn open_state() -> State {
    State {
        opening_state: OpeningState::Open,
        info_button_color: color::WHITE,
        canvas_w_h: (300.0, 150.0),
    }
}

pub fn close_state() -> State {
    State {
        opening_state: OpeningState::Close,
        info_button_color: color::LIGHT_CHARCOAL,
        canvas_w_h: (60.0, 60.0),
    }
}
