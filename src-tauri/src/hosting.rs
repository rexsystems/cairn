//! Runs a dedicated server built from an instance. Server files live in `servers/<instance id>/`,
//! apart from the instance, so the game and the server never share a world or config by accident.
//! Nothing here opens a router port or touches the firewall: the server binds to 127.0.0.1 unless
//! the player turns on LAN access, and the EULA is only written after the player accepts it.

use crate::instance::Instance;
use crate::mc::version;
use crate::modmeta::{self, JarMod, Side};
use crate::net::{self, Download};
use crate::paths::Paths;
use crate::settings::Settings;
use crate::tasks::Task;
use crate::util::{self, LockExt};
use crate::{java, loaders};
use anyhow::{anyhow, bail, Context, Result};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

const LOG_KEEP: usize = 3000;
const STOP_GRACE: Duration = Duration::from_secs(45);
const EULA_URL: &str = "https://aka.ms/MinecraftEULA";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct ServerConfig {
    pub port: u16,
    pub memory_mb: u32,
    pub max_players: u32,
    pub online_mode: bool,
    pub motd: String,
    /// Off: the server only listens on 127.0.0.1 (this computer and a playit tunnel).
    pub lan: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self { port: 25565, memory_mb: 2048, max_players: 8, online_mode: true, motd: "A Cairn server".into(), lan: false }
    }
}

/// How to start the installed server, relative to its folder.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Launch {
    /// `java -jar <file> nogui`
    Jar(String),
    /// `java @user_jvm_args.txt @<file> nogui` (modern Forge and NeoForge)
    Args(String),
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
struct Meta {
    config: ServerConfig,
    launch: Option<Launch>,
    /// `mc|loader|loader version` the server files were installed for.
    built_for: Option<String>,
    java: Option<String>,
    /// Mod jars Cairn copied in, so it can remove them later without touching jars the player added.
    synced: Vec<String>,
    client_only: Vec<String>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    pub dir: String,
    pub config: ServerConfig,
    pub prepared: bool,
    pub outdated: bool,
    pub eula: bool,
    pub state: String,
    pub lan_address: Option<String>,
    pub client_only: Vec<String>,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SyncReport {
    pub copied: usize,
    pub removed: usize,
    pub client_only: Vec<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct StateEvent<'a> {
    instance_id: &'a str,
    state: &'a str,
    exit_code: Option<i32>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct LogEvent<'a> {
    instance_id: &'a str,
    lines: &'a [String],
}

#[derive(Default)]
struct Proc {
    state: String,
    stdin: Option<Arc<tokio::sync::Mutex<tokio::process::ChildStdin>>>,
    kill: Option<tokio::sync::oneshot::Sender<()>>,
    log: VecDeque<String>,
}

static PROCS: Lazy<Mutex<HashMap<String, Proc>>> = Lazy::new(Default::default);

pub fn server_dir(paths: &Paths, id: &str) -> PathBuf {
    paths.root.join("servers").join(id)
}

fn meta_path(dir: &Path) -> PathBuf {
    dir.join(".cairn").join("server.json")
}

fn load_meta(dir: &Path) -> Meta {
    util::read_json(&meta_path(dir)).unwrap_or_default()
}

fn save_meta(dir: &Path, meta: &Meta) -> Result<()> {
    util::write_json_atomic(&meta_path(dir), meta)
}

fn build_key(inst: &Instance) -> String {
    format!("{}|{}|{}", inst.mc_version, inst.loader, inst.loader_version.as_deref().unwrap_or(""))
}

fn state_of(id: &str) -> String {
    PROCS.locked().get(id).map(|p| p.state.clone()).filter(|s| !s.is_empty()).unwrap_or_else(|| "stopped".into())
}

pub fn is_running(id: &str) -> bool {
    state_of(id) != "stopped"
}

pub fn info(paths: &Paths, inst: &Instance) -> ServerInfo {
    let dir = server_dir(paths, &inst.id);
    let meta = load_meta(&dir);
    let prepared = meta.launch.as_ref().is_some_and(|l| launch_file(&dir, l).exists());
    ServerInfo {
        dir: dir.to_string_lossy().into_owned(),
        prepared,
        outdated: prepared && meta.built_for.as_deref() != Some(build_key(inst).as_str()),
        eula: eula_accepted(&dir),
        state: state_of(&inst.id),
        lan_address: lan_address(),
        client_only: meta.client_only,
        config: meta.config,
    }
}

fn launch_file(dir: &Path, l: &Launch) -> PathBuf {
    match l {
        Launch::Jar(f) | Launch::Args(f) => dir.join(f),
    }
}

/// The address other devices on the network would use. Connecting a UDP socket sends no packets;
/// it only asks the OS which interface would route outward.
fn lan_address() -> Option<String> {
    let s = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    s.connect("192.0.2.1:9").ok()?;
    let ip = s.local_addr().ok()?.ip();
    (!ip.is_loopback() && !ip.is_unspecified()).then(|| ip.to_string())
}

// ---------------------------------------------------------------- EULA

pub fn eula_accepted(dir: &Path) -> bool {
    std::fs::read_to_string(dir.join("eula.txt")).map(|t| t.lines().any(|l| l.trim().eq_ignore_ascii_case("eula=true"))).unwrap_or(false)
}

/// Only called from the player's explicit "I accept" action in the Server tab.
pub fn accept_eula(dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    let when = chrono::Utc::now().to_rfc3339();
    std::fs::write(dir.join("eula.txt"), format!("# Accepted in Cairn by the player on {when} after reading {EULA_URL}\neula=true\n"))?;
    Ok(())
}

// ---------------------------------------------------------------- server.properties

fn escape_property(v: &str) -> String {
    let mut out = String::new();
    for c in v.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' | '\r' => out.push(' '),
            c if c.is_ascii() => out.push(c),
            c => {
                let mut buf = [0u16; 2];
                for u in c.encode_utf16(&mut buf) {
                    out.push_str(&format!("\\u{u:04x}"));
                }
            }
        }
    }
    out
}

