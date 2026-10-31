//! Optional playit.gg tunnel so friends outside the network can join without port forwarding.
//! Everything is started by the player: Cairn downloads the official agent (checked against a
//! pinned SHA-256), links it through playit's claim page, and runs it only while asked to.

use crate::net::{self, Download};
use crate::paths::Paths;
use crate::tasks::Task;
use crate::util::LockExt;
use anyhow::{anyhow, bail, Context, Result};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};

pub const VERSION: &str = "1.2.0";
const API: &str = "https://api.playit.gg";
const CLAIM_TIMEOUT: Duration = Duration::from_secs(15 * 60);

struct Asset {
    file: &'static str,
    sha256: &'static str,
}

/// The daemon build for this platform from the playit-agent v1.2.0 release. playit ships no macOS agent.
fn asset() -> Option<Asset> {
    if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some(Asset { file: "playit-linux-amd64", sha256: "602dea3288d5bad4c3addba70ecf6535b9ae085b8d4f2fc2fe9cde913bda78d0" })
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        Some(Asset { file: "playit-linux-aarch64", sha256: "3a6d69c348803c7041c49312345c18b1e9cb7dd7fc2adfbf2217e50d0d7c9a34" })
    } else if cfg!(all(windows, target_arch = "x86_64")) {
        Some(Asset { file: "playitd-windows-x86_64-signed.exe", sha256: "e8dff30a9269a10d2c6786c95a0618567772e441d07dd7ebe3c1a37d3f35534d" })
    } else {
        None
    }
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub supported: bool,
    pub installed: bool,
    pub linked: bool,
    pub running: bool,
    pub claim_url: Option<String>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Tunnel {
    pub name: String,
    pub address: String,
    pub kind: String,
    pub problem: Option<String>,
}

struct Agent {
    kill: tokio::sync::oneshot::Sender<()>,
}

static AGENT: Lazy<Mutex<Option<Agent>>> = Lazy::new(Default::default);
static CLAIM: Lazy<Mutex<Option<String>>> = Lazy::new(Default::default);

fn dir(paths: &Paths) -> PathBuf {
    paths.root.join("playit")
}

fn bin(paths: &Paths) -> PathBuf {
    dir(paths).join(if cfg!(windows) { "playitd.exe" } else { "playitd" })
}

fn secret_file(paths: &Paths) -> PathBuf {
    dir(paths).join("secret.key")
}

fn claim_url(code: &str) -> String {
    format!("https://playit.gg/claim/{code}")
}

pub fn status(paths: &Paths) -> Status {
    Status {
        supported: asset().is_some(),
        installed: bin(paths).exists(),
        linked: read_secret(paths).is_some(),
        running: AGENT.locked().is_some(),
        claim_url: CLAIM.locked().as_deref().map(claim_url),
    }
}

/// The secret file holds the bare key, or `secret_key = "..."` as written by playit's own tools.
pub fn parse_secret(text: &str) -> Option<String> {
    let t = text.trim();
    let key = match t.split_once('=') {
        Some((k, v)) if k.trim() == "secret_key" => v.trim().trim_matches('"').to_string(),
        Some(_) => return None,
        None => t.to_string(),
    };
    (!key.is_empty() && key.chars().all(|c| c.is_ascii_hexdigit())).then_some(key)
}

fn read_secret(paths: &Paths) -> Option<String> {
    parse_secret(&std::fs::read_to_string(secret_file(paths)).ok()?)
}

