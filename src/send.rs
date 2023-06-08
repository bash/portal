use crate::egui_ext::ContextExt;
use crate::font::{ICON_CHECK, ICON_CLIPBOARD_COPY, ICON_LINK, ICON_TICKET, ICON_UPLOAD, ICON_X};
use crate::transit_info::TransitInfoDisplay;
use crate::update;
use crate::widgets::{
    cancel_button, page, page_with_content, CancelLabel, PrimaryButton, MIN_BUTTON_SIZE,
};
use eframe::egui::{Button, Key, Modifiers, ProgressBar, Ui};
use egui::{InputState, RichText};
use portal_proc_macro::states;
use portal_wormhole::send::{send, SendRequest, SendingController, SendingProgress};
use portal_wormhole::{Code, PortalError, Progress, SharableWormholeTransferUri};
use rfd::{AsyncFileDialog, FileHandle};
use std::fmt;
use std::future::Future;
use std::path::{Path, PathBuf};

states! {
    pub enum SendView;

    state Ready();

    async state SelectingFile() -> Option<Vec<FileHandle>> {
        new(pick_future: impl Future<Output = Option<Vec<FileHandle>>> + Send + 'static) {
            (Box::pin(pick_future),)
        }
        next {
            None => Ready(),
            Some(paths) => {
                if let Some(request) = SendRequest::from_paths(paths.into_iter().map(|p| p.path().to_owned()).collect()) {
                    SendView::new_sending(ui, request)
                } else {
                    Ready()
                }
            }
        }
    }

    async state Sending(controller: SendingController, request: SendRequest) -> Result<(), (PortalError, SendRequest)> {
        new(request: SendRequest) {
            let ctx = ui.ctx().clone();
            let (future, controller) = send(request.clone(), move || ctx.request_repaint());
            (Box::pin(future), controller, request)
        }
        next {
            Ok(_) => Complete(request),
            Err((PortalError::Canceled, _)) => SendView::default(),
            Err((error, send_request)) => Error(error, send_request),
        }
    }

    state Error(error: PortalError, send_request: SendRequest);

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
            SendView::Ready() | SendView::SelectingFile(..) => self.show_file_selection_page(ui),
            SendView::Sending(_, ref mut controller, ref send_request) => {
                show_transfer_progress(ui, controller, send_request)
            }
            SendView::Error(ref error, _) => self.show_error_page(ui, error.to_string()),
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
            ICON_UPLOAD,
            |ui| self.show_file_selection(ui),
        );
    }

    fn show_file_selection(&mut self, ui: &mut Ui) {
        let select_file_button = PrimaryButton::new("Select File").min_size(MIN_BUTTON_SIZE);
        if ui.add(select_file_button).clicked()
            || ui.input_mut(|input| input.consume_key(Modifiers::COMMAND, Key::O))
        {
            *self = SendView::new_selecting_file(ui, AsyncFileDialog::new().pick_files());
        }

        ui.add_space(5.0);

        let select_folder_button = Button::new("Select Folder").min_size(MIN_BUTTON_SIZE);
        if ui.add(select_folder_button).clicked() {
            *self = SendView::new_selecting_file(ui, AsyncFileDialog::new().pick_folders());
        }
    }

    fn show_error_page(&mut self, ui: &mut Ui, error: String) {
        self.back_button(ui);

        page_with_content(ui, "File Transfer Failed", error, ICON_X, |ui| {
            if ui.button("Retry").clicked() {
                update!(
                    self,
                    SendView::Error(_, send_request) => SendView::new_sending(ui, send_request)
                );
            }
        });
    }

    fn show_transfer_completed_page(&mut self, ui: &mut Ui, send_request: SendRequest) {
        let request_title = SendRequestDisplay(&send_request).to_string();
        self.back_button(ui);
        page(
            ui,
            "File Transfer Successful",
            format!("Successfully sent {request_title}"),
            ICON_CHECK,
        );
    }

    fn accept_dropped_file(&mut self, ui: &mut Ui) {
        if ui.is_enabled() {
            let dropped_file_paths: Vec<_> = ui.ctx().input(dropped_file_paths);

            if let Some(send_request) = SendRequest::from_paths(dropped_file_paths) {
                *self = SendView::new_sending(ui, send_request)
            }
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
        SendingProgress::Packing => show_packing_progress(ui, send_request),
        SendingProgress::Connecting => show_transmit_code_progress(ui),
        SendingProgress::Connected(code) => show_transmit_code(ui, code, send_request),
        SendingProgress::PreparingToSend => page_with_content(
            ui,
            "Connected to Peer",
            format!("Preparing to send {}", SendRequestDisplay(send_request)),
            ICON_UPLOAD,
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
                ICON_UPLOAD,
                |ui| {
                    ui.add(ProgressBar::new((*sent as f64 / *total as f64) as f32).animate(true));
                },
            )
        }
    }
}

fn show_packing_progress(ui: &mut Ui, send_request: &SendRequest) {
    page_with_content(
        ui,
        "Send File",
        format!(
            "Packing {} to a Zip file...",
            SendRequestDisplay(send_request)
        ),
        ICON_UPLOAD,
        |ui| {
            ui.spinner();
        },
    )
}

fn show_transmit_code_progress(ui: &mut Ui) {
    page_with_content(
        ui,
        "Send File",
        "Generating transmit code...",
        ICON_UPLOAD,
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
        ICON_TICKET,
        |ui| {
            ui.label(RichText::new(&code.0).size(15.).strong());
            ui.add_space(5.);
            if ui
                .button(format!("{ICON_CLIPBOARD_COPY} Copy Code"))
                .on_hover_text("Click to copy")
                .clicked()
                || ui.input_mut(|input| input.consume_key(Modifiers::COMMAND, Key::C))
            {
                ui.output_mut(|output| output.copied_text = code.0.clone());
            }

            if ui
                .button(format!("{ICON_LINK} Copy Link"))
                .on_hover_text("Click to copy")
                .clicked()
            {
                ui.output_mut(|output| {
                    output.copied_text = SharableWormholeTransferUri::new(code.clone()).to_string();
                });
            }
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
            SendRequest::Selection(_) => write!(f, "selection"),
            SendRequest::Cached(original_request, _) => {
                write!(f, "{}", SendRequestDisplay(original_request))
            }
        }
    }
}

fn filename_or_self(path: &Path) -> &Path {
    path.file_name().map(Path::new).unwrap_or(path)
}