/// Sets the keys Cairn manages and keeps every other line as the player left it.
pub fn apply_properties(existing: &str, cfg: &ServerConfig) -> String {
    let wanted: Vec<(&str, String)> = vec![
        ("server-port", cfg.port.to_string()),
        ("server-ip", if cfg.lan { String::new() } else { "127.0.0.1".into() }),
        ("max-players", cfg.max_players.to_string()),
        ("online-mode", cfg.online_mode.to_string()),
        ("motd", escape_property(&cfg.motd)),
        ("enable-rcon", "false".into()),
    ];
    let mut seen = HashSet::new();
    let mut out: Vec<String> = existing
        .lines()
        .map(|line| {
            let key = line.split_once('=').map(|(k, _)| k.trim());
            match key.and_then(|k| wanted.iter().find(|(w, _)| *w == k)) {
                Some((k, v)) if !line.trim_start().starts_with('#') => {
                    seen.insert(*k);
                    format!("{k}={v}")
                }
                _ => line.to_string(),
            }
        })
        .collect();
    for (k, v) in &wanted {
        if !seen.contains(k) {
            out.push(format!("{k}={v}"));
        }
    }
    out.join("\n") + "\n"
}

fn write_properties(dir: &Path, cfg: &ServerConfig) -> Result<()> {
    let path = dir.join("server.properties");
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    std::fs::write(&path, apply_properties(&existing, cfg))?;
    Ok(())
}

