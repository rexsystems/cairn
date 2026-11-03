//! Friends, presence and notifications for Cairn Client players. Presence only ever comes from the
//! in-game heartbeat, and every read is scoped to the signed-in player's own friends.
use super::{lock, normalize_uuid, now, session, valid_name, ApiError, ApiResult, Shared};
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::{body::Bytes, Json};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::Notify;

pub const MAX_FRIENDS: i64 = 200;
const MAX_OUTGOING: i64 = 100;
const INBOX_SIZE: usize = 50;
const EVENT_TTL: Duration = Duration::from_secs(600);
/// Stays under the router's 20 s request timeout.
pub const MAX_WAIT_SECS: u64 = 15;
const MAX_WAITERS: usize = 10_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Activity {
    pub version: String,
    pub mode: String,
    pub server: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Presence {
    pub at: Instant,
    pub activity: Option<Activity>,
}

struct Event {
    id: u64,
    from: String,
    at: Instant,
    body: Value,
}

#[derive(Default)]
struct Inbox {
    events: VecDeque<Event>,
    notify: Arc<Notify>,
}

#[derive(Default)]
pub struct Social {
    inboxes: Mutex<HashMap<String, Inbox>>,
    next_id: AtomicU64,
    waiters: AtomicUsize,
}

impl Social {
    fn push(&self, to: &str, from: &str, body: Value) {
        // Ids are taken under the lock so a reader's cursor never skips an event still being queued.
        let mut inboxes = lock(&self.inboxes);
        let id = self.next_id.fetch_add(1, Ordering::Relaxed) + 1;
        let inbox = inboxes.entry(to.to_owned()).or_default();
        let mut body = body;
        body["id"] = json!(id);
        inbox.events.push_back(Event { id, from: from.to_owned(), at: Instant::now(), body });
        while inbox.events.len() > INBOX_SIZE {
            inbox.events.pop_front();
        }
        inbox.notify.notify_waiters();
    }

    /// Drops queued events between two players, e.g. a pending request toast after a block.
    fn purge(&self, a: &str, b: &str) {
        let mut inboxes = lock(&self.inboxes);
        for (owner, other) in [(a, b), (b, a)] {
            if let Some(inbox) = inboxes.get_mut(owner) {
                inbox.events.retain(|e| e.from != other);
            }
        }
    }

    pub fn sweep(&self) {
        lock(&self.inboxes).retain(|_, inbox| {
            inbox.events.retain(|e| e.at.elapsed() < EVENT_TTL);
            !inbox.events.is_empty() || Arc::strong_count(&inbox.notify) > 1
        });
    }

    fn newer(&self, uuid: &str, after: u64) -> (Vec<Value>, u64, Arc<Notify>) {
        let mut inboxes = lock(&self.inboxes);
        let inbox = inboxes.entry(uuid.to_owned()).or_default();
        let latest = self.next_id.load(Ordering::Relaxed);
        let events = inbox.events.iter().filter(|e| e.id > after && e.at.elapsed() < EVENT_TTL).map(|e| e.body.clone()).collect();
        (events, latest, inbox.notify.clone())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    /// "full" shares game and server, "game" hides the server, "status" shares only online/offline.
    pub share: String,
    pub appear_offline: bool,
    /// "everyone" or "none".
    pub requests: String,
    pub dnd: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self { share: "full".into(), appear_offline: false, requests: "everyone".into(), dnd: false }
    }
}

impl Settings {
    fn json(&self) -> Value {
        json!({ "share": self.share, "appearOffline": self.appear_offline, "requests": self.requests, "dnd": self.dnd })
    }
}

pub fn settings(conn: &Connection, uuid: &str) -> rusqlite::Result<Settings> {
    Ok(conn
        .query_row("SELECT share, appear_offline, requests, dnd FROM social_settings WHERE uuid = ?1", [uuid], |r| {
            Ok(Settings { share: r.get(0)?, appear_offline: r.get::<_, i64>(1)? != 0, requests: r.get(2)?, dnd: r.get::<_, i64>(3)? != 0 })
        })
        .optional()?
        .unwrap_or_default())
}

fn friend_ids(conn: &Connection, uuid: &str) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT friend FROM friends WHERE owner = ?1")?;
    let rows = stmt.query_map([uuid], |r| r.get(0))?;
    rows.collect()
}

