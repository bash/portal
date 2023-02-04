use async_std::fs::File;
use eframe::{
    egui::{self, Button, Context, Key, Modifiers, ProgressBar, RichText},
    epaint::Vec2,
};
use magic_wormhole::{
    transfer,
    transit::{self, Abilities},
    Wormhole, WormholeWelcome,
};
use poll_promise::Promise;
use rfd::FileDialog;
use std::sync::mpsc;
use std::{future, path::PathBuf};

pub enum SendView {
    Ready,
    Connecting(
        Promise<(WormholeWelcome, Promise<Option<Wormhole>>)>,
        PathBuf,
    ),
    Sending((u64, u64), Promise<()>, mpsc::Receiver<(u64, u64)>),
    Complete,
}

enum SendRequest {
    File(PathBuf),
    Folder(PathBuf),
}

impl Default for SendView {
    fn default() -> Self {
        SendView::Ready
    }
}

impl SendView {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        if !ui.ctx().input().raw.dropped_files.is_empty() {
            let promise = Promise::spawn_async(connect(ui.ctx().clone()));
            *self = SendView::Connecting(
                promise,
                ui.ctx().input().raw.dropped_files[0]
                    .path
                    .as_ref()
                    .unwrap()
                    .clone(),
            );
        }

        ui.label(RichText::new("📤").size(100.0).strong());
        ui.label(RichText::new("Send File").size(30.0).strong());
        ui.add_space(3.0);
        ui.weak("Select or drop the file or directory to send.");
        ui.add_space(5.0);

        if let SendView::Ready = self {
            if ui
                .add(Button::new("Select File").min_size(Vec2::new(100.0, 0.0)))
                .clicked()
                || ui.input_mut().consume_key(Modifiers::COMMAND, Key::O)
            {
                if let Some(file) = FileDialog::new().pick_file() {
                    let promise = Promise::spawn_async(connect(ui.ctx().clone()));
                    *self = SendView::Connecting(promise, file);
                }
            }

            ui.add_space(5.0);

            if ui
                .add(Button::new("Select Folder").min_size(Vec2::new(100.0, 0.0)))
                .clicked()
            {
                let folder = FileDialog::new().pick_folder();
            }
        }

        if let SendView::Connecting(ref mut promise, file_path) = self {
            match promise.ready_mut() {
                None => {
                    ui.spinner();
                }
                Some((welcome, connect_promise)) => match connect_promise.ready_mut() {
                    None => {
                        ui.horizontal(|ui| {
                            ui.label(&welcome.code.0);
                            if ui.button("📋").on_hover_text("Click to copy").clicked() {
                                ui.output().copied_text = welcome.code.0.clone();
                            }
                        });
                    }
                    Some(wormhole) => {
                        let (sender, receiver) = mpsc::channel();
                        let promise = Promise::spawn_async(send_file(
                            wormhole.take().unwrap(),
                            file_path.clone(),
                            sender,
                            ui.ctx().clone(),
                        ));
                        *self = SendView::Sending((0, 0), promise, receiver);
                        ui.spinner();
                    }
                },
            };
        }

        if let SendView::Sending(ref mut progress, sending_promise, progress_recv) = self {
            if let Ok(updated_progress) = progress_recv.try_recv() {
                dbg!(updated_progress);
                dbg!(progress.0 as f32 / progress.1 as f32);
                *progress = updated_progress;
            }
            match sending_promise.ready() {
                None => {
                    ui.add(ProgressBar::new(progress.0 as f32 / progress.1 as f32).animate(true));
                }
                Some(_) => {
                    *self = SendView::Complete;
                }
            }
        }

        if let SendView::Complete = self {
            ui.label("File sent");
            if ui.button("OK").clicked() {
                *self = SendView::Ready;
            }
        }
    }
}

async fn connect(ctx: Context) -> (WormholeWelcome, Promise<Option<Wormhole>>) {
    let (welcome, future) = Wormhole::connect_without_code(transfer::APP_CONFIG, 4)
        .await
        .unwrap();
    ctx.request_repaint();
    let ctx = ctx.clone();
    (
        welcome,
        Promise::spawn_async(async move {
            let result = future.await.unwrap();
            ctx.request_repaint();
            Some(result)
        }),
    )
}

async fn send_file(
    wormhole: Wormhole,
    path: PathBuf,
    progress: mpsc::Sender<(u64, u64)>,
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
            _ = progress.send((sent, total));
            context.request_repaint()
        },
        future::pending(),
    )
    .await
    .unwrap();
}
