use axum::body::Body;
use axum::extract::Query;
use axum::http::{header, Request, StatusCode};
use axum::routing::get;
use axum::Router;
use cairn_cosmetics_server::config::{Config, Moderation};
use cairn_cosmetics_server::{router, AppState};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

const ALEX: &str = "0f1e2d3c4b5a69788796a5b4c3d2e1f0";
const BOB: &str = "11112222333344445555666677778888";
const ADMIN: &str = "test-admin-token-0123456789abcdef0123";

/// Fake Mojang session server: a name counts as joined once the test registers its serverId.
struct Mojang {
    joined: Arc<Mutex<HashMap<String, (String, String)>>>,
    url: String,
}

impl Mojang {
    async fn start() -> Self {
        let joined: Arc<Mutex<HashMap<String, (String, String)>>> = Arc::default();
        let map = joined.clone();
        let app = Router::new().route(
            "/session/minecraft/hasJoined",
            get(move |Query(q): Query<HashMap<String, String>>| {
                let map = map.clone();
                async move {
                    let entry = map.lock().unwrap().get(q.get("serverId").map(String::as_str).unwrap_or("")).cloned();
                    match entry {
                        Some((name, id)) if Some(&name) == q.get("username") => (StatusCode::OK, axum::Json(json!({ "id": id, "name": name }))).into_response(),
                        _ => StatusCode::NO_CONTENT.into_response(),
                    }
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        Self { joined, url }
    }

    fn join(&self, server_id: &str, name: &str, uuid: &str) {
        self.joined.lock().unwrap().insert(server_id.into(), (name.into(), uuid.into()));
    }
}

use axum::response::IntoResponse;

struct Harness {
    app: Router,
    state: Arc<AppState>,
    mojang: Mojang,
    _dir: tempfile::TempDir,
}

async fn harness(tweak: impl FnOnce(&mut Config)) -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let mojang = Mojang::start().await;
    let mut cfg = Config::for_tests(dir.path().to_path_buf());
    cfg.session_server = mojang.url.clone();
    cfg.admin_token = Some(ADMIN.into());
    tweak(&mut cfg);
    let state = AppState::new(cfg).unwrap();
    Harness { app: router(state.clone()), state, mojang, _dir: dir }
}

impl Harness {
    fn restart(&mut self) {
        let cfg = self.state.cfg.clone();
        self.app = Router::new();
        self.state = AppState::new(cfg).unwrap();
        self.app = router(self.state.clone());
    }

    async fn call(&self, method: &str, uri: &str, token: Option<&str>, body: Option<Value>) -> (StatusCode, Value) {
        let mut req = Request::builder().method(method).uri(uri);
        if let Some(token) = token {
            req = req.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        let req = match body {
            Some(body) => req.header(header::CONTENT_TYPE, "application/json").body(Body::from(body.to_string())),
            None => req.body(Body::empty()),
        };
        self.send(req.unwrap()).await
    }

    async fn send(&self, req: Request<Body>) -> (StatusCode, Value) {
        let res = self.app.clone().oneshot(req).await.unwrap();
        let status = res.status();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
    }

    async fn upload(&self, token: &str, png: Vec<u8>, query: &str) -> (StatusCode, Value) {
        let req = Request::post(format!("/v1/me/cape{query}"))
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header(header::CONTENT_TYPE, "image/png")
            .body(Body::from(png))
            .unwrap();
        self.send(req).await
    }

    async fn login(&self, name: &str, uuid: &str) -> String {
        let (status, start) = self.call("POST", "/v1/auth/start", None, Some(json!({ "name": name }))).await;
        assert_eq!(status, StatusCode::OK);
        let server_id = start["serverId"].as_str().unwrap().to_owned();
        self.mojang.join(&server_id, name, uuid);
        let (status, done) = self.call("POST", "/v1/auth/finish", None, Some(json!({ "name": name, "serverId": server_id }))).await;
        assert_eq!(status, StatusCode::OK, "{done}");
        assert_eq!(done["uuid"], uuid);
        done["token"].as_str().unwrap().to_owned()
    }
}

fn png(width: u32, height: u32) -> Vec<u8> {
    let img = image::RgbaImage::from_fn(width, height, |x, y| image::Rgba([(x % 256) as u8, (y % 256) as u8, 90, 255]));
    let mut out = Vec::new();
    img.write_to(&mut Cursor::new(&mut out), image::ImageFormat::Png).unwrap();
    out
}

#[tokio::test]
async fn sign_in_needs_a_mojang_join_and_a_fresh_challenge() {
    let h = harness(|_| {}).await;
    let token = h.login("Alex", ALEX).await;
    let (status, me) = h.call("GET", "/v1/me", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me["role"], "player");

    // No join recorded at Mojang: refused.
    let (_, start) = h.call("POST", "/v1/auth/start", None, Some(json!({ "name": "Alex" }))).await;
    let sid = start["serverId"].as_str().unwrap().to_owned();
    let (status, _) = h.call("POST", "/v1/auth/finish", None, Some(json!({ "name": "Alex", "serverId": sid }))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    // The challenge is burnt even after a failure, so a later join cannot reuse it.
    h.mojang.join(&sid, "Alex", ALEX);
    let (status, _) = h.call("POST", "/v1/auth/finish", None, Some(json!({ "name": "Alex", "serverId": sid }))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // A challenge issued for one name cannot be finished as another.
    let (_, start) = h.call("POST", "/v1/auth/start", None, Some(json!({ "name": "Alex" }))).await;
    let sid = start["serverId"].as_str().unwrap().to_owned();
    h.mojang.join(&sid, "Bob", BOB);
    let (status, _) = h.call("POST", "/v1/auth/finish", None, Some(json!({ "name": "Bob", "serverId": sid }))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, _) = h.call("POST", "/v1/auth/start", None, Some(json!({ "name": "bad name!" }))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = h.call("GET", "/v1/me", Some("not-a-token"), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = h.call("GET", "/v1/me", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn expired_tokens_and_bans_lock_players_out() {
    let h = harness(|_| {}).await;
    let token = h.login("Alex", ALEX).await;
    h.state.db().execute("UPDATE sessions SET expires = 0", []).unwrap();
    let (status, _) = h.call("GET", "/v1/me", Some(&token), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let token = h.login("Alex", ALEX).await;
    let (status, _) = h.call("PUT", &format!("/v1/admin/users/{ALEX}/role"), Some(ADMIN), Some(json!({ "role": "banned" }))).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = h.call("GET", "/v1/me", Some(&token), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (_, start) = h.call("POST", "/v1/auth/start", None, Some(json!({ "name": "Alex" }))).await;
    let sid = start["serverId"].as_str().unwrap().to_owned();
    h.mojang.join(&sid, "Alex", ALEX);
    let (status, _) = h.call("POST", "/v1/auth/finish", None, Some(json!({ "name": "Alex", "serverId": sid }))).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn equipping_respects_the_catalog_and_roles() {
    let h = harness(|_| {}).await;
    let token = h.login("Alex", ALEX).await;
    let (status, _) = h.call("PUT", "/v1/me/equipped", Some(&token), Some(json!({ "cape": "pulse", "badge": "ember" }))).await;
    assert_eq!(status, StatusCode::OK);
    for body in [json!({ "cape": "nope" }), json!({ "cape": "staff" }), json!({ "badge": "gold" }), json!({ "cape": "custom" })] {
        let (status, _) = h.call("PUT", "/v1/me/equipped", Some(&token), Some(body.clone())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    }
    let (status, _) = h.call("PUT", &format!("/v1/admin/users/{ALEX}/role"), Some(ADMIN), Some(json!({ "role": "staff" }))).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = h.call("PUT", "/v1/me/equipped", Some(&token), Some(json!({ "cape": "staff", "badge": "gold" }))).await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = h.call("GET", &format!("/v1/players?uuids={ALEX},{BOB}"), None, None).await;
    assert_eq!(status, StatusCode::OK);
    let alex = &body["players"][ALEX];
    assert_eq!(alex["role"], "staff");
    assert_eq!(alex["badge"], "gold");
    assert_eq!(alex["cape"]["id"], "staff");
    assert!(body["players"].get(BOB).is_none(), "non-Cairn players are not listed");

    // Losing the role hides role-locked cosmetics without touching the stored choice.
    h.call("PUT", &format!("/v1/admin/users/{ALEX}/role"), Some(ADMIN), Some(json!({ "role": "player" }))).await;
    let (_, body) = h.call("GET", &format!("/v1/players?uuids={ALEX}"), None, None).await;
    assert_eq!(body["players"][ALEX]["cape"], Value::Null);
    assert_eq!(body["players"][ALEX]["badge"], "classic");
}

#[tokio::test]
async fn cape_uploads_are_validated() {
    let h = harness(|_| {}).await;
    let token = h.login("Alex", ALEX).await;
    let cases: Vec<(Vec<u8>, &str)> = vec![
        (b"GIF89a not a png".to_vec(), ""),
        (png(60, 30), ""),
        (png(64, 40), ""),
        (png(64, 32 * 33), "?frameMs=100"),
        (png(64, 64), "?frameMs=10"),
        (png(64, 64), "?frameMs=5000"),
        (png(1024, 512 * 9), "?frameMs=100"),
        (b"\x89PNG\r\n\x1a\ngarbage".to_vec(), ""),
    ];
    for (bytes, query) in cases {
        let (status, body) = h.upload(&token, bytes, query).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{query} {body}");
    }
    let (status, _) = h.upload(&token, vec![0u8; 1024 * 1024 + 1], "").await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    let (status, _) = h.upload("bogus", png(64, 32), "").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, body) = h.upload(&token, png(512, 256 * 16), "?frameMs=80").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["frames"], 16);
    assert_eq!(body["frameMs"], 80);
    assert_eq!(body["status"], "pending");
    let (status, body) = h.upload(&token, png(64, 32), "?frameMs=80").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["frames"], 1);
    assert_eq!(body["frameMs"], 0, "static capes ignore the frame time");
    // The animated upload was replaced, so only one cape waits for review.
    let (_, pending) = h.call("GET", "/v1/admin/capes", Some(ADMIN), None).await;
    assert_eq!(pending["capes"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn custom_capes_stay_hidden_until_approved() {
    let h = harness(|_| {}).await;
    let token = h.login("Alex", ALEX).await;
    let (_, cape) = h.upload(&token, png(128, 64 * 4), "?frameMs=120").await;
    let hash = cape["hash"].as_str().unwrap().to_owned();
    let lookup = format!("/v1/players?uuids={ALEX}");
    let (_, body) = h.call("GET", &lookup, None, None).await;
    assert_eq!(body["players"][ALEX]["cape"], Value::Null);
    let (status, _) = h.call("GET", &format!("/v1/capes/{hash}.png"), None, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _) = h.call("POST", &format!("/v1/admin/capes/{hash}/approve"), Some("wrong-token-wrong-token-wrong-token"), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = h.call("POST", &format!("/v1/admin/capes/{hash}/approve"), None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = h.call("POST", &format!("/v1/admin/capes/{hash}/approve"), Some(ADMIN), None).await;
    assert_eq!(status, StatusCode::OK);

    let (_, body) = h.call("GET", &lookup, None, None).await;
    let shown = &body["players"][ALEX]["cape"];
    assert_eq!(shown["id"], "custom");
    assert_eq!(shown["frames"], 4);
    assert_eq!(shown["frameMs"], 120);
    let res = h.app.clone().oneshot(Request::get(format!("/v1/capes/{hash}.png")).body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.headers()[header::CONTENT_TYPE], "image/png");
    assert_eq!(res.headers()[header::X_CONTENT_TYPE_OPTIONS], "nosniff");

    h.call("POST", &format!("/v1/admin/capes/{hash}/reject"), Some(ADMIN), None).await;
    let (_, body) = h.call("GET", &lookup, None, None).await;
    assert_eq!(body["players"][ALEX]["cape"], Value::Null);
    let (status, _) = h.call("GET", &format!("/v1/capes/{hash}.png"), None, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    // Re-uploading a rejected image does not resurrect it.
    let (status, _) = h.upload(&token, png(128, 64 * 4), "?frameMs=120").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn instant_moderation_shows_capes_right_away() {
    let h = harness(|cfg| cfg.moderation = Moderation::Instant).await;
    let token = h.login("Alex", ALEX).await;
    let (_, cape) = h.upload(&token, png(64, 32), "").await;
    let (_, body) = h.call("GET", &format!("/v1/players?uuids={ALEX}"), None, None).await;
    assert_eq!(body["players"][ALEX]["cape"]["url"], format!("/v1/capes/{}.png", cape["hash"].as_str().unwrap()));
}

#[tokio::test]
async fn admin_api_is_off_without_a_token() {
    let h = harness(|cfg| cfg.admin_token = None).await;
    let (status, _) = h.call("GET", "/v1/admin/capes", Some(""), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = h.call("PUT", &format!("/v1/admin/users/{ALEX}/role"), Some("x"), Some(json!({ "role": "admin" }))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn lookups_and_presence() {
    let h = harness(|_| {}).await;
    let alex = h.login("Alex", ALEX).await;
    let bob = h.login("Bob", BOB).await;
    let (status, _) = h.call("GET", "/v1/players?uuids=zzz", None, None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let many: Vec<String> = (0..101).map(|i| format!("{i:032x}")).collect();
    let (status, _) = h.call("GET", &format!("/v1/players?uuids={}", many.join(",")), None, None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let dashed = "0f1e2d3c-4b5a-6978-8796-a5b4c3d2e1f0";
    let (_, body) = h.call("GET", &format!("/v1/players?uuids={dashed}"), None, None).await;
    assert_eq!(body["players"][ALEX]["name"], "Alex");

    h.call("POST", "/v1/heartbeat", Some(&alex), None).await;
    let (status, beat) = h.call("POST", "/v1/heartbeat", Some(&bob), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(beat["online"], 2);
    let (_, stats) = h.call("GET", "/v1/stats", None, None).await;
    assert_eq!(stats["online"], 2);
    assert_eq!(stats["peak"], 2);
    assert_eq!(stats["total"], 2);
    let (status, _) = h.call("POST", "/v1/heartbeat", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn rate_limits_and_body_limits() {
    let h = harness(|_| {}).await;
    let mut limited = false;
    for _ in 0..20 {
        let (status, _) = h.call("POST", "/v1/auth/start", None, Some(json!({ "name": "Alex" }))).await;
        if status == StatusCode::TOO_MANY_REQUESTS {
            limited = true;
            break;
        }
    }
    assert!(limited, "sign-in attempts are throttled");
    let big = json!({ "cape": "x".repeat(20_000) });
    let (status, _) = h.call("PUT", "/v1/me/equipped", Some("t"), Some(big)).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
}

#[test]
fn config_refuses_unsafe_settings() {
    let env = |pairs: &'static [(&'static str, &'static str)]| move |key: &str| pairs.iter().find(|(k, _)| *k == key).map(|(_, v)| v.to_string());
    assert!(Config::from_lookup(env(&[("COSMETICS_ADMIN_TOKEN", "short")])).is_err());
    assert!(Config::from_lookup(env(&[("COSMETICS_DEV_INSECURE_AUTH", "1"), ("COSMETICS_BIND", "0.0.0.0:8787")])).is_err());
    assert!(Config::from_lookup(env(&[("COSMETICS_SESSION_SERVER", "http://evil.example")])).is_err());
    assert!(Config::from_lookup(env(&[("COSMETICS_CAPE_MODERATION", "sometimes")])).is_err());
    let cfg = Config::from_lookup(env(&[])).unwrap();
    assert!(cfg.admin_token.is_none() && cfg.cors_origins.is_empty() && !cfg.dev_insecure_auth && !cfg.trust_proxy);
    assert_eq!(cfg.moderation, Moderation::Approval);
}

#[test]
fn offline_uuid_matches_minecraft() {
    // Offline-mode UUID that vanilla servers assign to "Notch".
    assert_eq!(cairn_cosmetics_server::offline_uuid("Notch"), "b50ad385829d3141a2167e7d7539ba7f");
}

const CARA: &str = "aaaabbbbccccddddeeeeffff00001111";

fn beat(version: &str, mode: &str, server: Option<&str>) -> Value {
    json!({ "activity": { "version": version, "mode": mode, "server": server } })
}

fn friend<'a>(list: &'a Value, uuid: &str) -> Option<&'a Value> {
    list["friends"].as_array().unwrap().iter().find(|f| f["uuid"] == uuid)
}

async fn befriend(h: &Harness, a: &str, b: &str, b_name: &str, b_uuid: &str, a_uuid: &str) {
    let (status, sent) = h.call("POST", "/v1/friends/requests", Some(a), Some(json!({ "target": b_name.to_lowercase() }))).await;
    assert_eq!(status, StatusCode::OK, "{sent}");
    assert_eq!(sent["uuid"], b_uuid);
    let (status, _) = h.call("POST", &format!("/v1/friends/requests/{a_uuid}/accept"), Some(b), None).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn friend_requests_follow_the_rules() {
    let h = harness(|_| {}).await;
    let alex = h.login("Alex", ALEX).await;
    let bob = h.login("Bob", BOB).await;
    let (status, _) = h.call("POST", "/v1/friends/requests", Some(&alex), Some(json!({ "target": "Nobody" }))).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "only players who used Cairn Client can be added");
    let (status, _) = h.call("POST", "/v1/friends/requests", Some(&alex), Some(json!({ "target": "alex" }))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = h.call("POST", "/v1/friends/requests", Some(&alex), Some(json!({ "target": "bad name!" }))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (_, sent) = h.call("POST", "/v1/friends/requests", Some(&alex), Some(json!({ "target": BOB }))).await;
    assert_eq!(sent["status"], "sent");
    let (_, list) = h.call("GET", "/v1/friends", Some(&bob), None).await;
    assert_eq!(list["incoming"][0]["name"], "Alex");
    let (_, list) = h.call("GET", "/v1/friends", Some(&alex), None).await;
    assert_eq!(list["outgoing"][0]["name"], "Bob");
    // Someone else can't accept a request that wasn't sent to them.
    let cara = h.login("Cara", CARA).await;
    let (status, _) = h.call("POST", &format!("/v1/friends/requests/{ALEX}/accept"), Some(&cara), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    // A request back counts as accepting.
    let (_, back) = h.call("POST", "/v1/friends/requests", Some(&bob), Some(json!({ "target": "Alex" }))).await;
    assert_eq!(back["status"], "accepted");
    let (status, _) = h.call("POST", "/v1/friends/requests", Some(&alex), Some(json!({ "target": "Bob" }))).await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (_, list) = h.call("GET", "/v1/friends", Some(&alex), None).await;
    assert_eq!(friend(&list, BOB).unwrap()["status"], "offline");
    assert!(list["incoming"].as_array().unwrap().is_empty() && list["outgoing"].as_array().unwrap().is_empty());

    let (status, _) = h.call("PUT", "/v1/me/social", Some(&cara), Some(json!({ "requests": "none" }))).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = h.call("POST", "/v1/friends/requests", Some(&alex), Some(json!({ "target": "Cara" }))).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = h.call("PUT", "/v1/me/social", Some(&cara), Some(json!({ "requests": "friends-of-friends" }))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = h.call("PUT", "/v1/me/social", Some(&cara), Some(json!({ "share": "everything" }))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, _) = h.call("DELETE", &format!("/v1/friends/{BOB}"), Some(&alex), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, list) = h.call("GET", "/v1/friends", Some(&bob), None).await;
    assert!(friend(&list, ALEX).is_none(), "removing a friend removes it on both sides");
    let (status, _) = h.call("GET", "/v1/friends", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn presence_only_reaches_friends_and_respects_privacy() {
    let h = harness(|_| {}).await;
    let alex = h.login("Alex", ALEX).await;
    let bob = h.login("Bob", BOB).await;
    let cara = h.login("Cara", CARA).await;
    befriend(&h, &alex, &bob, "Bob", BOB, ALEX).await;

    // The launcher never heartbeats, so a signed-in player who isn't in game stays offline.
    let (_, list) = h.call("GET", "/v1/friends", Some(&bob), None).await;
    assert_eq!(friend(&list, ALEX).unwrap()["status"], "offline");
    let (status, _) = h.call("POST", "/v1/heartbeat", Some(&alex), Some(beat("26.3", "multiplayer", Some("Play.Example.net")))).await;
    assert_eq!(status, StatusCode::OK);
    let (_, list) = h.call("GET", "/v1/friends", Some(&bob), None).await;
    let a = friend(&list, ALEX).unwrap();
    assert_eq!(a["status"], "online");
    assert_eq!(a["activity"]["version"], "26.3");
    assert_eq!(a["activity"]["server"], "play.example.net");
    // Cara is not a friend and there is no endpoint that shows anyone else's presence.
    let (_, list) = h.call("GET", "/v1/friends", Some(&cara), None).await;
    assert!(list["friends"].as_array().unwrap().is_empty());
    assert!(!list.to_string().contains("play.example.net"));
    let (_, players) = h.call("GET", &format!("/v1/players?uuids={ALEX}"), None, None).await;
    assert!(!players.to_string().contains("play.example.net"), "the public lookup never shows activity");

    h.call("PUT", "/v1/me/social", Some(&alex), Some(json!({ "share": "game" }))).await;
    let (_, list) = h.call("GET", "/v1/friends", Some(&bob), None).await;
    let a = friend(&list, ALEX).unwrap();
    assert_eq!(a["activity"]["mode"], "multiplayer");
    assert!(a["activity"]["server"].is_null());
    assert!(!list.to_string().contains("example.net"));

    h.call("PUT", "/v1/me/social", Some(&alex), Some(json!({ "share": "status", "dnd": true }))).await;
    let (_, list) = h.call("GET", "/v1/friends", Some(&bob), None).await;
    let a = friend(&list, ALEX).unwrap();
    assert_eq!(a["status"], "dnd");
    assert!(a["activity"].is_null());

    let (_, me) = h.call("PUT", "/v1/me/social", Some(&alex), Some(json!({ "appearOffline": true }))).await;
    assert_eq!(me, json!({ "share": "status", "appearOffline": true, "requests": "everyone", "dnd": true }));
    let (_, list) = h.call("GET", "/v1/friends", Some(&bob), None).await;
    let a = friend(&list, ALEX).unwrap();
    assert_eq!(a["status"], "offline");
    assert!(a["activity"].is_null());

    let (status, _) = h.call("POST", "/v1/heartbeat", Some(&alex), Some(json!({ "activity": { "version": "26.3", "mode": "lan" } }))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = h.call("POST", "/v1/heartbeat", Some(&alex), Some(json!({}))).await;
    assert_eq!(status, StatusCode::OK, "older clients send an empty object");
}

#[tokio::test]
async fn blocking_cuts_everything_off() {
    let h = harness(|_| {}).await;
    let alex = h.login("Alex", ALEX).await;
    let bob = h.login("Bob", BOB).await;
    befriend(&h, &alex, &bob, "Bob", BOB, ALEX).await;
    h.call("POST", "/v1/heartbeat", Some(&alex), Some(beat("1.21.11", "multiplayer", Some("mc.example")))).await;

    let (status, _) = h.call("PUT", &format!("/v1/blocks/{ALEX}"), Some(&bob), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, list) = h.call("GET", "/v1/friends", Some(&alex), None).await;
    assert!(friend(&list, BOB).is_none());
    let (_, list) = h.call("GET", "/v1/friends", Some(&bob), None).await;
    assert!(friend(&list, ALEX).is_none() && !list.to_string().contains("mc.example"));
    assert_eq!(list["blocked"][0]["uuid"], ALEX);

    // Alex sees the usual reply, but Bob never gets the request.
    let (status, sent) = h.call("POST", "/v1/friends/requests", Some(&alex), Some(json!({ "target": "Bob" }))).await;
    assert_eq!((status, sent["status"].clone()), (StatusCode::OK, json!("sent")));
    let (_, list) = h.call("GET", "/v1/friends", Some(&bob), None).await;
    assert!(list["incoming"].as_array().unwrap().is_empty());
    let (_, events) = h.call("GET", "/v1/events?after=0", Some(&bob), None).await;
    assert!(events["events"].as_array().unwrap().iter().all(|e| e["from"]["uuid"] != ALEX));
    let (status, _) = h.call("POST", "/v1/friends/requests", Some(&bob), Some(json!({ "target": "Alex" }))).await;
    assert_eq!(status, StatusCode::CONFLICT, "the blocker has to unblock first");

    let (status, _) = h.call("DELETE", &format!("/v1/blocks/{ALEX}"), Some(&bob), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, sent) = h.call("POST", "/v1/friends/requests", Some(&bob), Some(json!({ "target": "Alex" }))).await;
    assert_eq!(sent["status"], "sent");
}

#[tokio::test]
async fn notifications_are_long_polled_and_private() {
    let h = harness(|_| {}).await;
    let alex = h.login("Alex", ALEX).await;
    let bob = h.login("Bob", BOB).await;
    let cara = h.login("Cara", CARA).await;
    let (_, start) = h.call("GET", "/v1/events", Some(&bob), None).await;
    assert!(start["events"].as_array().unwrap().is_empty(), "a fresh client gets a cursor, not a backlog");
    let cursor = start["cursor"].as_u64().unwrap();

    // Bob waits; Alex's request should wake him well before the wait runs out.
    let waiter = {
        let app = h.app.clone();
        let bob = bob.clone();
        tokio::spawn(async move {
            let req = Request::get(format!("/v1/events?after={cursor}&wait=10")).header(header::AUTHORIZATION, format!("Bearer {bob}")).body(Body::empty()).unwrap();
            let res = app.oneshot(req).await.unwrap();
            serde_json::from_slice::<Value>(&res.into_body().collect().await.unwrap().to_bytes()).unwrap()
        })
    };
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let started = std::time::Instant::now();
    h.call("POST", "/v1/friends/requests", Some(&alex), Some(json!({ "target": "Bob" }))).await;
    let got = waiter.await.unwrap();
    assert!(started.elapsed() < std::time::Duration::from_secs(5));
    assert_eq!(got["events"][0]["type"], "request");
    assert_eq!(got["events"][0]["from"]["name"], "Alex");
    let cursor = got["cursor"].as_u64().unwrap();

    let (_, start) = h.call("GET", "/v1/events", Some(&alex), None).await;
    let alex_cursor = start["cursor"].as_u64().unwrap();
    h.call("POST", &format!("/v1/friends/requests/{ALEX}/accept"), Some(&bob), None).await;
    let (_, got) = h.call("GET", &format!("/v1/events?after={alex_cursor}"), Some(&alex), None).await;
    assert_eq!(got["events"][0]["type"], "accepted");

    h.call("PUT", "/v1/me/social", Some(&bob), Some(json!({ "dnd": true }))).await;
    h.call("POST", "/v1/heartbeat", Some(&alex), Some(beat("26.3", "menu", None))).await;
    h.call("POST", "/v1/heartbeat", Some(&alex), Some(beat("26.3", "multiplayer", Some("mc.example")))).await;
    h.call("POST", "/v1/heartbeat", Some(&alex), Some(beat("26.3", "multiplayer", Some("mc.example")))).await;
    let (_, got) = h.call("GET", &format!("/v1/events?after={cursor}"), Some(&bob), None).await;
    let kinds: Vec<&str> = got["events"].as_array().unwrap().iter().map(|e| e["type"].as_str().unwrap()).collect();
    assert_eq!(kinds, ["settings", "online", "server"], "DND still queues events; the client decides not to toast");
    assert_eq!(got["events"][0]["settings"]["dnd"], true, "Bob's own clients hear his DND change");
    assert_eq!(got["events"][2]["server"], "mc.example");
    let (_, other) = h.call("GET", "/v1/events?after=0", Some(&cara), None).await;
    assert!(other["events"].as_array().unwrap().is_empty(), "non-friends hear nothing");

    // Hidden server: no server event; appear offline: no online event.
    let cursor = got["cursor"].as_u64().unwrap();
    h.call("PUT", "/v1/me/social", Some(&alex), Some(json!({ "share": "game" }))).await;
    h.call("POST", "/v1/heartbeat", Some(&alex), Some(beat("26.3", "multiplayer", Some("secret.example")))).await;
    let (_, got) = h.call("GET", &format!("/v1/events?after={cursor}"), Some(&bob), None).await;
    assert!(!got.to_string().contains("secret.example"));
    let kinds: Vec<&str> = got["events"].as_array().unwrap().iter().map(|e| e["type"].as_str().unwrap()).collect();
    assert_eq!(kinds, ["presence"], "only a silent list refresh for hiding the server");

    let (status, _) = h.call("GET", "/v1/events?after=999999&wait=1", Some(&bob), None).await;
    assert_eq!(status, StatusCode::OK, "a cursor from before a restart is reset, not an error");
}

#[tokio::test]
async fn appearing_offline_sends_no_online_toast() {
    let h = harness(|_| {}).await;
    let alex = h.login("Alex", ALEX).await;
    let bob = h.login("Bob", BOB).await;
    befriend(&h, &alex, &bob, "Bob", BOB, ALEX).await;
    let (_, start) = h.call("GET", "/v1/events", Some(&bob), None).await;
    let cursor = start["cursor"].as_u64().unwrap();
    h.call("PUT", "/v1/me/social", Some(&alex), Some(json!({ "appearOffline": true }))).await;
    h.call("POST", "/v1/heartbeat", Some(&alex), Some(beat("1.8.9", "multiplayer", Some("hypixel.net")))).await;
    let (_, got) = h.call("GET", &format!("/v1/events?after={cursor}"), Some(&bob), None).await;
    assert!(got["events"].as_array().unwrap().is_empty());
    // Turning it off while in game announces the player once.
    h.call("PUT", "/v1/me/social", Some(&alex), Some(json!({ "appearOffline": false }))).await;
    let (_, got) = h.call("GET", &format!("/v1/events?after={cursor}"), Some(&bob), None).await;
    assert_eq!(got["events"].as_array().unwrap().len(), 1);
    assert_eq!(got["events"][0]["type"], "online");
}

#[tokio::test]
async fn notifications_and_cursors_survive_restart_and_pruning() {
    let mut h = harness(|_| {}).await;
    let alex = h.login("Alex", ALEX).await;
    let bob = h.login("Bob", BOB).await;
    let (_, initial) = h.call("GET", "/v1/events", Some(&bob), None).await;
    let cursor = initial["cursor"].as_u64().unwrap();
    h.call("POST", "/v1/friends/requests", Some(&alex), Some(json!({ "target": "Bob" }))).await;
    let (_, before) = h.call("GET", &format!("/v1/events?after={cursor}"), Some(&bob), None).await;
    h.restart();
    let (status, after) = h.call("GET", &format!("/v1/events?after={cursor}"), Some(&bob), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(before, after);
    assert_eq!(after["events"][0]["type"], "request");
    let id = after["cursor"].as_u64().unwrap();
    h.state.db().execute("UPDATE social_events SET created = 0", []).unwrap();
    let (_, expired) = h.call("GET", "/v1/events?after=0", Some(&bob), None).await;
    assert!(expired["events"].as_array().unwrap().is_empty(), "expiry applies even before cleanup");
    h.state.sweep();
    h.restart();
    let (_, empty) = h.call("GET", "/v1/events", Some(&bob), None).await;
    assert_eq!(empty["cursor"], id, "pruning must not rewind the cursor");
    h.call("POST", &format!("/v1/friends/requests/{ALEX}/accept"), Some(&bob), None).await;
    let (_, accepted) = h.call("GET", &format!("/v1/events?after={id}"), Some(&alex), None).await;
    assert_eq!(accepted["events"][0]["type"], "accepted");
    assert!(accepted["events"][0]["id"].as_u64().unwrap() > id);
}

#[tokio::test]
async fn notification_retention_is_bounded_and_blocks_survive_restart() {
    let mut h = harness(|_| {}).await;
    let alex = h.login("Alex", ALEX).await;
    let bob = h.login("Bob", BOB).await;
    befriend(&h, &alex, &bob, "Bob", BOB, ALEX).await;
    for n in 0..60 {
        let (status, _) = h.call("POST", "/v1/heartbeat", Some(&alex), Some(beat("26.3", "multiplayer", Some(&format!("server{n}.example"))))).await;
        assert_eq!(status, StatusCode::OK);
    }
    h.restart();
    let (_, events) = h.call("GET", "/v1/events?after=0", Some(&bob), None).await;
    let events = events["events"].as_array().unwrap();
    assert_eq!(events.len(), 50);
    assert!(events.windows(2).all(|pair| pair[0]["id"].as_u64() < pair[1]["id"].as_u64()));
    let (status, _) = h.call("PUT", &format!("/v1/blocks/{ALEX}"), Some(&bob), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    h.restart();
    let (_, events) = h.call("GET", "/v1/events?after=0", Some(&bob), None).await;
    assert!(events["events"].as_array().unwrap().is_empty());
    let (_, list) = h.call("GET", "/v1/friends", Some(&bob), None).await;
    assert!(friend(&list, ALEX).is_none());
}

#[tokio::test]
async fn privacy_changes_revoke_queued_activity_before_and_after_restart() {
    for setting in [json!({ "share": "game" }), json!({ "share": "status" }), json!({ "appearOffline": true })] {
        let mut h = harness(|_| {}).await;
        let alex = h.login("Alex", ALEX).await;
        let bob = h.login("Bob", BOB).await;
        befriend(&h, &alex, &bob, "Bob", BOB, ALEX).await;
        h.call("POST", "/v1/heartbeat", Some(&alex), Some(beat("26.3", "multiplayer", Some("private.example")))).await;
        h.call("PUT", "/v1/me/social", Some(&alex), Some(setting)).await;
        for restarted in [false, true] {
            if restarted { h.restart(); }
            let (_, events) = h.call("GET", "/v1/events?after=0", Some(&bob), None).await;
            assert!(!events.to_string().contains("private.example"));
            assert!(events["events"].as_array().unwrap().iter().all(|e| e["type"] != "online" && e["type"] != "server"));
        }
    }
}
