use crate::accounts::{self, AccountsFile};
use crate::instance::{self, Instance};
use crate::mc::{install, launch, version};
use crate::paths::Paths;
use crate::settings::Settings;
use crate::tasks::Task;
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
}

#[derive(Default)]
pub struct Games {
    pub states: Mutex<HashMap<String, GameState>>,
    pub logs: Mutex<HashMap<String, VecDeque<String>>>,
    /// Signals a running Java process to exit.
    pub killers: Mutex<HashMap<String, tokio::sync::oneshot::Sender<()>>>,
    /// Set while prepare/launch is in flight so Stop works before the process starts.
    pub cancels: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

const LOG_KEEP: usize = 8000;

impl Games {
    fn set(&self, app: &AppHandle, s: GameState) {
        self.states.lock().unwrap().insert(s.instance_id.clone(), s.clone());
        let _ = app.emit("game-state", s);
    }
    fn push_log(&self, app: &AppHandle, id: &str, line: String, stream: &str) {
        {
            let mut logs = self.logs.lock().unwrap();
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
        let states = games.states.lock().unwrap();
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
    let profile = args
        .profile_id
        .as_deref()
        .or(inst.active_launch_profile.as_deref())
        .and_then(|pid| inst.launch_profiles.iter().find(|p| p.id == pid).cloned());
    let mut accs = AccountsFile::load(&paths);
    let active_id = accs.active.clone().ok_or_else(|| anyhow!("Add an account in Accounts before playing."))?;
    games.set(&app, GameState { instance_id: id.clone(), state: "preparing".into(), pid: None, started: crate::util::now_ms(), exit_code: None });
    games.logs.lock().unwrap().insert(id.clone(), VecDeque::new());

    let cancel = Arc::new(AtomicBool::new(false));
    games.cancels.lock().unwrap().insert(id.clone(), cancel.clone());
    let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();
    games.killers.lock().unwrap().insert(id.clone(), cancel_tx);

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
        games.killers.lock().unwrap().remove(&id);
        games.cancels.lock().unwrap().remove(&id);
        games.push_log(&app, &id, "[Cairn] Launch cancelled".into(), "launcher");
        games.set(&app, GameState { instance_id: id.clone(), state: "exited".into(), pid: None, started: 0, exit_code: None });
    };
    let (mut java_path, installed, acc) = tokio::select! {
        r = prep => match r {
            Ok(v) => {
                task.finish();
                v
            }
            Err(e) => {
                task.fail(&e);
                games.killers.lock().unwrap().remove(&id);
                games.cancels.lock().unwrap().remove(&id);
                games.set(&app, GameState { instance_id: id.clone(), state: "exited".into(), pid: None, started: 0, exit_code: None });
                return Err(e);
            }
        },
        _ = cancel_rx => {
            mark_cancelled(true);
            return Ok(());
        }
    };
    games.killers.lock().unwrap().remove(&id);
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
            crate::discord::clear();
            games.cancels.lock().unwrap().remove(&id);
            games.set(&app, GameState { instance_id: id.clone(), state: "exited".into(), pid: None, started: 0, exit_code: None });
            return Err(e);
        }
    };
    games.cancels.lock().unwrap().remove(&id);
    if cancel.load(Ordering::SeqCst) {
        let _ = child.kill().await;
        let _ = child.wait().await;
        games.push_log(&app, &id, "[Cairn] Launch cancelled".into(), "launcher");
        games.set(&app, GameState { instance_id: id.clone(), state: "exited".into(), pid: None, started: 0, exit_code: None });
        return Ok(());
    }
    let started = crate::util::now_ms();
    games.set(&app, GameState { instance_id: id.clone(), state: "running".into(), pid: child.id(), started, exit_code: None });
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
    if settings.discord_rpc {
        crate::discord::set_playing(&inst.name, &inst.mc_version, &inst.loader);
    }

    if settings.hide_on_launch {
        if let Some(w) = app.get_webview_window("main") {
            let _ = w.hide();
        }
    }

    let log_dir = dir.join(".cairn").join("logs");
    std::fs::create_dir_all(&log_dir)?;
    let log_file = Arc::new(Mutex::new(std::fs::File::create(log_dir.join(format!("launch-{}.log", chrono::Local::now().format("%Y-%m-%d_%H-%M-%S"))))?));
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
                        let _ = writeln!(lf.lock().unwrap(), "{line}");
                    }
                    games.push_log(&app, &id, line, stream);
                }
            }
        });
    }

    let (tx, rx) = tokio::sync::oneshot::channel();
    games.killers.lock().unwrap().insert(id.clone(), tx);
    let app2 = app.clone();
    tokio::spawn(async move {
        let code = tokio::select! {
            r = child.wait() => r.ok().and_then(|s| s.code()),
            _ = rx => { let _ = child.kill().await; child.wait().await.ok().and_then(|s| s.code()) }
        };
        games.killers.lock().unwrap().remove(&id);
        let secs = ((crate::util::now_ms() - started) / 1000).max(0) as u64;
        if let Ok(mut i) = instance::get(&paths, &id) {
            i.play_seconds += secs;
            let _ = instance::save(&paths, &i);
        }
        if shader_stash.exists() {
            let sp = dir.join("shaderpacks");
            let _ = std::fs::remove_dir_all(&sp);
            let _ = std::fs::rename(&shader_stash, &sp);
        }
        crate::discord::clear();
        games.push_log(&app2, &id, format!("[Cairn] Game exited with code {}", code.map(|c| c.to_string()).unwrap_or_else(|| "unknown".into())), "launcher");
        games.set(&app2, GameState { instance_id: id.clone(), state: "exited".into(), pid: None, started, exit_code: code });
        if let Some(w) = app2.get_webview_window("main") {
            let _ = w.show();
        }
    });
    Ok(())
}

pub fn kill(games: &Games, id: &str) -> bool {
    let mut hit = false;
    if let Some(flag) = games.cancels.lock().unwrap().get(id) {
        flag.store(true, Ordering::SeqCst);
        hit = true;
    }
    if let Some(tx) = games.killers.lock().unwrap().remove(id) {
        let _ = tx.send(());
        hit = true;
    }
    hit
}
