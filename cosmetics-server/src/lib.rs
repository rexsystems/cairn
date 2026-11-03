//! Cairn cosmetics backend: Mojang-verified sign-in, equipped cosmetics, moderated
//! custom capes, staff roles, a public online counter and Cairn-only friends with presence.
pub mod cape;
pub mod config;
pub mod db;
pub mod limit;
mod social;

use axum::extract::{ConnectInfo, DefaultBodyLimit, Path, Query, Request, State};
use axum::http::{header, HeaderMap, HeaderValue, Method, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post, put};
use axum::{body::Bytes, Json, Router};
use base64::Engine;
use config::{Config, Moderation};
use rand::RngCore;
use rusqlite::Connection;
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tower_http::set_header::SetResponseHeaderLayer;

pub const CATALOG_JSON: &str = include_str!("../catalog.json");
const CHALLENGE_TTL: Duration = Duration::from_secs(120);
const MAX_LOOKUP: usize = 100;

#[derive(Deserialize)]
struct Catalog {
    capes: Vec<CatalogCape>,
    badges: Vec<CatalogBadge>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CatalogCape {
    id: String,
    frames: u32,
    frame_ms: u32,
    role: Option<String>,
}

#[derive(Deserialize)]
struct CatalogBadge {
    id: String,
    role: Option<String>,
}

pub struct AppState {
    pub cfg: Config,
    db: Mutex<Connection>,
    catalog: Catalog,
    limiter: limit::RateLimiter,
    challenges: Mutex<HashMap<String, (String, Instant)>>,
    presence: Mutex<HashMap<String, social::Presence>>,
    social: social::Social,
    http: reqwest::Client,
}

type Shared = Arc<AppState>;

#[derive(Debug)]
pub struct ApiError(StatusCode, String);

impl ApiError {
    fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self(status, message.into())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({ "error": self.1 }))).into_response()
    }
}

impl From<rusqlite::Error> for ApiError {
    fn from(err: rusqlite::Error) -> Self {
        tracing::error!("database error: {err}");
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal error.")
    }
}

type ApiResult<T> = Result<T, ApiError>;

fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn random_bytes<const N: usize>() -> [u8; N] {
    let mut bytes = [0u8; N];
    rand::rng().fill_bytes(&mut bytes);
    bytes
}

fn sha256_hex(data: &[u8]) -> String {
    hex::encode(Sha256::digest(data))
}

pub fn normalize_uuid(raw: &str) -> Option<String> {
    let s: String = raw.trim().chars().filter(|c| *c != '-').collect::<String>().to_ascii_lowercase();
    (s.len() == 32 && s.bytes().all(|b| b.is_ascii_hexdigit())).then_some(s)
}

fn valid_name(name: &str) -> bool {
    (1..=16).contains(&name.len()) && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// Same UUID an offline-mode server gives this name; only used by the dev mock.
pub fn offline_uuid(name: &str) -> String {
    use md5::Md5;
    let mut hash: [u8; 16] = Md5::digest(format!("OfflinePlayer:{name}").as_bytes()).into();
    hash[6] = (hash[6] & 0x0f) | 0x30;
    hash[8] = (hash[8] & 0x3f) | 0x80;
    hex::encode(hash)
}

impl AppState {
    pub fn new(cfg: Config) -> Result<Shared, String> {
        std::fs::create_dir_all(cfg.data_dir.join("capes")).map_err(|e| format!("Could not create {}: {e}", cfg.data_dir.display()))?;
        let conn = db::open(&cfg.data_dir.join("cosmetics.db")).map_err(|e| format!("Could not open the database: {e}"))?;
        let catalog: Catalog = serde_json::from_str(CATALOG_JSON).map_err(|e| format!("Bad catalog: {e}"))?;
        let http = reqwest::Client::builder().timeout(Duration::from_secs(6)).user_agent("cairn-cosmetics-server").build().map_err(|e| e.to_string())?;
        Ok(Arc::new(Self {
            cfg,
            db: Mutex::new(conn),
            catalog,
            limiter: limit::RateLimiter::default(),
            challenges: Mutex::new(HashMap::new()),
            presence: Mutex::new(HashMap::new()),
            social: social::Social::default(),
            http,
        }))
    }

    pub fn db(&self) -> MutexGuard<'_, Connection> {
        lock(&self.db)
    }

    /// Drops expired sessions, challenges and presence entries.
    pub fn sweep(&self) {
        let _ = self.db().execute("DELETE FROM sessions WHERE expires <= ?1", [now()]);
        lock(&self.challenges).retain(|_, (_, at)| at.elapsed() < CHALLENGE_TTL);
        let window = Duration::from_secs(self.cfg.online_window_secs);
        lock(&self.presence).retain(|_, p| p.at.elapsed() < window);
        if let Err(err) = self.social.sweep(&self.db()) {
            tracing::error!("notification cleanup failed: {err}");
        }
    }

    fn online(&self) -> i64 {
        let window = Duration::from_secs(self.cfg.online_window_secs);
        lock(&self.presence).values().filter(|p| p.at.elapsed() < window).count() as i64
    }

    fn cape_allowed(&self, id: &str, role: &str) -> Option<&CatalogCape> {
        self.catalog.capes.iter().find(|c| c.id == id && role_allows(c.role.as_deref(), role))
    }

    fn badge_allowed(&self, id: &str, role: &str) -> bool {
        id == "none" || self.catalog.badges.iter().any(|b| b.id == id && role_allows(b.role.as_deref(), role))
    }

    /// Whether other players may see this custom cape under the current moderation mode.
    fn cape_public(&self, status: &str) -> bool {
        status == "approved" || (status == "pending" && self.cfg.moderation == Moderation::Instant)
    }
}