fn exists(conn: &Connection, sql: &str, a: &str, b: &str) -> rusqlite::Result<bool> {
    Ok(conn.query_row(sql, [a, b], |_| Ok(())).optional()?.is_some())
}

fn are_friends(conn: &Connection, a: &str, b: &str) -> rusqlite::Result<bool> {
    exists(conn, "SELECT 1 FROM friends WHERE owner = ?1 AND friend = ?2", a, b)
}

/// What a friend may see of this player right now. Hidden fields are never computed into the reply.
pub fn view(presence: Option<&Presence>, window: Duration, s: &Settings) -> (&'static str, Option<Value>) {
    let live = presence.filter(|p| p.at.elapsed() < window);
    let Some(live) = live.filter(|_| !s.appear_offline) else { return ("offline", None) };
    let status = if s.dnd { "dnd" } else { "online" };
    let activity = live.activity.as_ref().and_then(|a| match s.share.as_str() {
        "full" => Some(json!({ "version": a.version, "mode": a.mode, "server": a.server })),
        "game" => Some(json!({ "version": a.version, "mode": a.mode, "server": Value::Null })),
        _ => None,
    });
    (status, activity)
}

fn visible(presence: Option<&Presence>, window: Duration, s: &Settings) -> bool {
    view(presence, window, s).0 != "offline"
}

fn valid_version(v: &str) -> bool {
    (1..=32).contains(&v.len()) && v.bytes().all(|b| b.is_ascii_alphanumeric() || b".-_+ ".contains(&b))
}

fn valid_server(v: &str) -> bool {
    (1..=255).contains(&v.len()) && v.bytes().all(|b| b.is_ascii_alphanumeric() || b".-_:[]".contains(&b))
}

/// Parses the optional heartbeat body. Older clients send `{}` and simply share no activity.
pub fn parse_activity(body: &[u8]) -> ApiResult<Option<Activity>> {
    if body.iter().all(u8::is_ascii_whitespace) {
        return Ok(None);
    }
    let bad = |msg: &str| ApiError::new(StatusCode::BAD_REQUEST, msg.to_owned());
    let value: Value = serde_json::from_slice(body).map_err(|_| bad("Heartbeat body must be JSON."))?;
    let Some(a) = value.get("activity").filter(|a| !a.is_null()) else { return Ok(None) };
    let version = a.get("version").and_then(Value::as_str).unwrap_or("").trim();
    let mode = a.get("mode").and_then(Value::as_str).unwrap_or("");
    if !valid_version(version) {
        return Err(bad("Invalid game version."));
    }
    if !matches!(mode, "menu" | "singleplayer" | "multiplayer") {
        return Err(bad("mode must be menu, singleplayer or multiplayer."));
    }
    let server = match a.get("server").and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()) {
        Some(s) if mode == "multiplayer" => {
            if !valid_server(s) {
                return Err(bad("Invalid server address."));
            }
            Some(s.to_ascii_lowercase())
        }
        _ => None,
    };
    Ok(Some(Activity { version: version.to_owned(), mode: mode.to_owned(), server }))
}

fn person(uuid: &str, name: &str) -> Value {
    json!({ "uuid": uuid, "name": name })
}

/// Records a heartbeat and tells friends when the player appears or joins a server they may see.
pub fn heartbeat(state: &Shared, uuid: &str, name: &str, activity: Option<Activity>) -> ApiResult<()> {
    let window = Duration::from_secs(state.cfg.online_window_secs);
    let conn = state.db();
    let s = settings(&conn, uuid)?;
    let previous = lock(&state.presence).insert(uuid.to_owned(), Presence { at: Instant::now(), activity: activity.clone() });
    let was = visible(previous.as_ref(), window, &s);
    if s.appear_offline {
        return Ok(());
    }
    let old_server = previous.filter(|p| p.at.elapsed() < window).and_then(|p| p.activity).and_then(|a| a.server);
    let new_server = activity.as_ref().and_then(|a| a.server.clone());
    let joined = s.share == "full" && new_server.is_some() && new_server != old_server;
    if was && !joined {
        return Ok(());
    }
    for friend in friend_ids(&conn, uuid)? {
        if !was {
            state.social.push(&friend, uuid, json!({ "type": "online", "from": person(uuid, name), "dnd": s.dnd }));
        }
        if joined {
            let version = activity.as_ref().map(|a| a.version.clone());
            state.social.push(&friend, uuid, json!({ "type": "server", "from": person(uuid, name), "server": new_server, "version": version }));
        }
    }
    Ok(())
}

