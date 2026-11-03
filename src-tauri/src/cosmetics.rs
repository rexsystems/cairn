//! Client for the self-hosted Cairn cosmetics server (cosmetics-server/). Nothing is contacted until a URL is set.
use crate::accounts::Account;
use crate::settings::Settings;
use anyhow::{anyhow, bail, Context, Result};
use once_cell::sync::Lazy;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

const MOJANG_JOIN: &str = "https://sessionserver.mojang.com/session/minecraft/join";
/// Matches the server's upload limit, so obviously-too-big files fail before uploading.
pub const MAX_UPLOAD: usize = 1024 * 1024;

// The bearer token must never follow a redirect to another host.
static HTTP: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .user_agent(crate::net::USER_AGENT)
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .build()
        .expect("cosmetics http client")
});

/// Session tokens per (server, account uuid). Memory only, so nothing reusable is written to disk.
type Sessions = HashMap<(String, String), (String, i64)>;
static SESSIONS: Lazy<Mutex<Sessions>> = Lazy::new(Default::default);

/// https anywhere; plain http only to this machine, for testing a local server.
pub fn normalize_url(raw: &str) -> Option<String> {
    let url = reqwest::Url::parse(raw.trim()).ok()?;
    let loopback = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    let scheme_ok = url.scheme() == "https" || url.scheme() == "http" && loopback;
    if !scheme_ok || url.host_str().is_none() || !url.username().is_empty() || url.password().is_some() || url.query().is_some() || url.fragment().is_some() {
        return None;
    }
    Some(url.as_str().trim_end_matches('/').to_owned())
}

/// Runtime env (for testing) wins, then the user's setting, then a URL baked into the build.
pub fn server_url(settings: &Settings) -> Option<String> {
    let runtime = std::env::var("CAIRN_COSMETICS_URL").ok();
    pick_url(runtime.as_deref(), &settings.cosmetics_url, option_env!("CAIRN_COSMETICS_URL"))
}

fn pick_url(runtime: Option<&str>, setting: &str, baked: Option<&str>) -> Option<String> {
    [runtime, Some(setting), baked].into_iter().flatten().find(|v| !v.trim().is_empty()).and_then(normalize_url)
}

fn compact(uuid: &str) -> String {
    uuid.replace('-', "").to_ascii_lowercase()
}

async fn read(resp: reqwest::Response) -> Result<Value> {
    let status = resp.status();
    let body: Value = resp.json().await.unwrap_or(Value::Null);
    if status.is_success() {
        return Ok(body);
    }
    let msg = body.get("error").and_then(Value::as_str).unwrap_or("The cosmetics server returned an error.");
    Err(anyhow!(Status(status.as_u16(), msg.to_owned())))
}

#[derive(Debug)]
struct Status(u16, String);
impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.1)
    }
}
impl std::error::Error for Status {}
fn status_of(err: &anyhow::Error) -> Option<u16> {
    err.downcast_ref::<Status>().map(|s| s.0)
}

pub async fn stats(base: &str) -> Result<Value> {
    read(HTTP.get(format!("{base}/v1/stats")).send().await.context("reaching the cosmetics server")?).await
}

/// Proves account ownership the same way a Minecraft server does: Mojang join, then the server checks hasJoined.
async fn sign_in(base: &str, acc: &Account) -> Result<String> {
    let start = read(HTTP.post(format!("{base}/v1/auth/start")).json(&json!({ "name": acc.username })).send().await.context("reaching the cosmetics server")?).await?;
    let server_id = start.get("serverId").and_then(Value::as_str).ok_or_else(|| anyhow!("The cosmetics server sent an invalid reply."))?.to_owned();
    if acc.kind == "microsoft" {
        let join = HTTP
            .post(MOJANG_JOIN)
            .json(&json!({ "accessToken": acc.mc_token, "selectedProfile": compact(&acc.uuid), "serverId": server_id }))
            .send()
            .await
            .context("reaching Mojang")?;
        if !join.status().is_success() {
            bail!("Mojang refused the sign-in ({}). Sign in to {} again from Accounts.", join.status().as_u16(), acc.username);
        }
    }
    let finish = HTTP
        .post(format!("{base}/v1/auth/finish"))
        .json(&json!({ "name": acc.username, "serverId": server_id }))
        .send()
        .await
        .context("reaching the cosmetics server")?;
    let body = match read(finish).await {
        Ok(body) => body,
        Err(err) if acc.kind != "microsoft" && matches!(status_of(&err), Some(401 | 403)) => {
            bail!("Offline accounts can see cosmetics but not wear them. Use a Microsoft account.")
        }
        Err(err) => return Err(err),
    };
    let token = body.get("token").and_then(Value::as_str).ok_or_else(|| anyhow!("The cosmetics server sent an invalid reply."))?.to_owned();
    let expires = body.get("expiresIn").and_then(Value::as_i64).unwrap_or(3600);
    SESSIONS.lock().unwrap().insert((base.to_owned(), compact(&acc.uuid)), (token.clone(), crate::util::now_ms() + (expires - 60) * 1000));
    Ok(token)
}

