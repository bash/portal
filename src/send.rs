use crate::egui_ext::ContextExt;
use async_std::fs::File;
use eframe::{
    egui::{Button, Context, Key, Modifiers, ProgressBar, Ui},
    epaint::Vec2,
};
use magic_wormhole::{
    transfer,
    transit::{self, Abilities},
    Wormhole, WormholeWelcome,
};
use poll_promise::Promise;
use rfd::FileDialog;
use single_value_channel as svc;
use std::{
    future,
    path::{Path, PathBuf},
};

pub enum SendView {
    Ready,
    Connecting(
        Promise<(WormholeWelcome, Promise<Option<Wormhole>>)>,
        SendRequest,
    ),
    Sending(Promise<()>, svc::Receiver<Progress>),
    Complete,
}

#[derive(Default)]
pub struct Progress {
    sent: u64,
    total: u64,
}

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

impl SendView {
    pub fn ui(&mut self, ui: &mut Ui) {
        self.accept_dropped_file(ui);

        if let SendView::Connecting(ref mut promise, send_request) = self
            && let Some((_, ref mut connect_promise)) = promise.ready_mut()
            && let Some(wormhole) = connect_promise.ready_mut()
        {
            let (receiver, updater) = svc::channel_starting_with(Progress::default());
            let promise = match send_request {
                SendRequest::File(file_path) => ui.ctx().spawn_async(send_file(
                    wormhole.take().unwrap(),
                    file_path.clone(),
                    updater,
                    ui.ctx().clone(),
                )),
                SendRequest::Folder(folder_path) => ui.ctx().spawn_async(send_folder(
                    wormhole.take().unwrap(),
                    folder_path.clone(),
                    updater,
                    ui.ctx().clone(),
                )),
            };
            *self = SendView::Sending(promise, receiver);
        }

        if let SendView::Sending(sending_promise, _) = self
            && let Some(_) = sending_promise.ready()
        {
            *self = SendView::Complete;
        }

        match self {
            SendView::Ready => self.show_file_selection_page(ui),
            SendView::Connecting(ref promise, ref send_request)
                if let Some((welcome, _)) = promise.ready() =>
            {
                self.show_transmit_code(ui, welcome, send_request.path());
            }
            SendView::Connecting(..) => self.show_transmit_code_progress(ui),
            SendView::Sending(_, progress) => show_transfer_progress(ui, progress),
            SendView::Complete => self.show_transfer_completed_page(ui),
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

fn show_transfer_progress(ui: &mut Ui, progress: &mut svc::Receiver<Progress>) {
    let Progress { sent, total } = *progress.latest();
    ui.add(ProgressBar::new((sent as f64 / total as f64) as f32).animate(true));
}

async fn connect(ctx: Context) -> (WormholeWelcome, Promise<Option<Wormhole>>) {
    let (welcome, future) = Wormhole::connect_without_code(transfer::APP_CONFIG, 4)
        .await
        .unwrap();
    let promise = ctx.spawn_async(async { Some(future.await.unwrap()) });
    (welcome, promise)
}

async fn send_file(
    wormhole: Wormhole,
    path: PathBuf,
    progress: svc::Updater<Progress>,
    context: Context,
) {
    let mut file = File::open(&path).await.unwrap();
    let metadata = file.metadata().await.unwrap();
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
        |_, _| {},
        move |sent, total| {
            _ = progress.update(Progress { sent, total });
            context.request_repaint()
        },
        future::pending(),
    )
    .await
    .unwrap();
}

async fn send_folder(
    wormhole: Wormhole,
    path: PathBuf,
    progress: svc::Updater<Progress>,
    context: Context,
) {
    let relay_hint =
        transit::RelayHint::from_urls(None, [transit::DEFAULT_RELAY_SERVER.parse().unwrap()])
            .unwrap();
    transfer::send_folder(
        wormhole,
        vec![relay_hint],
        &path,
        path.file_name().unwrap(),
        Abilities::ALL_ABILITIES,
        |_, _| {},
        move |sent, total| {
            _ = progress.update(Progress { sent, total });
            context.request_repaint()
        },
        future::pending(),
    )
    .await
    .unwrap();
}