fn resolve(conn: &Connection, target: &str) -> ApiResult<(String, String)> {
    let target = target.trim();
    let found: Option<(String, String)> = if let Some(uuid) = normalize_uuid(target) {
        conn.query_row("SELECT uuid, name FROM users WHERE uuid = ?1 AND name != '' AND role != 'banned'", [&uuid], |r| Ok((r.get(0)?, r.get(1)?))).optional()?
    } else if valid_name(target) {
        conn.query_row("SELECT uuid, name FROM users WHERE name = ?1 COLLATE NOCASE AND role != 'banned' ORDER BY last_seen DESC LIMIT 1", [target], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .optional()?
    } else {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "Enter a Minecraft name or UUID."));
    };
    found.ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "No Cairn Client player with that name has signed in to this server yet."))
}

fn people(conn: &Connection, sql: &str, uuid: &str) -> rusqlite::Result<Vec<Value>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([uuid], |r| Ok(json!({ "uuid": r.get::<_, String>(0)?, "name": r.get::<_, String>(1)?, "at": r.get::<_, i64>(2)? })))?;
    rows.collect()
}

pub async fn list(State(state): State<Shared>, headers: HeaderMap) -> ApiResult<Json<Value>> {
    let user = session(&state, &headers)?;
    let window = Duration::from_secs(state.cfg.online_window_secs);
    let conn = state.db();
    let rows: Vec<(String, String, i64)> = {
        let mut stmt = conn.prepare(
            "SELECT f.friend, u.name, f.since FROM friends f JOIN users u ON u.uuid = f.friend WHERE f.owner = ?1 AND u.role != 'banned' ORDER BY u.name COLLATE NOCASE",
        )?;
        let rows = stmt.query_map([&user.uuid], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
        rows.collect::<Result<_, _>>()?
    };
    let presence: Vec<Option<Presence>> = {
        let map = lock(&state.presence);
        rows.iter().map(|(uuid, _, _)| map.get(uuid).cloned()).collect()
    };
    let mut friends = Vec::with_capacity(rows.len());
    for ((uuid, name, since), presence) in rows.into_iter().zip(presence) {
        let (status, activity) = view(presence.as_ref(), window, &settings(&conn, &uuid)?);
        friends.push(json!({ "uuid": uuid, "name": name, "since": since, "status": status, "activity": activity }));
    }
    let incoming =
        people(&conn, "SELECT r.sender, u.name, r.created FROM friend_requests r JOIN users u ON u.uuid = r.sender WHERE r.target = ?1 ORDER BY r.created", &user.uuid)?;
    let outgoing =
        people(&conn, "SELECT r.target, u.name, r.created FROM friend_requests r JOIN users u ON u.uuid = r.target WHERE r.sender = ?1 ORDER BY r.created", &user.uuid)?;
    let blocked = people(
        &conn,
        "SELECT b.blocked, COALESCE(u.name, ''), b.created FROM blocks b LEFT JOIN users u ON u.uuid = b.blocked WHERE b.owner = ?1 ORDER BY b.created",
        &user.uuid,
    )?;
    Ok(Json(json!({
        "friends": friends, "incoming": incoming, "outgoing": outgoing, "blocked": blocked,
        "settings": settings(&conn, &user.uuid)?.json(),
    })))
}

#[derive(Deserialize)]
pub struct RequestBody {
    target: String,
}

pub async fn send_request(State(state): State<Shared>, headers: HeaderMap, Json(body): Json<RequestBody>) -> ApiResult<Json<Value>> {
    let user = session(&state, &headers)?;
    let conn = state.db();
    let (target, name) = resolve(&conn, &body.target)?;
    if target == user.uuid {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "You can't add yourself."));
    }
    if exists(&conn, "SELECT 1 FROM blocks WHERE owner = ?1 AND blocked = ?2", &user.uuid, &target)? {
        return Err(ApiError::new(StatusCode::CONFLICT, "Unblock this player first."));
    }
    // Looks like a normal send, so nobody can probe whether they were blocked.
    if exists(&conn, "SELECT 1 FROM blocks WHERE owner = ?1 AND blocked = ?2", &target, &user.uuid)? {
        return Ok(Json(json!({ "status": "sent", "uuid": target, "name": name })));
    }
    if are_friends(&conn, &user.uuid, &target)? {
        return Err(ApiError::new(StatusCode::CONFLICT, "You are already friends."));
    }
    if exists(&conn, "SELECT 1 FROM friend_requests WHERE sender = ?1 AND target = ?2", &target, &user.uuid)? {
        drop(conn);
        accept_from(&state, &user.uuid, &user.name, &target)?;
        return Ok(Json(json!({ "status": "accepted", "uuid": target, "name": name })));
    }
    if settings(&conn, &target)?.requests == "none" {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "This player isn't accepting friend requests."));
    }
    let outgoing: i64 = conn.query_row("SELECT COUNT(*) FROM friend_requests WHERE sender = ?1", [&user.uuid], |r| r.get(0))?;
    if outgoing >= MAX_OUTGOING {
        return Err(ApiError::new(StatusCode::TOO_MANY_REQUESTS, "Too many pending requests. Cancel some first."));
    }
    let added = conn.execute("INSERT OR IGNORE INTO friend_requests (sender, target, created) VALUES (?1, ?2, ?3)", params![user.uuid, target, now()])?;
    if added > 0 {
        state.social.push(&target, &user.uuid, json!({ "type": "request", "from": person(&user.uuid, &user.name) }));
    }
    Ok(Json(json!({ "status": "sent", "uuid": target, "name": name })))
}

