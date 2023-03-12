use crate::egui_ext::ContextExt;
use crate::font::{ICON_CHECK, ICON_DOWNLOAD, ICON_X};
use crate::shell::open;
use crate::transit_info::TransitInfoDisplay;
use crate::update;
use crate::widgets::{
    cancel_button, page, page_with_content, CancelLabel, PrimaryButton, MIN_BUTTON_SIZE,
};
use eframe::egui::{Button, ProgressBar, TextEdit, Ui};
use egui::Key;
use opener::reveal;
use portal_proc_macro::states;
use portal_wormhole::receive::{
    connect, ConnectResult, ConnectingController, ReceiveRequestController, ReceiveResult,
    ReceivingController,
};
use portal_wormhole::{Code, PortalError, Progress, TransitInfo, WormholeTransferUri};
use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use ubyte::{ByteUnit, ToByteUnit};

#[derive(Default)]
pub struct ReceiveView {
    state: ReceiveState,
}

impl ReceiveView {
    pub fn new_with_uri(uri: String) -> Self {
        // TODO: handle error, is_leader and custom rendezvous_server
        let code = WormholeTransferUri::from_str(&uri)
            .map(|uri| uri.code.0)
            .unwrap_or_default();
        Self {
            state: ReceiveState::Initial(code),
        }
    }
}

impl Default for ReceiveState {
    fn default() -> Self {
        ReceiveState::Initial(String::default())
    }
}

states! {
    enum ReceiveState;

    state Initial(code: String);

    async state Connecting(controller: ConnectingController, code: Code) -> ConnectResult {
        new(code: Code) {
            let (future, controller) = connect(code.clone());
            (future, controller, code)
        }
        next {
            Ok(receive_request) => Connected(receive_request),
            Err(PortalError::Canceled) => Default::default(),
            Err(error) => Error(error),
        }
    }

    state Connected(controller: ReceiveRequestController);

    async state Rejecting() -> Result<(), PortalError> {
        new(request: ReceiveRequestController) { (request.reject(),) }
        next {
            Ok(()) => Default::default(),
            Err(error) => Error(error),
        }
    }

    async state Receiving(controller: ReceivingController, filename: PathBuf) -> ReceiveResult {
        new(receive_request: ReceiveRequestController) {
            let filename = receive_request.filename().to_owned();
            let ctx = ui.ctx().clone();
            let (future, controller) = receive_request.accept(move || ctx.request_repaint());
            (Box::pin(future), controller, filename)
        }
        next {
            Ok(path) => Completed(path),
            Err(PortalError::Canceled) => Default::default(),
            Err(error) => Error(error),
        }
    }

    state Error(error: PortalError);

    state Completed(path: PathBuf);
}

impl ReceiveView {
    pub fn show_switcher(&self) -> bool {
        matches!(self.state, ReceiveState::Initial(_))
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        self.state.next(ui);

        match &mut self.state {
            ReceiveState::Initial(ref mut code) => {
                if let Some(ReceivePageResponse::Connect) = show_receive_file_page(ui, code) {
                    update! {
                        &mut self.state,
                        ReceiveState::Initial(code) => ReceiveState::new_connecting(ui, Code(code))
                    }
                }
            }
            ReceiveState::Connecting(_, controller, code) => {
                show_connecting_page(ui, controller, code);
            }
            ReceiveState::Error(error) => {
                let error = error.to_string();
                self.back_button(ui);
                page(ui, "File Transfer Failed", error, ICON_X);
            }
            ReceiveState::Connected(ref receive_request) => {
                if let Some(response) = show_connected_page(ui, receive_request) {
                    update! {
                        &mut self.state,
                        ReceiveState::Connected(receive_request) => match response {
                            ConnectedPageResponse::Accept => ReceiveState::new_receiving(ui, receive_request),
                            ConnectedPageResponse::Reject => ReceiveState::new_rejecting(ui, receive_request),
                        }
                    }
                }
            }
            ReceiveState::Receiving(_, ref mut controller, ref filename) => {
                show_receiving_page(ui, controller, filename);
            }
            ReceiveState::Rejecting(_) => {
                page_with_content(
                    ui,
                    "Receive File",
                    "Rejecting File Transfer",
                    ICON_DOWNLOAD,
                    |ui| {
                        ui.spinner();
                    },
                );
            }
            ReceiveState::Completed(downloaded_path) => {
                if let Some(CompletedPageResponse::Back) = show_completed_page(ui, downloaded_path)
                {
                    self.state = ReceiveState::default();
                }
            }
        }
    }

    fn back_button(&mut self, ui: &mut Ui) {
        if cancel_button(ui, CancelLabel::Back) {
            self.state = ReceiveState::default();
        }
    }
}

#[must_use]
enum ReceivePageResponse {
    Connect,
}

