//! Launcher updates with a stable and a beta channel.
//!
//! Stable is the original `latest.json` that every installed build already reads, so
//! older builds keep updating. Beta lives next to it under `beta/latest.json`.

use anyhow::{Context, Result};
use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_updater::{Update, UpdaterExt};

const STABLE_ENDPOINT: &str = "https://pub-f68c946cc3b343619b3faea90c762d28.r2.dev/latest.json";
const BETA_ENDPOINT: &str = "https://pub-f68c946cc3b343619b3faea90c762d28.r2.dev/beta/latest.json";

pub fn endpoint(channel: &str) -> &'static str {
    if channel == "beta" { BETA_ENDPOINT } else { STABLE_ENDPOINT }
}

/// The update found by the last check, waiting for the user to install it.
#[derive(Default)]
pub struct Pending(tokio::sync::Mutex<Option<Update>>);

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AppUpdate {
    pub version: String,
    pub current_version: String,
    pub notes: Option<String>,
    pub channel: String,
}

pub async fn check(app: &AppHandle, pending: &Pending, channel: &str) -> Result<Option<AppUpdate>> {
    let url = endpoint(channel).parse().context("update endpoint")?;
    let found = app.updater_builder().endpoints(vec![url])?.build()?.check().await?;
    let view = found.as_ref().map(|u| AppUpdate {
        version: u.version.clone(),
        current_version: u.current_version.clone(),
        notes: u.body.clone(),
        channel: channel.to_string(),
    });
    *pending.0.lock().await = found;
    Ok(view)
}

pub async fn install(pending: &Pending) -> Result<()> {
    let update = pending.0.lock().await.take().context("No update is waiting. Check for updates again.")?;
    update.download_and_install(|_, _| {}, || {}).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn stable_is_the_original_manifest() {
        // Builds already in users' hands read this URL from tauri.conf.json; it must not move.
        let conf: serde_json::Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert_eq!(conf["plugins"]["updater"]["endpoints"][0], super::endpoint("stable"));
        assert_eq!(super::endpoint("anything-else"), super::endpoint("stable"));
        assert!(super::endpoint("beta").ends_with("/beta/latest.json"));
    }
}
