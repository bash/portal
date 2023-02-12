use crate::egui_ext::ContextExt;
use crate::widgets::{cancel_button, CancelLabel};
use crate::{
    error::PortalError,
    fs::{persist_temp_file, persist_with_conflict_resolution, sanitize_untrusted_filename},
    sync::BorrowingOneshotReceiver,
    update,
};
use async_std::fs::File;
use eframe::{
    egui::{Button, ProgressBar, TextEdit, Ui},
    epaint::Vec2,
};
use futures::future::{AbortHandle, AbortRegistration, Abortable, Aborted};
use futures::{channel::oneshot, Future};
use magic_wormhole::{
    transfer::{self, ReceiveRequest},
    transit::{self, Abilities, TransitInfo},
    Code, Wormhole,
};
use portal_proc_macro::states;
use single_value_channel as svc;
use std::path::PathBuf;

type ConnectResult = Result<Option<ReceiveRequest>, PortalError>;
type ReceiveResult = Result<Option<PathBuf>, PortalError>;

#[derive(Default)]
pub struct ReceiveView {
    state: ReceiveState,
}

#[derive(Default, Copy, Clone)]
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

    async state Connecting(controller: ConnectingController) -> ConnectResult {
        new(code: Code) { ConnectingController::new(code) }
        next {
            Ok(Some(receive_request)) => Connected(receive_request),
            Ok(None) => Default::default(),
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

    async state Receiving(controller: ReceivingController) -> ReceiveResult {
        new(receive_request: ReceiveRequest) { ReceivingController::new(receive_request) }
        next {
            Ok(Some(path)) => Completed(path),
            Ok(None) => Default::default(),
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
            ReceiveState::Connecting(_, controller) => {
                if cancel_button(ui, CancelLabel::Cancel) {
                    controller.cancel();
                }

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
                self.back_button(ui);
                crate::page(ui, "File Transfer Failed", error, "❌");
            }
            ReceiveState::Connected(ref receive_request) => {
                match show_connected_page(ui, receive_request) {
                    None => {}
                    Some(ConnectedPageResponse::Accept) => {
                        update! {
                            &mut self.state,
                            ReceiveState::Connected(receive_request) => ReceiveState::new_receiving(ui, receive_request)
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
            ReceiveState::Receiving(_, ref mut controller) => {
                let Progress { received, total } = *controller.progress();

                if cancel_button(ui, CancelLabel::Cancel) {
                    controller.cancel();
                }

                match controller.transit_info() {
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
                self.back_button(ui);
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
            return Some(ConnectedPageResponse::Reject);
        }

        ui.add_space(5.0);

        if ui.button("Accept").clicked() {
            return Some(ConnectedPageResponse::Accept);
        }

        None
    })
}

async fn reject(receive_request: ReceiveRequest) -> Result<(), PortalError> {
    receive_request.reject().await.map_err(Into::into)
}

struct ReceivingController {
    transit_info_receiver: BorrowingOneshotReceiver<TransitInfo>,
    progress: svc::Receiver<Progress>,
    cancel_sender: Option<oneshot::Sender<()>>,
}

impl ReceivingController {
    fn new(receive_request: ReceiveRequest) -> (impl Future<Output = ReceiveResult>, Self) {
        let (transit_info_sender, transit_info_receiver) = ::oneshot::channel();
        let (progress, progress_updater) = svc::channel_starting_with(Progress::default());
        let (cancel_sender, cancel_receiver) = oneshot::channel();
        let controller = ReceivingController {
            transit_info_receiver: transit_info_receiver.into(),
            progress,
            cancel_sender: Some(cancel_sender),
        };
        let future = accept(
            receive_request,
            transit_info_sender,
            progress_updater,
            cancel_receiver,
        );
        (future, controller)
    }

    fn transit_info(&mut self) -> Option<&TransitInfo> {
        self.transit_info_receiver.value()
    }

    fn progress(&mut self) -> &Progress {
        self.progress.latest()
    }

    fn cancel(&mut self) {
        self.cancel_sender.take().map(|c| c.send(()));
    }
}

async fn accept(
    receive_request: ReceiveRequest,
    transit_info_sender: ::oneshot::Sender<TransitInfo>,
    progress_updater: svc::Updater<Progress>,
    cancel: oneshot::Receiver<()>,
) -> ReceiveResult {
    let temp_file = tempfile::NamedTempFile::new()?;
    let mut temp_file_async = File::from(temp_file.reopen()?);

    let untrusted_filename = receive_request.filename.clone();

    let mut canceled = false;
    receive_request
        .accept(
            |transit_info, _| {
                _ = transit_info_sender.send(transit_info);
            },
            move |received, total| {
                _ = progress_updater.update(Progress { received, total });
            },
            &mut temp_file_async,
            async {
                _ = cancel.await;
                canceled = true;
            },
        )
        .await?;
    if canceled {
        return Ok(None);
    }

    let file_name = sanitize_untrusted_filename(
        &untrusted_filename,
        "Downloaded File".as_ref(),
        "bin".as_ref(),
    );
    let persisted_path = persist_with_conflict_resolution(
        temp_file,
        dirs::download_dir().expect("Unable to detect downloads directory"),
        file_name,
        persist_temp_file,
    )?;

    Ok(Some(persisted_path))
}

struct ConnectingController {
    wormhole_abort_handle: AbortHandle,
    cancel_sender: Option<oneshot::Sender<()>>,
}

impl ConnectingController {
    fn new(code: Code) -> (impl Future<Output = ConnectResult>, Self) {
        let (wormhole_abort_handle, abort_registration) = AbortHandle::new_pair();
        let (cancel_sender, cancel_receiver) = oneshot::channel();
        let cancel_future = async { _ = cancel_receiver.await };
        let controller = ConnectingController {
            cancel_sender: Some(cancel_sender),
            wormhole_abort_handle,
        };
        (connect(code, abort_registration, cancel_future), controller)
    }

    fn cancel(&mut self) {
        self.cancel_sender.take().map(|c| c.send(()));
        self.wormhole_abort_handle.abort();
    }
}

async fn connect(
    code: Code,
    abort_registration: AbortRegistration,
    cancel: impl Future<Output = ()>,
) -> ConnectResult {
    let (_, wormhole) = match Abortable::new(
        Wormhole::connect_with_code(transfer::APP_CONFIG, code),
        abort_registration,
    )
    .await
    {
        Ok(result) => result?,
        Err(Aborted) => return Ok(None),
    };

    let relay_hint =
        transit::RelayHint::from_urls(None, [transit::DEFAULT_RELAY_SERVER.parse().unwrap()])
            .unwrap();
    transfer::request_file(wormhole, vec![relay_hint], Abilities::ALL_ABILITIES, cancel)
        .await
        .map_err(Into::into)
}