fn role_allows(required: Option<&str>, role: &str) -> bool {
    match required {
        None => true,
        Some("staff") => matches!(role, "staff" | "admin"),
        Some(other) => other == role,
    }
}

pub fn router(state: Shared) -> Router {
    let upload = Router::new().route("/v1/me/cape", post(upload_cape).delete(delete_cape)).layer(DefaultBodyLimit::max(cape::MAX_UPLOAD_BYTES));
    let mut app = Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route("/v1/catalog", get(catalog))
        .route("/v1/stats", get(stats))
        .route("/v1/auth/start", post(auth_start))
        .route("/v1/auth/finish", post(auth_finish))
        .route("/v1/me", get(me))
        .route("/v1/me/equipped", put(set_equipped))
        .route("/v1/heartbeat", post(heartbeat))
        .route("/v1/players", get(players))
        .route("/v1/friends", get(social::list))
        .route("/v1/friends/requests", post(social::send_request))
        .route("/v1/friends/requests/{uuid}/{action}", post(social::answer_request))
        .route("/v1/friends/{uuid}", delete(social::remove))
        .route("/v1/blocks/{uuid}", put(social::block).delete(social::unblock))
        .route("/v1/me/social", get(social::get_settings).put(social::put_settings))
        .route("/v1/events", get(social::events))
        .route("/v1/capes/{file}", get(cape_image))
        .route("/v1/admin/capes", get(admin_capes))
        .route("/v1/admin/capes/{hash}/image", get(admin_cape_image))
        .route("/v1/admin/capes/{hash}/{action}", post(admin_review))
        .route("/v1/admin/users/{uuid}", get(admin_user))
        .route("/v1/admin/users/{uuid}/role", put(admin_role))
        .route("/v1/admin/players/{name}/role", put(admin_role_by_name))
        .layer(DefaultBodyLimit::max(16 * 1024))
        .merge(upload)
        .layer(middleware::from_fn_with_state(state.clone(), rate_limit))
        .layer(tower_http::timeout::TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, Duration::from_secs(20)))
        .layer(SetResponseHeaderLayer::overriding(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff")))
        .layer(SetResponseHeaderLayer::overriding(header::CONTENT_SECURITY_POLICY, HeaderValue::from_static("default-src 'none'; frame-ancestors 'none'")))
        .layer(SetResponseHeaderLayer::overriding(header::REFERRER_POLICY, HeaderValue::from_static("no-referrer")));
    if !state.cfg.cors_origins.is_empty() {
        let origins: Vec<HeaderValue> = state.cfg.cors_origins.iter().filter_map(|o| HeaderValue::from_str(o).ok()).collect();
        app = app.layer(
            tower_http::cors::CorsLayer::new()
                .allow_origin(origins)
                .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
                .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
                .max_age(Duration::from_secs(600)),
        );
    }
    app.with_state(state)
}

fn client_ip(state: &AppState, req: &Request) -> IpAddr {
    if state.cfg.trust_proxy {
        // The proxy appends the address it saw, so the last hop is the only trustworthy one.
        if let Some(ip) = req.headers().get("x-forwarded-for").and_then(|v| v.to_str().ok()).and_then(|v| v.rsplit(',').next()).and_then(|v| v.trim().parse().ok()) {
            return ip;
        }
    }
    req.extensions().get::<ConnectInfo<SocketAddr>>().map(|c| c.0.ip()).unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED))
}

