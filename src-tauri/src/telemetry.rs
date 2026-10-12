//! Product analytics via Umami (`/api/send`). Silent on failure — never blocks the UI.

use serde_json::json;

const UMAMI_ENDPOINT: &str = "https://stats.694206767.xyz/api/send";
const WEBSITE_ID: &str = "71e332bf-5d32-4a56-bcb2-b2d68528097f";
const HOSTNAME: &str = "app.cairn.launcher";

pub async fn track(name: &str, data: serde_json::Value) {
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

pub async fn track_app_open() {
    track(
        "app_open",
        json!({
            "version": env!("CARGO_PKG_VERSION"),
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
        }),
    )
    .await;
}
