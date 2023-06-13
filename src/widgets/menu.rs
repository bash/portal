use crate::font::{ICON_REFRESH_CW, ICON_TAG};
use crate::version::AppVersion;
use egui::{menu, Layout};

pub(crate) fn app_menu(ctx: &egui::Context, latest_version: Option<AppVersion>) {
    egui::TopBottomPanel::top("top panel").show(ctx, |ui| {
        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            menu::bar(ui, |ui| {
                let version = AppVersion::current();

                ui.menu_button("Help", |ui| {
                    if ui.button("Source code").clicked() {
                        ui.output_mut(|out| out.open_url(version.source_code_url));
                    }

                    if ui.button("Report a bug").clicked() {
                        ui.output_mut(|out| out.open_url(version.report_issue_url));
                    }

                    ui.separator();
                    ui.hyperlink_to(
                        format!("{ICON_TAG} {}", version.label),
                        version.release_notes_url,
                    );
                });

                if let Some(latest_version) =
                    latest_version.filter(|v| v.tag_name != version.tag_name)
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
    });
}