async fn rate_limit(State(state): State<Shared>, req: Request, next: Next) -> Response {
    let ip = client_ip(&state, &req);
    let path = req.uri().path();
    let base = state.cfg.rate_per_minute;
    let (class, per_minute, burst) = if path.starts_with("/v1/auth/") {
        ("auth", 6, 10)
    } else if path == "/v1/me/cape" && req.method() == Method::POST {
        ("upload", 1, state.cfg.upload_burst)
    } else if path == "/v1/events" {
        ("events", 60, 20)
    } else if (path.starts_with("/v1/friends") || path.starts_with("/v1/blocks")) && req.method() != Method::GET {
        ("social", 30, 15)
    } else if path.starts_with("/v1/admin/") {
        ("admin", base, base)
    } else {
        ("api", base, base.max(20))
    };
    if !state.limiter.check(ip, class, per_minute, burst) {
        let mut res = ApiError::new(StatusCode::TOO_MANY_REQUESTS, "Too many requests, slow down.").into_response();
        res.headers_mut().insert(header::RETRY_AFTER, HeaderValue::from_static("30"));
        return res;
    }
    next.run(req).await
}

fn cached(json: Value, seconds: u32) -> Response {
    let mut res = Json(json).into_response();
    if let Ok(v) = HeaderValue::from_str(&format!("public, max-age={seconds}")) {
        res.headers_mut().insert(header::CACHE_CONTROL, v);
    }
    res
}

fn bearer(headers: &HeaderMap) -> Option<&str> {
    headers.get(header::AUTHORIZATION)?.to_str().ok()?.strip_prefix("Bearer ").map(str::trim)
}

fn session(state: &AppState, headers: &HeaderMap) -> ApiResult<db::User> {
    let unauthorized = || ApiError::new(StatusCode::UNAUTHORIZED, "Sign in again.");
    let token = bearer(headers).filter(|t| !t.is_empty() && t.len() <= 128).ok_or_else(unauthorized)?;
    let conn = state.db();
    let uuid: Option<String> = rusqlite::OptionalExtension::optional(conn.query_row(
        "SELECT uuid FROM sessions WHERE token_hash = ?1 AND expires > ?2",
        rusqlite::params![sha256_hex(token.as_bytes()), now()],
        |r| r.get(0),
    ))?;
    let user = match uuid {
        Some(uuid) => db::user(&conn, &uuid)?,
        None => None,
    }
    .ok_or_else(unauthorized)?;
    if user.role == "banned" {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "This account is banned from cosmetics."));
    }
    Ok(user)
}

fn require_admin(state: &AppState, headers: &HeaderMap) -> ApiResult<()> {
    use subtle::ConstantTimeEq;
    let Some(expected) = &state.cfg.admin_token else {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "Admin API is disabled."));
    };
    let given = bearer(headers).unwrap_or("");
    // Hash both sides so the comparison does not leak the token length either.
    let ok: bool = Sha256::digest(given.as_bytes()).ct_eq(&Sha256::digest(expected.as_bytes())).into();
    if ok {
        Ok(())
    } else {
        Err(ApiError::new(StatusCode::UNAUTHORIZED, "Wrong admin token."))
    }
}

async fn catalog() -> Response {
    let mut res = cached(serde_json::from_str(CATALOG_JSON).unwrap_or(Value::Null), 3600);
    res.headers_mut().insert(header::VARY, HeaderValue::from_static("accept-encoding"));
    res
}

