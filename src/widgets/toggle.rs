use egui::{vec2, Galley, Pos2, Rect, Response, TextStyle, Ui, Vec2, WidgetText};
use std::sync::Arc;

/// Two-state toggle switch with labels on both sides.
/// ``` text
///         _____________
///        /       /.....\
///  Off  |       |.......|  On
///        \_______\_____/
/// ```
///
/// ## Example:
/// ```ignore
/// ui.add(toggle(&mut my_bool, "Off", "On"));
/// ```
pub fn toggle<'a>(
    on: &'a mut bool,
    text_left: impl Into<WidgetText> + 'a,
    text_right: impl Into<WidgetText> + 'a,
) -> impl egui::Widget + 'a {
    move |ui: &mut egui::Ui| toggle_ui(ui, on, text_left, text_right)
}

fn toggle_ui(
    ui: &mut egui::Ui,
    on: &mut bool,
    text_left: impl Into<WidgetText>,
    text_right: impl Into<WidgetText>,
) -> egui::Response {
    let (space, mut response) = allocate_space(ui, text_left.into(), text_right.into());

    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }

    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), *on, "")
    });

    if ui.is_rect_visible(space.rect) {
        paint(ui, *on, &response, space);
    }

    response
}

fn allocate_space(
    ui: &mut egui::Ui,
    text_left: WidgetText,
    text_right: WidgetText,
) -> (AllocatedSpace, Response) {
    let toggle_size = ui.spacing().interact_size.y * egui::vec2(2.5, 1.25);
    let toggle_spacing = ui.spacing().item_spacing.y * 3.5;

    let available_width = ui.available_width() - toggle_size.x - toggle_spacing * 2.;
    let text_left = text_left.into_galley(ui, None, available_width / 2.0, TextStyle::Button);
    let text_right = text_right.into_galley(ui, None, available_width / 2.0, TextStyle::Button);

    // We want the toggle button to be centered, even if the
    // two texts have different sizes, so we allocate twice the max size.
    let max_text_size = max_size(text_left.size(), text_right.size());

    let (rect, response) = ui.allocate_exact_size(
        toggle_size + max_text_size * vec2(2., 0.) + vec2(toggle_spacing * 2., 0.),
        egui::Sense::click(),
    );

    let space = AllocatedSpace {
        rect,
        text_left,
        text_right,
        max_text_size,
        toggle_size,
    };

    (space, response)
}

fn partition_space(
    AllocatedSpace {
        rect,
        toggle_size,
        text_left,
        text_right,
        max_text_size,
    }: &AllocatedSpace,
) -> (Rect, Rect, Rect) {
    let toggle_rect = Rect::from_center_size(rect.center(), *toggle_size);

    let text_left_rect = Rect::from_center_size(
        rect.left_center() + text_offset(text_left.size(), *max_text_size),
        text_left.size(),
    );

    let text_right_rect = Rect::from_center_size(
        rect.right_center() - text_offset(text_right.size(), *max_text_size),
        text_right.size(),
    );

    (text_left_rect, toggle_rect, text_right_rect)
}

fn paint(ui: &mut egui::Ui, on: bool, response: &Response, space: AllocatedSpace) {
    let (text_left_rect, toggle_rect, text_right_rect) = partition_space(&space);
    paint_text(ui, response, space.text_left, text_left_rect.min, !on);
    paint_toggle(ui, response, toggle_rect, on);
    paint_text(ui, response, space.text_right, text_right_rect.min, on);
}

fn paint_text(ui: &mut Ui, response: &Response, text: Arc<Galley>, pos: Pos2, selected: bool) {
    let visuals = ui.style().interact_selectable(response, selected);

    let color = if selected {
        visuals.bg_fill
    } else {
        ui.style().visuals.strong_text_color()
    };

    ui.painter().galley(pos, text, color);
}

// Adopted from https://github.com/emilk/egui/blob/master/crates/egui_demo_lib/src/demo/toggle_switch.rs
fn paint_toggle(ui: &mut Ui, response: &Response, rect: Rect, on: bool) {
    let visuals = ui.style().interact_selectable(response, true);

    let radius = 0.5 * rect.height();
    ui.painter()
        .rect(rect, radius, visuals.bg_fill, visuals.bg_stroke);

    let how_on = ui.ctx().animate_bool(response.id, on);
    let circle_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
    let center = egui::pos2(circle_x, rect.center().y);
    ui.painter()
        .circle(center, 0.75 * radius, visuals.bg_fill, visuals.fg_stroke);
}

fn text_offset(text_size: Vec2, max_text_size: Vec2) -> Vec2 {
    vec2(text_size.x / 2. + (max_text_size.x - text_size.x), 0.)
}

struct AllocatedSpace {
    rect: Rect,
    text_left: Arc<Galley>,
    text_right: Arc<Galley>,
    max_text_size: Vec2,
    toggle_size: Vec2,
}

fn max_size(v1: Vec2, v2: Vec2) -> Vec2 {
    vec2(partial_max(v1.x, v2.x), partial_max(v1.y, v2.y))
}

fn partial_max<T>(v1: T, v2: T) -> T
where
    T: PartialOrd,
{
    if v2 > v1 {
        v2
    } else {
        v1
    }
}
