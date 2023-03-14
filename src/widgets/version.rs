use egui::{Align, Layout};

pub fn app_version(ctx: &egui::Context) {
    egui::TopBottomPanel::bottom("bottom panel")
        .show_separator_line(false)
        .show(ctx, |ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.visuals_mut().hyperlink_color = ui.visuals().weak_text_color();

                const REPOSITORY_URL: &str = "https://github.com/bash/portal";

                #[cfg(debug_assertions)]
                ui.hyperlink_to("dev", REPOSITORY_URL);
                #[cfg(not(debug_assertions))]
                ui.hyperlink_to(
                    env!("CARGO_PKG_VERSION"),
                    format!(
                        "{REPOSITORY_URL}/releases/tag/v{}",
                        env!("CARGO_PKG_VERSION")
                    ),
                );
            })
        });
}