async fn stats(State(state): State<Shared>) -> ApiResult<Response> {
    let online = state.online();
    let conn = state.db();
    let total: i64 = conn.query_row("SELECT COUNT(*) FROM users WHERE name != ''", [], |r| r.get(0))?;
    Ok(cached(json!({ "online": online, "peak": db::stat(&conn, "peak_online")?.max(online), "peakAt": db::stat(&conn, "peak_at")?, "total": total }), 30))
}

#[derive(Deserialize)]
struct StartBody {
    name: String,
}

async fn auth_start(State(state): State<Shared>, Json(body): Json<StartBody>) -> ApiResult<Json<Value>> {
    if !valid_name(&body.name) {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "Invalid player name."));
    }
    let server_id = hex::encode(random_bytes::<20>());
    let mut challenges = lock(&state.challenges);
    if challenges.len() >= 20_000 {
        challenges.retain(|_, (_, at)| at.elapsed() < CHALLENGE_TTL);
        if challenges.len() >= 20_000 {
            return Err(ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "Too many sign-ins in progress, try again soon."));
        }
    }
    challenges.insert(server_id.clone(), (body.name, Instant::now()));
    Ok(Json(json!({ "serverId": server_id })))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FinishBody {
    name: String,
    server_id: String,
}

#[derive(Deserialize)]
struct HasJoined {
    id: String,
    name: String,
}

async fn auth_finish(State(state): State<Shared>, Json(body): Json<FinishBody>) -> ApiResult<Json<Value>> {
    let invalid = || ApiError::new(StatusCode::UNAUTHORIZED, "Sign-in could not be verified.");
    // One use per challenge, whatever the outcome, so a captured serverId cannot be replayed.
    let challenge = lock(&state.challenges).remove(&body.server_id);
    match challenge {
        Some((name, at)) if name == body.name && at.elapsed() < CHALLENGE_TTL => {}
        _ => return Err(invalid()),
    }
    let (uuid, name) = if state.cfg.dev_insecure_auth {
        tracing::warn!("dev auth: trusting {} without Mojang", body.name);
        (offline_uuid(&body.name), body.name.clone())
    } else {
        let url = format!("{}/session/minecraft/hasJoined", state.cfg.session_server);
        let res = state.http.get(url).query(&[("username", body.name.as_str()), ("serverId", body.server_id.as_str())]).send().await.map_err(|e| {
            tracing::warn!("session server unreachable: {e}");
            ApiError::new(StatusCode::BAD_GATEWAY, "Mojang could not be reached, try again.")
        })?;
        if res.status() != reqwest::StatusCode::OK {
            return Err(invalid());
        }
        let joined: HasJoined = res.json().await.map_err(|_| invalid())?;
        let uuid = normalize_uuid(&joined.id).ok_or_else(invalid)?;
        if !joined.name.eq_ignore_ascii_case(&body.name) || !valid_name(&joined.name) {
            return Err(invalid());
        }
        (uuid, joined.name)
    };
    let token = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(random_bytes::<32>());
    let conn = state.db();
    if db::user(&conn, &uuid)?.is_some_and(|u| u.role == "banned") {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "This account is banned from cosmetics."));
    }
    let now = now();
    db::upsert_user(&conn, &uuid, &name, now, !state.cfg.dev_insecure_auth)?;
    conn.execute(
        "INSERT INTO sessions (token_hash, uuid, expires) VALUES (?1, ?2, ?3)",
        rusqlite::params![sha256_hex(token.as_bytes()), uuid, now + state.cfg.token_ttl_secs],
    )?;
    let role = db::user(&conn, &uuid)?.map(|u| u.role).unwrap_or_else(|| "player".into());
    Ok(Json(json!({ "token": token, "expiresIn": state.cfg.token_ttl_secs, "uuid": uuid, "name": name, "role": role })))
}

fn custom_cape_json(state: &AppState, conn: &Connection, hash: &str) -> ApiResult<Option<Value>> {
    Ok(db::cape(conn, hash)?.map(|c| {
        json!({ "hash": c.hash, "status": c.status, "visible": state.cape_public(&c.status), "frames": c.frames, "frameMs": c.frame_ms, "url": format!("/v1/capes/{}.png", c.hash) })
    }))
}

