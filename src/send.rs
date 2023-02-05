use crate::egui_ext::ContextExt;
use async_std::fs::File;
use eframe::{
    egui::{Button, Context, Key, Modifiers, ProgressBar, Ui},
    epaint::Vec2,
};
use magic_wormhole::{
    transfer::{self, TransferError},
    transit::{self, Abilities, TransitInfo},
    Wormhole, WormholeError, WormholeWelcome,
};
use poll_promise::Promise;
use rfd::FileDialog;
use single_value_channel as svc;
use std::{
    ffi::OsStr,
    fmt, future,
    path::{Path, PathBuf},
};
use take_mut::take;

pub enum SendView {
    Ready,
    Connecting(
        Promise<Result<(WormholeWelcome, Promise<Result<Wormhole, SendError>>), SendError>>,
        SendRequest,
    ),
    Connected(
        WormholeWelcome,
        SendRequest,
        Promise<Result<Wormhole, SendError>>,
    ),
    Sending(
        Promise<Result<(), SendError>>,
        svc::Receiver<Option<TransitInfo>>,
        svc::Receiver<Progress>,
    ),
    Error(SendError),
    Complete,
}

#[derive(Debug)]
pub enum SendError {
    Wormhole(WormholeError),
    WormholeTransfer(magic_wormhole::transfer::TransferError),
    Io(std::io::Error),
}

impl From<WormholeError> for SendError {
    fn from(value: WormholeError) -> Self {
        SendError::Wormhole(value)
    }
}

impl From<magic_wormhole::transfer::TransferError> for SendError {
    fn from(value: TransferError) -> Self {
        SendError::WormholeTransfer(value)
    }
}

impl From<std::io::Error> for SendError {
    fn from(value: std::io::Error) -> Self {
        SendError::Io(value)
    }
}

impl fmt::Display for SendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use SendError::*;
        match self {
            Wormhole(error) => write!(f, "{}", error),
            WormholeTransfer(error) => write!(f, "{}", error),
            Io(error) => write!(f, "{}", error),
        }
    }
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
        SendView::Ready
    }
}

macro_rules! update {
    ($target:expr, $pattern:pat => $match_arm:expr) => {
        take($target, |target| match target {
            $pattern => $match_arm,
            _ => target,
        });
    };
}

impl SendView {
    pub fn ui(&mut self, ui: &mut Ui) {
        self.transition_from_connecting_to_connected();
        self.transition_from_connected_to_sending(ui);
        self.transition_from_sending_to_complete();

        if let SendView::Ready | SendView::Complete = self {
            self.accept_dropped_file(ui);
        }

        match self {
            SendView::Ready => self.show_file_selection_page(ui),
            SendView::Connecting(..) => self.show_transmit_code_progress(ui),
            SendView::Connected(ref welcome, ref send_request, _) => {
                self.show_transmit_code(ui, welcome, send_request.path())
            }
            SendView::Sending(_, transit_info, progress) => {
                show_transfer_progress(ui, progress, transit_info)
            }
            SendView::Error(ref error) => self.show_error_page(ui, error.to_string()),
            SendView::Complete => self.show_transfer_completed_page(ui),
        }
    }

    fn transition_from_connecting_to_connected(&mut self) {
        update! {
            self,
            SendView::Connecting(connecting_promise, send_request) => match connecting_promise.try_take()
            {
                Ok(Ok((welcome, wormhole_promise))) => SendView::Connected(welcome, send_request, wormhole_promise),
                Ok(Err(error)) => SendView::Error(error),
                Err(connecting_promise) => SendView::Connecting(connecting_promise, send_request),
            }
        }
    }

    fn transition_from_connected_to_sending(&mut self, ui: &mut Ui) {
        update! {
            self,
            SendView::Connected(welcome, send_request, wormhole_promise) => {
                match wormhole_promise.try_take() {
                    Ok(Ok(wormhole)) => send(ui, &send_request, wormhole),
                    Ok(Err(error)) => SendView::Error(error),
                    Err(wormhole_promise) => {
                        SendView::Connected(welcome, send_request, wormhole_promise)
                    }
                }
            }
        }
    }

