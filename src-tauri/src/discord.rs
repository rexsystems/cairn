//! Discord Rich Presence while a game is running.
//!
//! A single worker thread owns the IPC connection. It reconnects when Discord starts
//! later or restarts, and shows the most recently started game when several run at once.

use crate::util::LockExt;
use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Cairn's Discord application. Override with `CAIRN_DISCORD_APP_ID` or `discordAppId` in settings.
pub const DEFAULT_APP_ID: &str = "1566104003291390072";

/// Rich presence art keys uploaded to the Discord application.
pub const LARGE_IMAGE: &str = "cairn_logo";

const RETRY_EVERY: Duration = Duration::from_secs(20);
/// Re-send the activity now and then so a silently dropped pipe is noticed.
const REFRESH_EVERY: Duration = Duration::from_secs(60);
const DOWNLOAD_URL: &str = "https://github.com/rexsystems/cairn/releases/latest";

#[derive(Clone, Debug, PartialEq)]
pub struct Session {
    pub instance_id: String,
    pub instance_name: String,
    pub mc_version: String,
    pub loader: String,
    pub loader_version: Option<String>,
    /// Unix seconds; Discord shows elapsed time from here.
    pub started: i64,
}

impl Session {
    pub fn new(instance_id: &str, instance_name: &str, mc_version: &str, loader: &str, loader_version: Option<&str>) -> Self {
        Session {
            instance_id: instance_id.to_string(),
            instance_name: instance_name.to_string(),
            mc_version: mc_version.to_string(),
            loader: loader.to_string(),
            loader_version: loader_version.filter(|v| !v.is_empty()).map(String::from),
            started: SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    pub enabled: bool,
    pub show_instance_name: bool,
    pub app_id: String,
}

impl Options {
    pub fn from_settings(s: &crate::settings::Settings) -> Self {
        let app_id = std::env::var("CAIRN_DISCORD_APP_ID")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| valid_app_id(v))
            .or_else(|| Some(s.discord_app_id.trim().to_string()).filter(|v| valid_app_id(v)))
            .unwrap_or_else(|| DEFAULT_APP_ID.to_string());
        Options { enabled: s.discord_rpc, show_instance_name: s.discord_show_instance, app_id }
    }
}

fn valid_app_id(v: &str) -> bool {
    !v.is_empty() && v != "0" && v.bytes().all(|b| b.is_ascii_digit())
}

enum Msg {
    Start(Session),
    Stop(String),
    Configure(Options),
}

fn worker() -> &'static Mutex<Sender<Msg>> {
    static TX: OnceLock<Mutex<Sender<Msg>>> = OnceLock::new();
    TX.get_or_init(|| {
        let (tx, rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("discord-rpc".into())
            .spawn(move || run(rx))
            .expect("spawning the Discord presence thread");
        Mutex::new(tx)
    })
}

fn send(msg: Msg) {
    let tx = worker().locked();
    let _ = tx.send(msg);
}

/// Apply settings (on startup and whenever they are saved).
pub fn configure(options: Options) {
    send(Msg::Configure(options));
}

pub fn set_playing(session: Session) {
    send(Msg::Start(session));
}

pub fn clear(instance_id: &str) {
    send(Msg::Stop(instance_id.to_string()));
}

struct Worker {
    options: Options,
    sessions: Vec<Session>,
    client: Option<DiscordIpcClient>,
    shown: Option<Session>,
    last_attempt: Option<Instant>,
    last_sent: Option<Instant>,
}

fn run(rx: mpsc::Receiver<Msg>) {
    let mut w = Worker {
        options: Options { enabled: false, show_instance_name: true, app_id: DEFAULT_APP_ID.into() },
        sessions: Vec::new(),
        client: None,
        shown: None,
        last_attempt: None,
        last_sent: None,
    };
    loop {
        match rx.recv_timeout(RETRY_EVERY) {
            Ok(Msg::Start(s)) => {
                w.sessions.retain(|x| x.instance_id != s.instance_id);
                w.sessions.push(s);
                w.last_attempt = None;
            }
            Ok(Msg::Stop(id)) => w.sessions.retain(|x| x.instance_id != id),
            Ok(Msg::Configure(o)) => {
                if o.app_id != w.options.app_id {
                    w.disconnect();
                }
                w.options = o;
                w.shown = None;
                w.last_attempt = None;
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        w.sync();
    }
    w.disconnect();
}

impl Worker {
    fn wanted(&self) -> Option<&Session> {
        if self.options.enabled {
            self.sessions.last()
        } else {
            None
        }
    }

    fn sync(&mut self) {
        let Some(session) = self.wanted().cloned() else {
            if self.client.is_some() {
                self.disconnect();
            }
            return;
        };
        let stale = self.last_sent.is_none_or(|t| t.elapsed() >= REFRESH_EVERY);
        if self.client.is_some() && self.shown.as_ref() == Some(&session) && !stale {
            return;
        }
        if self.client.is_none() {
            if self.last_attempt.is_some_and(|t| t.elapsed() < RETRY_EVERY) {
                return;
            }
            self.last_attempt = Some(Instant::now());
            let Ok(mut client) = DiscordIpcClient::new(&self.options.app_id) else { return };
            if client.connect().is_err() {
                return;
            }
            self.client = Some(client);
        }
        let texts = PresenceText::new(&session, self.options.show_instance_name);
        let ok = self.client.as_mut().is_some_and(|c| c.set_activity(texts.activity(session.started)).is_ok());
        if ok {
            self.shown = Some(session);
            self.last_sent = Some(Instant::now());
        } else {
            // Discord closed or restarted; reconnect on the next tick.
            self.client = None;
            self.shown = None;
            self.last_attempt = None;
        }
    }

    fn disconnect(&mut self) {
        if let Some(mut c) = self.client.take() {
            let _ = c.clear_activity();
            let _ = c.close();
        }
        self.shown = None;
        self.last_sent = None;
    }
}

/// Owned strings for one activity, kept separate so the text is unit-testable.
#[derive(Debug, PartialEq)]
pub struct PresenceText {
    pub details: String,
    pub state: String,
    pub large_text: String,
    pub small_image: &'static str,
    pub small_text: String,
}

impl PresenceText {
    pub fn new(s: &Session, show_instance_name: bool) -> Self {
        let label = loader_label(&s.loader);
        let details = if show_instance_name && !s.instance_name.trim().is_empty() {
            truncate(&s.instance_name, 128)
        } else {
            "Playing Minecraft".to_string()
        };
        let small_text = match &s.loader_version {
            Some(v) if s.loader != "vanilla" && s.loader != "horizon" => format!("{label} {v}"),
            _ => label.to_string(),
        };
        PresenceText {
            details,
            state: truncate(&format!("Minecraft {} · {label}", s.mc_version), 128),
            large_text: format!("Cairn {}", env!("CARGO_PKG_VERSION")),
            small_image: loader_asset(&s.loader),
            small_text: truncate(&small_text, 128),
        }
    }

    fn activity(&self, started: i64) -> activity::Activity<'_> {
        activity::Activity::new()
            .details(&self.details)
            .state(&self.state)
            .assets(
                activity::Assets::new()
                    .large_image(LARGE_IMAGE)
                    .large_text(&self.large_text)
                    .small_image(self.small_image)
                    .small_text(&self.small_text),
            )
            .timestamps(activity::Timestamps::new().start(started))
            .buttons(vec![activity::Button::new("Get Cairn", DOWNLOAD_URL)])
    }
}

/// Discord rejects fields longer than 128 characters.
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max - 1).collect();
    out.push('…');
    out
}