pub fn save_config(paths: &Paths, id: &str, cfg: ServerConfig) -> Result<ServerConfig> {
    if cfg.port < 1024 {
        bail!("Pick a port from 1024 to 65535.");
    }
    if !(512..=65536).contains(&cfg.memory_mb) {
        bail!("Memory must be between 512 MB and 64 GB.");
    }
    let dir = server_dir(paths, id);
    std::fs::create_dir_all(&dir)?;
    let mut meta = load_meta(&dir);
    meta.config = cfg;
    save_meta(&dir, &meta)?;
    write_properties(&dir, &meta.config)?;
    Ok(meta.config)
}

// ---------------------------------------------------------------- mods

/// Splits enabled mods into the ones a server can load and the client-only ones it must skip.
pub fn plan_mods(mods: &[JarMod]) -> (Vec<String>, Vec<String>) {
    let mut copy = vec![];
    let mut client = vec![];
    for m in mods.iter().filter(|m| m.enabled) {
        match &m.info {
            Some(i) if i.side == Side::Client => client.push(if i.name.is_empty() { m.file.clone() } else { i.name.clone() }),
            _ => copy.push(m.file.clone()),
        }
    }
    client.sort_by_key(|n| n.to_lowercase());
    (copy, client)
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (std::fs::metadata(a), std::fs::metadata(b)) {
        (Ok(x), Ok(y)) => x.len() == y.len() && x.modified().ok() <= y.modified().ok(),
        _ => false,
    }
}

/// Copies files that exist in `from` but not in `to`, so the player's server-side edits survive.
fn copy_missing(from: &Path, to: &Path) -> Result<()> {
    let Ok(rd) = std::fs::read_dir(from) else {
        return Ok(());
    };
    for e in rd.flatten() {
        let (src, dst) = (e.path(), to.join(e.file_name()));
        if src.is_dir() {
            copy_missing(&src, &dst)?;
        } else if !dst.exists() {
            std::fs::create_dir_all(to)?;
            std::fs::copy(&src, &dst)?;
        }
    }
    Ok(())
}

fn sync_mods(inst_dir: &Path, dir: &Path, loader: &str, meta: &mut Meta) -> Result<SyncReport> {
    let mods_dir = dir.join("mods");
    let (copy, client_only) = if loader == "vanilla" { (vec![], vec![]) } else { plan_mods(&modmeta::scan(inst_dir)) };
    let keep: HashSet<&String> = copy.iter().collect();
    let mut report = SyncReport { client_only: client_only.clone(), ..Default::default() };
    for old in &meta.synced {
        if !keep.contains(old) && std::fs::remove_file(mods_dir.join(old)).is_ok() {
            report.removed += 1;
        }
    }
    if !copy.is_empty() {
        std::fs::create_dir_all(&mods_dir)?;
    }
    for f in &copy {
        let (src, dst) = (inst_dir.join("mods").join(f), mods_dir.join(f));
        if !same_file(&src, &dst) {
            std::fs::copy(&src, &dst).with_context(|| format!("copying {f}"))?;
            report.copied += 1;
        }
    }
    if loader != "vanilla" {
        copy_missing(&inst_dir.join("config"), &dir.join("config"))?;
    }
    meta.synced = copy;
    meta.client_only = client_only;
    Ok(report)
}

// ---------------------------------------------------------------- install

async fn run_java(java: &str, dir: &Path, args: &[String], what: &str) -> Result<()> {
    let mut cmd = tokio::process::Command::new(java);
    cmd.arg("-jar").args(args).current_dir(dir).stdin(std::process::Stdio::null());
    hide_window(&mut cmd);
    let out = cmd.output().await.with_context(|| format!("starting the {what}"))?;
    let log = String::from_utf8_lossy(&out.stdout).to_string() + &String::from_utf8_lossy(&out.stderr);
    let _ = std::fs::write(dir.join(".cairn").join("install.log"), &log);
    if !out.status.success() {
        let tail: Vec<&str> = log.lines().rev().take(6).collect();
        bail!("The {what} failed. Last output: {}", tail.into_iter().rev().collect::<Vec<_>>().join(" | "));
    }
    Ok(())
}