    fn transition_from_sending_to_complete(&mut self) {
        update! {
            self,
            SendView::Sending(sending_promise, transit_info, progress) => {
                match sending_promise.try_take() {
                    Ok(Ok(_)) => SendView::Complete,
                    Ok(Err(error)) => SendView::Error(error),
                    Err(sending_promise) => {
                        SendView::Sending(sending_promise, transit_info, progress)
                    }
                }
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
            || ui.input_mut().consume_key(Modifiers::COMMAND, Key::O)
        {
            if let Some(file_path) = FileDialog::new().pick_file() {
                self.connect(ui, SendRequest::File(file_path));
            }
        }

        ui.add_space(5.0);

        let select_folder_button = Button::new("Select Folder").min_size(min_button_size);
        if ui.add(select_folder_button).clicked() {
            if let Some(folder_path) = FileDialog::new().pick_folder() {
                self.connect(ui, SendRequest::Folder(folder_path));
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
                        ui.output().copied_text = welcome.code.0.clone();
                    }
                });
            }
        );
    }

    fn show_error_page(&mut self, ui: &mut Ui, error: String) {
        ui.horizontal(|ui| {
            if ui.button("Back").clicked() {
                *self = SendView::Ready;
            }
        });

        crate::page(ui, "File Transfer Failed", error, "❌");
    }

    fn show_transfer_completed_page(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if ui.button("Back").clicked() {
                *self = SendView::Ready;
            }
        });

        crate::page(
            ui,
            "File Transfer Successful",
            format!("Successfully sent file \"{}\"", "FILENAME"),
            "✅",
        );
    }

    fn accept_dropped_file(&mut self, ui: &mut Ui) {
        let file_path = ui
            .ctx()
            .input()
            .raw
            .dropped_files
            .iter()
            .find_map(|f| f.path.clone());
        if let Some(file_path) = file_path {
            self.connect(ui, SendRequest::File(file_path));
        }
    }

    fn connect(&mut self, ui: &mut Ui, send_request: SendRequest) {
        let promise = ui.ctx().spawn_async(connect(ui.ctx().clone()));
        *self = SendView::Connecting(promise, send_request);
    }
}

fn show_transfer_progress(
    ui: &mut Ui,
    progress: &mut svc::Receiver<Progress>,
    transit_info: &mut svc::Receiver<Option<TransitInfo>>,
) {
    let Progress { sent, total } = *progress.latest();
    match transit_info.latest() {
        Some(transit_info) => crate::page_with_content(
            ui,
            "Sending File",
            transit_info_message(transit_info, "FILENAME".as_ref()),
            "📤",
            |ui| {
                ui.add(ProgressBar::new((sent as f64 / total as f64) as f32).animate(true));
            },
        ),
        None => crate::page_with_content(
            ui,
            "Connected to Peer",
            "Preparing to send file",
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

async fn connect(
    ctx: Context,
) -> Result<(WormholeWelcome, Promise<Result<Wormhole, SendError>>), SendError> {
    let (welcome, future) = Wormhole::connect_without_code(transfer::APP_CONFIG, 4).await?;
    Ok((welcome, ctx.spawn_async(async { Ok(future.await?) })))
}

// TODO: this function needs refactoring
fn send(ui: &mut Ui, send_request: &SendRequest, wormhole: Wormhole) -> SendView {
    let (progress_receiver, progress_updater) = svc::channel_starting_with(Progress::default());
    let (transit_receiver, transit_updater) = svc::channel::<TransitInfo>();
    let promise = match send_request {
        SendRequest::File(file_path) => ui.ctx().spawn_async(send_file(
            wormhole,
            file_path.clone(),
            progress_updater,
            transit_updater,
            ui.ctx().clone(),
        )),
        SendRequest::Folder(folder_path) => ui.ctx().spawn_async(send_folder(
            wormhole,
            folder_path.clone(),
            progress_updater,
            transit_updater,
            ui.ctx().clone(),
        )),
    };
    SendView::Sending(promise, transit_receiver, progress_receiver)
}

async fn send_file(
    wormhole: Wormhole,
    path: PathBuf,
    progress: svc::Updater<Progress>,
    transit_info_updater: svc::Updater<Option<TransitInfo>>,
    ctx: Context,
) -> Result<(), SendError> {
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
                _ = transit_info_updater.update(Some(transit_info));
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
    transit_info_updater: svc::Updater<Option<TransitInfo>>,
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
                _ = transit_info_updater.update(Some(transit_info));
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
