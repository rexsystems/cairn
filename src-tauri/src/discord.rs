//! Discord Rich Presence while a game runs or a server is hosted, plus join invites.
//! Sessions are joinable only after the host opts in, and stop being joinable when they end.

use crate::discord_ipc::Conn;
use crate::util::LockExt;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
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
/// How often the worker checks the pipe for join events.
const POLL_EVERY: Duration = Duration::from_millis(250);
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
    /// A Host-tab server rather than the game itself.
    pub hosting: bool,
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
            hosting: false,
        }
    }
}

/// What makes a session joinable. Lives in memory only, so every launch starts private.
#[derive(Clone, Debug, PartialEq)]
pub struct Join {
    /// Random per opt-in; Discord groups party members by it, so it carries no address.
    pub party_id: String,
    pub secret: String,
    /// Party size shown to friends. Discord hides Join without one.
    pub max: u32,
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

/// Something Discord told us that the UI has to act on.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Event {
    /// The user clicked Join (or the host accepted their request); carries the raw secret.
    Join { secret: String },
    /// Someone asked to join the user's session.
    #[serde(rename_all = "camelCase")]
    JoinRequest { user_id: String, username: String, avatar_url: Option<String> },
}

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub enabled: bool,
    pub connected: bool,
    /// Discord username the connection belongs to.
    pub user: Option<String>,
    /// Last error Discord returned for a command, such as a refused activity.
    pub error: Option<String>,
    /// Instances whose sessions are joinable right now.
    pub joinable: Vec<String>,
}

enum Msg {
    Start(Session),
    Stop { id: String, hosting: bool },
    Configure(Options),
    SetJoin(String, Option<Join>),
    Players(String, u32),
    Reply { user_id: String, accept: bool },
}

fn worker() -> &'static Mutex<Sender<Msg>> {
    static TX: OnceLock<Mutex<Sender<Msg>>> = OnceLock::new();
    TX.get_or_init(|| {
        let (tx, rx) = mpsc::channel();
        std::thread::Builder::new().name("discord-rpc".into()).spawn(move || run(rx)).expect("spawning the Discord presence thread");
        Mutex::new(tx)
    })
}

fn send(msg: Msg) {
    let tx = worker().locked();
    let _ = tx.send(msg);
}

type Sink = Box<dyn Fn(Event) + Send + Sync>;
static SINK: OnceLock<Sink> = OnceLock::new();
static STATUS: Mutex<Status> = Mutex::new(Status { enabled: false, connected: false, user: None, error: None, joinable: Vec::new() });
/// A join that arrived before the window was listening (Discord starts Cairn to deliver it).
static PENDING_JOIN: Mutex<Option<String>> = Mutex::new(None);

/// Where join events go. Set once at startup.
pub fn on_event(f: impl Fn(Event) + Send + Sync + 'static) {
    let _ = SINK.set(Box::new(f));
}

pub fn status() -> Status {
    STATUS.locked().clone()
}

pub fn take_pending_join() -> Option<String> {
    PENDING_JOIN.locked().take()
}

/// Apply settings (on startup and whenever they are saved).
pub fn configure(options: Options) {
    send(Msg::Configure(options));
}

pub fn set_playing(session: Session) {
    send(Msg::Start(session));
}

pub fn clear(instance_id: &str) {
    send(Msg::Stop { id: instance_id.to_string(), hosting: false });
}

pub fn set_hosting(mut session: Session) {
    session.hosting = true;
    send(Msg::Start(session));
}

pub fn clear_hosting(instance_id: &str) {
    send(Msg::Stop { id: instance_id.to_string(), hosting: true });
}

/// Makes an instance's sessions joinable (Some) or private again (None).
pub fn set_join(instance_id: &str, join: Option<Join>) {
    send(Msg::SetJoin(instance_id.to_string(), join));
}

pub fn set_players(instance_id: &str, online: u32) {
    send(Msg::Players(instance_id.to_string(), online));
}

/// Answers an ask-to-join request.
pub fn reply(user_id: &str, accept: bool) -> Result<(), String> {
    if !valid_user_id(user_id) {
        return Err("Unknown Discord user".into());
    }
    send(Msg::Reply { user_id: user_id.to_string(), accept });
    Ok(())
}

fn valid_user_id(v: &str) -> bool {
    !v.is_empty() && v.len() <= 20 && v.bytes().all(|b| b.is_ascii_digit())
}