async fn token(base: &str, acc: &Account) -> Result<String> {
    let cached = SESSIONS.lock().unwrap().get(&(base.to_owned(), compact(&acc.uuid))).cloned();
    match cached {
        Some((token, until)) if until > crate::util::now_ms() => Ok(token),
        _ => sign_in(base, acc).await,
    }
}

/// Runs an authorized request, signing in again once if the server forgot the session (restart, expiry).
async fn authed(base: &str, acc: &Account, build: impl Fn(&str) -> reqwest::RequestBuilder) -> Result<Value> {
    let first = token(base, acc).await?;
    match read(build(&first).send().await.context("reaching the cosmetics server")?).await {
        Err(err) if status_of(&err) == Some(401) => {
            SESSIONS.lock().unwrap().remove(&(base.to_owned(), compact(&acc.uuid)));
            let fresh = sign_in(base, acc).await?;
            read(build(&fresh).send().await.context("reaching the cosmetics server")?).await
        }
        other => other,
    }
}

pub async fn me(base: &str, acc: &Account) -> Result<Value> {
    authed(base, acc, |t| HTTP.get(format!("{base}/v1/me")).bearer_auth(t)).await
}

#[cfg(test)]
pub async fn equip(base: &str, acc: &Account, cape: Option<&str>, badge: Option<&str>) -> Result<Value> {
    let body = json!({ "cape": cape, "badge": badge });
    authed(base, acc, |t| HTTP.put(format!("{base}/v1/me/equipped")).bearer_auth(t).json(&body)).await?;
    me(base, acc).await
}

pub async fn upload(base: &str, acc: &Account, png: Vec<u8>, frame_ms: u32) -> Result<Value> {
    if png.len() > MAX_UPLOAD {
        bail!("Capes must be under 1 MB.");
    }
    if !png.starts_with(b"\x89PNG\r\n\x1a\n") {
        bail!("Capes must be PNG images.");
    }
    authed(base, acc, |t| HTTP.post(format!("{base}/v1/me/cape?frameMs={}", frame_ms)).bearer_auth(t).header("Content-Type", "image/png").body(png.clone())).await?;
    me(base, acc).await
}

pub async fn remove_custom(base: &str, acc: &Account) -> Result<Value> {
    authed(base, acc, |t| HTTP.delete(format!("{base}/v1/me/cape")).bearer_auth(t)).await?;
    me(base, acc).await
}

/// The UI's CSP and the cosmetics server's CORS both stay strict, so cape images reach the page as data URLs.
pub async fn cape_data_url(base: &str, hash: &str) -> Result<String> {
    if hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        bail!("Unknown cape.");
    }
    let resp = HTTP.get(format!("{base}/v1/capes/{hash}.png")).send().await.context("reaching the cosmetics server")?;
    if !resp.status().is_success() {
        bail!("The cape image is not available ({}).", resp.status().as_u16());
    }
    let bytes = resp.bytes().await?;
    // Re-encoded strips can outgrow the upload limit, but never by this much.
    if bytes.len() > 8 * MAX_UPLOAD || !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        bail!("The cosmetics server sent an invalid cape image.");
    }
    use base64::Engine;
    Ok(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(&bytes)))
}

/// Cairn Client friends. The launcher only reads and manages the list; it never sends a heartbeat,
/// so it can't make anyone look in-game. Presence comes from the mod alone.
pub async fn friends(base: &str, acc: &Account) -> Result<Value> {
    authed(base, acc, |t| HTTP.get(format!("{base}/v1/friends")).bearer_auth(t)).await
}

/// Builds the request for one friend action; the target is checked so it can't change the path.
fn friend_request(base: &str, action: &str, target: &str) -> Result<(reqwest::Method, String, Option<Value>)> {
    let target = target.trim();
    let uuid = || -> Result<String> {
        let id = compact(target);
        if id.len() != 32 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
            bail!("That isn't a player id.");
        }
        Ok(id)
    };
    use reqwest::Method;
    Ok(match action {
        "add" => {
            let name_ok = (1..=16).contains(&target.len()) && target.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_');
            if !name_ok && uuid().is_err() {
                bail!("Enter a Minecraft name or UUID.");
            }
            (Method::POST, format!("{base}/v1/friends/requests"), Some(json!({ "target": target })))
        }
        "accept" | "decline" | "cancel" => (Method::POST, format!("{base}/v1/friends/requests/{}/{action}", uuid()?), None),
        "remove" => (Method::DELETE, format!("{base}/v1/friends/{}", uuid()?), None),
        "block" => (Method::PUT, format!("{base}/v1/blocks/{}", uuid()?), None),
        "unblock" => (Method::DELETE, format!("{base}/v1/blocks/{}", uuid()?), None),
        _ => bail!("Unknown friend action."),
    })
}

/// Runs one action and returns the refreshed list, plus the server's reply for "add".
pub async fn friend_action(base: &str, acc: &Account, action: &str, target: &str) -> Result<Value> {
    let (method, url, body) = friend_request(base, action, target)?;
    let reply = authed(base, acc, |t| {
        let req = HTTP.request(method.clone(), &url).bearer_auth(t);
        match &body {
            Some(body) => req.json(body),
            None => req,
        }
    })
    .await?;
    let mut list = friends(base, acc).await?;
    list["reply"] = reply;
    Ok(list)
}

