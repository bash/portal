use egui::{Context, Id};
use log::warn;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AppVersion {
    pub(crate) label: String,
    pub(crate) tag_name: String,
    pub(crate) release_notes_url: String,
    pub(crate) source_code_url: String,
    pub(crate) report_issue_url: String,
}

impl AppVersion {
    #[cfg(debug_assertions)]
    pub(crate) fn current() -> AppVersion {
        AppVersion {
            label: "dev".to_owned(),
            tag_name: TagName(env!("CARGO_PKG_VERSION")).to_string(),
            release_notes_url: env!("CARGO_PKG_REPOSITORY").to_owned(),
            source_code_url: env!("CARGO_PKG_REPOSITORY").to_owned(),
            report_issue_url: report_issues_url(),
        }
    }

    #[cfg(not(debug_assertions))]
    pub(crate) fn current() -> AppVersion {
        AppVersion {
            label: env!("CARGO_PKG_VERSION").to_owned(),
            tag_name: TagName(env!("CARGO_PKG_VERSION")).to_string(),
            release_notes_url: format!(
                "{repo}/releases/tag/{tag}",
                repo = env!("CARGO_PKG_REPOSITORY"),
                tag = TagName(env!("CARGO_PKG_VERSION"))
            ),
            source_code_url: format!(
                "{repo}/tree/{tag}",
                repo = env!("CARGO_PKG_REPOSITORY"),
                tag = TagName(env!("CARGO_PKG_VERSION"))
            ),
            report_issue_url: report_issues_url(),
        }
    }
}

pub(crate) async fn get_or_update_latest_app_version(ctx: Context) -> Option<AppVersion> {
    match ctx
        .memory_mut(|m| m.data.get_persisted::<SavedAppVersion>(Id::NULL))
        .filter(is_recent)
    {
        Some(saved) => saved.version,
        None => {
            let version = fetch_latest_version().await;
            store_app_version(ctx, version.clone());
            version
        }
    }
}

fn is_recent(saved: &SavedAppVersion) -> bool {
    let day = Duration::from_secs(60 * 60 * 24);
    saved
        .last_checked
        .elapsed()
        .map(|d| d < day)
        .unwrap_or(false)
}

fn store_app_version(ctx: Context, version: Option<AppVersion>) {
    let last_checked = SystemTime::now();
    let saved_version = SavedAppVersion {
        version,
        last_checked,
    };
    ctx.memory_mut(|m| m.data.insert_persisted(Id::NULL, saved_version));
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SavedAppVersion {
    version: Option<AppVersion>,
    last_checked: SystemTime,
}

async fn fetch_latest_version() -> Option<AppVersion> {
    let result = surf::get(latest_release_api_url())
        .recv_json::<GitHubRelease>()
        .await;
    match result {
        Ok(release) => Some(AppVersion {
            label: release.name,
            tag_name: release.tag_name.clone(),
            release_notes_url: format!(
                "{repo}/releases/latest",
                repo = env!("CARGO_PKG_REPOSITORY")
            ),
            source_code_url: format!(
                "{repo}/tree/{tag}",
                repo = env!("CARGO_PKG_REPOSITORY"),
                tag = release.tag_name
            ),
            report_issue_url: report_issues_url(),
        }),
        Err(err) => {
            warn!(
                err:? = err;
                "Failed to fetch latest version from GitHub"
            );
            None
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct GitHubRelease {
    html_url: String,
    tag_name: String,
    name: String,
}

struct TagName<'a>(&'a str);

impl<'a> fmt::Display for TagName<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v")?;
        self.0.fmt(f)
    }
}

fn report_issues_url() -> String {
    format!("{repo}/issues/new", repo = env!("CARGO_PKG_REPOSITORY"))
}

fn latest_release_api_url() -> String {
    format!("{}/releases/latest", env!("CARGO_PKG_REPOSITORY"))
        .replace("https://github.com/", "https://api.github.com/repos/")
}