async fn me(State(state): State<Shared>, headers: HeaderMap) -> ApiResult<Json<Value>> {
    let user = session(&state, &headers)?;
    let conn = state.db();
    let custom = match &user.custom_cape {
        Some(hash) => custom_cape_json(&state, &conn, hash)?,
        None => None,
    };
    let capes: Vec<&str> = state.catalog.capes.iter().filter(|c| role_allows(c.role.as_deref(), &user.role)).map(|c| c.id.as_str()).collect();
    let badges: Vec<&str> = state.catalog.badges.iter().filter(|b| role_allows(b.role.as_deref(), &user.role)).map(|b| b.id.as_str()).collect();
    let equipped_cape = user.cape.as_deref().filter(|id| *id == "custom" || state.cape_allowed(id, &user.role).is_some());
    let equipped_badge = user.badge.as_deref().filter(|id| state.badge_allowed(id, &user.role));
    Ok(Json(json!({
        "uuid": user.uuid, "name": user.name, "role": user.role,
        "equipped": { "cape": equipped_cape, "badge": equipped_badge },
        "customCape": custom,
        "unlocked": { "capes": capes, "badges": badges },
        "moderation": if state.cfg.moderation == Moderation::Instant { "instant" } else { "approval" },
    })))
}

#[derive(Deserialize)]
struct EquipBody {
    cape: Option<String>,
    badge: Option<String>,
}

async fn set_equipped(State(state): State<Shared>, headers: HeaderMap, Json(body): Json<EquipBody>) -> ApiResult<Json<Value>> {
    let user = session(&state, &headers)?;
    let conn = state.db();
    if let Some(cape) = body.cape.as_deref() {
        let ok = if cape == "custom" {
            match &user.custom_cape {
                Some(hash) => db::cape(&conn, hash)?.is_some_and(|c| c.status != "rejected"),
                None => false,
            }
        } else {
            state.cape_allowed(cape, &user.role).is_some()
        };
        if !ok {
            return Err(ApiError::new(StatusCode::BAD_REQUEST, "That cape is not available to you."));
        }
    }
    if let Some(badge) = body.badge.as_deref() {
        if !state.badge_allowed(badge, &user.role) {
            return Err(ApiError::new(StatusCode::BAD_REQUEST, "That badge is not available to you."));
        }
    }
    conn.execute("UPDATE users SET cape = ?2, badge = ?3 WHERE uuid = ?1", rusqlite::params![user.uuid, body.cape, body.badge])?;
    Ok(Json(json!({ "cape": body.cape, "badge": body.badge })))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UploadQuery {
    frame_ms: Option<u32>,
}

async fn upload_cape(State(state): State<Shared>, headers: HeaderMap, Query(query): Query<UploadQuery>, body: Bytes) -> ApiResult<Json<Value>> {
    let user = session(&state, &headers)?;
    let parsed = tokio::task::spawn_blocking(move || cape::validate(&body, query.frame_ms))
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal error."))?
        .map_err(|msg| ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, msg))?;
    let path = state.cfg.data_dir.join("capes").join(format!("{}.png", parsed.hash));
    if !path.exists() {
        let tmp = path.with_extension(format!("tmp{}", hex::encode(random_bytes::<4>())));
        tokio::fs::write(&tmp, &parsed.png).await.map_err(|e| {
            tracing::error!("write cape: {e}");
            ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Could not store the cape.")
        })?;
        tokio::fs::rename(&tmp, &path).await.map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Could not store the cape."))?;
    }
    let conn = state.db();
    let now = now();
    let status = if state.cfg.moderation == Moderation::Instant { "approved" } else { "pending" };
    if let Some(existing) = db::cape(&conn, &parsed.hash)? {
        if existing.status == "rejected" {
            return Err(ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "This cape was rejected by a moderator."));
        }
        if existing.status == "pending" && state.cfg.moderation == Moderation::Instant {
            db::set_cape_status(&conn, &parsed.hash, "approved", now)?;
        }
    } else {
        conn.execute(
            "INSERT INTO capes (hash, owner, status, frames, frame_ms, width, height, created) VALUES (?1, ?2, ?8, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![parsed.hash, user.uuid, parsed.frames, parsed.frame_ms, parsed.width, parsed.height, now, status],
        )?;
    }
    // Replacing a cape withdraws the old pending one so the review queue cannot be flooded.
    let stale: Vec<String> = {
        let mut stmt = conn.prepare("SELECT hash FROM capes WHERE owner = ?1 AND status = 'pending' AND hash != ?2")?;
        let rows = stmt.query_map(rusqlite::params![user.uuid, parsed.hash], |r| r.get(0))?;
        rows.collect::<Result<_, _>>()?
    };
    for hash in stale {
        conn.execute("DELETE FROM capes WHERE hash = ?1", [&hash])?;
        let _ = std::fs::remove_file(state.cfg.data_dir.join("capes").join(format!("{hash}.png")));
    }
    conn.execute("UPDATE users SET custom_cape = ?2, cape = 'custom' WHERE uuid = ?1", rusqlite::params![user.uuid, parsed.hash])?;
    Ok(Json(custom_cape_json(&state, &conn, &parsed.hash)?.unwrap_or(Value::Null)))
}

