use crate::accounts::{self, AccountsFile};
use crate::instance::{self, Instance};
use crate::mc::{install, launch, version};
use crate::paths::Paths;
use crate::settings::Settings;
use crate::tasks::Task;
use crate::util::LockExt;
use crate::{java, loaders};
use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LogLine {
    pub instance_id: String,
    pub line: String,
    pub stream: String,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GameState {
    pub instance_id: String,
    pub state: String, // preparing | running | exited
    pub pid: Option<u32>,
    pub started: i64,
    pub exit_code: Option<i32>,
    /// Exited on its own with an error or a crash report (not stopped from Cairn).
    pub crashed: bool,
}

#[derive(Default)]
pub struct Games {
    pub states: Mutex<HashMap<String, GameState>>,
    pub logs: Mutex<HashMap<String, VecDeque<String>>>,
    /// Signals a running game process (Java or Horizon) to exit.
    pub killers: Mutex<HashMap<String, tokio::sync::oneshot::Sender<()>>>,
    /// Set while prepare/launch is in flight so Stop works before the process starts.
    pub cancels: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

const LOG_KEEP: usize = 8000;

impl Games {
    fn set(&self, app: &AppHandle, s: GameState) {
        self.states.locked().insert(s.instance_id.clone(), s.clone());
        let _ = app.emit("game-state", s);
    }
    fn push_log(&self, app: &AppHandle, id: &str, line: String, stream: &str) {
        {
            let mut logs = self.logs.locked();
            let q = logs.entry(id.to_string()).or_default();
            q.push_back(line.clone());
            while q.len() > LOG_KEEP {
                q.pop_front();
            }
        }
        let _ = app.emit("game-log", LogLine { instance_id: id.into(), line, stream: stream.into() });
    }
}

pub async fn prepare(paths: &Paths, settings: &Settings, inst: &mut Instance, task: &Task) -> Result<(String, String, install::Installed)> {
    let vanilla = version::ensure_vanilla_json(paths, &inst.mc_version).await?;
    let mut major = version::java_major(&vanilla);
    let mut java_path = match &inst.java_path {
        Some(p) if !p.is_empty() => p.clone(),
        _ => java::ensure(paths, settings, major, task).await?,
    };
    let launch_version = match &inst.launch_version {
        Some(v) if paths.version_json(v).exists() => v.clone(),
        _ => loaders::install(paths, &inst.loader, &inst.mc_version, inst.loader_version.as_deref(), Some(&java_path), task).await?,
    };
    if inst.launch_version.as_deref() != Some(&launch_version) {
        inst.launch_version = Some(launch_version.clone());
        instance::save(paths, inst)?;
    }
    let installed = install::install_version(paths, &launch_version, &inst.dir(paths), task, settings.concurrent_downloads).await?;
    let merged_major = version::java_major(&installed.json);
    if merged_major != major && inst.java_path.as_deref().unwrap_or("").is_empty() {
        major = merged_major;
        java_path = java::ensure(paths, settings, major, task).await?;
    }
    Ok((java_path, launch_version, installed))
}

pub struct LaunchArgs {
    pub world: Option<String>,
    pub server: Option<String>,
    pub profile_id: Option<String>,
}

pub async fn launch(app: AppHandle, paths: Paths, games: Arc<Games>, id: String, args: LaunchArgs) -> Result<()> {
    {
        let states = games.states.locked();
        if let Some(s) = states.get(&id) {
            if s.state != "exited" {
                anyhow::bail!("This instance is already running");
            }
        }
    }
    if args.world.is_some() && args.server.is_some() {
        anyhow::bail!("Pick either a world or a server, not both");
    }
    let settings = Settings::load(&paths.settings_file());
    let mut inst = instance::get(&paths, &id)?;
    if inst.use_horizon {
        return launch_horizon(app, paths, games, id, args, inst, settings).await;
    }
    let profile = args
        .profile_id
        .as_deref()
        .or(inst.active_launch_profile.as_deref())
        .and_then(|pid| inst.launch_profiles.iter().find(|p| p.id == pid).cloned());
    let mut accs = AccountsFile::load(&paths);
    let active_id = accs.active.clone().ok_or_else(|| anyhow!("Add an account in Accounts before playing."))?;
    games.set(&app, GameState { instance_id: id.clone(), state: "preparing".into(), pid: None, started: crate::util::now_ms(), exit_code: None, crashed: false });
    games.logs.locked().insert(id.clone(), VecDeque::new());

    let cancel = Arc::new(AtomicBool::new(false));
    games.cancels.locked().insert(id.clone(), cancel.clone());
    let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();
    games.killers.locked().insert(id.clone(), cancel_tx);

    let task = Task::new(&app, format!("Preparing {}", inst.name));
    let prep = async {
        let (java_path, _vid, installed) = prepare(&paths, &settings, &mut inst, &task).await?;
        task.stage("Signing in", 0, 0);
        let acc = accs.accounts.iter_mut().find(|a| a.id == active_id).ok_or_else(|| anyhow!("The selected account was removed."))?;
        accounts::fresh_token(acc).await?;
        let acc = acc.clone();
        accs.save(&paths)?;
        Ok::<_, anyhow::Error>((java_path, installed, acc))
    };
    let mark_cancelled = |fail_task: bool| {
        if fail_task {
            task.fail(&anyhow!("Launch cancelled"));
        }
        games.killers.locked().remove(&id);
        games.cancels.locked().remove(&id);
        games.push_log(&app, &id, "[Cairn] Launch cancelled".into(), "launcher");
        games.set(&app, GameState { instance_id: id.clone(), state: "exited".into(), pid: None, started: 0, exit_code: None, crashed: false });
    };
    let (mut java_path, installed, acc) = tokio::select! {
        r = prep => match r {
            Ok(v) => {
                task.finish();
                v
            }
            Err(e) => {
                task.fail(&e);
                games.killers.locked().remove(&id);
                games.cancels.locked().remove(&id);
                games.set(&app, GameState { instance_id: id.clone(), state: "exited".into(), pid: None, started: 0, exit_code: None, crashed: false });
                return Err(e);
            }
        },
        _ = cancel_rx => {
            mark_cancelled(true);
            return Ok(());
        }
    };
    games.killers.locked().remove(&id);
    if cancel.load(Ordering::SeqCst) {
        mark_cancelled(false);
        return Ok(());
    }

    let dir = inst.dir(&paths);
    let shaders_off = profile.as_ref().map(|p| p.disable_shaders).unwrap_or(false);
    let shader_stash = dir.join("shaderpacks.cairn-off");
    if shaders_off {
        let sp = dir.join("shaderpacks");
        if sp.exists() && !shader_stash.exists() {
            let _ = std::fs::rename(&sp, &shader_stash);
            let _ = std::fs::create_dir_all(&sp);
        }
    }
    let auth = launch::LaunchAuth {
        name: acc.username.clone(),
        uuid: acc.uuid.replace('-', ""),
        access_token: if acc.kind == "microsoft" { acc.mc_token.clone() } else { "0".into() },
        user_type: if acc.kind == "microsoft" { "msa".into() } else { "legacy".into() },
        xuid: "0".into(),
    };
    let mut extra: Vec<String> = settings.default_jvm_args.split_whitespace().map(String::from).collect();
    extra.extend(inst.jvm_args.split_whitespace().map(String::from));
    if let Some(p) = &profile {
        extra.extend(p.jvm_args.split_whitespace().map(String::from));
        if let Some(jp) = &p.java_path {
            if !jp.trim().is_empty() {
                java_path = jp.clone();
            }
        }
    }
    let memory_mb = profile
        .as_ref()
        .and_then(|p| p.memory_mb)
        .or(inst.memory_mb)
        .unwrap_or(settings.default_memory_mb);
    let opts = launch::LaunchOptions {
        memory_mb,
        extra_jvm_args: extra,
        width: Some(inst.width.unwrap_or(settings.default_width)),
        height: Some(inst.height.unwrap_or(settings.default_height)),
        game_dir: dir.clone(),
        natives_dir: installed.natives_dir.clone(),
        jar_id: installed.jar_id.clone(),
        quick_play_world: args.world,
        quick_play_server: args.server,
    };
    let args = launch::build_args(&paths, &installed.json, &auth, &opts);
    games.push_log(&app, &id, format!("[Cairn] Java: {java_path}"), "launcher");
    games.push_log(&app, &id, format!("[Cairn] Account: {} ({})", acc.username, acc.kind), "launcher");
    games.push_log(&app, &id, format!("[Cairn] Command: {}", launch::redact(&args, &auth.access_token).join(" ")), "launcher");

    let mut cmd = tokio::process::Command::new(&java_path);
    cmd.args(&args)
        .current_dir(&dir)
        .env_clear()
        .envs(settings.game_process_env())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(false);
    if settings.isolate_game_env {
        games.push_log(&app, &id, "[Cairn] Game env: launcher GTK/WebKit vars stripped".into(), "launcher");
    }
    if !settings.game_env.is_empty() {
        games.push_log(&app, &id, format!("[Cairn] Game env overrides: {}", settings.game_env.len()), "launcher");
    }
    if cancel.load(Ordering::SeqCst) {
        mark_cancelled(false);
        return Ok(());
    }
    let mut child = match cmd.spawn().context("starting Java") {
        Ok(c) => c,
        Err(e) => {
            if shader_stash.exists() {
                let sp = dir.join("shaderpacks");
                let _ = std::fs::remove_dir_all(&sp);
                let _ = std::fs::rename(&shader_stash, &sp);
            }
            crate::discord::clear(&id);
            games.cancels.locked().remove(&id);
            games.set(&app, GameState { instance_id: id.clone(), state: "exited".into(), pid: None, started: 0, exit_code: None, crashed: false });
            return Err(e);
        }
    };
    games.cancels.locked().remove(&id);
    if cancel.load(Ordering::SeqCst) {
        let _ = child.kill().await;
        let _ = child.wait().await;
        games.push_log(&app, &id, "[Cairn] Launch cancelled".into(), "launcher");
        games.set(&app, GameState { instance_id: id.clone(), state: "exited".into(), pid: None, started: 0, exit_code: None, crashed: false });
        return Ok(());
    }
    let started = crate::util::now_ms();
    games.set(&app, GameState { instance_id: id.clone(), state: "running".into(), pid: child.id(), started, exit_code: None, crashed: false });
    inst.last_played = Some(started);
    instance::save(&paths, &inst)?;
    crate::telemetry::spawn(
        paths.settings_file(),
        "game_launch",
        serde_json::json!({
            "loader": inst.loader,
            "mc_version": inst.mc_version,
        }),
    );
    crate::discord::set_playing(crate::discord::Session::new(&id, &inst.name, &inst.mc_version, &inst.loader, inst.loader_version.as_deref()));

    if settings.hide_on_launch {
        if let Some(w) = app.get_webview_window("main") {
            let _ = w.hide();
        }
    }

    let log_dir = dir.join(".cairn").join("logs");
    std::fs::create_dir_all(&log_dir)?;
    let log_name = format!("launch-{}.log", chrono::Local::now().format("%Y-%m-%d_%H-%M-%S"));
    let session_log = format!(".cairn/logs/{log_name}");
    let log_file = Arc::new(Mutex::new(std::fs::File::create(log_dir.join(&log_name))?));
    for (stream, reader) in [
        ("stdout", Box::new(child.stdout.take().unwrap()) as Box<dyn tokio::io::AsyncRead + Unpin + Send>),
        ("stderr", Box::new(child.stderr.take().unwrap())),
    ] {
        let app = app.clone();
        let games = games.clone();
        let id = id.clone();
        let lf = log_file.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(reader).lines();
            let mut parser = crate::logparse::Log4jParser::default();
            while let Ok(Some(raw)) = lines.next_line().await {
                for line in parser.feed(&raw) {
                    {
                        use std::io::Write;
                        let _ = writeln!(lf.locked(), "{line}");
                    }
                    games.push_log(&app, &id, line, stream);
                }
            }
        });
    }

    let (tx, rx) = tokio::sync::oneshot::channel();
    games.killers.locked().insert(id.clone(), tx);
    let app2 = app.clone();
    tokio::spawn(async move {
        let (code, stopped) = tokio::select! {
            r = child.wait() => (r.ok().and_then(|s| s.code()), false),
            _ = rx => { let _ = child.kill().await; (child.wait().await.ok().and_then(|s| s.code()), true) }
        };
        games.killers.locked().remove(&id);
        let crashed = finish_session(paths.clone(), id.clone(), started, code, stopped, session_log).await;
        if shader_stash.exists() {
            let sp = dir.join("shaderpacks");
            let _ = std::fs::remove_dir_all(&sp);
            let _ = std::fs::rename(&shader_stash, &sp);
        }
        crate::discord::clear(&id);
        games.push_log(&app2, &id, format!("[Cairn] Game exited with code {}", code.map(|c| c.to_string()).unwrap_or_else(|| "unknown".into())), "launcher");
        games.set(&app2, GameState { instance_id: id.clone(), state: "exited".into(), pid: None, started, exit_code: code, crashed });
        if let Some(w) = app2.get_webview_window("main") {
            let _ = w.show();
        }
    });
    Ok(())
}