struct Worker {
    options: Options,
    sessions: Vec<Session>,
    joins: HashMap<String, Join>,
    online: HashMap<String, u32>,
    conn: Option<Conn>,
    shown: Option<Value>,
    last_attempt: Option<Instant>,
    last_sent: Option<Instant>,
}

fn run(rx: mpsc::Receiver<Msg>) {
    let mut w = Worker::new();
    loop {
        let wait = if w.conn.is_some() { POLL_EVERY } else { RETRY_EVERY.min(Duration::from_secs(2)) };
        match rx.recv_timeout(wait) {
            Ok(m) => w.handle(m),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        w.sync();
    }
    w.disconnect();
}

impl Worker {
    fn new() -> Worker {
        Worker {
            options: Options { enabled: false, show_instance_name: true, app_id: DEFAULT_APP_ID.into() },
            sessions: Vec::new(),
            joins: HashMap::new(),
            online: HashMap::new(),
            conn: None,
            shown: None,
            last_attempt: None,
            last_sent: None,
        }
    }

    fn handle(&mut self, m: Msg) {
        match m {
            Msg::Start(s) => {
                self.sessions.retain(|x| !(x.instance_id == s.instance_id && x.hosting == s.hosting));
                self.sessions.push(s);
            }
            Msg::Stop { id, hosting } => {
                self.sessions.retain(|x| !(x.instance_id == id && x.hosting == hosting));
                if hosting {
                    self.online.remove(&id);
                }
                // Any session ending can take the invited address with it, so the host opts in again.
                self.joins.remove(&id);
            }
            Msg::Configure(o) => {
                if o.app_id != self.options.app_id || !o.enabled {
                    self.disconnect();
                }
                self.options = o;
                self.shown = None;
                self.last_attempt = None;
            }
            // Only a running session can be joined; a leftover invite would leak into the next one.
            Msg::SetJoin(id, Some(j)) => {
                if self.sessions.iter().any(|x| x.instance_id == id) {
                    self.joins.insert(id, j);
                }
            }
            Msg::SetJoin(id, None) => {
                self.joins.remove(&id);
            }
            Msg::Players(id, n) => {
                self.online.insert(id, n);
            }
            Msg::Reply { user_id, accept } => {
                let cmd = if accept { "SEND_ACTIVITY_JOIN_INVITE" } else { "CLOSE_ACTIVITY_REQUEST" };
                let ok = self.conn.as_mut().is_some_and(|c| c.command(cmd, json!({ "user_id": user_id }), None).is_ok());
                if !ok {
                    self.drop_conn();
                }
            }
        }
        self.publish_status();
    }

    fn publish_status(&self) {
        let mut st = STATUS.locked();
        st.enabled = self.options.enabled;
        st.connected = self.conn.is_some();
        let mut j: Vec<String> = self.joins.keys().cloned().collect();
        j.sort();
        st.joinable = j;
        if !st.connected {
            st.user = None;
        }
    }

    fn wanted(&self) -> Option<Value> {
        let s = self.sessions.last()?;
        let texts = PresenceText::new(s, self.options.show_instance_name);
        let online = self.online.get(&s.instance_id).copied().unwrap_or(0);
        Some(activity_json(&texts, s.started, self.joins.get(&s.instance_id), online))
    }

    fn sync(&mut self) {
        if !self.options.enabled {
            return;
        }
        if self.conn.is_none() {
            if self.last_attempt.is_some_and(|t| t.elapsed() < RETRY_EVERY) {
                return;
            }
            self.last_attempt = Some(Instant::now());
            let Ok(mut c) = Conn::connect(&self.options.app_id) else { return };
            let subscribed = ["ACTIVITY_JOIN", "ACTIVITY_JOIN_REQUEST"].iter().all(|evt| c.command("SUBSCRIBE", json!({}), Some(evt)).is_ok());
            if !subscribed {
                return;
            }
            self.conn = Some(c);
            self.shown = None;
            self.publish_status();
        }
        let got = match self.conn.as_mut().map(|c| c.poll()) {
            Some(Ok(v)) => v,
            _ => return self.drop_conn(),
        };
        for v in got {
            self.on_message(&v);
        }
        let want = self.wanted();
        let stale = self.last_sent.is_none_or(|t| t.elapsed() >= REFRESH_EVERY);
        if self.shown.is_some() && self.shown == want && !stale {
            return;
        }
        if self.shown.is_none() && want.is_none() && self.last_sent.is_some() {
            return;
        }
        let mut args = json!({ "pid": std::process::id() });
        if let Some(a) = &want {
            args["activity"] = a.clone();
        }
        if self.conn.as_mut().is_some_and(|c| c.command("SET_ACTIVITY", args, None).is_ok()) {
            self.shown = want;
            self.last_sent = Some(Instant::now());
        } else {
            self.drop_conn();
        }
    }

    fn on_message(&mut self, v: &Value) {
        if v.get("evt").and_then(|e| e.as_str()) == Some("READY") {
            STATUS.locked().user = v.pointer("/data/user/username").and_then(|u| u.as_str()).map(clean_name);
        }
        if v.get("evt").and_then(|e| e.as_str()) == Some("ERROR") {
            let msg = v.pointer("/data/message").and_then(|m| m.as_str()).unwrap_or("Discord refused a request");
            STATUS.locked().error = Some(clean_name(msg));
            return;
        }
        if v.get("cmd").and_then(|c| c.as_str()) == Some("SET_ACTIVITY") {
            STATUS.locked().error = None;
        }
        let Some(ev) = parse_event(v) else { return };
        if let Event::Join { secret } = &ev {
            *PENDING_JOIN.locked() = Some(secret.clone());
        }
        if let Some(sink) = SINK.get() {
            sink(ev);
        }
    }

    fn drop_conn(&mut self) {
        self.conn = None;
        self.shown = None;
        self.last_sent = None;
        self.publish_status();
    }

    fn disconnect(&mut self) {
        if let Some(mut c) = self.conn.take() {
            let _ = c.command("SET_ACTIVITY", json!({ "pid": std::process::id() }), None);
            c.close();
        }
        self.shown = None;
        self.last_sent = None;
        self.publish_status();
    }
}

fn clean_name(s: &str) -> String {
    truncate(&s.chars().filter(|c| !c.is_control()).collect::<String>(), 64)
}

/// Turns a DISPATCH frame into an event. Unknown or malformed payloads are dropped.
pub fn parse_event(v: &Value) -> Option<Event> {
    if v.get("cmd").and_then(|c| c.as_str()) != Some("DISPATCH") {
        return None;
    }
    let data = v.get("data")?;
    match v.get("evt")?.as_str()? {
        "ACTIVITY_JOIN" => {
            let secret = data.get("secret")?.as_str()?;
            (secret.len() <= crate::invite::MAX_SECRET).then(|| Event::Join { secret: secret.to_string() })
        }
        "ACTIVITY_JOIN_REQUEST" => {
            let user = data.get("user")?;
            let user_id = user.get("id")?.as_str()?;
            if !valid_user_id(user_id) {
                return None;
            }
            let name = user.get("global_name").and_then(|n| n.as_str()).or_else(|| user.get("username").and_then(|n| n.as_str())).unwrap_or("Someone");
            let avatar_url = user
                .get("avatar")
                .and_then(|a| a.as_str())
                .filter(|a| !a.is_empty() && a.len() <= 40 && a.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'))
                .map(|a| format!("https://cdn.discordapp.com/avatars/{user_id}/{a}.png?size=64"));
            Some(Event::JoinRequest { user_id: user_id.to_string(), username: clean_name(name), avatar_url })
        }
        _ => None,
    }
}

/// The SET_ACTIVITY payload. Joinable sessions trade the download button for Discord's own Join
/// button, as Discord shows one or the other.
pub fn activity_json(t: &PresenceText, started: i64, join: Option<&Join>, online: u32) -> Value {
    let mut a = json!({
        "details": t.details,
        "state": t.state,
        "timestamps": { "start": started },
        "assets": { "large_image": LARGE_IMAGE, "large_text": t.large_text, "small_image": t.small_image, "small_text": t.small_text },
        "instance": false,
    });
    match join {
        Some(j) => {
            let cur = online.max(1);
            a["party"] = json!({ "id": j.party_id, "size": [cur, j.max.max(cur).max(2)] });
            a["secrets"] = json!({ "join": j.secret });
        }
        None => a["buttons"] = json!([{ "label": "Get Cairn", "url": DOWNLOAD_URL }]),
    }
    a
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
        let named = show_instance_name && !s.instance_name.trim().is_empty();
        let details = match (s.hosting, named) {
            (true, true) => truncate(&format!("Hosting {}", s.instance_name), 128),
            (true, false) => "Hosting a Minecraft server".to_string(),
            (false, true) => truncate(&s.instance_name, 128),
            (false, false) => "Playing Minecraft".to_string(),
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

    #[test]
    fn hosting_sessions_say_so() {
        let mut s = session("fabric", None);
        s.hosting = true;
        assert_eq!(PresenceText::new(&s, true).details, "Hosting Survival SMP");
        assert_eq!(PresenceText::new(&s, false).details, "Hosting a Minecraft server");
    }

    #[test]
    fn joinable_activity_has_party_and_secret_but_no_button() {
        let t = PresenceText::new(&session("fabric", None), true);
        let private = activity_json(&t, 5, None, 0);
        assert!(private.get("party").is_none() && private.get("secrets").is_none());
        assert_eq!(private["buttons"][0]["label"], "Get Cairn");
        let j = Join { party_id: "p1".into(), secret: "c1;x".into(), max: 10 };
        let open = activity_json(&t, 5, Some(&j), 3);
        assert_eq!(open["party"], json!({ "id": "p1", "size": [3, 10] }));
        assert_eq!(open["secrets"]["join"], "c1;x");
        assert!(open.get("buttons").is_none());
        assert!(!open.to_string().contains("25565"));
        let no_cap = activity_json(&t, 5, Some(&Join { max: 0, ..j }), 0);
        assert_eq!(no_cap["party"]["size"], json!([1, 2]));
    }

    #[test]
    fn parses_join_events_and_drops_bad_ones() {
        let join = json!({ "cmd": "DISPATCH", "evt": "ACTIVITY_JOIN", "data": { "secret": "c1;abc" } });
        assert_eq!(parse_event(&join), Some(Event::Join { secret: "c1;abc".into() }));
        let long = json!({ "cmd": "DISPATCH", "evt": "ACTIVITY_JOIN", "data": { "secret": "x".repeat(300) } });
        assert_eq!(parse_event(&long), None);
        let req = json!({ "cmd": "DISPATCH", "evt": "ACTIVITY_JOIN_REQUEST", "data": { "user": { "id": "80351110224678912", "username": "nelly\u{7}", "avatar": "a_abc123" } } });
        assert_eq!(
            parse_event(&req),
            Some(Event::JoinRequest {
                user_id: "80351110224678912".into(),
                username: "nelly".into(),
                avatar_url: Some("https://cdn.discordapp.com/avatars/80351110224678912/a_abc123.png?size=64".into()),
            })
        );
        let bad_id = json!({ "cmd": "DISPATCH", "evt": "ACTIVITY_JOIN_REQUEST", "data": { "user": { "id": "../x", "username": "x" } } });
        assert_eq!(parse_event(&bad_id), None);
        let bad_avatar = json!({ "cmd": "DISPATCH", "evt": "ACTIVITY_JOIN_REQUEST", "data": { "user": { "id": "1", "username": "x", "avatar": "../../x" } } });
        assert!(matches!(parse_event(&bad_avatar), Some(Event::JoinRequest { avatar_url: None, .. })));
        assert_eq!(parse_event(&json!({ "cmd": "SET_ACTIVITY", "evt": "ACTIVITY_JOIN", "data": { "secret": "x" } })), None);
        assert!(reply("../1", true).is_err());
    }

    #[test]
    fn invites_need_a_running_session_and_end_with_it() {
        let join = || Join { party_id: "p".into(), secret: "c1;a.example:1;1.21;vanilla;;;".into(), max: 8 };
        let mut w = Worker::new();
        w.handle(Msg::SetJoin("a".into(), Some(join())));
        assert!(w.joins.is_empty(), "nothing running, so nothing to join");
        w.handle(Msg::Start(Session::new("a", "A", "1.21", "vanilla", None)));
        w.handle(Msg::Start(Session { hosting: true, ..Session::new("a", "A", "1.21", "vanilla", None) }));
        w.handle(Msg::SetJoin("a".into(), Some(join())));
        assert!(w.wanted().unwrap()["secrets"]["join"].is_string());
        w.handle(Msg::Stop { id: "a".into(), hosting: true });
        assert!(w.joins.is_empty(), "the server that was invited to is gone");
        assert!(w.wanted().unwrap().get("secrets").is_none());
    }
}
