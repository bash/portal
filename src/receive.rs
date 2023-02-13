use crate::egui_ext::ContextExt;
use crate::transmit_info::transit_info_message;
use crate::update;
use crate::widgets::{cancel_button, page, page_with_content, CancelLabel};
use eframe::{
    egui::{Button, ProgressBar, TextEdit, Ui},
    epaint::Vec2,
};
use magic_wormhole::{transfer::ReceiveRequest, Code};
use portal_proc_macro::states;
use portal_wormhole::receive::{
    ConnectResult, ConnectingController, ReceiveProgress, ReceiveResult, ReceivingController,
};
use portal_wormhole::PortalError;
use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct ReceiveView {
    state: ReceiveState,
}

impl Default for ReceiveState {
    fn default() -> Self {
        ReceiveState::Initial(String::default())
    }
}

states! {
    enum ReceiveState;

    state Initial(code: String);

    async state Connecting(controller: ConnectingController) -> ConnectResult {
        new(code: Code) { ConnectingController::new(code) }
        next {
            Ok(receive_request) => Connected(receive_request),
            Err(PortalError::Canceled) => Default::default(),
            Err(error) => Error(error),
        }
    }

    state Connected(request: ReceiveRequest);

    async state Rejecting() -> Result<(), PortalError> {
        new(request: ReceiveRequest) { (reject(request),) }
        next {
            Ok(()) => Default::default(),
            Err(error) => Error(error),
        }
    }

    async state Receiving(controller: ReceivingController, filename: PathBuf) -> ReceiveResult {
        new(receive_request: ReceiveRequest) {
            let filename = receive_request.filename.clone();
            let (future, controller) = ReceivingController::new(receive_request);
            (future, controller, filename)
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
            ReceiveState::Connecting(_, controller) => {
                show_connecting_page(ui, controller);
            }
            ReceiveState::Error(error) => {
                let error = error.to_string();
                self.back_button(ui);
                page(ui, "File Transfer Failed", error, "❌");
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
                page_with_content(ui, "Receive File", "Rejecting File Transfer", "📥", |ui| {
                    ui.spinner();
                });
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
        "Enter the code from your peer below:",
        "📥",
        |ui| {
            ui.add(TextEdit::singleline(code).hint_text("Code"));
            ui.add_space(5.0);

            let input_empty = code.is_empty() || code.chars().all(|c| c.is_whitespace());

            let min_button_size = Vec2::new(100.0, 0.0);

            ui.add_enabled_ui(!input_empty, |ui| {
                if ui
                    .add(Button::new("Receive File").min_size(min_button_size))
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

fn show_connecting_page(ui: &mut Ui, controller: &mut ConnectingController) {
    if cancel_button(ui, CancelLabel::Cancel) {
        controller.cancel();
    }

    page_with_content(
        ui,
        "Connecting with peer",
        "Preparing to Receive File",
        "📥",
        |ui| {
            ui.spinner();
        },
    );
}

fn show_connected_page(
    ui: &mut Ui,
    _receive_request: &ReceiveRequest,
) -> Option<ConnectedPageResponse> {
    page_with_content(ui, "Receive File", "TODO", "📥", |ui| {
        if ui.button("Reject").clicked() {
            return Some(ConnectedPageResponse::Reject);
        }

        ui.add_space(5.0);

        if ui.button("Accept").clicked() {
            return Some(ConnectedPageResponse::Accept);
        }

        None
    })
}

fn show_receiving_page(ui: &mut Ui, controller: &mut ReceivingController, filename: &Path) {
    let ReceiveProgress { received, total } = *controller.progress();

    if cancel_button(ui, CancelLabel::Cancel) {
        controller.cancel();
    }

    match controller.transit_info() {
        Some(transit_info) => page_with_content(
            ui,
            "Receiving File",
            transit_info_message(transit_info, filename.as_os_str()),
            "📥",
            |ui| {
                ui.add(ProgressBar::new((received as f64 / total as f64) as f32).animate(true));
            },
        ),
        None => page_with_content(
            ui,
            "Connected to Peer",
            format!("Preparing to receive file \"{}\"", filename.display()),
            "📥",
            |ui| {
                ui.spinner();
            },
        ),
    }
}

fn show_completed_page(ui: &mut Ui, downloaded_path: &Path) -> Option<CompletedPageResponse> {
    if cancel_button(ui, CancelLabel::Back) {
        return Some(CompletedPageResponse::Back);
    }

    ui.label("Completed");

    if ui.button("Open File").clicked() {
        _ = opener::open(downloaded_path);
    }

    if ui.button("Show in Folder").clicked() {
        _ = crate::utils::open_file_in_folder(downloaded_path);
    }

    None
}

#[must_use]
enum CompletedPageResponse {
    Back,
}

async fn reject(receive_request: ReceiveRequest) -> Result<(), PortalError> {
    receive_request.reject().await.map_err(Into::into)
}