async fn launch_horizon(
    app: AppHandle,
    paths: Paths,
    games: Arc<Games>,
    id: String,
    args: LaunchArgs,
    mut inst: Instance,
    settings: Settings,
) -> Result<()> {
    crate::horizon::validate_instance(&inst)?;
    let mut accs = AccountsFile::load(&paths);
    let active_id = accs
        .active
        .clone()
        .ok_or_else(|| anyhow!("Add an account in Accounts before playing."))?;
    games.set(
        &app,
        GameState {
            instance_id: id.clone(),
            state: "preparing".into(),
            pid: None,
            started: crate::util::now_ms(),
            exit_code: None,
            crashed: false,
        },
    );
    games.logs.locked().insert(id.clone(), VecDeque::new());

    let cancel = Arc::new(AtomicBool::new(false));
    games.cancels.locked().insert(id.clone(), cancel.clone());
    let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();
    games.killers.locked().insert(id.clone(), cancel_tx);

    let width = inst.width.unwrap_or(settings.default_width);
    let height = inst.height.unwrap_or(settings.default_height);
    let task = Task::new(&app, format!("Preparing {}", inst.name));
    let prep = async {
        let (bin, mut hz_args, game_home) = crate::horizon::prepare_launch(
            &paths,
            &inst,
            &task,
            args.world.as_deref(),
            args.server.as_deref(),
            width,
            height,
        )
        .await?;
        task.stage("Signing in", 0, 0);
        let idx = accs
            .accounts
            .iter()
            .position(|a| a.id == active_id)
            .ok_or_else(|| anyhow!("The selected account was removed."))?;
        if accs.accounts[idx].kind == "microsoft" {
            accounts::fresh_token(&mut accs.accounts[idx]).await?;
            accs.save(&paths)?;
            let session = crate::horizon::write_ms_session_file(&game_home, &accs.accounts[idx])?;
            hz_args.push("--session-file".into());
            hz_args.push(session.to_string_lossy().into_owned());
        } else {
            let name = accs.accounts[idx].username.clone();
            crate::horizon::write_offline_player_name(&game_home, &name)?;
            hz_args.push("--name".into());
            hz_args.push(name);
        }
        let who = (accs.accounts[idx].username.clone(), accs.accounts[idx].kind.clone());
        Ok::<_, anyhow::Error>((bin, hz_args, game_home, who))
    };
    let mark_cancelled = |fail_task: bool| {
        if fail_task {
            task.fail(&anyhow!("Launch cancelled"));
        }
        games.killers.locked().remove(&id);
        games.cancels.locked().remove(&id);
        games.push_log(&app, &id, "[Cairn] Launch cancelled".into(), "launcher");
        games.set(
            &app,
            GameState {
                instance_id: id.clone(),
                state: "exited".into(),
                pid: None,
                started: 0,
                exit_code: None,
                crashed: false,
            },
        );
    };
    let (bin, hz_args, game_home, (username, kind)) = tokio::select! {
        r = prep => match r {
            Ok(v) => {
                task.finish();
                v
            }
            Err(e) => {
                task.fail(&e);
                games.killers.locked().remove(&id);
                games.cancels.locked().remove(&id);
                games.set(&app, GameState { instance_id: id.clone(), state: "exited".into(), pid: None, started: 0, exit_code: None, crashed: false });
                return Err(e);
            }
        },
        _ = cancel_rx => {
            mark_cancelled(true);
            return Ok(());
        }
    };
    games.killers.locked().remove(&id);
    if cancel.load(Ordering::SeqCst) {
        mark_cancelled(false);
        return Ok(());
    }

    games.push_log(&app, &id, format!("[Cairn] Horizon: {}", bin.display()), "launcher");
    games.push_log(&app, &id, format!("[Cairn] Account: {username} ({kind})"), "launcher");
    if kind != "microsoft" {
        games.push_log(
            &app,
            &id,
            "[Cairn] Offline name — online-mode servers need a Microsoft account.".into(),
            "launcher",
        );
    }
    // Redact session path contents; keep argv readable without the token file body.
    let redacted: Vec<String> = hz_args
        .iter()
        .enumerate()
        .map(|(i, a)| {
            if i > 0 && hz_args[i - 1] == "--session-file" {
                "<session>".into()
            } else {
                a.clone()
            }
        })
        .collect();
    games.push_log(
        &app,
        &id,
        format!("[Cairn] Command: {} {}", bin.display(), redacted.join(" ")),
        "launcher",
    );

    let mut cmd = tokio::process::Command::new(&bin);
    cmd.args(&hz_args)
        .current_dir(&game_home)
        .env_clear()
        .envs(settings.game_process_env())
        .env("HORIZON_HOME", &game_home)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(false);
    if settings.isolate_game_env {
        games.push_log(&app, &id, "[Cairn] Game env: launcher GTK/WebKit vars stripped".into(), "launcher");
    }
    if cancel.load(Ordering::SeqCst) {
        mark_cancelled(false);
        return Ok(());
    }
    let mut child = match cmd.spawn().context("starting Horizon") {
        Ok(c) => c,
        Err(e) => {
            crate::discord::clear(&id);
            games.cancels.locked().remove(&id);
            games.set(&app, GameState { instance_id: id.clone(), state: "exited".into(), pid: None, started: 0, exit_code: None, crashed: false });
            return Err(e);
        }
    };
    games.cancels.locked().remove(&id);
    if cancel.load(Ordering::SeqCst) {
        let _ = child.kill().await;
        let _ = child.wait().await;
        games.push_log(&app, &id, "[Cairn] Launch cancelled".into(), "launcher");
        games.set(&app, GameState { instance_id: id.clone(), state: "exited".into(), pid: None, started: 0, exit_code: None, crashed: false });
        return Ok(());
    }

    let started = crate::util::now_ms();
    games.set(&app, GameState { instance_id: id.clone(), state: "running".into(), pid: child.id(), started, exit_code: None, crashed: false });
    inst.last_played = Some(started);
    instance::save(&paths, &inst)?;
    crate::telemetry::spawn(
        paths.settings_file(),
        "game_launch",
        serde_json::json!({
            "loader": "horizon",
            "mc_version": inst.mc_version,
        }),
    );
    crate::discord::set_playing(crate::discord::Session::new(&id, &inst.name, &inst.mc_version, "horizon", None));
    if settings.hide_on_launch {
        if let Some(w) = app.get_webview_window("main") {
            let _ = w.hide();
        }
    }

    let log_dir = game_home.join(".cairn").join("logs");
    std::fs::create_dir_all(&log_dir)?;
    let log_file = Arc::new(Mutex::new(std::fs::File::create(log_dir.join(format!(
        "launch-{}.log",
        chrono::Local::now().format("%Y-%m-%d_%H-%M-%S")
    )))?));
    for (stream, reader) in [
        ("stdout", Box::new(child.stdout.take().unwrap()) as Box<dyn tokio::io::AsyncRead + Unpin + Send>),
        ("stderr", Box::new(child.stderr.take().unwrap())),
    ] {
        let app = app.clone();
        let games = games.clone();
        let id = id.clone();
        let lf = log_file.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(reader).lines();
            let mut parser = crate::logparse::Log4jParser::default();
            while let Ok(Some(raw)) = lines.next_line().await {
                for line in parser.feed(&raw) {
                    {
                        use std::io::Write;
                        let _ = writeln!(lf.locked(), "{line}");
                    }
                    games.push_log(&app, &id, line, stream);
                }
            }
        });
    }

    let (tx, rx) = tokio::sync::oneshot::channel();
    games.killers.locked().insert(id.clone(), tx);
    let app2 = app.clone();
    tokio::spawn(async move {
        let (code, stopped) = tokio::select! {
            r = child.wait() => (r.ok().and_then(|s| s.code()), false),
            _ = rx => { let _ = child.kill().await; (child.wait().await.ok().and_then(|s| s.code()), true) }
        };
        games.killers.locked().remove(&id);
        let crashed = finish_session(paths.clone(), id.clone(), started, code, stopped, String::new()).await;
        crate::discord::clear(&id);
        games.push_log(
            &app2,
            &id,
            format!(
                "[Cairn] Game exited with code {}",
                code.map(|c| c.to_string()).unwrap_or_else(|| "unknown".into())
            ),
            "launcher",
        );
        games.set(
            &app2,
            GameState {
                instance_id: id.clone(),
                state: "exited".into(),
                pid: None,
                started,
                exit_code: code,
                crashed,
            },
        );
        if let Some(w) = app2.get_webview_window("main") {
            let _ = w.show();
        }
    });
    Ok(())
}