#[cfg(windows)]
fn hide_window(cmd: &mut tokio::process::Command) {
    cmd.creation_flags(0x0800_0000);
}

#[cfg(not(windows))]
fn hide_window(_: &mut tokio::process::Command) {}

async fn vanilla_server_jar(vjson: &Value, dir: &Path, mc: &str) -> Result<()> {
    let s = &vjson["downloads"]["server"];
    let url = s["url"].as_str().ok_or_else(|| anyhow!("Mojang has no server download for Minecraft {mc}"))?;
    net::download(&Download { url: url.to_string(), dest: dir.join("server.jar"), sha1: s["sha1"].as_str().map(str::to_string), sha256: None, size: s["size"].as_u64() })
        .await
        .context("downloading the Minecraft server")
}

fn find_args_file(dir: &Path, loader: &str, mc: &str, lv: &str) -> Option<String> {
    let name = if cfg!(windows) { "win_args.txt" } else { "unix_args.txt" };
    let rel = if loader == "forge" { format!("libraries/net/minecraftforge/forge/{mc}-{lv}/{name}") } else { format!("libraries/net/neoforged/neoforge/{lv}/{name}") };
    dir.join(&rel).exists().then_some(rel)
}

fn find_legacy_jar(dir: &Path) -> Option<String> {
    let rd = std::fs::read_dir(dir).ok()?;
    let mut jars: Vec<String> = rd
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".jar") && (n.starts_with("forge-") || n.starts_with("minecraftforge-")) && !n.contains("installer"))
        .collect();
    jars.sort();
    jars.pop()
}

async fn install_server(paths: &Paths, inst: &Instance, vjson: &Value, java: &str, dir: &Path, task: &Task) -> Result<Launch> {
    let mc = inst.mc_version.as_str();
    let loader = inst.loader.as_str();
    if loader == "vanilla" {
        task.stage("Downloading the Minecraft server", 0, 0);
        vanilla_server_jar(vjson, dir, mc).await?;
        return Ok(Launch::Jar("server.jar".into()));
    }
    let lv = loaders::resolve_version(loader, mc, inst.loader_version.as_deref()).await?;
    match loader {
        "fabric" => {
            task.stage("Downloading the Minecraft server", 0, 0);
            vanilla_server_jar(vjson, dir, mc).await?;
            task.stage("Downloading the Fabric server launcher", 0, 0);
            let list: Value = net::get_json("https://meta.fabricmc.net/v2/versions/installer").await?;
            let arr = list.as_array().cloned().unwrap_or_default();
            let installer = arr
                .iter()
                .find(|v| v["stable"].as_bool() == Some(true))
                .or(arr.first())
                .and_then(|v| v["version"].as_str())
                .ok_or_else(|| anyhow!("Fabric lists no installer"))?;
            let url = format!("https://meta.fabricmc.net/v2/versions/loader/{mc}/{lv}/{installer}/server/jar");
            let dest = dir.join("fabric-server-launch.jar");
            let tmp = dir.join(".cairn").join("fabric-server-launch.jar.part");
            let bytes = net::CLIENT.get(&url).send().await?.error_for_status()?.bytes().await?;
            if !bytes.starts_with(b"PK") {
                bail!("Fabric sent something that isn't a jar for {mc} / {lv}");
            }
            std::fs::write(&tmp, &bytes)?;
            std::fs::rename(&tmp, &dest)?;
            std::fs::write(dir.join("fabric-server-launcher.properties"), "serverJar=server.jar\n")?;
            Ok(Launch::Jar("fabric-server-launch.jar".into()))
        }
        "quilt" => {
            task.stage("Downloading the Quilt installer", 0, 0);
            let list: Value = net::get_json("https://meta.quiltmc.org/v3/versions/installer").await?;
            let v = list.get(0).and_then(|v| v["version"].as_str()).ok_or_else(|| anyhow!("Quilt lists no installer"))?;
            let url = format!("https://maven.quiltmc.org/repository/release/org/quiltmc/quilt-installer/{v}/quilt-installer-{v}.jar");
            let installer = paths.cache.join("installers").join(format!("quilt-installer-{v}.jar"));
            let (sha1, sha256) = loaders::maven_checksums(&url).await;
            net::download(&Download { url, dest: installer.clone(), sha1, sha256, size: None }).await?;
            task.stage("Installing the Quilt server", 0, 0);
            let args = vec![
                installer.to_string_lossy().into_owned(),
                "install".into(),
                "server".into(),
                mc.into(),
                lv.clone(),
                "--download-server".into(),
                format!("--install-dir={}", dir.display()),
            ];
            run_java(java, dir, &args, "Quilt installer").await?;
            if !dir.join("quilt-server-launch.jar").exists() {
                bail!("The Quilt installer finished without quilt-server-launch.jar");
            }
            Ok(Launch::Jar("quilt-server-launch.jar".into()))
        }
        "forge" | "neoforge" => {
            let installer = loaders::fetch_installer(paths, loader, mc, &lv, task).await?;
            let pretty = if loader == "forge" { "Forge" } else { "NeoForge" };
            task.stage(&format!("Installing the {pretty} server (this can take a few minutes)"), 0, 0);
            let args = vec![installer.to_string_lossy().into_owned(), "--installServer".into(), dir.to_string_lossy().into_owned()];
            run_java(java, dir, &args, &format!("{pretty} installer")).await?;
            if let Some(rel) = find_args_file(dir, loader, mc, &lv) {
                return Ok(Launch::Args(rel));
            }
            find_legacy_jar(dir).map(Launch::Jar).ok_or_else(|| anyhow!("The {pretty} installer finished but left no way to start the server"))
        }
        other => bail!("Cairn can't host {other} servers"),
    }
}