fn friend_count(conn: &Connection, uuid: &str) -> rusqlite::Result<i64> {
    conn.query_row("SELECT COUNT(*) FROM friends WHERE owner = ?1", [uuid], |r| r.get(0))
}

fn accept_from(state: &Shared, me: &str, my_name: &str, sender: &str) -> ApiResult<()> {
    let conn = state.db();
    if friend_count(&conn, me)? >= MAX_FRIENDS || friend_count(&conn, sender)? >= MAX_FRIENDS {
        return Err(ApiError::new(StatusCode::CONFLICT, format!("Friend lists are limited to {MAX_FRIENDS} players.")));
    }
    let t = now();
    conn.execute("DELETE FROM friend_requests WHERE (sender = ?1 AND target = ?2) OR (sender = ?2 AND target = ?1)", [me, sender])?;
    conn.execute("INSERT OR IGNORE INTO friends (owner, friend, since) VALUES (?1, ?2, ?3), (?2, ?1, ?3)", params![me, sender, t])?;
    state.social.push(sender, me, json!({ "type": "accepted", "from": person(me, my_name) }));
    Ok(())
}

pub async fn answer_request(State(state): State<Shared>, headers: HeaderMap, Path((other, action)): Path<(String, String)>) -> ApiResult<Json<Value>> {
    let user = session(&state, &headers)?;
    let other = normalize_uuid(&other).ok_or_else(|| ApiError::new(StatusCode::BAD_REQUEST, "Invalid UUID."))?;
    let (sender, target) = if action == "cancel" { (user.uuid.as_str(), other.as_str()) } else { (other.as_str(), user.uuid.as_str()) };
    if !matches!(action.as_str(), "accept" | "decline" | "cancel") {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "Unknown action."));
    }
    let pending = exists(&state.db(), "SELECT 1 FROM friend_requests WHERE sender = ?1 AND target = ?2", sender, target)?;
    if !pending {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "No such friend request."));
    }
    if action == "accept" {
        accept_from(&state, &user.uuid, &user.name, &other)?;
    } else {
        state.db().execute("DELETE FROM friend_requests WHERE sender = ?1 AND target = ?2", [sender, target])?;
        state.social.purge(sender, target);
    }
    Ok(Json(json!({ "ok": true })))
}

fn unfriend(conn: &Connection, a: &str, b: &str) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM friends WHERE (owner = ?1 AND friend = ?2) OR (owner = ?2 AND friend = ?1)", [a, b])?;
    Ok(())
}

