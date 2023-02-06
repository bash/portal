use crate::egui_ext::ContextExt;
use crate::error::PortalError as SendError;
use async_std::fs::File;
use eframe::{
    egui::{Button, Context, Key, Modifiers, ProgressBar, Ui},
    epaint::Vec2,
};
use futures::{Future, future::BoxFuture};
use magic_wormhole::{
    transfer::{self},
    transit::{self, Abilities, TransitInfo},
    Wormhole, WormholeWelcome,
};
use poll_promise::Promise;
use rfd::FileDialog;
use single_value_channel as svc;
use std::{
    ffi::OsStr,
    fmt, future,
    path::{Path, PathBuf},
}; // TODO: rename usages in this file
use crate::states;

states! {
    pub enum SendView;

    state Ready() { }

    state Connecting(request: SendRequest) {
        execute() -> Result<(WormholeWelcome, BoxFuture<'static, Result<Wormhole, SendError>>), SendError> {
            connect().await
        }
        next(ui) {
            Ok((welcome, wormhole)) => Self::new_connected(ui, wormhole, welcome, request),
            Err(error) => Error(error),
        }
    }

    state Connected(welcome: WormholeWelcome, request: SendRequest) {
        execute(wormhole: BoxFuture<'static, Result<Wormhole, SendError>>) -> Result<Wormhole, SendError> {
            wormhole.await
        }
        next(ui) {
            Ok(wormhole) => {
                dbg!("starting to send...");
                let (future, transit_info, progress) = send(ui.ctx().clone(), &request, wormhole);
                Self::new_sending(ui, future, transit_info, progress, request)
            },
            Err(error) => Error(error),
        }
    }

    state Sending(transit_info: Promise<TransitInfo>, progress: svc::Receiver<Progress>, request: SendRequest) {
        execute(future: impl Future<Output = Result<(), SendError>> + Send + 'static) -> Result<(), SendError> {
            {
                dbg!("awaiting send future");
                future.await
            }
        }
        next(_ui) {
            Ok(_) => Complete(request),
            Err(error) => Error(error),
        }
    }

    state Error(error: SendError) { }

    state Complete(request: SendRequest) { }
}

#[derive(Default)]
pub struct Progress {
    sent: u64,
    total: u64,
}

#[derive(Clone)]
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
            SendView::Sending(_, ref transit_info, progress, send_request) => {
                show_transfer_progress(ui, progress, transit_info, send_request)
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
        ui.horizontal(|ui| {
            if ui.button("Back").clicked() {
                *self = SendView::Ready();
            }
        });

        crate::page(ui, "File Transfer Failed", error, "❌");
    }

    fn show_transfer_completed_page(&mut self, ui: &mut Ui, send_request: SendRequest) {
        let filename = send_request.path().file_name().unwrap();

        ui.horizontal(|ui| {
            if ui.button("Back").clicked() {
                *self = SendView::Ready();
            }
        });

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
}

fn show_transfer_progress(
    ui: &mut Ui,
    progress: &mut svc::Receiver<Progress>,
    transit_info: &Promise<TransitInfo>,
    send_request: &SendRequest,
) {
    let Progress { sent, total } = *progress.latest();
    let filename = send_request.path().file_name().unwrap();
    match transit_info.ready() {
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

struct TransitInfoDisplay<'a>(&'a TransitInfo);

impl<'a> fmt::Display for TransitInfoDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use TransitInfo::*;
        match self.0 {
            Direct => write!(f, " via direct transfer"),
            Relay { name: None } => write!(f, " via relay"),
            Relay { name: Some(relay) } => write!(f, " via relay \"{relay}\""),
            _ => Ok(()),
        }
    }
}

fn transit_info_message(transit_info: &TransitInfo, filename: &OsStr) -> String {
    let filename = filename.to_string_lossy();
    format!("File \"{filename}\"{}", TransitInfoDisplay(transit_info))
}

async fn connect() -> Result<(WormholeWelcome, BoxFuture<'static, Result<Wormhole, SendError>>), SendError> {
    let (welcome, future) = Wormhole::connect_without_code(transfer::APP_CONFIG, 4).await?;
    Ok((welcome, Box::pin(async { Ok(future.await?) })))
}

// TODO: this function needs refactoring
fn send(ctx: Context, send_request: &SendRequest, wormhole: Wormhole) -> (impl Future<Output = Result<(), SendError>> + Send, Promise<TransitInfo>, svc::Receiver<Progress>) {
    let (progress_receiver, progress_updater) = svc::channel_starting_with(Progress::default());
    let (transit_sender, transit_info) = Promise::<TransitInfo>::new();
    let future = {
        let send_request = send_request.clone();
        async {
            dbg!("matching on send_request");
            match send_request {
                SendRequest::File(file_path) => send_file(
                    wormhole,
                    file_path.clone(),
                    progress_updater,
                    transit_sender,
                    ctx,
                ).await,
                SendRequest::Folder(folder_path) => send_folder(
                    wormhole,
                    folder_path.clone(),
                    progress_updater,
                    transit_sender,
                    ctx,
                ).await,
            }
        }
    };
    (future, transit_info, progress_receiver)
}

async fn send_file(
    wormhole: Wormhole,
    path: PathBuf,
    progress: svc::Updater<Progress>,
    transit_info_sender: poll_promise::Sender<TransitInfo>,
    ctx: Context,
) -> Result<(), SendError> {
    let mut file = File::open(&path).await?;
    let metadata = file.metadata().await?;
    let file_size = metadata.len();
    let relay_hint =
        transit::RelayHint::from_urls(None, [transit::DEFAULT_RELAY_SERVER.parse().unwrap()])
            .unwrap();
    dbg!("before transfer::send_file");
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
    println!("after transfer::send_file");
    Ok(())
}

async fn send_folder(
    wormhole: Wormhole,
    path: PathBuf,
    progress: svc::Updater<Progress>,
    transit_info_sender: poll_promise::Sender<TransitInfo>,
    ctx: Context,
) -> Result<(), SendError> {
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