fn loader_label(l: &str) -> &'static str {
    match l {
        "fabric" => "Fabric",
        "quilt" => "Quilt",
        "forge" => "Forge",
        "neoforge" => "NeoForge",
        "horizon" => "Horizon",
        _ => "Vanilla",
    }
}

/// Small image keys to upload under Rich Presence → Art Assets.
pub fn loader_asset(l: &str) -> &'static str {
    match l {
        "fabric" => "loader_fabric",
        "quilt" => "loader_quilt",
        "forge" => "loader_forge",
        "neoforge" => "loader_neoforge",
        "horizon" => "loader_horizon",
        _ => "loader_vanilla",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(loader: &str, lv: Option<&str>) -> Session {
        Session::new("i1", "Survival SMP", "1.21.1", loader, lv)
    }

    #[test]
    fn describes_modded_instances() {
        let t = PresenceText::new(&session("fabric", Some("0.16.9")), true);
        assert_eq!(t.details, "Survival SMP");
        assert_eq!(t.state, "Minecraft 1.21.1 · Fabric");
        assert_eq!(t.small_image, "loader_fabric");
        assert_eq!(t.small_text, "Fabric 0.16.9");
    }

    #[test]
    fn hides_instance_name_when_asked() {
        let t = PresenceText::new(&session("vanilla", None), false);
        assert_eq!(t.details, "Playing Minecraft");
        assert_eq!(t.small_image, "loader_vanilla");
        assert_eq!(t.small_text, "Vanilla");
    }

    #[test]
    fn truncates_long_names() {
        let mut s = session("forge", None);
        s.instance_name = "x".repeat(300);
        assert_eq!(PresenceText::new(&s, true).details.chars().count(), 128);
    }

    #[test]
    fn app_id_falls_back_to_default() {
        let mut settings = crate::settings::Settings { discord_app_id: "not-a-number".into(), ..Default::default() };
        if std::env::var("CAIRN_DISCORD_APP_ID").is_err() {
            assert_eq!(Options::from_settings(&settings).app_id, DEFAULT_APP_ID);
            settings.discord_app_id = "123456789".into();
            assert_eq!(Options::from_settings(&settings).app_id, "123456789");
        }
    }
}
