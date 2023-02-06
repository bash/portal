use std::{ffi::OsString, future::{self, Future}, path::PathBuf};

use crate::{error::PortalError, update};
use async_std::fs::File;
use eframe::{
    egui::{Button, ProgressBar, TextBuffer, TextEdit, Ui},
    epaint::Vec2,
};
use magic_wormhole::{
    transfer::{self, ReceiveRequest},
    transit::{self, Abilities, TransitInfo},
    Code, Wormhole,
};
use poll_promise::Promise;
use single_value_channel as svc;
use futures::channel::oneshot;

use crate::egui_ext::ContextExt;

#[derive(Default)]
pub struct ReceiveView {
    code: String,
    state: ReceiveState,
}

enum ReceiveState {
    Initial,
    Connecting(Promise<Result<ReceiveRequest, PortalError>>),
    Connected(ReceiveRequest),
    Rejecting(Promise<Result<(), PortalError>>),
    Receiving(
        Promise<Result<PathBuf, PortalError>>,
        Promise<TransitInfo>,
        svc::Receiver<Progress>,
        Option<oneshot::Sender<()>>,
    ),
    Error(PortalError),
    Completed(PathBuf),
}

#[derive(Default)]
struct Progress {
    received: u64,
    total: u64,
}

impl Default for ReceiveState {
    fn default() -> Self {
        ReceiveState::Initial
    }
}

impl ReceiveView {
    pub fn show_switcher(&self) -> bool {
        matches!(self.state, ReceiveState::Initial)
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        self.transition_from_connecting_to_connected();
        self.transition_from_rejecting_to_initial();
        self.transition_from_receiving_to_completed();

        match &mut self.state {
            ReceiveState::Initial => self.show_receive_file_page(ui),
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
                        self.state = ReceiveState::Initial;
                    }
                });

                crate::page(ui, "File Transfer Failed", error, "❌");
            }
            ReceiveState::Connected(_) => {
                self.show_connected_page(ui);
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
                        self.state = ReceiveState::Initial;
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

    fn transition_from_connecting_to_connected(&mut self) {
        update! {
            &mut self.state,
            ReceiveState::Connecting(connecting_promise) => match connecting_promise.try_take() {
                Ok(Ok(receive_request)) => ReceiveState::Connected(receive_request),
                Ok(Err(error)) => ReceiveState::Error(error),
                Err(connecting_promise) => ReceiveState::Connecting(connecting_promise),
            }
        }
    }

    fn transition_from_rejecting_to_initial(&mut self) {
        update! {
            &mut self.state,
            ReceiveState::Rejecting(rejecting_promise) => match rejecting_promise.try_take() {
                Ok(Ok(())) => ReceiveState::Initial,
                Ok(Err(error)) => ReceiveState::Error(error),
                Err(rejecting_promise) => ReceiveState::Rejecting(rejecting_promise),
            }
        }
    }

    fn transition_from_receiving_to_completed(&mut self) {
        update! {
            &mut self.state,
            ReceiveState::Receiving(receiving_promise, transit_info, progress, cancel) => match receiving_promise.try_take() {
                Ok(Ok(path)) => ReceiveState::Completed(path),
                Ok(Err(error)) => ReceiveState::Error(error),
                Err(rejecting_promise) => ReceiveState::Receiving(rejecting_promise, transit_info, progress, cancel),
            }
        }
    }

    fn show_receive_file_page(&mut self, ui: &mut Ui) {
        crate::page_with_content(
            ui,
            "Receive File",
            "Enter the code from your peer below:",
            "📥",
            |ui| {
                ui.add(TextEdit::singleline(&mut self.code).hint_text("Code"));
                ui.add_space(5.0);

                let input_empty =
                    self.code.is_empty() || self.code.chars().all(|c| c.is_whitespace());

                let min_button_size = Vec2::new(100.0, 0.0);

                ui.add_enabled_ui(!input_empty, |ui| {
                    if ui
                        .add(Button::new("Receive File").min_size(min_button_size))
                        .clicked()
                    {
                        let promise = ui.ctx().spawn_async(connect(Code(self.code.take())));
                        self.state = ReceiveState::Connecting(promise);
                    }
                });
            },
        );
    }

    fn show_connected_page(&mut self, ui: &mut Ui) {
        crate::page_with_content(ui, "Receive File", "TODO", "📥", |ui| {
            if ui.button("Reject").clicked() {
                update! {
                    &mut self.state,
                    ReceiveState::Connected(receive_request) => ReceiveState::Rejecting(ui.ctx().spawn_async(reject(receive_request)))
                }
            }

            ui.add_space(5.0);

            if ui.button("Accept").clicked() {
                let (transit_sender, transit_promise) = Promise::new();
                let (progress, progress_updater) = svc::channel_starting_with(Progress::default());
                let (cancel_sender, cancel_receiver) = oneshot::channel();
                update! {
                    &mut self.state,
                    ReceiveState::Connected(receive_request) => ReceiveState::Receiving(
                        ui.ctx().spawn_async(accept(receive_request, transit_sender, progress_updater)),
                        transit_promise,
                        progress,
                        Some(cancel_sender))
                }
            }
        });
    }
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
                progress_updater.update(Progress { received, total });
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
