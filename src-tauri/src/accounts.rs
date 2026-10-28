use crate::net::CLIENT;
use crate::paths::Paths;
use crate::util;
use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Locally remembered skins. Mojang's profile API only returns the active skin,
/// so Cairn keeps a short history of URLs seen/uploaded for this account.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct SkinHistoryEntry {
    pub id: String,
    pub url: String,
    /// classic | slim
    pub model: String,
    pub seen: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub kind: String, // microsoft | offline
    pub username: String,
    pub uuid: String,
    pub skin_url: Option<String>,
    pub cape_url: Option<String>,
    /// classic | slim
    pub skin_model: String,
    #[serde(default)]
    pub skin_history: Vec<SkinHistoryEntry>,
    pub added: i64,
    pub ms_flow: String, // live | msal
    pub client_id: String,
    pub refresh_token: String,
    pub mc_token: String,
    pub mc_expires: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct AccountsFile {
    pub accounts: Vec<Account>,
    pub active: Option<String>,
}

/// What the UI sees: never includes tokens.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AccountView {
    pub id: String,
    pub kind: String,
    pub username: String,
    pub uuid: String,
    pub skin_url: Option<String>,
    pub cape_url: Option<String>,
    pub skin_model: String,
    pub added: i64,
    pub active: bool,
    pub needs_sign_in: bool,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CapeOption {
    pub id: String,
    pub alias: String,
    pub url: String,
    pub active: bool,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SkinOption {
    pub id: String,
    pub url: String,
    pub model: String,
    pub active: bool,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AccountCosmetics {
    pub account: AccountView,
    pub capes: Vec<CapeOption>,
    pub skins: Vec<SkinOption>,
}

const SKIN_HISTORY_MAX: usize = 24;

fn skin_id_for_url(url: &str) -> String {
    url.rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or(url)
        .chars()
        .take(40)
        .collect()
}

fn remember_skin(acc: &mut Account, url: &str, model: &str) {
    if url.is_empty() {
        return;
    }
    let model = if model.eq_ignore_ascii_case("slim") { "slim" } else { "classic" };
    let now = util::now_ms();
    if let Some(pos) = acc.skin_history.iter().position(|h| h.url == url) {
        let mut entry = acc.skin_history.remove(pos);
        entry.model = model.into();
        entry.seen = now;
        acc.skin_history.insert(0, entry);
    } else {
        acc.skin_history.insert(
            0,
            SkinHistoryEntry {
                id: skin_id_for_url(url),
                url: url.into(),
                model: model.into(),
                seen: now,
            },
        );
    }
    acc.skin_history.truncate(SKIN_HISTORY_MAX);
}

fn skins_payload(acc: &Account) -> Vec<SkinOption> {
    let active = acc.skin_url.as_deref();
    let mut out: Vec<SkinOption> = acc
        .skin_history
        .iter()
        .map(|h| SkinOption {
            id: h.id.clone(),
            url: h.url.clone(),
            model: h.model.clone(),
            active: active == Some(h.url.as_str()),
        })
        .collect();
    if let Some(url) = &acc.skin_url {
        if !out.iter().any(|s| &s.url == url) {
            out.insert(
                0,
                SkinOption {
                    id: skin_id_for_url(url),
                    url: url.clone(),
                    model: if acc.skin_model.is_empty() {
                        "classic".into()
                    } else {
                        acc.skin_model.clone()
                    },
                    active: true,
                },
            );
        }
    }
    out
}

fn cosmetics_response(file: &AccountsFile, idx: usize, capes: Vec<CapeOption>) -> AccountCosmetics {
    AccountCosmetics {
        account: file.view(&file.accounts[idx]),
        capes,
        skins: skins_payload(&file.accounts[idx]),
    }
}

impl AccountsFile {
    pub fn load(paths: &Paths) -> Self {
        util::read_json(&paths.accounts_file()).unwrap_or_default()
    }
    pub fn save(&self, paths: &Paths) -> Result<()> {
        let p = paths.accounts_file();
        util::write_json_atomic(&p, self)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o600));
        }
        Ok(())
    }
    pub fn views(&self) -> Vec<AccountView> {
        self.accounts
            .iter()
            .map(|a| self.view(a))
            .collect()
    }
    pub fn view(&self, a: &Account) -> AccountView {
        AccountView {
            id: a.id.clone(),
            kind: a.kind.clone(),
            username: a.username.clone(),
            uuid: a.uuid.clone(),
            skin_url: a.skin_url.clone(),
            cape_url: a.cape_url.clone(),
            skin_model: if a.skin_model.is_empty() { "classic".into() } else { a.skin_model.clone() },
            added: a.added,
            active: self.active.as_deref() == Some(&a.id),
            needs_sign_in: a.kind == "microsoft" && a.refresh_token.is_empty(),
        }
    }
    pub fn has_microsoft(&self) -> bool {
        self.accounts.iter().any(|a| a.kind == "microsoft")
    }
    pub fn upsert(&mut self, acc: Account) {
        let id = acc.id.clone();
        if let Some(existing) = self.accounts.iter_mut().find(|a| a.kind == acc.kind && a.uuid == acc.uuid) {
            let keep_id = existing.id.clone();
            let keep_history = existing.skin_history.clone();
            *existing = Account {
                id: keep_id.clone(),
                skin_history: keep_history,
                ..acc
            };
            if let Some(url) = existing.skin_url.clone() {
                let model = existing.skin_model.clone();
                remember_skin(existing, &url, &model);
            }
            self.active = Some(keep_id);
        } else {
            self.accounts.push(acc);
            self.active = Some(id);
        }
    }
    pub fn remove(&mut self, id: &str) {
        self.accounts.retain(|a| a.id != id);
        if self.active.as_deref() == Some(id) {
            self.active = self.accounts.first().map(|a| a.id.clone());
        }
    }
}