fn show_receive_file_page(ui: &mut Ui, code: &mut String) -> Option<ReceivePageResponse> {
    page_with_content(
        ui,
        "Receive File",
        "Enter the transmit code from the sender",
        ICON_DOWNLOAD,
        |ui| {
            if ui
                .add(TextEdit::singleline(code).hint_text("Code"))
                .lost_focus()
                && ui.input(|input| input.key_pressed(Key::Enter))
            {
                return Some(ReceivePageResponse::Connect);
            }
            ui.add_space(5.0);

            let input_empty = code.is_empty() || code.chars().all(|c| c.is_whitespace());

            ui.add_enabled_ui(!input_empty, |ui| {
                if ui
                    .add(PrimaryButton::new("Receive File").min_size(MIN_BUTTON_SIZE))
                    .clicked()
                {
                    Some(ReceivePageResponse::Connect)
                } else {
                    None
                }
            })
            .inner
        },
    )
}

#[must_use]
enum ConnectedPageResponse {
    Accept,
    Reject,
}

fn show_connecting_page(ui: &mut Ui, controller: &mut ConnectingController, code: &Code) {
    if cancel_button(ui, CancelLabel::Cancel) {
        controller.cancel();
    }

    page_with_content(
        ui,
        "Receive File",
        format!("Connecting with peer using transfer code \"{code}\""),
        ICON_DOWNLOAD,
        |ui| {
            ui.spinner();
        },
    );
}

fn show_connected_page(
    ui: &mut Ui,
    receive_request: &ReceiveRequestController,
) -> Option<ConnectedPageResponse> {
    if cancel_button(ui, CancelLabel::Cancel) {
        return Some(ConnectedPageResponse::Reject);
    }

    let text = format!(
        "Your peer wants to send you \"{}\" (Size: {}). Do you want to download this file?",
        receive_request.filename().display(),
        ByteDisplay(receive_request.filesize().bytes())
    );

    page_with_content(ui, "Receive File", text, ICON_DOWNLOAD, |ui| {
        if ui
            .add(PrimaryButton::new("Accept").min_size(MIN_BUTTON_SIZE))
            .clicked()
        {
            return Some(ConnectedPageResponse::Accept);
        }

        None
    })
}

fn show_receiving_page(ui: &mut Ui, controller: &mut ReceivingController, filename: &Path) {
    let Progress {
        value: received,
        total,
    } = *controller.progress();

    if cancel_button(ui, CancelLabel::Cancel) {
        controller.cancel();
    }

    match controller.transit_info() {
        Some(transit_info) => page_with_content(
            ui,
            "Receiving File",
            transit_info_message(transit_info, filename),
            ICON_DOWNLOAD,
            |ui| {
                ui.add(ProgressBar::new((received as f64 / total as f64) as f32).animate(true));
            },
        ),
        None => page_with_content(
            ui,
            "Connected to Peer",
            format!("Preparing to receive file \"{}\"", filename.display()),
            ICON_DOWNLOAD,
            |ui| {
                ui.spinner();
            },
        ),
    }
}

fn transit_info_message(transit_info: &TransitInfo, filename: &Path) -> String {
    format!(
        "File \"{}\"{}",
        filename.display(),
        TransitInfoDisplay(transit_info)
    )
}

fn show_completed_page(ui: &mut Ui, downloaded_path: &Path) -> Option<CompletedPageResponse> {
    if cancel_button(ui, CancelLabel::Back) {
        return Some(CompletedPageResponse::Back);
    }

    let filename = downloaded_path.file_name().unwrap();

    page_with_content(
        ui,
        "File Transfer Successful",
        format!(
            "File \"{}\" has been saved to your Downloads folder",
            filename.to_string_lossy()
        ),
        ICON_CHECK,
        |ui| {
            if ui
                .add(PrimaryButton::new("Open File").min_size(MIN_BUTTON_SIZE))
                .clicked()
            {
                _ = open(downloaded_path);
            }

            ui.add_space(5.0);

            if ui
                .add(Button::new("Show in Folder").min_size(MIN_BUTTON_SIZE))
                .clicked()
            {
                _ = reveal(downloaded_path);
            }
        },
    );

    None
}

#[must_use]
enum CompletedPageResponse {
    Back,
}

struct ByteDisplay(ByteUnit);

// Same as https://github.com/SergioBenitez/ubyte/blob/master/src/byte_unit.rs#L442
// except with a space between value and suffix.
impl fmt::Display for ByteDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const NO_BREAK_SPACE: &str = "\u{00A0}";
        let (whole, rem, suffix, unit) = self.0.repr();
        let width = f.width().unwrap_or(0);
        if rem != 0f64 && f.precision().map(|p| p > 0).unwrap_or(true) {
            let p = f.precision().unwrap_or(2);
            let k = 10u64.saturating_pow(p as u32) as f64;
            write!(
                f,
                "{:0width$}.{:0p$.0}{NO_BREAK_SPACE}{}",
                whole,
                rem * k,
                suffix,
            )
        } else if rem > 0.5f64 {
            ((whole.bytes() + 1) * unit).fmt(f)
        } else {
            write!(f, "{whole:0width$}{NO_BREAK_SPACE}{suffix}")
        }
    }
}
