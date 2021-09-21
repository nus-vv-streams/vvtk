use kiss3d::conrod;
use kiss3d::conrod::position::Positionable;
use kiss3d::conrod::widget_ids;

use crate::gui_states::{close_state, open_state, State};
use crate::renderer::Renderer;

pub const WIN_W: u32 = 600;
pub const WIN_H: u32 = 420;

const MARGIN: conrod::Scalar = 10.0;
const INFO_BUTTON_SIZE: conrod::Scalar = 40.0;
const TEXT_INFO_HEIGHT: conrod::Scalar = 100.0;

pub static INFO_BUTTON_LABEL: &str = "info";

widget_ids! {
    pub struct Ids {
        canvas,
        toggle,
        text_info,
    }
}

/// A demonstration of some application state we want to control with a conrod GUI.
pub struct InfoBar {
    state: State,
    information_text: Option<String>,
}

impl InfoBar {
    pub fn new_closed_state() -> Self {
        InfoBar {
            state: close_state(),
            information_text: None,
        }
    }
}

/// Instantiate a GUI demonstrating every widget available in conrod.
pub fn gui(ids: &Ids, app: &mut InfoBar, renderer: &mut Renderer) {
    use conrod::{widget, Labelable, Sizeable, Widget};

    let (eye_pos, at_pos) = renderer.get_eye_at_info();
    app.information_text = Some(format!(
        "Camera's positions:\n * eye: {}\n * at: {}",
        eye_pos, at_pos
    ));

    let ui = &mut renderer.window.conrod_ui_mut().set_widgets();

    let (canvas_w, canvas_h) = app.state.get_canvas_w_h();

    widget::Canvas::new()
        .pad(MARGIN)
        .align_right()
        .align_top()
        .w_h(canvas_w, canvas_h)
        .scroll_kids_vertically()
        .set(ids.canvas, ui);

    let is_open = app.state.is_opening();

    for is_opening in widget::Toggle::new(is_open)
        .label(INFO_BUTTON_LABEL)
        .medium_font(ui)
        .label_color(*app.state.get_info_button_color())
        .top_right_with_margin_on(ids.canvas, 0.0)
        .w_h(INFO_BUTTON_SIZE, INFO_BUTTON_SIZE)
        .set(ids.toggle, ui)
    {
        app.state = if is_opening {
            open_state()
        } else {
            close_state()
        };
    }

    widget::TextEdit::new(app.information_text.as_ref().unwrap())
        .down_from(ids.toggle, MARGIN)
        .align_middle_x_of(ids.canvas)
        .padded_w_of(ids.canvas, MARGIN)
        .font_size(14)
        .h(TEXT_INFO_HEIGHT)
        .set(ids.text_info, ui);
}

/*
 *
 * This is he example taken from conrods' repository.
 *
 */
/// A set of reasonable stylistic defaults that works for the `gui` below.
pub fn theme() -> conrod::Theme {
    use conrod::position::{Align, Direction, Padding, Position, Relative};
    conrod::Theme {
        name: "Demo Theme".to_string(),
        padding: Padding::none(),
        x_position: Position::Relative(Relative::Align(Align::Start), None),
        y_position: Position::Relative(Relative::Direction(Direction::Backwards, 20.0), None),
        background_color: conrod::color::DARK_CHARCOAL,
        shape_color: conrod::color::LIGHT_CHARCOAL,
        border_color: conrod::color::BLACK,
        border_width: 0.0,
        label_color: conrod::color::WHITE,
        font_id: None,
        font_size_large: 26,
        font_size_medium: 14,
        font_size_small: 12,
        widget_styling: conrod::theme::StyleMap::default(),
        mouse_drag_threshold: 0.0,
        double_click_threshold: std::time::Duration::from_millis(500),
    }
}