fn write_secret(path: &Path, key: &str) -> Result<()> {
    std::fs::create_dir_all(path.parent().unwrap_or(Path::new(".")))?;
    std::fs::write(path, key)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

/// Downloads the agent if it isn't there yet, verified against the pinned SHA-256.
pub async fn install(paths: &Paths, task: &Task) -> Result<()> {
    let a = asset().ok_or_else(|| anyhow!("playit.gg has no agent for this system"))?;
    let dest = bin(paths);
    task.stage("Downloading the playit.gg agent", 0, 0);
    net::download(&Download {
        url: format!("https://github.com/playit-cloud/playit-agent/releases/download/v{VERSION}/{}", a.file),
        dest: dest.clone(),
        sha1: None,
        sha256: Some(a.sha256.to_string()),
        size: None,
    })
    .await
    .context("downloading the playit.gg agent")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}

#[derive(Deserialize)]
struct Reply {
    status: String,
    #[serde(default)]
    data: Value,
}

async fn call(path: &str, body: Value, key: Option<&str>) -> Result<Value> {
    let mut req = net::CLIENT.post(format!("{API}{path}")).json(&body);
    if let Some(k) = key {
        req = req.header("Authorization", format!("Agent-Key {k}"));
    }
    let r: Reply = req.send().await?.json().await.context("reading playit.gg's reply")?;
    if r.status != "success" {
        bail!("playit.gg said {}", r.data.as_str().map(str::to_string).unwrap_or_else(|| r.data.to_string()));
    }
    Ok(r.data)
}

/// Starts linking: returns the page the player opens to approve this computer.
pub fn begin_claim() -> String {
    let code = hex::encode(&uuid::Uuid::new_v4().as_bytes()[..5]);
    let url = claim_url(&code);
    *CLAIM.locked() = Some(code);
    url
}

pub fn cancel_claim() {
    *CLAIM.locked() = None;
}

/// Waits for the player to approve the claim on playit.gg, then stores the agent key.
pub async fn finish_claim(paths: &Paths) -> Result<()> {
    let code = CLAIM.locked().clone().ok_or_else(|| anyhow!("Start linking first."))?;
    let body = json!({ "code": code, "agent_type": "self-managed", "version": format!("Cairn {}", env!("CARGO_PKG_VERSION")) });
    let started = std::time::Instant::now();
    loop {
        if CLAIM.locked().as_deref() != Some(code.as_str()) {
            bail!("Linking was cancelled.");
        }
        if started.elapsed() > CLAIM_TIMEOUT {
            cancel_claim();
            bail!("Linking timed out. Try again and approve it on playit.gg.");
        }
        match call("/claim/setup", body.clone(), None).await?.as_str() {
            Some("UserAccepted") => break,
            Some("UserRejected") => {
                cancel_claim();
                bail!("The link was rejected on playit.gg.");
            }
            _ => tokio::time::sleep(Duration::from_secs(2)).await,
        }
    }
    let mut last = None;
    for _ in 0..30 {
        match call("/claim/exchange", json!({ "code": code }), None).await {
            Ok(d) => {
                let key = d["secret_key"].as_str().and_then(parse_secret).ok_or_else(|| anyhow!("playit.gg sent no agent key"))?;
                write_secret(&secret_file(paths), &key)?;
                cancel_claim();
                return Ok(());
            }
            Err(e) => last = Some(e),
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    cancel_claim();
    Err(last.unwrap_or_else(|| anyhow!("playit.gg never handed over the agent key")))
}

pub fn parse_tunnels(data: &Value) -> Vec<Tunnel> {
    data["tunnels"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|t| Tunnel {
                    name: t["name"].as_str().unwrap_or("Tunnel").to_string(),
                    address: t["display_address"].as_str().unwrap_or_default().to_string(),
                    kind: t["tunnel_type_display"].as_str().unwrap_or_default().to_string(),
                    problem: t["disabled_reason"].as_str().map(str::to_string),
                })
                .collect()
        })
        .unwrap_or_default()
}

pub async fn tunnels(paths: &Paths) -> Result<Vec<Tunnel>> {
    let key = read_secret(paths).ok_or_else(|| anyhow!("Link playit.gg first."))?;
    Ok(parse_tunnels(&call("/v1/agents/rundata", json!({}), Some(&key)).await?))
}

pub fn start(app: &AppHandle, paths: &Paths) -> Result<()> {
    if AGENT.locked().is_some() {
        return Ok(());
    }
    let exe = bin(paths);
    if !exe.exists() {
        bail!("Download the playit.gg agent first.");
    }
    if read_secret(paths).is_none() {
        bail!("Link playit.gg first.");
    }
    let d = dir(paths);
    let socket = if cfg!(windows) { r"\\.\pipe\cairn-playitd".to_string() } else { d.join("playitd.sock").to_string_lossy().into_owned() };
    let mut cmd = tokio::process::Command::new(&exe);
    cmd.arg("--secret-path")
        .arg(secret_file(paths))
        .arg("--socket-path")
        .arg(socket)
        .arg("-l")
        .arg(d.join("playitd.log"))
        .current_dir(&d)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000);
    let mut child = cmd.spawn().context("starting the playit.gg agent")?;
    let (kill_tx, kill_rx) = tokio::sync::oneshot::channel();
    *AGENT.locked() = Some(Agent { kill: kill_tx });
    let _ = app.emit("playit-state", true);
    for out in
        [child.stdout.take().map(|o| Box::new(o) as Box<dyn tokio::io::AsyncRead + Send + Unpin>), child.stderr.take().map(|o| Box::new(o) as _)].into_iter().flatten()
    {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            let mut lines = BufReader::new(out).lines();
            while let Ok(Some(l)) = lines.next_line().await {
                let _ = app.emit("playit-log", l);
            }
        });
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::select! {
            _ = child.wait() => {}
            _ = kill_rx => { let _ = child.kill().await; }
        }
        *AGENT.locked() = None;
        let _ = app.emit("playit-state", false);
    });
    Ok(())
}