pub async fn remove(State(state): State<Shared>, headers: HeaderMap, Path(other): Path<String>) -> ApiResult<StatusCode> {
    let user = session(&state, &headers)?;
    let other = normalize_uuid(&other).ok_or_else(|| ApiError::new(StatusCode::BAD_REQUEST, "Invalid UUID."))?;
    unfriend(&state.db(), &user.uuid, &other)?;
    state.social.purge(&user.uuid, &other);
    Ok(StatusCode::NO_CONTENT)
}

pub async fn block(State(state): State<Shared>, headers: HeaderMap, Path(other): Path<String>) -> ApiResult<StatusCode> {
    let user = session(&state, &headers)?;
    let other = normalize_uuid(&other).ok_or_else(|| ApiError::new(StatusCode::BAD_REQUEST, "Invalid UUID."))?;
    if other == user.uuid {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "You can't block yourself."));
    }
    let conn = state.db();
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM blocks WHERE owner = ?1", [&user.uuid], |r| r.get(0))?;
    if count >= 1000 {
        return Err(ApiError::new(StatusCode::CONFLICT, "Your block list is full."));
    }
    unfriend(&conn, &user.uuid, &other)?;
    conn.execute("DELETE FROM friend_requests WHERE (sender = ?1 AND target = ?2) OR (sender = ?2 AND target = ?1)", [&user.uuid, &other])?;
    conn.execute("INSERT OR IGNORE INTO blocks (owner, blocked, created) VALUES (?1, ?2, ?3)", params![user.uuid, other, now()])?;
    state.social.purge(&user.uuid, &other);
    Ok(StatusCode::NO_CONTENT)
}

