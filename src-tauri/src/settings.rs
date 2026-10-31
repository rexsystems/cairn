use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::path::Path;

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
    /// Kept for older settings.json files; ignored; see msa_client.rs.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub msa_client_id: String,
    pub concurrent_downloads: usize,
    pub hide_on_launch: bool,
    pub reduce_motion: bool,
    /// Drop launcher-only env vars (GDK_BACKEND, WebKit flags, …) before starting the game.
    pub isolate_game_env: bool,
    /// Extra KEY → value for the game process. Empty value removes that key.
    pub game_env: HashMap<String, String>,
    /// Show Discord Rich Presence while Minecraft is running.
    pub discord_rpc: bool,
    /// Put the instance name in the presence; off shows "Playing Minecraft".
    pub discord_show_instance: bool,
    /// Override for the Discord application id (CAIRN_DISCORD_APP_ID wins over this).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub discord_app_id: String,
    /// Launcher update channel: "stable" or "beta".
    pub update_channel: String,
    /// Move focus with a gamepad in the launcher UI.
    pub gamepad_nav: bool,
    /// Anonymous usage events (Umami). Only sent once the user has answered the privacy prompt.
    pub telemetry_enabled: bool,
    /// Crash and error reports (GlitchTip). Same consent rule as telemetry.
    pub crash_reports_enabled: bool,
    /// The user has seen and answered the first-run privacy prompt.
    pub privacy_prompt_answered: bool,
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
            discord_show_instance: true,
            discord_app_id: String::new(),
            update_channel: "stable".into(),
            gamepad_nav: true,
            telemetry_enabled: false,
            crash_reports_enabled: false,
            privacy_prompt_answered: false,
            curseforge_api_key: String::new(),
        }
    }
}

impl Settings {
    /// Usage events may be sent. Older files saved `telemetryEnabled: true` before consent existed,
    /// so the opt-in only counts after the prompt was answered.
    pub fn usage_stats_allowed(&self) -> bool {
        self.privacy_prompt_answered && self.telemetry_enabled
    }

    pub fn crash_reports_allowed(&self) -> bool {
        self.privacy_prompt_answered && self.crash_reports_enabled
    }

    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        crate::util::write_json_atomic(path, self)
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
    fn telemetry_needs_answered_prompt() {
        assert!(!Settings::default().usage_stats_allowed());
        assert!(!Settings::default().crash_reports_allowed());
        let legacy: Settings = serde_json::from_str(r#"{"telemetryEnabled":true}"#).unwrap();
        assert!(!legacy.usage_stats_allowed());
        let opted_in: Settings =
            serde_json::from_str(r#"{"telemetryEnabled":true,"crashReportsEnabled":true,"privacyPromptAnswered":true}"#).unwrap();
        assert!(opted_in.usage_stats_allowed());
        assert!(opted_in.crash_reports_allowed());
    }

    #[test]
    fn strips_gdk_when_isolated() {
        // Cannot safely mutate process env in parallel tests; unit-check the strip list membership.
        assert!(LAUNCHER_ENV_STRIP.contains(&"GDK_BACKEND"));
        assert!(LAUNCHER_ENV_STRIP.contains(&"WEBKIT_DISABLE_DMABUF_RENDERER"));
        let s = Settings::default();
        assert!(s.isolate_game_env);
    }
}