pub fn valid_offline_name(name: &str) -> bool {
    (3..=16).contains(&name.len()) && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub fn add_offline(file: &mut AccountsFile, name: &str) -> Result<Account> {
    let name = name.trim();
    if !valid_offline_name(name) {
        bail!("Use 3 to 16 characters: letters, numbers and underscores.");
    }
    let acc = Account {
        id: uuid::Uuid::new_v4().to_string(),
        kind: "offline".into(),
        username: name.into(),
        uuid: crate::mc::launch::offline_uuid(name),
        added: util::now_ms(),
        ..Default::default()
    };
    file.upsert(acc.clone());
    Ok(acc)
}

// Microsoft sign-in
// Browser window uses the classic Live OAuth code flow (same client ID / redirect as the Minecraft launcher and Modrinth App).

pub fn flow_for(client_id: &str) -> &'static str {
    // Azure app registrations use GUID client IDs; Windows Live IDs are 16 hex digits.
    if client_id.len() == 36 && client_id.chars().filter(|c| *c == '-').count() == 4 { "msal" } else { "live" }
}

const LIVE_SCOPE: &str = "service::user.auth.xboxlive.com::MBI_SSL";
const MSAL_SCOPE: &str = "XboxLive.signin offline_access";
pub const AUTH_REDIRECT: &str = "https://login.live.com/oauth20_desktop.srf";

/// URL opened in the sign-in webview. Only the Live (Minecraft launcher) client ID supports this redirect.
pub fn authorize_url(client_id: &str) -> Result<String> {
    if flow_for(client_id) != "live" {
        bail!("Window sign-in needs the built-in Minecraft launcher client ID. Unset CAIRN_MSA_CLIENT_ID.");
    }
    Ok(format!(
        "https://login.live.com/oauth20_authorize.srf?client_id={}&response_type=code&redirect_uri={}&scope={}&prompt=select_account",
        urlencoding::encode(client_id),
        urlencoding::encode(AUTH_REDIRECT),
        urlencoding::encode(LIVE_SCOPE),
    ))
}

