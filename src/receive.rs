use std::{
    ffi::OsString,
    future::{self},
    path::PathBuf,
};

use crate::{error::PortalError, update};
use async_std::fs::File;
use eframe::{
    egui::{Button, ProgressBar, TextEdit, Ui},
    epaint::Vec2,
};
use futures::channel::oneshot;
use magic_wormhole::{
    transfer::{self, ReceiveRequest},
    transit::{self, Abilities, TransitInfo},
    Code, Wormhole,
};
use poll_promise::Promise;
use portal_proc_macro::states;
use single_value_channel as svc;

use crate::egui_ext::ContextExt;

#[derive(Default)]
pub struct ReceiveView {
    state: ReceiveState,
}

#[derive(Default)]
struct Progress {
    received: u64,
    total: u64,
}

impl Default for ReceiveState {
    fn default() -> Self {
        ReceiveState::Initial(String::default())
    }
}

states! {
    enum ReceiveState;

    state Initial(code: String);

    state Connecting() {
        execute(code: Code) -> Result<ReceiveRequest, PortalError> { connect(code).await }
        next {
            Ok(receive_request) => Connected(receive_request),
            Err(error) => Error(error),
        }
    }

    state Connected(request: ReceiveRequest);

    state Rejecting() {
        execute(request: ReceiveRequest) -> Result<(), PortalError> { reject(request).await }
        next {
            Ok(()) => Default::default(),
            Err(error) => Error(error),
        }
    }

    state Receiving(transit_info: Promise<TransitInfo>, progress: svc::Receiver<Progress>, cancel: Option<oneshot::Sender<()>>) {
        execute(
            request: ReceiveRequest,
            transit_info_sender: poll_promise::Sender<TransitInfo>,
            progress_updater: svc::Updater<Progress>,
            cancel_receiver: oneshot::Receiver<()>) -> Result<PathBuf, PortalError>
        {
            accept(request, transit_info_sender, progress_updater, cancel_receiver).await
        }
        next {
            Ok(path) => Completed(path),
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
            ReceiveState::Initial(ref mut code) => match show_receive_file_page(ui, code) {
                None => {}
                Some(ReceivePageResponse::Connect) => {
                    update! {
                        &mut self.state,
                        ReceiveState::Initial(code) => ReceiveState::new_connecting(ui, Code(code))
                    }
                }
            },
            ReceiveState::Connecting(_) => {
                crate::page_with_content(
                    ui,
                    "Connecting with peer",
                    "Preparing to Receive File",
                    "📥",
                    |ui| {
                        ui.spinner();
                    },
                );
            }
            ReceiveState::Error(error) => {
                let error = error.to_string();
                ui.horizontal(|ui| {
                    if ui.button("Back").clicked() {
                        self.state = ReceiveState::default();
                    }
                });

                crate::page(ui, "File Transfer Failed", error, "❌");
            }
            ReceiveState::Connected(ref receive_request) => {
                match show_connected_page(ui, receive_request) {
                    None => {}
                    Some(ConnectedPageResponse::Accept) => {
                        let (transit_sender, transit_promise) = Promise::new();
                        let (progress, progress_updater) =
                            svc::channel_starting_with(Progress::default());
                        let (cancel_sender, cancel_receiver) = oneshot::channel();
                        update! {
                            &mut self.state,
                            ReceiveState::Connected(receive_request) => ReceiveState::new_receiving(
                                ui,
                                receive_request,
                                transit_sender,
                                progress_updater,
                                cancel_receiver,
                                transit_promise,
                                progress,
                                Some(cancel_sender))
                        }
                    }
                    Some(ConnectedPageResponse::Reject) => {
                        update! {
                            &mut self.state,
                            ReceiveState::Connected(receive_request) => ReceiveState::new_rejecting(ui, receive_request)
                        }
                    }
                }
            }
            ReceiveState::Receiving(_, ref transit_info, ref mut progress, ref mut cancel) => {
                let Progress { received, total } = *progress.latest();

                if ui.horizontal(|ui| ui.button("Cancel").clicked()).inner {
                    cancel.take().map(|cancel| cancel.send(()));
                }

                match transit_info.ready() {
                    Some(_transit_info) => {
                        // TODO: show transit info
                        crate::page_with_content(ui, "Sending File", "TODO", "📥", |ui| {
                            ui.add(
                                ProgressBar::new((received as f64 / total as f64) as f32)
                                    .animate(true),
                            );
                        })
                    }
                    None => crate::page_with_content(
                        ui,
                        "Connected to Peer",
                        format!("Preparing to receive file \"{}\"", "TODO"),
                        "📥",
                        |ui| {
                            ui.spinner();
                        },
                    ),
                }
            }
            ReceiveState::Rejecting(_) => {
                crate::page_with_content(
                    ui,
                    "Receive File",
                    "Rejecting File Transfer",
                    "📥",
                    |ui| {
                        ui.spinner();
                    },
                );
            }
            ReceiveState::Completed(downloaded_path) => {
                let downloaded_path = downloaded_path.clone();
                ui.horizontal(|ui| {
                    if ui.button("Back").clicked() {
                        self.state = ReceiveState::default();
                    }
                });
                ui.label("Completed");

                if ui.button("Open File").clicked() {
                    _ = opener::open(&downloaded_path);
                }

                if ui.button("Show in Folder").clicked() {
                    _ = crate::utils::open_file_in_folder(&downloaded_path);
                }
            }
        }
    }
}

