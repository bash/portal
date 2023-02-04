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
use std::{future, path::PathBuf};

pub enum SendView {
    Ready,
    Connecting(
        Promise<(WormholeWelcome, Promise<Option<Wormhole>>)>,
        PathBuf,
    ),
    Sending(Promise<()>, svc::Receiver<(u64, u64)>),
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
    pub fn ui(&mut self, ui: &mut Ui) {
        self.accept_dropped_file(ui);

        if let SendView::Ready = self {
            crate::page_with_content(
                ui,
                "Send File",
                "Select or drop the file or directory to send.",
                "📤",
                |ui| self.show_file_selection(ui),
            );
        }

        if let SendView::Connecting(ref mut promise, file_path) = self {
            match promise.ready_mut() {
                None => {
                    crate::page_with_content(
                        ui,
                        "Send File",
                        "Generating transmit code...",
                        "📤",
                        |ui| { ui.spinner(); },
                    );
                }
                Some((welcome, connect_promise)) => match connect_promise.ready_mut() {
                    None => {
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
                    Some(wormhole) => {
                        let (receiver, updater) = svc::channel_starting_with((0, 0));
                        let promise = Promise::spawn_async(send_file(
                            wormhole.take().unwrap(),
                            file_path.clone(),
                            updater,
                            ui.ctx().clone(),
                        ));
                        *self = SendView::Sending(promise, receiver);
                        ui.spinner();
                    }
                },
            };
        }

        if let SendView::Sending(sending_promise, progress_recv) = self {
            match sending_promise.ready() {
                None => {
                    let (sent, total) = *progress_recv.latest();
                    ui.add(ProgressBar::new((sent as f64 / total as f64) as f32).animate(true));
                }
                Some(_) => {
                    *self = SendView::Complete;
                }
            }
        }

        if let SendView::Complete = self {
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
    }

    fn show_file_selection(&mut self, ui: &mut Ui) {
        if ui
            .add(Button::new("Select File").min_size(Vec2::new(100.0, 0.0)))
            .clicked()
            || ui.input_mut().consume_key(Modifiers::COMMAND, Key::O)
        {
            if let Some(file_path) = FileDialog::new().pick_file() {
                self.connect(ui, file_path);
            }
        }

        ui.add_space(5.0);

        if ui
            .add(Button::new("Select Folder").min_size(Vec2::new(100.0, 0.0)))
            .clicked()
        {
            let _folder = FileDialog::new().pick_folder();
        }
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
            self.connect(ui, file_path);
        }
    }

    fn connect(&mut self, ui: &mut Ui, file_path: PathBuf) {
        let promise = Promise::spawn_async(connect(ui.ctx().clone()));
        *self = SendView::Connecting(promise, file_path);
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
    progress: svc::Updater<(u64, u64)>,
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
            _ = progress.update((sent, total));
            context.request_repaint()
        },
        future::pending(),
    )
    .await
    .unwrap();
}
