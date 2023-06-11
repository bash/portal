use crate::font::{ICON_DOWNLOAD, ICON_UPLOAD};
use crate::widgets::toggle;
use crate::{ReceiveFileAction, ReceiveView, SendView};
use egui::{hex_color, RichText, Ui};

#[derive(Default)]
pub(crate) struct MainViewState {
    send_view: SendView,
    receive_view: ReceiveView,
    view_toggle: bool,
}

impl From<ReceiveFileAction> for MainViewState {
    fn from(value: ReceiveFileAction) -> Self {
        MainViewState {
            receive_view: ReceiveView::new(value),
            view_toggle: true,
            ..Default::default()
        }
    }
}

pub(crate) fn show_main_view(state: &mut MainViewState, ui: &mut Ui, frame: &mut eframe::Frame) {
    let view = View::from(state.view_toggle);

    apply_style_overrides(view, ui.style_mut());

    ui.add_enabled_ui(ui_enabled(state, view), |ui| {
        if show_switcher(state, view) {
            let font_size = 14.;
            ui.add_space(12.);
            ui.add(toggle(
                &mut state.view_toggle,
                RichText::new(format!("{ICON_UPLOAD} Send")).size(font_size),
                RichText::new(format!("{ICON_DOWNLOAD} Receive")).size(font_size),
            ));
        }

        state_ui(state, view, ui, frame);
    });
}

fn apply_style_overrides(view: View, style: &mut egui::Style) {
    let (fill, stroke) = match view {
        View::Send if style.visuals.dark_mode => (hex_color!("#DB8400"), hex_color!("#38270E")),
        View::Send => (hex_color!("#FF9D0A"), hex_color!("#523A16")),
        View::Receive if style.visuals.dark_mode => (hex_color!("#27A7D8"), hex_color!("#183039")),
        View::Receive => (hex_color!("#73CDF0"), hex_color!("#183039")),
    };
    style.visuals.selection.bg_fill = fill;
    style.visuals.selection.stroke.color = stroke;
}

fn show_switcher(state: &MainViewState, view: View) -> bool {
    match view {
        View::Send => matches!(
            state.send_view,
            SendView::Ready(..) | SendView::SelectingFile(..)
        ),
        View::Receive => state.receive_view.show_switcher(),
    }
}

fn ui_enabled(state: &MainViewState, view: View) -> bool {
    match view {
        View::Send => !matches!(state.send_view, SendView::SelectingFile(..)),
        View::Receive => true,
    }
}

fn state_ui(state: &mut MainViewState, view: View, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
    match view {
        View::Send => state.send_view.ui(ui, frame),
        View::Receive => state.receive_view.ui(ui),
    }
}

#[derive(PartialEq, Copy, Clone)]
enum View {
    Send,
    Receive,
}

impl From<bool> for View {
    fn from(value: bool) -> Self {
        if value {
            View::Receive
        } else {
            View::Send
        }
    }
}