/// Installs (or refreshes) the server files and syncs mods. Safe to run again at any time.
pub async fn prepare(paths: &Paths, settings: &Settings, inst: &Instance, task: &Task) -> Result<SyncReport> {
    if is_running(&inst.id) {
        bail!("Stop the server first.");
    }
    let dir = server_dir(paths, &inst.id);
    std::fs::create_dir_all(dir.join(".cairn"))?;
    let mut meta = load_meta(&dir);
    task.stage("Checking Minecraft", 0, 0);
    let vjson = version::ensure_vanilla_json(paths, &inst.mc_version).await?;
    let java_bin = java::ensure(paths, settings, version::java_major(&vjson), task).await?;
    let key = build_key(inst);
    let fresh = meta.launch.as_ref().is_some_and(|l| launch_file(&dir, l).exists()) && meta.built_for.as_deref() == Some(key.as_str());
    if !fresh {
        meta.launch = Some(install_server(paths, inst, &vjson, &java_bin, &dir, task).await?);
        meta.built_for = Some(key);
    }
    meta.java = Some(java_bin);
    task.stage("Copying mods", 0, 0);
    let report = sync_mods(&inst.dir(paths), &dir, &inst.loader, &mut meta)?;
    write_properties(&dir, &meta.config)?;
    save_meta(&dir, &meta)?;
    Ok(report)
}

// ---------------------------------------------------------------- process

fn emit_state(app: &AppHandle, id: &str, state: &str, exit_code: Option<i32>) {
    if let Some(p) = PROCS.locked().get_mut(id) {
        p.state = state.to_string();
    }
    let _ = app.emit("server-state", StateEvent { instance_id: id, state, exit_code });
}

fn push_lines(id: &str, lines: &[String]) {
    let mut procs = PROCS.locked();
    let p = procs.entry(id.to_string()).or_default();
    for l in lines {
        if p.log.len() >= LOG_KEEP {
            p.log.pop_front();
        }
        p.log.push_back(l.clone());
    }
}