async fn delete_cape(State(state): State<Shared>, headers: HeaderMap) -> ApiResult<StatusCode> {
    let user = session(&state, &headers)?;
    state.db().execute("UPDATE users SET custom_cape = NULL, cape = CASE WHEN cape = 'custom' THEN NULL ELSE cape END WHERE uuid = ?1", [&user.uuid])?;
    Ok(StatusCode::NO_CONTENT)
}

/// Only the in-game mod calls this; its optional `activity` feeds what friends see.
async fn heartbeat(State(state): State<Shared>, headers: HeaderMap, body: Bytes) -> ApiResult<Json<Value>> {
    let user = session(&state, &headers)?;
    let activity = social::heartbeat_body(&body)?;
    social::heartbeat(&state, &user.uuid, &user.name, activity)?;
    let online = state.online();
    let conn = state.db();
    conn.execute("UPDATE users SET last_seen = ?2 WHERE uuid = ?1", rusqlite::params![user.uuid, now()])?;
    if online > db::stat(&conn, "peak_online")? {
        db::set_stat(&conn, "peak_online", online)?;
        db::set_stat(&conn, "peak_at", now())?;
    }
    Ok(Json(json!({ "online": online, "next": 60 })))
}

#[derive(Deserialize)]
struct PlayersQuery {
    uuids: String,
}

async fn players(State(state): State<Shared>, Query(query): Query<PlayersQuery>) -> ApiResult<Response> {
    let mut seen = HashSet::new();
    let mut uuids = Vec::new();
    for raw in query.uuids.split(',').filter(|s| !s.trim().is_empty()) {
        let uuid = normalize_uuid(raw).ok_or_else(|| ApiError::new(StatusCode::BAD_REQUEST, "Invalid UUID in the list."))?;
        if seen.insert(uuid.clone()) {
            uuids.push(uuid);
        }
    }
    if uuids.len() > MAX_LOOKUP {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, format!("Look up at most {MAX_LOOKUP} players per request.")));
    }
    let conn = state.db();
    let mut out = serde_json::Map::new();
    for uuid in uuids {
        let Some(user) = db::user(&conn, &uuid)? else { continue };
        if user.role == "banned" || user.name.is_empty() {
            continue;
        }
        let cape = match user.cape.as_deref() {
            Some("custom") => match user.custom_cape.as_deref() {
                Some(hash) => db::cape(&conn, hash)?
                    .filter(|c| state.cape_public(&c.status))
                    .map(|c| json!({ "id": "custom", "url": format!("/v1/capes/{}.png", c.hash), "frames": c.frames, "frameMs": c.frame_ms })),
                None => None,
            },
            Some(id) => state.cape_allowed(id, &user.role).map(|c| json!({ "id": c.id, "frames": c.frames, "frameMs": c.frame_ms })),
            None => None,
        };
        let badge = match user.badge.as_deref() {
            Some(id) if state.badge_allowed(id, &user.role) => id.to_owned(),
            _ => "classic".to_owned(),
        };
        out.insert(uuid, json!({ "name": user.name, "role": user.role, "badge": badge, "cape": cape }));
    }
    Ok(cached(json!({ "players": out }), 60))
}

async fn serve_png(path: std::path::PathBuf, cache: &'static str) -> ApiResult<Response> {
    let bytes = tokio::fs::read(path).await.map_err(|_| ApiError::new(StatusCode::NOT_FOUND, "No such cape."))?;
    Ok(([(header::CONTENT_TYPE, "image/png"), (header::CACHE_CONTROL, cache)], bytes).into_response())
}

