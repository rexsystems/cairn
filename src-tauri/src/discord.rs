//! Discord Rich Presence while a game is running.
use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

static PRESENCE: Mutex<Option<DiscordIpcClient>> = Mutex::new(None);

/// Set `CAIRN_DISCORD_APP_ID` to a Discord application id to enable presence.
fn app_id() -> Option<String> {
    std::env::var("CAIRN_DISCORD_APP_ID")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s != "0")
}

pub fn set_playing(instance_name: &str, mc_version: &str, loader: &str) {
    let Some(id) = app_id() else {
        return;
    };
    clear();
    let Ok(mut client) = DiscordIpcClient::new(&id) else {
        return;
    };
    if client.connect().is_err() {
        return;
    }
    let start = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    let details = format!("Playing {instance_name}");
    let state = format!("{} · {}", loader_label(loader), mc_version);
    let activity = activity::Activity::new()
        .details(&details)
        .state(&state)
        .assets(activity::Assets::new().large_text("Cairn"))
        .timestamps(activity::Timestamps::new().start(start));
    let _ = client.set_activity(activity);
    *PRESENCE.lock().unwrap() = Some(client);
}

pub fn clear() {
    if let Some(mut client) = PRESENCE.lock().unwrap().take() {
        let _ = client.clear_activity();
        let _ = client.close();
    }
}

fn loader_label(l: &str) -> &str {
    match l {
        "fabric" => "Fabric",
        "quilt" => "Quilt",
        "forge" => "Forge",
        "neoforge" => "NeoForge",
        _ => "Vanilla",
    }
}