#[must_use]
enum ReceivePageResponse {
    Connect,
}

fn show_receive_file_page(ui: &mut Ui, code: &mut String) -> Option<ReceivePageResponse> {
    crate::page_with_content(
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

fn show_connected_page(
    ui: &mut Ui,
    _receive_request: &ReceiveRequest,
) -> Option<ConnectedPageResponse> {
    crate::page_with_content(ui, "Receive File", "TODO", "📥", |ui| {
        if ui.button("Reject").clicked() {
            return Some(ConnectedPageResponse::Accept);
        }

        ui.add_space(5.0);

        if ui.button("Accept").clicked() {
            return Some(ConnectedPageResponse::Reject);
        }

        None
    })
}

async fn reject(receive_request: ReceiveRequest) -> Result<(), PortalError> {
    receive_request.reject().await.map_err(Into::into)
}

async fn accept(
    receive_request: ReceiveRequest,
    transit_info_sender: poll_promise::Sender<TransitInfo>,
    progress_updater: svc::Updater<Progress>,
    cancel: oneshot::Receiver<()>,
) -> Result<PathBuf, PortalError> {
    let temp_file = tempfile::NamedTempFile::new()?;
    let mut temp_file_async = File::from(temp_file.reopen()?);

    let filename = receive_request.filename.clone();

    receive_request
        .accept(
            |transit_info, _| {
                transit_info_sender.send(transit_info);
            },
            move |received, total| {
                _ = progress_updater.update(Progress { received, total });
            },
            &mut temp_file_async,
            async { _ = cancel.await },
        )
        .await?;

    // TODO: re-attempt to save with added extension if file already exists
    let mut file_stem = filename.file_stem().unwrap_or_default();
    if file_stem.is_empty() {
        file_stem = "Downloaded File".as_ref();
    }
    let mut extension = filename.extension().unwrap_or_default();
    if extension.is_empty() {
        extension = "bin".as_ref();
    }

    let mut download_path = dirs::download_dir().expect("Unable to detect downloads directory");

    let mut filename = OsString::with_capacity(file_stem.len() + extension.len() + 1);
    filename.push(file_stem);
    filename.push(".");
    filename.push(extension);
    download_path.push(filename);
    temp_file
        .persist_noclobber(&download_path)
        .map_err(|error| error.error)?;

    Ok(download_path)
}

async fn connect(code: Code) -> Result<ReceiveRequest, PortalError> {
    let (_, wormhole) = Wormhole::connect_with_code(transfer::APP_CONFIG, code).await?;
    let relay_hint =
        transit::RelayHint::from_urls(None, [transit::DEFAULT_RELAY_SERVER.parse().unwrap()])
            .unwrap();
    let receive_request = transfer::request_file(
        wormhole,
        vec![relay_hint],
        Abilities::ALL_ABILITIES,
        future::pending(),
    )
    .await?
    .unwrap();
    // TODO: Support cancellation (handle None and pass a cancel future)
    Ok(receive_request)
}
