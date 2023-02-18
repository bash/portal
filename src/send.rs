use crate::egui_ext::ContextExt;
use crate::transit_info::TransitInfoDisplay;
use crate::widgets::{cancel_button, page, page_with_content, CancelLabel, MIN_BUTTON_SIZE};
use eframe::egui::{Button, Key, Modifiers, ProgressBar, Ui};
use egui::InputState;
use portal_proc_macro::states;
use portal_wormhole::send::{send, SendRequest, SendingController, SendingProgress};
use portal_wormhole::{Code, PortalError, Progress};
use rfd::FileDialog;
use std::fmt;
use std::path::{Path, PathBuf};

states! {
    pub enum SendView;

    state Ready();

    async state Sending(controller: SendingController, request: SendRequest) -> Result<(), PortalError> {
        new(request: SendRequest) {
            let ctx = ui.ctx().clone();
            let (future, controller) = send(request.clone(), move || ctx.request_repaint());
            (Box::pin(future), controller, request)
        }
        next {
            Ok(_) => Complete(request),
            Err(PortalError::Canceled) => SendView::default(),
            Err(error) => Error(error),
        }
    }

    state Error(error: PortalError);

    state Complete(request: SendRequest);
}

impl Default for SendView {
    fn default() -> Self {
        SendView::Ready()
    }
}

impl SendView {
    pub fn ui(&mut self, ui: &mut Ui) {
        self.next(ui);

        if let SendView::Ready() | SendView::Complete(..) = self {
            self.accept_dropped_file(ui);
        }

        match self {
            SendView::Ready() => self.show_file_selection_page(ui),
            SendView::Sending(_, ref mut controller, ref send_request) => {
                show_transfer_progress(ui, controller, send_request)
            }
            SendView::Error(ref error) => self.show_error_page(ui, error.to_string()),
            SendView::Complete(ref send_request) => {
                self.show_transfer_completed_page(ui, send_request.clone())
            }
        }
    }

    fn show_file_selection_page(&mut self, ui: &mut Ui) {
        page_with_content(
            ui,
            "Send File",
            "Select or drop the file or directory to send.",
            "📤",
            |ui| self.show_file_selection(ui),
        );
    }

    fn show_file_selection(&mut self, ui: &mut Ui) {
        let select_file_button = Button::new("Select File").min_size(MIN_BUTTON_SIZE);
        if ui.add(select_file_button).clicked()
            || ui.input_mut(|input| input.consume_key(Modifiers::COMMAND, Key::O))
        {
            if let Some(file_path) = FileDialog::new().pick_file() {
                *self = SendView::new_sending(ui, SendRequest::File(file_path))
            }
        }

        ui.add_space(5.0);

        let select_folder_button = Button::new("Select Folder").min_size(MIN_BUTTON_SIZE);
        if ui.add(select_folder_button).clicked() {
            if let Some(folder_path) = FileDialog::new().pick_folder() {
                *self = SendView::new_sending(ui, SendRequest::Folder(folder_path))
            }
        }
    }

    fn show_error_page(&mut self, ui: &mut Ui, error: String) {
        self.back_button(ui);
        page(ui, "File Transfer Failed", error, "❌");
    }

    fn show_transfer_completed_page(&mut self, ui: &mut Ui, send_request: SendRequest) {
        let request_title = SendRequestDisplay(&send_request).to_string();
        self.back_button(ui);
        page(
            ui,
            "File Transfer Successful",
            format!("Successfully sent {request_title}"),
            "✅",
        );
    }

    fn accept_dropped_file(&mut self, ui: &mut Ui) {
        let dropped_file_paths: Vec<_> = ui.ctx().input(dropped_file_paths);
        if let Some(send_request) = SendRequest::from_paths(dropped_file_paths) {
            *self = SendView::new_sending(ui, send_request)
        }
    }

    fn back_button(&mut self, ui: &mut Ui) {
        if cancel_button(ui, CancelLabel::Back) {
            *self = SendView::default();
        }
    }
}

fn dropped_file_paths(input: &InputState) -> Vec<PathBuf> {
    input
        .raw
        .dropped_files
        .iter()
        .filter_map(|f| f.path.clone())
        .collect()
}

fn show_transfer_progress(
    ui: &mut Ui,
    controller: &mut SendingController,
    send_request: &SendRequest,
) {
    if cancel_button(ui, CancelLabel::Cancel) {
        controller.cancel();
    }

    match controller.progress() {
        SendingProgress::Connecting => show_transmit_code_progress(ui),
        SendingProgress::Connected(code) => show_transmit_code(ui, code, send_request),
        SendingProgress::PreparingToSend => page_with_content(
            ui,
            "Connected to Peer",
            format!("Preparing to send {}", SendRequestDisplay(send_request)),
            "📤",
            |ui| {
                ui.spinner();
            },
        ),
        SendingProgress::Sending(transit_info, Progress { value: sent, total }) => {
            page_with_content(
                ui,
                "Sending File",
                format!(
                    "{}{}",
                    SendRequestDisplay(send_request),
                    TransitInfoDisplay(transit_info)
                ),
                "📤",
                |ui| {
                    ui.add(ProgressBar::new((*sent as f64 / *total as f64) as f32).animate(true));
                },
            )
        }
    }
}

fn show_transmit_code_progress(ui: &mut Ui) {
    page_with_content(
        ui,
        "Send File",
        "Generating transmit code...",
        "📤",
        |ui| {
            ui.spinner();
        },
    )
}

fn show_transmit_code(ui: &mut Ui, code: &Code, send_request: &SendRequest) {
    page_with_content(
        ui,
        "Your Transmit Code",
        format!(
            "Ready to send {}.\nThe receiver needs to enter this code to begin the file transfer.",
            SendRequestDisplay(send_request)
        ),
        "✨",
        |ui| {
            ui.horizontal(|ui| {
                ui.label(&code.0);
                if ui.button("📋").on_hover_text("Click to copy").clicked() {
                    ui.output_mut(|output| output.copied_text = code.0.clone());
                }
            });
        },
    );
}

struct SendRequestDisplay<'a>(&'a SendRequest);

impl<'a> fmt::Display for SendRequestDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            SendRequest::File(path) => write!(f, "file \"{}\"", filename_or_self(path).display()),
            SendRequest::Folder(path) => {
                write!(f, "folder \"{}\"", filename_or_self(path).display())
            }
        }
    }
}

fn filename_or_self(path: &Path) -> &Path {
    path.file_name().map(Path::new).unwrap_or(path)
}