pub async fn exchange_auth_code(client_id: &str, code: &str) -> Result<(String, String)> {
    let r = CLIENT
        .post(token_url("live"))
        .form(&[
            ("client_id", client_id),
            ("code", code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", AUTH_REDIRECT),
            ("scope", LIVE_SCOPE),
        ])
        .send()
        .await
        .context("exchanging Microsoft sign-in code")?;
    let status = r.status();
    let v: Value = r.json().await.context("reading Microsoft token reply")?;
    if let Some(at) = v.get("access_token").and_then(|x| x.as_str()) {
        return Ok((at.into(), v["refresh_token"].as_str().unwrap_or_default().into()));
    }
    bail!(
        "Microsoft sign-in failed: {}",
        v.get("error_description").or(v.get("error")).and_then(|x| x.as_str()).unwrap_or(&status.to_string())
    );
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCode {
    pub user_code: String,
    pub device_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
    pub flow: String,
    pub client_id: String,
}

pub async fn begin_device_code(client_id: &str) -> Result<DeviceCode> {
    let flow = flow_for(client_id);
    let (url, scope) = if flow == "live" {
        ("https://login.live.com/oauth20_connect.srf", LIVE_SCOPE)
    } else {
        ("https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode", MSAL_SCOPE)
    };
    let mut form = vec![("client_id", client_id), ("scope", scope)];
    if flow == "live" {
        form.push(("response_type", "device_code"));
    }
    let r = CLIENT.post(url).form(&form).send().await.context("contacting Microsoft")?;
    let status = r.status();
    let v: Value = r.json().await.context("reading Microsoft's reply")?;
    if !status.is_success() {
        bail!("Microsoft refused the sign-in request: {}", v.get("error_description").or(v.get("error")).and_then(|x| x.as_str()).unwrap_or("unknown error"));
    }
    Ok(DeviceCode {
        user_code: v["user_code"].as_str().unwrap_or_default().into(),
        device_code: v["device_code"].as_str().unwrap_or_default().into(),
        verification_uri: v["verification_uri"].as_str().unwrap_or("https://www.microsoft.com/link").into(),
        expires_in: v["expires_in"].as_u64().unwrap_or(900),
        interval: v["interval"].as_u64().unwrap_or(5),
        flow: flow.into(),
        client_id: client_id.into(),
    })
}

fn token_url(flow: &str) -> &'static str {
    if flow == "live" { "https://login.live.com/oauth20_token.srf" } else { "https://login.microsoftonline.com/consumers/oauth2/v2.0/token" }
}

pub enum Poll {
    Pending,
    SlowDown,
    Done { access: String, refresh: String },
}

pub async fn poll_device_code(dc: &DeviceCode) -> Result<Poll> {
    let r = CLIENT
        .post(token_url(&dc.flow))
        .form(&[
            ("client_id", dc.client_id.as_str()),
            ("device_code", dc.device_code.as_str()),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ])
        .send()
        .await?;
    let v: Value = r.json().await?;
    if let Some(at) = v.get("access_token").and_then(|x| x.as_str()) {
        return Ok(Poll::Done { access: at.into(), refresh: v["refresh_token"].as_str().unwrap_or_default().into() });
    }
    match v.get("error").and_then(|x| x.as_str()) {
        Some("authorization_pending") => Ok(Poll::Pending),
        Some("slow_down") => Ok(Poll::SlowDown),
        Some("expired_token") => bail!("The code expired before sign-in finished. Start again."),
        Some("authorization_declined") | Some("access_denied") => bail!("Sign-in was cancelled in the browser."),
        Some(e) => bail!("Microsoft sign-in failed: {}", v.get("error_description").and_then(|x| x.as_str()).unwrap_or(e)),
        None => bail!("Unexpected reply from Microsoft"),
    }
}

async fn refresh_ms(flow: &str, client_id: &str, refresh: &str) -> Result<(String, String)> {
    let scope = if flow == "live" { LIVE_SCOPE } else { MSAL_SCOPE };
    let r = CLIENT
        .post(token_url(flow))
        .form(&[("client_id", client_id), ("refresh_token", refresh), ("grant_type", "refresh_token"), ("scope", scope)])
        .send()
        .await?;
    let v: Value = r.json().await?;
    let at = v.get("access_token").and_then(|x| x.as_str()).ok_or_else(|| anyhow!("Your Microsoft session expired. Sign in again."))?;
    Ok((at.into(), v["refresh_token"].as_str().unwrap_or(refresh).into()))
}

fn xsts_error(code: u64) -> &'static str {
    match code {
        2148916233 => "This Microsoft account has no Xbox profile. Sign in once at minecraft.net to create one.",
        2148916235 => "Xbox Live is not available in this account's country.",
        2148916236 | 2148916237 => "This account needs adult verification on xbox.com before it can play.",
        2148916238 => "This is a child account. An adult must add it to a Microsoft family group first.",
        _ => "Xbox Live rejected the sign-in.",
    }
}