async fn cape_image(State(state): State<Shared>, Path(file): Path<String>) -> ApiResult<Response> {
    let hash = file.strip_suffix(".png").filter(|h| cape::valid_hash(h)).ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "No such cape."))?;
    let visible = db::cape(&state.db(), hash)?.is_some_and(|c| state.cape_public(&c.status));
    if !visible {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "No such cape."));
    }
    serve_png(state.cfg.data_dir.join("capes").join(format!("{hash}.png")), "public, max-age=86400, immutable").await
}

#[derive(Deserialize)]
struct StatusQuery {
    status: Option<String>,
}

async fn admin_capes(State(state): State<Shared>, headers: HeaderMap, Query(query): Query<StatusQuery>) -> ApiResult<Json<Value>> {
    require_admin(&state, &headers)?;
    let status = query.status.unwrap_or_else(|| "pending".into());
    if !matches!(status.as_str(), "pending" | "approved" | "rejected") {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "status must be pending, approved or rejected."));
    }
    Ok(Json(json!({ "capes": db::capes_with_status(&state.db(), &status, 200)? })))
}

async fn admin_cape_image(State(state): State<Shared>, headers: HeaderMap, Path(hash): Path<String>) -> ApiResult<Response> {
    require_admin(&state, &headers)?;
    if !cape::valid_hash(&hash) {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "No such cape."));
    }
    serve_png(state.cfg.data_dir.join("capes").join(format!("{hash}.png")), "no-store").await
}

async fn admin_review(State(state): State<Shared>, headers: HeaderMap, Path((hash, action)): Path<(String, String)>) -> ApiResult<Json<Value>> {
    require_admin(&state, &headers)?;
    let status = match action.as_str() {
        "approve" => "approved",
        "reject" => "rejected",
        _ => return Err(ApiError::new(StatusCode::NOT_FOUND, "Unknown action.")),
    };
    if !cape::valid_hash(&hash) || !db::set_cape_status(&state.db(), &hash, status, now())? {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "No such cape."));
    }
    tracing::info!("admin set cape {hash} to {status}");
    Ok(Json(json!({ "hash": hash, "status": status })))
}

async fn admin_user(State(state): State<Shared>, headers: HeaderMap, Path(uuid): Path<String>) -> ApiResult<Json<Value>> {
    require_admin(&state, &headers)?;
    let uuid = normalize_uuid(&uuid).ok_or_else(|| ApiError::new(StatusCode::BAD_REQUEST, "Invalid UUID."))?;
    let user = db::user(&state.db(), &uuid)?.ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "Unknown player."))?;
    Ok(Json(json!({ "uuid": user.uuid, "name": user.name, "role": user.role, "cape": user.cape, "badge": user.badge, "customCape": user.custom_cape })))
}

#[derive(Deserialize)]
struct RoleBody {
    role: String,
}

async fn admin_role_by_name(State(state): State<Shared>, headers: HeaderMap, Path(name): Path<String>, Json(body): Json<RoleBody>) -> ApiResult<Json<Value>> {
    require_admin(&state, &headers)?;
    if !valid_name(&name) || !db::ROLES.contains(&body.role.as_str()) {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "Use a Minecraft name and a valid role."));
    }
    let conn = state.db();
    let uuid = db::verified_uuid_by_name(&conn, &name)?.ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "Player must sign in with Mojang first; offline or ambiguous names cannot receive roles."))?;
    db::set_role(&conn, &uuid, &body.role, now())?;
    Ok(Json(json!({ "uuid": uuid, "name": name, "role": body.role })))
}

async fn admin_role(State(state): State<Shared>, headers: HeaderMap, Path(uuid): Path<String>, Json(body): Json<RoleBody>) -> ApiResult<Json<Value>> {
    require_admin(&state, &headers)?;
    let uuid = normalize_uuid(&uuid).ok_or_else(|| ApiError::new(StatusCode::BAD_REQUEST, "Invalid UUID."))?;
    if !db::ROLES.contains(&body.role.as_str()) {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "role must be player, staff, admin or banned."));
    }
    db::set_role(&state.db(), &uuid, &body.role, now())?;
    tracing::info!("admin set role of {uuid} to {}", body.role);
    Ok(Json(json!({ "uuid": uuid, "role": body.role })))
}
