use crate::font::{ICON_REFRESH_CW, ICON_TAG};
use crate::version::AppVersion;
use egui::{menu, OpenUrl};
use egui_theme_switch::global_theme_switch;

pub(crate) fn app_menu(ctx: &egui::Context, latest_version: Option<AppVersion>) {
    egui::TopBottomPanel::top("top panel").show(ctx, |ui| {
        menu::bar(ui, |ui| {
            let version = AppVersion::current();

            ui.menu_button("View", |ui| {
                global_theme_switch(ui);
            });

            ui.menu_button("Help", |ui| {
                if ui.button("Source code").clicked() {
                    ctx.open_url(OpenUrl::new_tab(version.source_code_url))
                }

                if ui.button("Report a bug").clicked() {
                    ctx.open_url(OpenUrl::new_tab(version.report_issue_url));
                }

                ui.separator();
                ui.hyperlink_to(
                    format!("{ICON_TAG} {}", version.label),
                    version.release_notes_url,
                );
            });

            if let Some(latest_version) = latest_version.filter(|v| v.tag_name != version.tag_name)
            {
                ui.add_space(2.);
                ui.hyperlink_to(
                    format!(
                        "{ICON_REFRESH_CW} Update to {version}",
                        version = latest_version.label
                    ),
                    latest_version.release_notes_url,
                );
            }
        });
    });
}