pub fn stop() {
    if let Some(a) = AGENT.locked().take() {
        let _ = a.kill.send(());
    }
}

/// Stops the agent and forgets the key. The agent stays listed on playit.gg until removed there.
pub fn unlink(paths: &Paths) -> Result<()> {
    stop();
    cancel_claim();
    match std::fs::remove_file(secret_file(paths)) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.into()),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_parses_bare_and_toml_forms() {
        assert_eq!(parse_secret("ab12CD\n").as_deref(), Some("ab12CD"));
        assert_eq!(parse_secret("secret_key = \"00ff\"\n").as_deref(), Some("00ff"));
        assert_eq!(parse_secret("other = \"00ff\""), None);
        assert_eq!(parse_secret("not hex"), None);
        assert_eq!(parse_secret(""), None);
    }

    #[test]
    fn tunnels_read_from_rundata() {
        let data = json!({ "tunnels": [
            { "name": "mc", "display_address": "fun-cat.joinmc.link", "tunnel_type_display": "Minecraft Java", "disabled_reason": null },
            { "name": "old", "display_address": "x.ply.gg:1", "tunnel_type_display": "TCP", "disabled_reason": "BySystem" }
        ]});
        let t = parse_tunnels(&data);
        assert_eq!(t[0], Tunnel { name: "mc".into(), address: "fun-cat.joinmc.link".into(), kind: "Minecraft Java".into(), problem: None });
        assert_eq!(t[1].problem.as_deref(), Some("BySystem"));
        assert!(parse_tunnels(&json!({})).is_empty());
    }

    #[test]
    fn pinned_assets_are_sha256_hex() {
        if let Some(a) = asset() {
            assert_eq!(a.sha256.len(), 64);
            assert!(a.sha256.chars().all(|c| c.is_ascii_hexdigit()));
        }
    }

    #[test]
    fn claim_codes_are_ten_hex_chars() {
        let url = begin_claim();
        let code = url.rsplit('/').next().unwrap();
        assert_eq!(code.len(), 10);
        assert!(code.chars().all(|c| c.is_ascii_hexdigit()));
        cancel_claim();
    }
}
