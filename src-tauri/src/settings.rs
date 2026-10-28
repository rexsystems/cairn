use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::path::Path;

/// Client ID of the official Minecraft launcher on the Windows Live (login.live.com) endpoint.
pub const LIVE_DEFAULT_CLIENT_ID: &str = "00000000402b5328";

/// Vars the launcher may set for its own GTK/WebKit window. Minecraft must not inherit them.
const LAUNCHER_ENV_STRIP: &[&str] = &[
    "GDK_BACKEND",
    "GTK_THEME",
    "GTK_DEBUG",
    "WEBKIT_DISABLE_DMABUF_RENDERER",
    "WEBKIT_DISABLE_COMPOSITING_MODE",
    "WEBKIT_FORCE_COMPOSITING_MODE",
    "WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS",
];

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub theme: String,
    pub default_memory_mb: u32,
    pub default_jvm_args: String,
    pub default_width: u32,
    pub default_height: u32,
    pub java_overrides: HashMap<String, String>,
    pub auto_install_java: bool,
    /// Kept for older settings.json files; sign-in uses the built-in Live client ID unless CAIRN_MSA_CLIENT_ID is set.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub msa_client_id: String,
    pub concurrent_downloads: usize,
    pub hide_on_launch: bool,
    pub reduce_motion: bool,
    /// Drop launcher-only env vars (GDK_BACKEND, WebKit flags, …) before starting the game.
    pub isolate_game_env: bool,
    /// Extra KEY → value for the game process. Empty value removes that key.
    pub game_env: HashMap<String, String>,
    /// Show Discord Rich Presence while Minecraft is running (needs CAIRN_DISCORD_APP_ID).
    pub discord_rpc: bool,
    /// Move focus with a gamepad in the launcher UI.
    pub gamepad_nav: bool,
    /// Anonymous usage events (Umami). Errors still go to GlitchTip when enabled separately.
    pub telemetry_enabled: bool,
    /// CurseForge Core API key for importing CF modpack zips (Settings → or CAIRN_CURSEFORGE_API_KEY).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub curseforge_api_key: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            theme: "dark".into(),
            default_memory_mb: 4096,
            default_jvm_args: String::new(),
            default_width: 1280,
            default_height: 720,
            java_overrides: HashMap::new(),
            auto_install_java: true,
            msa_client_id: String::new(),
            concurrent_downloads: 16,
            hide_on_launch: false,
            reduce_motion: false,
            isolate_game_env: true,
            game_env: HashMap::new(),
            discord_rpc: true,
            gamepad_nav: true,
            telemetry_enabled: true,
            curseforge_api_key: String::new(),
        }
    }
}

impl Settings {
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        crate::util::write_json_atomic(path, self)
    }
    /// Env var wins; otherwise the built-in Live client ID (no Azure app required).
    pub fn effective_client_id(&self) -> String {
        if let Ok(v) = std::env::var("CAIRN_MSA_CLIENT_ID") {
            if !v.trim().is_empty() {
                return v.trim().to_string();
            }
        }
        LIVE_DEFAULT_CLIENT_ID.to_string()
    }

    /// Environment for the Minecraft Java process.
    pub fn game_process_env(&self) -> Vec<(OsString, OsString)> {
        let strip: HashSet<&str> = LAUNCHER_ENV_STRIP.iter().copied().collect();
        let mut map: HashMap<String, String> = std::env::vars()
            .filter(|(k, _)| {
                if self.isolate_game_env && strip.contains(k.as_str()) {
                    return false;
                }
                true
            })
            .collect();
        for (k, v) in &self.game_env {
            let key = k.trim();
            if key.is_empty() {
                continue;
            }
            if v.is_empty() {
                map.remove(key);
            } else {
                map.insert(key.to_string(), v.clone());
            }
        }
        map.into_iter().map(|(k, v)| (OsString::from(k), OsString::from(v))).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_gdk_when_isolated() {
        // Cannot safely mutate process env in parallel tests; unit-check the strip list membership.
        assert!(LAUNCHER_ENV_STRIP.contains(&"GDK_BACKEND"));
        assert!(LAUNCHER_ENV_STRIP.contains(&"WEBKIT_DISABLE_DMABUF_RENDERER"));
        let s = Settings::default();
        assert!(s.isolate_game_env);
        assert_eq!(s.effective_client_id(), LIVE_DEFAULT_CLIENT_ID);
    }
}