/// Partial privacy update; the server validates values and applies only the given fields.
pub async fn social_settings(base: &str, acc: &Account, patch: &Value) -> Result<Value> {
    authed(base, acc, |t| HTTP.put(format!("{base}/v1/me/social")).bearer_auth(t).json(patch)).await
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub url: Option<String>,
    pub stats: Option<Value>,
    pub me: Option<Value>,
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn friend_actions_keep_targets_out_of_the_path() {
        let id = "069a79f4-44e9-4726-a5be-fca90e38aaf5";
        let (m, url, body) = friend_request("https://c.example", "accept", id).unwrap();
        assert_eq!((m, url.as_str(), body), (reqwest::Method::POST, "https://c.example/v1/friends/requests/069a79f444e94726a5befca90e38aaf5/accept", None));
        let (m, url, body) = friend_request("https://c.example", "add", " Notch ").unwrap();
        assert_eq!((m, url.as_str(), body.unwrap()["target"].as_str()), (reqwest::Method::POST, "https://c.example/v1/friends/requests", Some("Notch")));
        assert_eq!(friend_request("https://c.example", "unblock", id).unwrap().0, reqwest::Method::DELETE);
        assert!(friend_request("https://c.example", "remove", "../../admin").is_err());
        assert!(friend_request("https://c.example", "block", "Notch").is_err());
        assert!(friend_request("https://c.example", "add", "bad name").is_err());
        assert!(friend_request("https://c.example", "heartbeat", id).is_err());
    }

    #[test]
    fn urls_need_https_except_on_loopback() {
        assert_eq!(normalize_url(" https://cosmetics.example.com/ ").as_deref(), Some("https://cosmetics.example.com"));
        assert_eq!(normalize_url("http://127.0.0.1:8787").as_deref(), Some("http://127.0.0.1:8787"));
        assert_eq!(normalize_url("http://localhost:8787/").as_deref(), Some("http://localhost:8787"));
        assert_eq!(normalize_url("http://cosmetics.example.com"), None);
        assert_eq!(normalize_url("https://user:pw@cosmetics.example.com"), None);
        assert_eq!(normalize_url("https://cosmetics.example.com/?a=1"), None);
        assert_eq!(normalize_url("ftp://cosmetics.example.com"), None);
        assert_eq!(normalize_url(""), None);
    }

    #[test]
    fn runtime_env_then_setting_then_build_default() {
        assert_eq!(pick_url(Some("https://a.example"), "https://b.example", Some("https://c.example")).as_deref(), Some("https://a.example"));
        assert_eq!(pick_url(None, "https://b.example", Some("https://c.example")).as_deref(), Some("https://b.example"));
        assert_eq!(pick_url(Some(" "), "", Some("https://c.example")).as_deref(), Some("https://c.example"));
        assert_eq!(pick_url(None, "", None), None);
        assert_eq!(pick_url(None, "http://evil.example", Some("https://c.example")), None, "a bad setting is not silently replaced");
    }

    #[tokio::test]
    async fn uploads_are_checked_before_sending() {
        let acc = Account { username: "Steve".into(), uuid: "00000000-0000-0000-0000-000000000000".into(), kind: "offline".into(), ..Default::default() };
        let err = upload("http://127.0.0.1:9", &acc, b"GIF89a".to_vec(), 100).await.unwrap_err();
        assert!(err.to_string().contains("PNG"));
        let err = upload("http://127.0.0.1:9", &acc, vec![0; MAX_UPLOAD + 1], 100).await.unwrap_err();
        assert!(err.to_string().contains("1 MB"));
        assert!(cape_data_url("http://127.0.0.1:9", "../../etc/passwd").await.unwrap_err().to_string().contains("Unknown"));
    }

    /// Against a real server started with COSMETICS_DEV_INSECURE_AUTH=1, e.g.
    /// `CAIRN_COSMETICS_LIVE=http://127.0.0.1:8787 cargo test cosmetics -- --ignored`.
    #[tokio::test]
    #[ignore]
    async fn live_server_round_trip() {
        let Ok(base) = std::env::var("CAIRN_COSMETICS_LIVE") else { return };
        let acc = Account { username: "LauncherTest".into(), uuid: "0f6f2b0e-6c2a-3d9b-8a40-111111111111".into(), kind: "offline".into(), ..Default::default() };
        assert!(stats(&base).await.unwrap().get("online").is_some());
        let denied = equip(&base, &acc, None, Some("gold")).await.unwrap_err();
        assert!(denied.to_string().contains("not available"), "staff badges stay staff-only: {denied:#}");
        let me_now = equip(&base, &acc, Some("ember"), Some("moss")).await.unwrap();
        assert_eq!(me_now["equipped"]["cape"], "ember");
        SESSIONS.lock().unwrap().values_mut().for_each(|s| s.0 = "stale".into());
        assert_eq!(me(&base, &acc).await.unwrap()["equipped"]["badge"], "moss", "a forgotten session signs in again");
    }
}
