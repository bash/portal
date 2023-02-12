use crate::egui_ext::ContextExt;
use crate::error::PortalError;
use crate::sync::BorrowingOneshotReceiver;
use crate::transmit_info::transit_info_message;
use crate::widgets::{cancel_button, CancelLabel};
use async_std::fs::File;
use eframe::{
    egui::{Button, Context, Key, Modifiers, ProgressBar, Ui},
    epaint::Vec2,
};
use futures::future::BoxFuture;
use magic_wormhole::{
    transfer::{self},
    transit::{self, Abilities, TransitInfo},
    Wormhole, WormholeWelcome,
};
use portal_proc_macro::states;
use rfd::FileDialog;
use single_value_channel as svc;
use std::{
    future,
    path::{Path, PathBuf},
};

type ConnectResult = Result<
    (
        WormholeWelcome,
        BoxFuture<'static, Result<Wormhole, PortalError>>,
    ),
    PortalError,
>;

states! {
    pub enum SendView;

    state Ready();

    async state Connecting(request: SendRequest) -> ConnectResult {
        new(request: SendRequest) {
            (connect(), request)
        }
        next {
            Ok((welcome, wormhole)) => Self::new_connected(ui, wormhole, welcome, request),
            Err(error) => Error(error),
        }
    }

    async state Connected(welcome: WormholeWelcome, request: SendRequest) -> Result<Wormhole, PortalError> {
        new(wormhole: BoxFuture<'static, Result<Wormhole, PortalError>>, welcome: WormholeWelcome, request: SendRequest) {
            (wormhole, welcome, request)
        }
        next {
            Ok(wormhole) => Self::new_sending(ui, wormhole, request),
            Err(error) => Error(error),
        }
    }

    async state Sending(controller: SendingController, request: SendRequest) -> Result<(), PortalError> {
        new(wormhole: Wormhole, request: SendRequest) {
            let (future, controller) = SendingController::new(ui.ctx().clone(), &request, wormhole);
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

#[derive(Default, Copy, Clone)]
pub struct Progress {
    sent: u64,
    total: u64,
}

#[derive(Clone, Debug)]
pub enum SendRequest {
    File(PathBuf),
    Folder(PathBuf),
}

impl SendRequest {
    fn path(&self) -> &Path {
        match self {
            SendRequest::File(path) => path,
            SendRequest::Folder(path) => path,
        }
    }
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
            SendView::Connecting(..) => self.show_transmit_code_progress(ui),
            SendView::Connected(_, ref welcome, ref send_request) => {
                self.show_transmit_code(ui, welcome, send_request.path())
            }
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
        crate::page_with_content(
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
                *self = SendView::new_connecting(ui, SendRequest::File(file_path))
            }
        }

        ui.add_space(5.0);

        let select_folder_button = Button::new("Select Folder").min_size(min_button_size);
        if ui.add(select_folder_button).clicked() {
            if let Some(folder_path) = FileDialog::new().pick_folder() {
                *self = SendView::new_connecting(ui, SendRequest::Folder(folder_path))
            }
        }
    }

    fn show_transmit_code_progress(&self, ui: &mut Ui) {
        crate::page_with_content(
            ui,
            "Send File",
            "Generating transmit code...",
            "📤",
            |ui| {
                ui.spinner();
            },
        )
    }

    fn show_transmit_code(&self, ui: &mut Ui, welcome: &WormholeWelcome, file_path: &Path) {
        crate::page_with_content(
            ui,
            "Your Transmit Code",
            format!(
                "Ready to send \"{}\".\nThe receiver needs to enter this code to begin the file transfer.",
                file_path.file_name().unwrap().to_string_lossy()
            ),
            "✨",
            |ui| {
                ui.horizontal(|ui| {
                    ui.label(&welcome.code.0);
                    if ui.button("📋").on_hover_text("Click to copy").clicked() {
                        ui.output_mut(|output| output.copied_text = welcome.code.0.clone());
                    }
                });
            }
        );
    }

    fn show_error_page(&mut self, ui: &mut Ui, error: String) {
        self.back_button(ui);
        crate::page(ui, "File Transfer Failed", error, "❌");
    }

    fn show_transfer_completed_page(&mut self, ui: &mut Ui, send_request: SendRequest) {
        let filename = send_request.path().file_name().unwrap();
        self.back_button(ui);
        crate::page(
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
            *self = SendView::new_connecting(ui, SendRequest::File(file_path))
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
    let Progress { sent, total } = controller.progress();
    let filename = send_request.path().file_name().unwrap();
    match controller.transit_info() {
        Some(transit_info) => crate::page_with_content(
            ui,
            "Sending File",
            transit_info_message(transit_info, filename),
            "📤",
            |ui| {
                ui.add(ProgressBar::new((sent as f64 / total as f64) as f32).animate(true));
            },
        ),
        None => crate::page_with_content(
            ui,
            "Connected to Peer",
            format!("Preparing to send file \"{}\"", filename.to_string_lossy()),
            "📤",
            |ui| {
                ui.spinner();
            },
        ),
    }
}

async fn connect() -> ConnectResult {
    let (welcome, future) = Wormhole::connect_without_code(transfer::APP_CONFIG, 4).await?;
    Ok((welcome, Box::pin(async { Ok(future.await?) })))
}

pub struct SendingController {
    transit_info_receiver: BorrowingOneshotReceiver<TransitInfo>,
    progress_receiver: svc::Receiver<Progress>,
}

impl SendingController {
    fn transit_info(&mut self) -> Option<&TransitInfo> {
        self.transit_info_receiver.value()
    }

    fn progress(&mut self) -> Progress {
        *self.progress_receiver.latest()
    }
}

impl SendingController {
    // TODO: this function needs refactoring
    fn new(
        ctx: Context,
        send_request: &SendRequest,
        wormhole: Wormhole,
    ) -> (BoxFuture<'static, Result<(), PortalError>>, Self) {
        let (progress_receiver, progress_updater) = svc::channel_starting_with(Progress::default());
        let (transit_sender, transit_info_receiver) = oneshot::channel();
        let future = {
            let send_request = send_request.clone();
            async {
                match send_request {
                    SendRequest::File(file_path) => {
                        send_file(
                            wormhole,
                            file_path.clone(),
                            progress_updater,
                            transit_sender,
                            ctx,
                        )
                        .await
                    }
                    SendRequest::Folder(folder_path) => {
                        send_folder(
                            wormhole,
                            folder_path.clone(),
                            progress_updater,
                            transit_sender,
                            ctx,
                        )
                        .await
                    }
                }
            }
        };
        let controller = SendingController {
            transit_info_receiver: transit_info_receiver.into(),
            progress_receiver,
        };
        (Box::pin(future), controller)
    }
}

async fn send_file(
    wormhole: Wormhole,
    path: PathBuf,
    progress: svc::Updater<Progress>,
    transit_info_sender: oneshot::Sender<TransitInfo>,
    ctx: Context,
) -> Result<(), PortalError> {
    let mut file = File::open(&path).await?;
    let metadata = file.metadata().await?;
    let file_size = metadata.len();
    let relay_hint =
        transit::RelayHint::from_urls(None, [transit::DEFAULT_RELAY_SERVER.parse().unwrap()])
            .unwrap();
    transfer::send_file(
        wormhole,
        vec![relay_hint],
        &mut file,
        path.file_name().unwrap(),
        file_size,
        Abilities::ALL_ABILITIES,
        {
            let ctx = ctx.clone();
            move |transit_info, _| {
                _ = transit_info_sender.send(transit_info);
                ctx.request_repaint();
            }
        },
        move |sent, total| {
            _ = progress.update(Progress { sent, total });
            ctx.request_repaint()
        },
        future::pending(),
    )
    .await?;
    Ok(())
}

async fn send_folder(
    wormhole: Wormhole,
    path: PathBuf,
    progress: svc::Updater<Progress>,
    transit_info_sender: oneshot::Sender<TransitInfo>,
    ctx: Context,
) -> Result<(), PortalError> {
    let relay_hint =
        transit::RelayHint::from_urls(None, [transit::DEFAULT_RELAY_SERVER.parse().unwrap()])
            .unwrap();
    transfer::send_folder(
        wormhole,
        vec![relay_hint],
        &path,
        path.file_name().unwrap(),
        Abilities::ALL_ABILITIES,
        {
            let ctx = ctx.clone();
            move |transit_info, _| {
                _ = transit_info_sender.send(transit_info);
                ctx.request_repaint();
            }
        },
        move |sent, total| {
            _ = progress.update(Progress { sent, total });
            ctx.request_repaint()
        },
        future::pending(),
    )
    .await?;
    Ok(())
}