pub async fn unblock(State(state): State<Shared>, headers: HeaderMap, Path(other): Path<String>) -> ApiResult<StatusCode> {
    let user = session(&state, &headers)?;
    let other = normalize_uuid(&other).ok_or_else(|| ApiError::new(StatusCode::BAD_REQUEST, "Invalid UUID."))?;
    state.db().execute("DELETE FROM blocks WHERE owner = ?1 AND blocked = ?2", [&user.uuid, &other])?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SettingsBody {
    share: Option<String>,
    appear_offline: Option<bool>,
    requests: Option<String>,
    dnd: Option<bool>,
}

pub async fn get_settings(State(state): State<Shared>, headers: HeaderMap) -> ApiResult<Json<Value>> {
    let user = session(&state, &headers)?;
    Ok(Json(settings(&state.db(), &user.uuid)?.json()))
}

pub async fn put_settings(State(state): State<Shared>, headers: HeaderMap, Json(body): Json<SettingsBody>) -> ApiResult<Json<Value>> {
    let user = session(&state, &headers)?;
    let bad = |m: &str| ApiError::new(StatusCode::BAD_REQUEST, m.to_owned());
    if body.share.as_deref().is_some_and(|s| !matches!(s, "full" | "game" | "status")) {
        return Err(bad("share must be full, game or status."));
    }
    if body.requests.as_deref().is_some_and(|s| !matches!(s, "everyone" | "none")) {
        return Err(bad("requests must be everyone or none."));
    }
    let window = Duration::from_secs(state.cfg.online_window_secs);
    let conn = state.db();
    let old = settings(&conn, &user.uuid)?;
    let new = Settings {
        share: body.share.unwrap_or_else(|| old.share.clone()),
        appear_offline: body.appear_offline.unwrap_or(old.appear_offline),
        requests: body.requests.unwrap_or_else(|| old.requests.clone()),
        dnd: body.dnd.unwrap_or(old.dnd),
    };
    conn.execute(
        "INSERT INTO social_settings (uuid, share, appear_offline, requests, dnd) VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(uuid) DO UPDATE SET share = excluded.share, appear_offline = excluded.appear_offline, requests = excluded.requests, dnd = excluded.dnd",
        params![user.uuid, new.share, new.appear_offline as i64, new.requests, new.dnd as i64],
    )?;
    if old.json() != new.json() {
        // The player's other clients (launcher, a second game) learn DND/privacy changes without polling.
        state.social.push(&user.uuid, &user.uuid, json!({ "type": "settings", "settings": new.json() }));
    }
    let presence = lock(&state.presence).get(&user.uuid).cloned();
    if !visible(presence.as_ref(), window, &old) && visible(presence.as_ref(), window, &new) {
        for friend in friend_ids(&conn, &user.uuid)? {
            state.social.push(&friend, &user.uuid, json!({ "type": "online", "from": person(&user.uuid, &user.name), "dnd": new.dnd }));
        }
    }
    Ok(Json(new.json()))
}

#[derive(Deserialize)]
pub struct EventsQuery {
    after: Option<u64>,
    wait: Option<u64>,
}

/// Long poll: answers at once when newer events exist, otherwise waits up to `wait` seconds.
/// Without `after` it only returns the current cursor, so a fresh client does not replay old toasts.
pub async fn events(State(state): State<Shared>, headers: HeaderMap, Query(q): Query<EventsQuery>) -> ApiResult<Json<Value>> {
    let user = session(&state, &headers)?;
    let wait = Duration::from_secs(q.wait.unwrap_or(0).min(MAX_WAIT_SECS));
    let Some(after) = q.after else {
        let (_, latest, _) = state.social.newer(&user.uuid, u64::MAX);
        return Ok(Json(json!({ "events": [], "cursor": latest })));
    };
    let deadline = Instant::now() + wait;
    loop {
        let (events, latest, notify) = state.social.newer(&user.uuid, after);
        // A cursor from before a server restart is ahead of the counter; restart from now.
        if !events.is_empty() || after > latest || Instant::now() >= deadline {
            return Ok(Json(json!({ "events": events, "cursor": latest })));
        }
        if state.social.waiters.fetch_add(1, Ordering::Relaxed) >= MAX_WAITERS {
            state.social.waiters.fetch_sub(1, Ordering::Relaxed);
            return Ok(Json(json!({ "events": [], "cursor": after, "retryAfter": 30 })));
        }
        let notified = notify.notified();
        tokio::pin!(notified);
        notified.as_mut().enable();
        // Re-check after registering, so an event pushed in between is not missed.
        let raced = !state.social.newer(&user.uuid, after).0.is_empty();
        if !raced {
            let _ = tokio::time::timeout(deadline.saturating_duration_since(Instant::now()), notified).await;
        }
        state.social.waiters.fetch_sub(1, Ordering::Relaxed);
    }
}

pub fn heartbeat_body(body: &Bytes) -> ApiResult<Option<Activity>> {
    if body.len() > 4096 {
        return Err(ApiError::new(StatusCode::PAYLOAD_TOO_LARGE, "Heartbeat body too large."));
    }
    parse_activity(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activity_is_validated_and_server_only_kept_in_multiplayer() {
        assert_eq!(parse_activity(b"").unwrap(), None);
        assert_eq!(parse_activity(b"{}").unwrap(), None);
        let a = parse_activity(br#"{"activity":{"version":"1.21.11","mode":"multiplayer","server":"Play.Example.net:25565"}}"#).unwrap().unwrap();
        assert_eq!(a.server.as_deref(), Some("play.example.net:25565"));
        let a = parse_activity(br#"{"activity":{"version":"26.3","mode":"singleplayer","server":"leak.example"}}"#).unwrap().unwrap();
        assert_eq!(a.server, None);
        assert!(parse_activity(br#"{"activity":{"version":"26.3","mode":"hacking"}}"#).is_err());
        assert!(parse_activity(br#"{"activity":{"version":"<script>","mode":"menu"}}"#).is_err());
        assert!(parse_activity(br#"{"activity":{"version":"1.8.9","mode":"multiplayer","server":"a b"}}"#).is_err());
        assert!(parse_activity(b"nope").is_err());
    }

    #[test]
    fn view_hides_what_settings_hide() {
        let window = Duration::from_secs(90);
        let p = Presence { at: Instant::now(), activity: Some(Activity { version: "26.3".into(), mode: "multiplayer".into(), server: Some("mc.example".into()) }) };
        let full = Settings::default();
        assert_eq!(view(Some(&p), window, &full).1.unwrap()["server"], "mc.example");
        let game = Settings { share: "game".into(), ..Settings::default() };
        assert_eq!(view(Some(&p), window, &game).1.unwrap()["server"], Value::Null);
        let status = Settings { share: "status".into(), dnd: true, ..Settings::default() };
        assert_eq!(view(Some(&p), window, &status), ("dnd", None));
        let ghost = Settings { appear_offline: true, ..Settings::default() };
        assert_eq!(view(Some(&p), window, &ghost), ("offline", None));
        assert_eq!(view(None, window, &full), ("offline", None));
    }
}