pub fn log(id: &str) -> Vec<String> {
    PROCS.locked().get(id).map(|p| p.log.iter().cloned().collect()).unwrap_or_default()
}

fn is_ready_line(l: &str) -> bool {
    l.contains("Done (") && l.contains("For help, type")
}

fn port_free(port: u16) -> bool {
    std::net::TcpListener::bind(("127.0.0.1", port)).is_ok()
}

fn launch_command(java: &str, dir: &Path, launch: &Launch, memory_mb: u32) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new(java);
    cmd.arg(format!("-Xmx{memory_mb}M"));
    match launch {
        Launch::Jar(f) => {
            cmd.arg("-jar").arg(f);
        }
        Launch::Args(f) => {
            if dir.join("user_jvm_args.txt").exists() {
                cmd.arg("@user_jvm_args.txt");
            }
            cmd.arg(format!("@{f}"));
        }
    }
    cmd.arg("nogui").current_dir(dir).stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).kill_on_drop(true);
    hide_window(&mut cmd);
    cmd
}

pub async fn start(app: &AppHandle, paths: &Paths, inst: &Instance) -> Result<()> {
    let id = inst.id.clone();
    if is_running(&id) {
        bail!("The server is already running.");
    }
    let dir = server_dir(paths, &id);
    let mut meta = load_meta(&dir);
    let launch = meta.launch.clone().filter(|l| launch_file(&dir, l).exists()).ok_or_else(|| anyhow!("Set the server up first."))?;
    if !eula_accepted(&dir) {
        bail!("Accept the Minecraft EULA before starting the server.");
    }
    if !port_free(meta.config.port) {
        bail!("Port {} is already in use. Pick another port or stop whatever is using it.", meta.config.port);
    }
    let java_bin = meta.java.clone().filter(|j| Path::new(j).exists()).ok_or_else(|| anyhow!("Java is missing. Run setup again."))?;
    sync_mods(&inst.dir(paths), &dir, &inst.loader, &mut meta)?;
    write_properties(&dir, &meta.config)?;
    save_meta(&dir, &meta)?;

    let mut child = launch_command(&java_bin, &dir, &launch, meta.config.memory_mb).spawn().context("starting Java")?;
    let stdin = child.stdin.take().map(|s| Arc::new(tokio::sync::Mutex::new(s)));
    let (kill_tx, kill_rx) = tokio::sync::oneshot::channel();
    {
        let mut procs = PROCS.locked();
        let p = procs.entry(id.clone()).or_default();
        p.log.clear();
        p.stdin = stdin;
        p.kill = Some(kill_tx);
    }
    emit_state(app, &id, "starting", None);

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    if let Some(out) = child.stdout.take() {
        let tx = tx.clone();
        tauri::async_runtime::spawn(async move {
            let mut lines = BufReader::new(out).lines();
            while let Ok(Some(l)) = lines.next_line().await {
                let _ = tx.send(l);
            }
        });
    }
    if let Some(err) = child.stderr.take() {
        let tx = tx.clone();
        tauri::async_runtime::spawn(async move {
            let mut lines = BufReader::new(err).lines();
            while let Ok(Some(l)) = lines.next_line().await {
                let _ = tx.send(l);
            }
        });
    }
    drop(tx);

    // Batch log lines so a chatty modded startup doesn't flood the UI with one event per line.
    let (app2, id2) = (app.clone(), id.clone());
    let pump = tauri::async_runtime::spawn(async move {
        while let Some(first) = rx.recv().await {
            let mut batch = vec![first];
            while batch.len() < 500 {
                match rx.try_recv() {
                    Ok(l) => batch.push(l),
                    Err(_) => break,
                }
            }
            push_lines(&id2, &batch);
            if state_of(&id2) == "starting" && batch.iter().any(|l| is_ready_line(l)) {
                emit_state(&app2, &id2, "running", None);
            }
            let _ = app2.emit("server-log", LogEvent { instance_id: &id2, lines: &batch });
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    let app3 = app.clone();
    tauri::async_runtime::spawn(async move {
        let status = tokio::select! {
            s = child.wait() => s.ok(),
            _ = kill_rx => {
                let _ = child.kill().await;
                child.wait().await.ok()
            }
        };
        let _ = pump.await;
        if let Some(p) = PROCS.locked().get_mut(&id) {
            p.stdin = None;
            p.kill = None;
        }
        emit_state(&app3, &id, "stopped", status.and_then(|s| s.code()));
    });
    Ok(())
}

pub async fn command(id: &str, line: &str) -> Result<()> {
    let line = line.trim();
    if line.is_empty() {
        return Ok(());
    }
    let stdin = PROCS.locked().get(id).and_then(|p| p.stdin.clone()).ok_or_else(|| anyhow!("The server isn't running."))?;
    let mut s = stdin.lock().await;
    s.write_all(format!("{}\n", line.trim_start_matches('/')).as_bytes()).await?;
    s.flush().await?;
    push_lines(id, &[format!("> {line}")]);
    Ok(())
}

/// Asks the server to save and stop; kills it if it hasn't exited after the grace period.
pub async fn stop(app: &AppHandle, id: &str, force: bool) -> Result<()> {
    if !is_running(id) {
        return Ok(());
    }
    if force {
        if let Some(k) = PROCS.locked().get_mut(id).and_then(|p| p.kill.take()) {
            let _ = k.send(());
        }
        return Ok(());
    }
    emit_state(app, id, "stopping", None);
    if command(id, "stop").await.is_err() {
        if let Some(k) = PROCS.locked().get_mut(id).and_then(|p| p.kill.take()) {
            let _ = k.send(());
        }
        return Ok(());
    }
    let id = id.to_string();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(STOP_GRACE).await;
        if let Some(k) = PROCS.locked().get_mut(&id).and_then(|p| p.kill.take()) {
            let _ = k.send(());
        }
    });
    Ok(())
}

pub fn running_ids() -> Vec<String> {
    PROCS.locked().iter().filter(|(_, p)| !p.state.is_empty() && p.state != "stopped").map(|(id, _)| id.clone()).collect()
}

/// Called when Cairn quits so no server keeps running headless in the background.
pub fn kill_all() {
    for p in PROCS.locked().values_mut() {
        if let Some(k) = p.kill.take() {
            let _ = k.send(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modmeta::ModInfo;

    fn jar(file: &str, enabled: bool, side: Option<Side>) -> JarMod {
        JarMod { file: file.into(), enabled, info: side.map(|side| ModInfo { name: file.trim_end_matches(".jar").into(), side, ..Default::default() }) }
    }

    #[test]
    fn client_only_and_disabled_mods_stay_off_the_server() {
        let mods = vec![
            jar("sodium.jar", true, Some(Side::Client)),
            jar("lithium.jar", true, Some(Side::Both)),
            jar("ferritecore.jar", true, None),
            jar("old.jar.disabled", false, Some(Side::Both)),
            jar("servercore.jar", true, Some(Side::Server)),
        ];
        let (copy, client) = plan_mods(&mods);
        assert_eq!(copy, vec!["lithium.jar", "ferritecore.jar", "servercore.jar"]);
        assert_eq!(client, vec!["sodium"]);
    }

    #[test]
    fn properties_keep_unrelated_lines_and_bind_locally_by_default() {
        let before = "#Minecraft server properties\nlevel-name=world\nserver-port=1\nmotd=old\n";
        let after = apply_properties(before, &ServerConfig::default());
        assert!(after.starts_with("#Minecraft server properties\nlevel-name=world\nserver-port=25565\nmotd=A Cairn server\n"));
        assert!(after.contains("server-ip=127.0.0.1\n"));
        assert!(after.contains("online-mode=true\n"));
        let lan = apply_properties(&after, &ServerConfig { lan: true, motd: "Bună\\".into(), ..Default::default() });
        assert!(lan.contains("server-ip=\n"));
        assert!(lan.contains("motd=Bun\\u0103\\\\\n"));
        assert_eq!(lan.matches("server-port=").count(), 1);
    }

    #[test]
    fn eula_is_only_true_after_accepting() {
        let dir = std::env::temp_dir().join(format!("cairn-eula-{}", uuid::Uuid::new_v4()));
        assert!(!eula_accepted(&dir));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("eula.txt"), "eula=false\n").unwrap();
        assert!(!eula_accepted(&dir));
        accept_eula(&dir).unwrap();
        assert!(eula_accepted(&dir));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sync_removes_only_jars_cairn_copied() {
        let root = std::env::temp_dir().join(format!("cairn-sync-{}", uuid::Uuid::new_v4()));
        let (inst, srv) = (root.join("inst"), root.join("srv"));
        std::fs::create_dir_all(inst.join("mods")).unwrap();
        std::fs::create_dir_all(srv.join("mods")).unwrap();
        std::fs::write(inst.join("mods/a.jar"), b"not a real jar").unwrap();
        std::fs::write(srv.join("mods/mine.jar"), b"player added").unwrap();
        std::fs::write(srv.join("mods/gone.jar"), b"stale").unwrap();
        let mut meta = Meta { synced: vec!["gone.jar".into()], ..Default::default() };
        let r = sync_mods(&inst, &srv, "fabric", &mut meta).unwrap();
        assert_eq!((r.copied, r.removed), (1, 1));
        assert!(srv.join("mods/a.jar").exists() && srv.join("mods/mine.jar").exists() && !srv.join("mods/gone.jar").exists());
        assert_eq!(meta.synced, vec!["a.jar"]);
        let again = sync_mods(&inst, &srv, "fabric", &mut meta).unwrap();
        assert_eq!(again.copied, 0);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn ready_line_matches_vanilla_and_forge_output() {
        assert!(is_ready_line("[12:00:01] [Server thread/INFO]: Done (3.214s)! For help, type \"help\""));
        assert!(!is_ready_line("[12:00:01] [Server thread/INFO]: Preparing spawn area: 40%"));
    }

    /// Installs and boots real servers. Downloads a lot, so run it by hand:
    /// `cargo test live_servers_boot -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn live_servers_boot() {
        let root = std::env::temp_dir().join("cairn-live-hosting");
        let paths = Paths::at(&root);
        let settings = Settings::default();
        for (loader, port) in [("vanilla", 25601), ("fabric", 25602), ("quilt", 25603), ("neoforge", 25604), ("forge", 25605)] {
            let inst = Instance { id: format!("live-{loader}"), name: loader.into(), mc_version: "1.21.1".into(), loader: loader.into(), ..Default::default() };
            std::fs::create_dir_all(inst.dir(&paths)).unwrap();
            prepare(&paths, &settings, &inst, &Task::headless(loader)).await.unwrap_or_else(|e| panic!("{loader}: {e:#}"));
            let dir = server_dir(&paths, &inst.id);
            save_config(&paths, &inst.id, ServerConfig { port, ..Default::default() }).unwrap();
            accept_eula(&dir).unwrap();
            let meta = load_meta(&dir);
            let mut child = launch_command(meta.java.as_deref().unwrap(), &dir, meta.launch.as_ref().unwrap(), 2048).spawn().unwrap();
            let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
            let ready = tokio::time::timeout(Duration::from_secs(240), async {
                while let Ok(Some(l)) = lines.next_line().await {
                    if is_ready_line(&l) {
                        return true;
                    }
                }
                false
            })
            .await
            .unwrap_or(false);
            let _ = child.kill().await;
            println!("{loader}: {:?} ready={ready}", meta.launch);
            assert!(ready, "{loader} server never finished starting");
        }
    }
}