/// Microsoft token -> Xbox Live -> XSTS -> Minecraft services, then checks ownership and profile.
pub async fn minecraft_login(ms_access: &str, flow: &str) -> Result<(String, i64, Value)> {
    let ticket = if flow == "live" { format!("t={ms_access}") } else { format!("d={ms_access}") };
    let xbl: Value = CLIENT
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .header("Accept", "application/json")
        .json(&json!({
            "Properties": {"AuthMethod": "RPS", "SiteName": "user.auth.xboxlive.com", "RpsTicket": ticket},
            "RelyingParty": "http://auth.xboxlive.com", "TokenType": "JWT"
        }))
        .send()
        .await?
        .error_for_status()
        .context("Xbox Live sign-in failed")?
        .json()
        .await?;
    let xbl_token = xbl["Token"].as_str().ok_or_else(|| anyhow!("Xbox Live returned no token"))?;
    let uhs = xbl.pointer("/DisplayClaims/xui/0/uhs").and_then(|x| x.as_str()).unwrap_or_default().to_string();

    let r = CLIENT
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .header("Accept", "application/json")
        .json(&json!({
            "Properties": {"SandboxId": "RETAIL", "UserTokens": [xbl_token]},
            "RelyingParty": "rp://api.minecraftservices.com/", "TokenType": "JWT"
        }))
        .send()
        .await?;
    let status = r.status();
    let xsts: Value = r.json().await.unwrap_or(Value::Null);
    if !status.is_success() {
        bail!("{}", xsts_error(xsts["XErr"].as_u64().unwrap_or(0)));
    }
    let xsts_token = xsts["Token"].as_str().ok_or_else(|| anyhow!("XSTS returned no token"))?;

    let r = CLIENT
        .post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .json(&json!({"identityToken": format!("XBL3.0 x={uhs};{xsts_token}")}))
        .send()
        .await?;
    let status = r.status();
    let mc: Value = r.json().await.unwrap_or(Value::Null);
    if !status.is_success() {
        let msg = mc["errorMessage"].as_str().or(mc["error"].as_str()).unwrap_or("");
        if status.as_u16() == 403 || msg.contains("Invalid app registration") {
            bail!("Minecraft services refused this client ID. Azure apps need Mojang's approval before they can sign in to Minecraft ({msg}).");
        }
        bail!("Minecraft sign-in failed ({status}) {msg}");
    }
    let token = mc["access_token"].as_str().ok_or_else(|| anyhow!("Minecraft returned no token"))?.to_string();
    let expires = util::now_ms() + mc["expires_in"].as_i64().unwrap_or(86400) * 1000;

    let profile_resp = CLIENT.get("https://api.minecraftservices.com/minecraft/profile").bearer_auth(&token).send().await?;
    if profile_resp.status().as_u16() == 404 {
        let ent: Value = CLIENT
            .get("https://api.minecraftservices.com/entitlements/mcstore")
            .bearer_auth(&token)
            .send()
            .await?
            .json()
            .await
            .unwrap_or(Value::Null);
        let owns = ent["items"].as_array().map(|a| !a.is_empty()).unwrap_or(false);
        if owns {
            bail!("This account owns Minecraft but has no profile yet. Pick a username at minecraft.net, then sign in again.");
        }
        bail!("This Microsoft account does not own Minecraft: Java Edition.");
    }
    let profile: Value = profile_resp.error_for_status()?.json().await?;
    Ok((token, expires, profile))
}

fn dashed(uuid: &str) -> String {
    if uuid.len() == 32 {
        format!("{}-{}-{}-{}-{}", &uuid[0..8], &uuid[8..12], &uuid[12..16], &uuid[16..20], &uuid[20..32])
    } else {
        uuid.to_string()
    }
}

