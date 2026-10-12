//! Product analytics via Umami (`/api/send`). Silent on failure — never blocks the UI.

use serde_json::{json, Value};
use std::path::Path;

const UMAMI_ENDPOINT: &str = "https://stats.694206767.xyz/api/send";
const WEBSITE_ID: &str = "71e332bf-5d32-4a56-bcb2-b2d68528097f";
const HOSTNAME: &str = "app.cairn.launcher";

fn enabled(settings_file: &Path) -> bool {
    crate::settings::Settings::load(settings_file).telemetry_enabled
}

pub async fn track(settings_file: &Path, name: &str, data: Value) {
    if !enabled(settings_file) {
        return;
    }
    let body = json!({
        "type": "event",
        "payload": {
            "website": WEBSITE_ID,
            "hostname": HOSTNAME,
            "language": "en-US",
            "referrer": "",
            "screen": "",
            "title": "Cairn",
            "url": "/",
            "name": name,
            "data": data,
        }
    });
    let _ = crate::net::CLIENT
        .post(UMAMI_ENDPOINT)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await;
}

pub async fn track_app_open(settings_file: &Path) {
    track(
        settings_file,
        "app_open",
        json!({
            "version": env!("CARGO_PKG_VERSION"),
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
        }),
    )
    .await;
}

pub fn spawn(settings_file: impl Into<std::path::PathBuf>, name: &'static str, data: Value) {
    let settings_file = settings_file.into();
    tauri::async_runtime::spawn(async move {
        track(&settings_file, name, data).await;
    });
}