/// Adds the playtime, records the session and works out whether it crashed. `log` is the
/// session log relative to the instance folder (empty when it lives elsewhere).
/// Reads mod jars when it crashed, so it runs on the blocking pool.
async fn finish_session(paths: Paths, id: String, started: i64, code: Option<i32>, stopped: bool, log: String) -> bool {
    tokio::task::spawn_blocking(move || record_session(&paths, &id, started, code, stopped, &log)).await.unwrap_or(false)
}

fn record_session(paths: &Paths, id: &str, started: i64, code: Option<i32>, stopped: bool, log: &str) -> bool {
    let ended = crate::util::now_ms();
    let seconds = ((ended - started) / 1000).max(0) as u64;
    let Ok(mut inst) = instance::get(paths, id) else { return false };
    inst.play_seconds += seconds;
    let _ = instance::save(paths, &inst);
    let dir = inst.dir(paths);
    let report = crate::stats::crash_report_since(&dir, started);
    let crashed = crate::stats::is_crash(code, stopped, report.is_some());
    let log = report.or_else(|| (!log.is_empty()).then(|| log.to_string()));
    let reason = if crashed { log.as_deref().and_then(|l| crate::crashlog::summary(&diagnose_file(paths, &inst, l))) } else { None };
    let _ = crate::stats::record(&dir, crate::stats::Session { started, ended, seconds, exit_code: code, crashed, reason, log });
    crashed
}