fn cosmetics_from_profile(profile: &Value) -> (Option<String>, Option<String>, String, Vec<CapeOption>) {
    let skin = profile["skins"].as_array().and_then(|s| s.iter().find(|x| x["state"] == "ACTIVE").or(s.first()));
    let skin_url = skin.and_then(|s| s["url"].as_str()).map(String::from);
    let model = skin
        .and_then(|s| s["variant"].as_str())
        .map(|v| if v.eq_ignore_ascii_case("SLIM") { "slim" } else { "classic" })
        .unwrap_or("classic")
        .to_string();
    let capes: Vec<CapeOption> = profile["capes"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|c| {
                    Some(CapeOption {
                        id: c["id"].as_str()?.into(),
                        alias: c["alias"].as_str().unwrap_or("Cape").into(),
                        url: c["url"].as_str()?.into(),
                        active: c["state"] == "ACTIVE",
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let cape_url = capes.iter().find(|c| c.active).map(|c| c.url.clone());
    (skin_url, cape_url, model, capes)
}

pub fn account_from_login(flow: &str, client_id: &str, refresh: String, token: String, expires: i64, profile: &Value) -> Account {
    let (skin_url, cape_url, skin_model, _) = cosmetics_from_profile(profile);
    let mut acc = Account {
        id: uuid::Uuid::new_v4().to_string(),
        kind: "microsoft".into(),
        username: profile["name"].as_str().unwrap_or("Player").into(),
        uuid: dashed(profile["id"].as_str().unwrap_or_default()),
        skin_url: skin_url.clone(),
        cape_url,
        skin_model: skin_model.clone(),
        skin_history: Vec::new(),
        added: util::now_ms(),
        ms_flow: flow.into(),
        client_id: client_id.into(),
        refresh_token: refresh,
        mc_token: token,
        mc_expires: expires,
    };
    if let Some(url) = &skin_url {
        remember_skin(&mut acc, url, &skin_model);
    }
    acc
}

fn apply_profile(acc: &mut Account, profile: &Value) {
    if let Some(n) = profile["name"].as_str() {
        acc.username = n.into();
    }
    if let Some(id) = profile["id"].as_str() {
        acc.uuid = dashed(id);
    }
    let (skin_url, cape_url, skin_model, _) = cosmetics_from_profile(profile);
    // Remember every skin Mojang still lists (usually just ACTIVE), plus the active one.
    if let Some(arr) = profile["skins"].as_array() {
        for s in arr {
            let Some(url) = s["url"].as_str() else { continue };
            let model = s["variant"]
                .as_str()
                .map(|v| if v.eq_ignore_ascii_case("SLIM") { "slim" } else { "classic" })
                .unwrap_or("classic");
            remember_skin(acc, url, model);
        }
    }
    if let Some(url) = &skin_url {
        remember_skin(acc, url, &skin_model);
    }
    acc.skin_url = skin_url;
    acc.cape_url = cape_url;
    acc.skin_model = skin_model;
}

async fn fetch_profile(acc: &Account) -> Result<Value> {
    let profile_resp = CLIENT
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(&acc.mc_token)
        .send()
        .await
        .context("loading Minecraft profile")?;
    if !profile_resp.status().is_success() {
        bail!("Could not load this account's Minecraft profile. Try signing in again.");
    }
    Ok(profile_resp.json().await?)
}

/// Returns a Minecraft token for the account, refreshing the whole chain when it is near expiry.
pub async fn fresh_token(acc: &mut Account) -> Result<()> {
    if acc.kind != "microsoft" {
        return Ok(());
    }
    if !acc.mc_token.is_empty() && acc.mc_expires - util::now_ms() > 5 * 60 * 1000 {
        return Ok(());
    }
    if acc.refresh_token.is_empty() {
        bail!("Sign in to {} again from Accounts.", acc.username);
    }
    let (access, refresh) = refresh_ms(&acc.ms_flow, &acc.client_id, &acc.refresh_token).await?;
    let (token, expires, profile) = minecraft_login(&access, &acc.ms_flow).await?;
    acc.refresh_token = refresh;
    acc.mc_token = token;
    acc.mc_expires = expires;
    apply_profile(acc, &profile);
    Ok(())
}

pub async fn load_cosmetics(file: &mut AccountsFile, paths: &Paths, id: &str) -> Result<AccountCosmetics> {
    let idx = file.accounts.iter().position(|a| a.id == id).ok_or_else(|| anyhow!("Account not found"))?;
    if file.accounts[idx].kind != "microsoft" {
        bail!("Only Microsoft accounts have skins and capes.");
    }
    fresh_token(&mut file.accounts[idx]).await?;
    let profile = fetch_profile(&file.accounts[idx]).await?;
    apply_profile(&mut file.accounts[idx], &profile);
    let (_, _, _, capes) = cosmetics_from_profile(&profile);
    file.save(paths)?;
    Ok(cosmetics_response(file, idx, capes))
}

pub async fn set_active_cape(file: &mut AccountsFile, paths: &Paths, id: &str, cape_id: Option<&str>) -> Result<AccountCosmetics> {
    let idx = file.accounts.iter().position(|a| a.id == id).ok_or_else(|| anyhow!("Account not found"))?;
    if file.accounts[idx].kind != "microsoft" {
        bail!("Only Microsoft accounts have capes.");
    }
    fresh_token(&mut file.accounts[idx]).await?;
    let token = file.accounts[idx].mc_token.clone();
    let resp = if let Some(cape_id) = cape_id {
        CLIENT
            .put("https://api.minecraftservices.com/minecraft/profile/capes/active")
            .bearer_auth(&token)
            .json(&json!({ "capeId": cape_id }))
            .send()
            .await
            .context("equipping cape")?
    } else {
        CLIENT
            .delete("https://api.minecraftservices.com/minecraft/profile/capes/active")
            .bearer_auth(&token)
            .send()
            .await
            .context("removing cape")?
    };
    if !resp.status().is_success() {
        let body: Value = resp.json().await.unwrap_or(Value::Null);
        let msg = body["errorMessage"].as_str().or(body["error"].as_str()).unwrap_or("Could not change cape.");
        bail!("{msg}");
    }
    let profile = fetch_profile(&file.accounts[idx]).await?;
    apply_profile(&mut file.accounts[idx], &profile);
    let (_, _, _, capes) = cosmetics_from_profile(&profile);
    file.save(paths)?;
    Ok(cosmetics_response(file, idx, capes))
}

async fn upload_skin_bytes(file: &mut AccountsFile, paths: &Paths, idx: usize, bytes: Vec<u8>, variant: &str, file_name: &str) -> Result<AccountCosmetics> {
    if bytes.len() < 8 || &bytes[0..8] != b"\x89PNG\r\n\x1a\n" {
        bail!("Skin must be a PNG file.");
    }
    if bytes.len() > 2 * 1024 * 1024 {
        bail!("Skin file is too large (max 2 MB).");
    }
    fresh_token(&mut file.accounts[idx]).await?;
    let token = file.accounts[idx].mc_token.clone();
    let part = reqwest::multipart::Part::bytes(bytes)
        .file_name(file_name.to_string())
        .mime_str("image/png")
        .context("skin mime")?;
    let form = reqwest::multipart::Form::new().text("variant", variant.to_string()).part("file", part);
    let resp = CLIENT
        .post("https://api.minecraftservices.com/minecraft/profile/skins")
        .bearer_auth(&token)
        .multipart(form)
        .send()
        .await
        .context("uploading skin")?;
    if !resp.status().is_success() {
        let body: Value = resp.json().await.unwrap_or(Value::Null);
        let msg = body["errorMessage"].as_str().or(body["error"].as_str()).unwrap_or("Could not change skin.");
        bail!("{msg}");
    }
    let profile = fetch_profile(&file.accounts[idx]).await?;
    apply_profile(&mut file.accounts[idx], &profile);
    let (_, _, _, capes) = cosmetics_from_profile(&profile);
    file.save(paths)?;
    Ok(cosmetics_response(file, idx, capes))
}

/// Upload a local PNG as the account skin (`classic` or `slim` arms).
pub async fn set_account_skin(file: &mut AccountsFile, paths: &Paths, id: &str, path: &str, model: &str) -> Result<AccountCosmetics> {
    let idx = file.accounts.iter().position(|a| a.id == id).ok_or_else(|| anyhow!("Account not found"))?;
    if file.accounts[idx].kind != "microsoft" {
        bail!("Only Microsoft accounts have skins.");
    }
    let variant = match model.trim().to_ascii_lowercase().as_str() {
        "slim" => "slim",
        "classic" | "" => "classic",
        other => bail!("Unknown skin model '{other}'. Use classic or slim."),
    };
    let bytes = std::fs::read(path).with_context(|| format!("reading skin {path}"))?;
    let file_name = std::path::Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("skin.png")
        .to_string();
    upload_skin_bytes(file, paths, idx, bytes, variant, &file_name).await
}

/// Re-equip a previously seen skin from local history (downloads the texture URL and re-uploads).
pub async fn set_account_skin_from_history(file: &mut AccountsFile, paths: &Paths, id: &str, skin_id: &str) -> Result<AccountCosmetics> {
    let idx = file.accounts.iter().position(|a| a.id == id).ok_or_else(|| anyhow!("Account not found"))?;
    if file.accounts[idx].kind != "microsoft" {
        bail!("Only Microsoft accounts have skins.");
    }
    let entry = file.accounts[idx]
        .skin_history
        .iter()
        .find(|h| h.id == skin_id)
        .cloned()
        .ok_or_else(|| anyhow!("That past skin is not in history anymore."))?;
    if file.accounts[idx].skin_url.as_deref() == Some(entry.url.as_str()) {
        fresh_token(&mut file.accounts[idx]).await?;
        let profile = fetch_profile(&file.accounts[idx]).await?;
        apply_profile(&mut file.accounts[idx], &profile);
        let (_, _, _, capes) = cosmetics_from_profile(&profile);
        file.save(paths)?;
        return Ok(cosmetics_response(file, idx, capes));
    }
    let cache = paths.cache.join("skins").join(format!("{}.png", entry.id));
    let bytes = if cache.is_file() {
        std::fs::read(&cache).with_context(|| format!("reading cached skin {}", cache.display()))?
    } else {
        let resp = CLIENT
            .get(&entry.url)
            .send()
            .await
            .context("downloading past skin")?;
        if !resp.status().is_success() {
            bail!("Could not download that past skin. It may no longer be available.");
        }
        let bytes = resp.bytes().await.context("reading past skin")?.to_vec();
        if let Some(parent) = cache.parent() {
            let _ = std::fs::create_dir_all(parent);
            let _ = std::fs::write(&cache, &bytes);
        }
        bytes
    };
    let variant = if entry.model.eq_ignore_ascii_case("slim") { "slim" } else { "classic" };
    upload_skin_bytes(file, paths, idx, bytes, variant, "skin.png").await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_without_microsoft() {
        let mut f = AccountsFile::default();
        let a = add_offline(&mut f, "Steve").unwrap();
        assert_eq!(a.uuid, crate::mc::launch::offline_uuid("Steve"));
        assert_eq!(f.active.as_deref(), Some(a.id.as_str()));
        assert!(add_offline(&mut f, "no spaces").is_err());
        assert!(add_offline(&mut f, "ab").is_err());
        f.upsert(Account { id: "m".into(), kind: "microsoft".into(), uuid: "x".into(), username: "Real".into(), ..Default::default() });
        f.remove("m");
        assert_eq!(f.accounts.len(), 1);
        assert_eq!(f.accounts[0].kind, "offline");
    }

    #[test]
    fn multiple_accounts_and_switching() {
        let mut f = AccountsFile::default();
        f.upsert(Account { id: "a".into(), kind: "microsoft".into(), uuid: "1".into(), ..Default::default() });
        f.upsert(Account { id: "b".into(), kind: "microsoft".into(), uuid: "2".into(), ..Default::default() });
        assert_eq!(f.accounts.len(), 2);
        assert_eq!(f.active.as_deref(), Some("b"));
        // Signing in again with the same profile updates the entry instead of duplicating it.
        f.upsert(Account { id: "c".into(), kind: "microsoft".into(), uuid: "1".into(), username: "new".into(), ..Default::default() });
        assert_eq!(f.accounts.len(), 2);
        assert_eq!(f.active.as_deref(), Some("a"));
        f.remove("a");
        assert_eq!(f.active.as_deref(), Some("b"));
    }

    #[test]
    fn flow_detection() {
        assert_eq!(flow_for("00000000402b5328"), "live");
        assert_eq!(flow_for("c36a9fb6-4f2a-41ff-90bd-ae7cc92031eb"), "msal");
    }
}
