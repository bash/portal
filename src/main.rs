#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{path::PathBuf, future};

use async_std::fs::File;
use eframe::{egui::{self, RichText, ProgressBar, Context, TextStyle, Layout, Button}, epaint::Vec2, emath::Align};
use magic_wormhole::{WormholeWelcome, Wormhole, transfer, transit::{self, Abilities}};
use poll_promise::Promise;
use rfd::FileDialog;
use std::sync::mpsc;

#[tokio::main]
async fn main() {
    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(320.0, 240.0)),
        follow_system_theme: true,
        centered: true,
        ..Default::default()
    };
    eframe::run_native(
        "Portal",
        options,
        Box::new(|_cc| Box::new(PortalApp::default())),
    )
}

struct PortalApp {
    view: PortalView,
}

enum PortalView {
    Send,
    Connecting(Promise<(WormholeWelcome, Promise<Option<Wormhole>>)>, PathBuf),
    Sending((u64, u64), Promise<()>, mpsc::Receiver<(u64, u64)>),
    SendComplete,
    Receive,
}

enum SendRequest {
    File(PathBuf),
    Folder(PathBuf),
}

impl Default for PortalApp {
    fn default() -> Self {
        Self {
            view: PortalView::Send,
        }
    }
}

impl eframe::App for PortalApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(Layout::top_down(Align::Center), |ui| {
                if !ctx.input().raw.dropped_files.is_empty() {
                    let promise = Promise::spawn_async(connect(ctx.clone()));
                    self.view = PortalView::Connecting(promise, ctx.input().raw.dropped_files[0].path.as_ref().unwrap().clone());
                }

                let enable_tabs = matches!(self.view, PortalView::Send | PortalView::Receive);

                ui.add_enabled_ui(enable_tabs, |ui| {
                    ui.scope(|ui| {
                        ui.style_mut().spacing.button_padding = Vec2::new(10.0, 8.0);
                        ui.horizontal(|ui| {
                            if ui.selectable_label(matches!(self.view, PortalView::Send), RichText::new("📤 Send").size(14.0)).clicked() {
                                self.view = PortalView::Send;
                            }

                            if ui.selectable_label(matches!(self.view, PortalView::Receive), RichText::new("📥 Receive").size(14.0)).clicked() {
                                self.view = PortalView::Receive;
                            }
                        });
                    });
                });

                if let PortalView::Send | PortalView::Connecting(..) | PortalView::Sending(..) | PortalView::SendComplete = self.view {
                    ui.label(RichText::new("📤").size(100.0).strong());
                    ui.label(RichText::new("Send File").size(40.0).strong());
                    ui.add_space(5.0);
                }

                if let PortalView::Send = self.view {
                    if ui.add(Button::new("Select File").min_size(Vec2::new(100.0, 0.0))).clicked() {
                        if let Some(file) = FileDialog::new().pick_file() {
                            let promise = Promise::spawn_async(connect(ctx.clone()));
                            self.view = PortalView::Connecting(promise, file);
                        }
                    }

                    ui.add_space(5.0);

                    if ui.add(Button::new("Select Folder").min_size(Vec2::new(100.0, 0.0))).clicked() {
                        let folder = FileDialog::new().pick_folder();
                    }
                }

                if let PortalView::Connecting(ref mut promise, file_path) = &mut self.view {
                    match promise.ready_mut() {
                        None => { ui.spinner(); },
                        Some((welcome, connect_promise)) => {
                            match connect_promise.ready_mut() {
                                None => {
                                    ui.horizontal(|ui| {
                                        ui.label(&welcome.code.0);
                                        if ui.button("📋").on_hover_text("Click to copy").clicked() {
                                            ui.output().copied_text = welcome.code.0.clone();
                                        }
                                    });
                                },
                                Some(wormhole) => {
                                    let (sender, receiver) = mpsc::channel();
                                    let promise = Promise::spawn_async(send_file(wormhole.take().unwrap(), file_path.clone(), sender, ctx.clone()));
                                    self.view = PortalView::Sending((0, 0), promise, receiver);
                                    ui.spinner();
                                }
                            }
                        },
                    };
                }

                if let PortalView::Sending(ref mut progress, sending_promise, progress_recv) = &mut self.view {
                    if let Ok(updated_progress) = progress_recv.try_recv() {
                        dbg!(updated_progress);
                        dbg!(progress.0 as f32 / progress.1 as f32);
                        *progress = updated_progress;
                    }
                    match sending_promise.ready() {
                        None => {
                            ui.add(ProgressBar::new(progress.0 as f32 / progress.1 as f32).animate(true));
                        },
                        Some(_) => {
                            self.view = PortalView::SendComplete;
                        }
                    }
                }

                if let PortalView::SendComplete = self.view {
                    ui.label("File sent");
                    if ui.button("OK").clicked() {
                        self.view = PortalView::Send;
                    }
                }

                // ui.heading("My egui Application");
                // ui.horizontal(|ui| {
                //     let name_label = ui.label("Your name: ");
                //     ui.text_edit_singleline(&mut self.name)
                //         .labelled_by(name_label.id);
                // });
                // ui.add(egui::Slider::new(&mut self.age, 0..=120).text("age"));
                // if ui.button("Click each year").clicked() {
                //     self.age += 1;
                // }
                // ui.label(format!("Hello '{}', age {}", self.name, self.age));
            });
        });
    }
}

async fn connect(ctx: Context) -> (WormholeWelcome, Promise<Option<Wormhole>>) {
    let (welcome, future) = Wormhole::connect_without_code(transfer::APP_CONFIG, 4).await.unwrap();
    ctx.request_repaint();
    let ctx = ctx.clone();
    (welcome, Promise::spawn_async(async move {
        let result = future.await.unwrap();
        ctx.request_repaint();
        Some(result)
     }))
}

async fn send_file(wormhole: Wormhole, path: PathBuf, progress: mpsc::Sender<(u64, u64)>, context: Context) {
    let mut file = File::open(&path).await.unwrap();
    let metadata = file.metadata().await.unwrap();
    let file_size = metadata.len();
    let relay_hint = transit::RelayHint::from_urls(None, [transit::DEFAULT_RELAY_SERVER.parse().unwrap()]).unwrap();
    transfer::send_file(
        wormhole,
        vec![relay_hint],
        &mut file,
        path.file_name().unwrap(),
        file_size,
        Abilities::ALL_ABILITIES,
        |_, _| { },
        move |sent, total| { _ = progress.send((sent, total)); context.request_repaint() },
        future::pending()).await.unwrap();
}