/// Diagnoses one log or crash report inside the instance folder.
pub fn diagnose_file(paths: &Paths, inst: &Instance, rel: &str) -> Vec<crate::compat::Issue> {
    let dir = inst.dir(paths);
    let Some(text) = read_log(&dir, rel) else { return vec![] };
    let mods = crate::modmeta::scan(&dir);
    let memory_mb = inst.memory_mb.unwrap_or_else(|| Settings::load(&paths.settings_file()).default_memory_mb);
    let advice = crate::perf::advise(crate::perf::system_memory_mb(), inst.loader != "vanilla", mods.len());
    crate::crashlog::diagnose(&text, &crate::crashlog::Ctx { mods: &mods, memory_mb, max_safe_mb: advice.max_safe_mb })
}

/// Reads a log (plain or .gz) that must sit inside the instance folder.
pub fn read_log(dir: &std::path::Path, rel: &str) -> Option<String> {
    use std::io::Read;
    let path = dir.join(rel);
    let real = path.canonicalize().ok()?;
    if !real.starts_with(dir.canonicalize().ok()?) {
        return None;
    }
    let bytes = std::fs::read(&real).ok()?;
    let mut text = String::new();
    if rel.ends_with(".gz") {
        flate2::read::GzDecoder::new(&bytes[..]).read_to_string(&mut text).ok()?;
    } else {
        text = String::from_utf8_lossy(&bytes).into_owned();
    }
    Some(text)
}

pub fn kill(games: &Games, id: &str) -> bool {
    let mut hit = false;
    if let Some(flag) = games.cancels.locked().get(id) {
        flag.store(true, Ordering::SeqCst);
        hit = true;
    }
    if let Some(tx) = games.killers.locked().remove(id) {
        let _ = tx.send(());
        hit = true;
    }
    hit
}
