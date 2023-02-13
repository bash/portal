use crate::egui_ext::ContextExt;
use crate::transmit_info::transit_info_message;
use crate::widgets::{cancel_button, page, page_with_content, CancelLabel};
use eframe::{
    egui::{Button, Key, Modifiers, ProgressBar, Ui},
    epaint::Vec2,
};
use portal_proc_macro::states;
use portal_wormhole::send::{send, SendRequest, SendingController, SendingProgress};
use portal_wormhole::{Code, PortalError, Progress};
use rfd::FileDialog;
use std::path::Path;

states! {
    pub enum SendView;

    state Ready();

    async state Sending(controller: SendingController, request: SendRequest) -> Result<(), PortalError> {
        new(request: SendRequest) {
            let ctx = ui.ctx().clone();
            let (future, controller) = send(request.clone(), move || ctx.request_repaint());
            (future, controller, request)
        }
        next {
            Ok(_) => Complete(request),
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
        let min_button_size = Vec2::new(100.0, 0.0);

        let select_file_button = Button::new("Select File").min_size(min_button_size);
        if ui.add(select_file_button).clicked()
            || ui.input_mut(|input| input.consume_key(Modifiers::COMMAND, Key::O))
        {
            if let Some(file_path) = FileDialog::new().pick_file() {
                *self = SendView::new_sending(ui, SendRequest::File(file_path))
            }
        }

        ui.add_space(5.0);

        let select_folder_button = Button::new("Select Folder").min_size(min_button_size);
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
        let filename = send_request.path().file_name().unwrap();
        self.back_button(ui);
        page(
            ui,
            "File Transfer Successful",
            format!("Successfully sent file \"{}\"", filename.to_string_lossy()),
            "✅",
        );
    }

    fn accept_dropped_file(&mut self, ui: &mut Ui) {
        let file_path = ui
            .ctx()
            .input(|input| input.raw.dropped_files.iter().find_map(|f| f.path.clone()));
        if let Some(file_path) = file_path {
            *self = SendView::new_sending(ui, SendRequest::File(file_path))
        }
    }

    fn back_button(&mut self, ui: &mut Ui) {
        if cancel_button(ui, CancelLabel::Back) {
            *self = SendView::default();
        }
    }
}

fn show_transfer_progress(
    ui: &mut Ui,
    controller: &mut SendingController,
    send_request: &SendRequest,
) {
    let filename = send_request.path().file_name().unwrap();
    match controller.progress() {
        SendingProgress::Connecting => show_transmit_code_progress(ui),
        SendingProgress::Connected(code) => show_transmit_code(ui, code, send_request.path()),
        SendingProgress::PreparingToSend => page_with_content(
            ui,
            "Connected to Peer",
            format!("Preparing to send file \"{}\"", filename.to_string_lossy()),
            "📤",
            |ui| {
                ui.spinner();
            },
        ),
        SendingProgress::Sending(transit_info, Progress { value: sent, total }) => {
            page_with_content(
                ui,
                "Sending File",
                transit_info_message(transit_info, filename),
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

fn show_transmit_code(ui: &mut Ui, code: &Code, file_path: &Path) {
    page_with_content(
        ui,
        "Your Transmit Code",
        format!(
            "Ready to send \"{}\".\nThe receiver needs to enter this code to begin the file transfer.",
            file_path.file_name().unwrap().to_string_lossy()
        ),
        "✨",
        |ui| {
            ui.horizontal(|ui| {
                ui.label(&code.0);
                if ui.button("📋").on_hover_text("Click to copy").clicked() {
                    ui.output_mut(|output| output.copied_text = code.0.clone());
                }
            });
        }
    );
}
