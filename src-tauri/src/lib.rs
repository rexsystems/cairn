pub mod accounts;
pub mod backup;
pub mod content;
pub mod discord;
pub mod game;
pub mod instance;
pub mod java;
pub mod loaders;
pub mod logparse;
pub mod mc;
pub mod modrinth;
pub mod modrinth_app;
pub mod net;
pub mod paths;
pub mod servers;
pub mod settings;
pub mod tasks;
pub mod telemetry;
pub mod util;
pub mod version_art;

use accounts::AccountsFile;
use instance::Instance;
use paths::Paths;
use serde::Serialize;
use serde_json::{json, Value};
use settings::Settings;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tasks::Task;
use tauri::{AppHandle, Manager, State};

pub struct AppState {
    paths: Paths,
    games: Arc<game::Games>,
    sign_ins: Mutex<HashMap<String, Arc<AtomicBool>>>,
    accounts_lock: tokio::sync::Mutex<()>,
}

type R<T> = Result<T, String>;
fn e<E: std::fmt::Display>(err: E) -> String {
    format!("{err:#}")
}
fn ea(err: anyhow::Error) -> String {
    format!("{err:#}")
}

#[tauri::command]
async fn mc_version_art(version: String) -> Option<String> {
    version_art::image_url(&version).await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    version: String,
    data_dir: String,
    client_id: String,
    client_id_source: String,
    auth_flow: String,
    os: String,
}

#[tauri::command]
fn app_info(state: State<AppState>) -> AppInfo {
    let s = Settings::load(&state.paths.settings_file());
    let id = s.effective_client_id();
    let source = if std::env::var("CAIRN_MSA_CLIENT_ID").map(|v| !v.trim().is_empty()).unwrap_or(false) {
        "environment"
    } else {
        "default"
    };
    AppInfo {
        version: env!("CARGO_PKG_VERSION").into(),
        data_dir: state.paths.root.to_string_lossy().into(),
        auth_flow: accounts::flow_for(&id).into(),
        client_id: id,
        client_id_source: source.into(),
        os: mc::rules::os_name().into(),
    }
}

#[tauri::command]
fn get_settings(state: State<AppState>) -> Settings {
    Settings::load(&state.paths.settings_file())
}

#[tauri::command]
fn save_settings(state: State<AppState>, settings: Settings) -> R<Settings> {
    let mut s = settings;
    s.concurrent_downloads = s.concurrent_downloads.clamp(1, 64);
    s.default_memory_mb = s.default_memory_mb.clamp(512, 65536);
    s.save(&state.paths.settings_file()).map_err(ea)?;
    Ok(s)
}

// Instances

#[tauri::command]
fn list_instances(state: State<AppState>) -> Vec<instance::InstanceView> {
    instance::list(&state.paths).into_iter().map(|i| instance::view(&state.paths, i)).collect()
}

#[tauri::command]
fn get_instance(state: State<AppState>, id: String) -> R<instance::InstanceView> {
    Ok(instance::view(&state.paths, instance::get(&state.paths, &id).map_err(ea)?))
}

#[tauri::command]
fn create_instance(state: State<AppState>, inst: Instance) -> R<instance::InstanceView> {
    let created = instance::create(&state.paths, inst).map_err(ea)?;
    telemetry::spawn(
        state.paths.settings_file(),
        "instance_create",
        serde_json::json!({
            "loader": created.loader,
            "mc_version": created.mc_version,
        }),
    );
    Ok(instance::view(&state.paths, created))
}

#[tauri::command]
fn update_instance(state: State<AppState>, inst: Instance) -> R<instance::InstanceView> {
    let mut cur = instance::get(&state.paths, &inst.id).map_err(ea)?;
    if cur.mc_version != inst.mc_version || cur.loader != inst.loader || cur.loader_version != inst.loader_version {
        cur.launch_version = None;
    }
    if inst.name.trim().is_empty() {
        return Err("Give the instance a name".into());
    }
    cur.name = inst.name.trim().into();
    cur.mc_version = inst.mc_version;
    cur.loader = inst.loader;
    cur.loader_version = inst.loader_version;
    cur.memory_mb = inst.memory_mb;
    cur.jvm_args = inst.jvm_args;
    cur.java_path = inst.java_path.filter(|p| !p.trim().is_empty());
    cur.width = inst.width;
    cur.height = inst.height;
    cur.group = inst.group.filter(|g| !g.trim().is_empty());
    instance::save(&state.paths, &cur).map_err(ea)?;
    Ok(instance::view(&state.paths, cur))
}

#[tauri::command]
fn delete_instance(state: State<AppState>, id: String) -> R<()> {
    instance::delete(&state.paths, &id).map_err(ea)
}

#[tauri::command]
fn duplicate_instance(state: State<AppState>, id: String) -> R<instance::InstanceView> {
    Ok(instance::view(&state.paths, instance::duplicate(&state.paths, &id).map_err(ea)?))
}

#[tauri::command]
fn set_instance_icon(state: State<AppState>, id: String, path: Option<String>) -> R<instance::InstanceView> {
    let mut inst = instance::get(&state.paths, &id).map_err(ea)?;
    let dir = inst.dir(&state.paths);
    if let Some(old) = &inst.icon {
        let _ = std::fs::remove_file(dir.join(old));
    }
    inst.icon = match path {
        Some(p) => {
            let ext = PathBuf::from(&p).extension().map(|x| x.to_string_lossy().to_lowercase()).unwrap_or_else(|| "png".into());
            if !["png", "jpg", "jpeg", "webp", "gif"].contains(&ext.as_str()) {
                return Err("Pick a PNG, JPG, WebP or GIF image".into());
            }
            let name = format!("icon-{}.{ext}", util::now_ms());
            std::fs::copy(&p, dir.join(&name)).map_err(e)?;
            Some(name)
        }
        None => None,
    };
    instance::save(&state.paths, &inst).map_err(ea)?;
    Ok(instance::view(&state.paths, inst))
}

#[tauri::command]
async fn import_mrpack(app: AppHandle, state: State<'_, AppState>, path: String) -> R<instance::InstanceView> {
    let s = Settings::load(&state.paths.settings_file());
    let task = Task::new(&app, "Importing modpack");
    let t2 = task.clone();
    let inst = task.run(modrinth::install_mrpack(&state.paths, &PathBuf::from(path), None, None, s.concurrent_downloads, &t2)).await.map_err(ea)?;
    Ok(instance::view(&state.paths, inst))
}

#[tauri::command]
fn list_modrinth_app_profiles() -> R<Vec<modrinth_app::ModrinthAppProfile>> {
    modrinth_app::list_profiles().map_err(ea)
}

#[tauri::command]
async fn import_modrinth_app_profile(app: AppHandle, state: State<'_, AppState>, key: String) -> R<instance::InstanceView> {
    let task = Task::new(&app, "Importing from Modrinth App");
    let t2 = task.clone();
    let paths = state.paths.clone();
    let paths2 = paths.clone();
    let key2 = key.clone();
    let inst = task
        .run(async move {
            let inst = tokio::task::spawn_blocking(move || modrinth_app::import_profile(&paths2, &key2, &t2))
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))??;
            let _ = modrinth::identify(&inst.dir(&paths)).await;
            Ok(inst)
        })
        .await
        .map_err(ea)?;
    Ok(instance::view(&state.paths, inst))
}

#[tauri::command]
fn export_mrpack(state: State<AppState>, id: String, dest: String, extra: Vec<String>) -> R<String> {
    let inst = instance::get(&state.paths, &id).map_err(ea)?;
    let p = modrinth::export_mrpack(&state.paths, &inst, &PathBuf::from(dest), &extra).map_err(ea)?;
    Ok(p.to_string_lossy().into())
}

// Content

#[tauri::command]
fn list_content(state: State<AppState>, id: String, kind: String) -> R<Vec<content::ContentItem>> {
    let inst = instance::get(&state.paths, &id).map_err(ea)?;
    content::list(&state.paths, &inst.dir(&state.paths), &kind).map_err(ea)
}

#[tauri::command]
fn toggle_content(state: State<AppState>, id: String, kind: String, file: String) -> R<String> {
    let inst = instance::get(&state.paths, &id).map_err(ea)?;
    content::toggle(&inst.dir(&state.paths), &kind, &file).map_err(ea)
}

#[tauri::command]
fn remove_content(state: State<AppState>, id: String, kind: String, file: String) -> R<()> {
    let inst = instance::get(&state.paths, &id).map_err(ea)?;
    content::remove(&inst.dir(&state.paths), &kind, &file).map_err(ea)
}

#[tauri::command]
fn add_content_files(state: State<AppState>, id: String, kind: String, files: Vec<String>) -> R<Vec<String>> {
    let inst = instance::get(&state.paths, &id).map_err(ea)?;
    let dir = inst.dir(&state.paths);
    files.iter().map(|f| content::add_local(&dir, &kind, &PathBuf::from(f)).map_err(ea)).collect()
}

#[tauri::command]
async fn installed_projects(state: State<'_, AppState>, id: String) -> R<Vec<String>> {
    let inst = instance::get(&state.paths, &id).map_err(ea)?;
    let dir = inst.dir(&state.paths);
    // Files dropped in by hand get matched to Modrinth by hash; offline is fine, we just know less.
    let _ = modrinth::identify(&dir).await;
    Ok(content::installed_projects(&dir))
}

#[tauri::command]
async fn check_updates(state: State<'_, AppState>, id: String) -> R<Vec<modrinth::UpdateInfo>> {
    let inst = instance::get(&state.paths, &id).map_err(ea)?;
    modrinth::check_updates(&state.paths, &inst).await.map_err(ea)
}

#[tauri::command]
async fn apply_updates(app: AppHandle, state: State<'_, AppState>, id: String, updates: Vec<Value>) -> R<u32> {
    let inst = instance::get(&state.paths, &id).map_err(ea)?;
    let task = Task::new(&app, format!("Updating content in {}", inst.name));
    let paths = state.paths.clone();
    let t2 = task.clone();
    task.run(async move {
        let mut n = 0;
        let total = updates.len() as u64;
        for (i, u) in updates.into_iter().enumerate() {
            let u = modrinth::UpdateInfo {
                kind: u["kind"].as_str().unwrap_or_default().into(),
                file_name: u["fileName"].as_str().unwrap_or_default().into(),
                title: u["title"].as_str().unwrap_or_default().into(),
                icon: None,
                project_id: u["projectId"].as_str().unwrap_or_default().into(),
                current_version: String::new(),
                new_version: u["newVersion"].as_str().unwrap_or_default().into(),
                new_version_id: u["newVersionId"].as_str().unwrap_or_default().into(),
            };
            t2.stage(&format!("Updating {}", u.title), i as u64, total);
            modrinth::apply_update(&paths, &inst, &u, &t2).await?;
            n += 1;
        }
        Ok(n)
    })
    .await
    .map_err(ea)
}

#[tauri::command]
fn list_worlds(state: State<AppState>, id: String) -> R<Vec<content::World>> {
    let inst = instance::get(&state.paths, &id).map_err(ea)?;
    Ok(content::worlds(&inst.dir(&state.paths)))
}

#[tauri::command]
fn list_screenshots(state: State<AppState>, id: String) -> R<Vec<content::FileEntry>> {
    let inst = instance::get(&state.paths, &id).map_err(ea)?;
    Ok(content::files_in(&inst.dir(&state.paths).join("screenshots"), &[".png", ".jpg"]))
}

#[tauri::command]
fn list_log_files(state: State<AppState>, id: String) -> R<Vec<content::FileEntry>> {
    let inst = instance::get(&state.paths, &id).map_err(ea)?;
    let dir = inst.dir(&state.paths);
    let label = |mut v: Vec<content::FileEntry>, prefix: &str| {
        for f in v.iter_mut() {
            f.name = format!("{prefix}{}", f.name);
        }
        v
    };
    let mut v = label(content::files_in(&dir.join(".cairn").join("logs"), &[".log"]), "Cairn session: ");
    v.extend(label(content::files_in(&dir.join("logs"), &[".log", ".log.gz"]), "Game log: "));
    v.extend(label(content::files_in(&dir.join("crash-reports"), &[".txt"]), "Crash report: "));
    v.sort_by(|a, b| b.modified.cmp(&a.modified));
    Ok(v)
}

#[tauri::command]
fn read_instance_file(state: State<AppState>, id: String, path: String) -> R<String> {
    let inst = instance::get(&state.paths, &id).map_err(ea)?;
    let dir = std::fs::canonicalize(inst.dir(&state.paths)).map_err(e)?;
    let p = std::fs::canonicalize(&path).map_err(e)?;
    if !p.starts_with(&dir) {
        return Err("That file is outside the instance folder".into());
    }
    let mut bytes = std::fs::read(&p).map_err(e)?;
    if p.extension().map(|x| x == "gz").unwrap_or(false) {
        use std::io::Read;
        let mut out = vec![];
        flate2::read::GzDecoder::new(&bytes[..]).read_to_end(&mut out).map_err(e)?;
        bytes = out;
    }
    let start = bytes.len().saturating_sub(2_000_000);
    Ok(String::from_utf8_lossy(&bytes[start..]).into())
}

// Game

#[tauri::command]
async fn launch_instance(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    world: Option<String>,
    server: Option<String>,
    profile_id: Option<String>,
) -> R<()> {
    let breadcrumb_inst = instance::get(&state.paths, &id).ok();
    if let Some(ref inst) = breadcrumb_inst {
        sentry::configure_scope(|scope| {
            scope.set_tag("mc_version", &inst.mc_version);
            scope.set_tag("loader", &inst.loader);
            scope.set_tag("instance_id", &inst.id);
        });
        sentry::add_breadcrumb(sentry::Breadcrumb {
            category: Some("launch".into()),
            message: Some(format!(
                "launch {} ({} {})",
                inst.name, inst.loader, inst.mc_version
            )),
            level: sentry::Level::Info,
            data: {
                let mut m = std::collections::BTreeMap::new();
                m.insert("world".into(), json!(world));
                m.insert("server".into(), json!(server));
                m.insert("profile_id".into(), json!(profile_id));
                m
            },
            ..Default::default()
        });
    }

    match game::launch(
        app,
        state.paths.clone(),
        state.games.clone(),
        id,
        game::LaunchArgs { world, server, profile_id },
    )
    .await
    {
        Ok(()) => Ok(()),
        Err(err) => {
            sentry::add_breadcrumb(sentry::Breadcrumb {
                category: Some("launch".into()),
                message: Some(format!("launch failed: {err:#}")),
                level: sentry::Level::Error,
                ..Default::default()
            });
            sentry::capture_message(&format!("Launch failed: {err:#}"), sentry::Level::Error);
            Err(ea(err))
        }
    }
}

#[tauri::command]
fn list_servers(state: State<AppState>, id: String) -> R<Vec<servers::ServerEntry>> {
    let inst = instance::get(&state.paths, &id).map_err(ea)?;
    servers::list(&inst.dir(&state.paths)).map_err(ea)
}

#[tauri::command]
fn save_servers(state: State<AppState>, id: String, servers_list: Vec<servers::ServerEntry>) -> R<()> {
    let inst = instance::get(&state.paths, &id).map_err(ea)?;
    servers::save(&inst.dir(&state.paths), &servers_list).map_err(ea)
}

#[tauri::command]
async fn backup_instance(app: AppHandle, state: State<'_, AppState>, id: String, dest: String) -> R<String> {
    let task = Task::new(&app, "Backing up instance");
    let t2 = task.clone();
    let paths = state.paths.clone();
    let dest = PathBuf::from(dest);
    let p = task
        .run(async move {
            tokio::task::spawn_blocking(move || backup::backup(&paths, &id, &dest, &t2))
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?
        })
        .await
        .map_err(ea)?;
    Ok(p.to_string_lossy().into())
}

#[tauri::command]
async fn restore_instance_backup(app: AppHandle, state: State<'_, AppState>, path: String) -> R<instance::InstanceView> {
    let task = Task::new(&app, "Restoring backup");
    let t2 = task.clone();
    let paths = state.paths.clone();
    let zip = PathBuf::from(path);
    let inst = task
        .run(async move {
            tokio::task::spawn_blocking(move || backup::restore(&paths, &zip, &t2))
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?
        })
        .await
        .map_err(ea)?;
    Ok(instance::view(&state.paths, inst))
}

#[tauri::command]
fn search_library_content(state: State<AppState>, query: String) -> R<Vec<content::ContentHit>> {
    content::search_all(&state.paths, &query).map_err(ea)
}

#[tauri::command]
async fn modrinth_version_deps(version_id: String) -> R<Vec<modrinth::DepInfo>> {
    modrinth::version_dependencies(&version_id).await.map_err(ea)
}

#[tauri::command]
fn kill_instance(state: State<AppState>, id: String) -> bool {
    game::kill(&state.games, &id)
}

#[tauri::command]
fn game_states(state: State<AppState>) -> Vec<game::GameState> {
    state.games.states.lock().unwrap().values().cloned().collect()
}

#[tauri::command]
fn game_log(state: State<AppState>, id: String) -> Vec<String> {
    state.games.logs.lock().unwrap().get(&id).map(|q| q.iter().cloned().collect()).unwrap_or_default()
}

#[tauri::command]
async fn install_instance(app: AppHandle, state: State<'_, AppState>, id: String) -> R<()> {
    let settings = Settings::load(&state.paths.settings_file());
    let mut inst = instance::get(&state.paths, &id).map_err(ea)?;
    let task = Task::new(&app, format!("Installing {}", inst.name));
    let t2 = task.clone();
    task.run(async { game::prepare(&state.paths, &settings, &mut inst, &t2).await.map(|_| ()) }).await.map_err(ea)
}

// Versions

#[tauri::command]
async fn mc_versions(state: State<'_, AppState>) -> R<Vec<mc::version::ManifestVersion>> {
    Ok(mc::version::manifest(&state.paths).await.map_err(ea)?.versions)
}

#[tauri::command]
async fn loader_versions(loader: String, mc: String) -> R<Vec<loaders::LoaderVersion>> {
    loaders::versions(&loader, &mc).await.map_err(ea)
}

// Modrinth

#[tauri::command]
async fn modrinth_search(params: Value) -> R<Value> {
    let p: modrinth::SearchParams = serde_json::from_value(params).map_err(e)?;
    modrinth::search(&p).await.map_err(ea)
}

#[tauri::command]
async fn modrinth_project(id: String) -> R<Value> {
    modrinth::project(&id).await.map_err(ea)
}

#[tauri::command]
async fn modrinth_versions(id: String, loaders: Vec<String>, game_versions: Vec<String>) -> R<Vec<Value>> {
    modrinth::versions(&id, &loaders, &game_versions).await.map_err(ea)
}

#[tauri::command]
async fn modrinth_tags(state: State<'_, AppState>) -> R<Value> {
    let cache = state.paths.cache.join("modrinth-tags.json");
    if util::mtime_ms(&cache) > util::now_ms() - 24 * 3600 * 1000 {
        if let Ok(v) = util::read_json::<Value>(&cache) {
            return Ok(v);
        }
    }
    let (cu, lu, gu) = (
        format!("{}/tag/category", modrinth::API),
        format!("{}/tag/loader", modrinth::API),
        format!("{}/tag/game_version", modrinth::API),
    );
    let (c, l, g) = tokio::join!(net::get_json::<Value>(&cu), net::get_json::<Value>(&lu), net::get_json::<Value>(&gu));
    let v = json!({"categories": c.map_err(ea)?, "loaders": l.map_err(ea)?, "gameVersions": g.map_err(ea)?});
    let _ = util::write_json_atomic(&cache, &v);
    Ok(v)
}

#[tauri::command]
async fn modrinth_install(
    app: AppHandle,
    state: State<'_, AppState>,
    instance_id: String,
    project_id: String,
    version_id: Option<String>,
    include_optional: Option<bool>,
) -> R<Vec<modrinth::Installed>> {
    let inst = instance::get(&state.paths, &instance_id).map_err(ea)?;
    let task = Task::new(&app, format!("Installing into {}", inst.name));
    let t2 = task.clone();
    let settings_file = state.paths.settings_file();
    let installed = task
        .run(modrinth::install_project(
            &state.paths,
            &inst,
            &project_id,
            version_id.as_deref(),
            include_optional.unwrap_or(false),
            &t2,
        ))
        .await
        .map_err(ea)?;
    telemetry::spawn(
        settings_file,
        "mod_install",
        json!({
            "loader": inst.loader,
            "mc_version": inst.mc_version,
            "count": installed.len(),
        }),
    );
    Ok(installed)
}

#[tauri::command]
async fn modrinth_install_modpack(app: AppHandle, state: State<'_, AppState>, project_id: String, version_id: Option<String>) -> R<instance::InstanceView> {
    let s = Settings::load(&state.paths.settings_file());
    let task = Task::new(&app, "Installing modpack");
    let t2 = task.clone();
    let settings_file = state.paths.settings_file();
    let inst = task
        .run(modrinth::install_modpack_project(
            &state.paths,
            &project_id,
            version_id.as_deref(),
            s.concurrent_downloads,
            &t2,
        ))
        .await
        .map_err(ea)?;
    telemetry::spawn(
        settings_file,
        "instance_create",
        json!({
            "loader": inst.loader,
            "mc_version": inst.mc_version,
            "source": "modpack",
        }),
    );
    Ok(instance::view(&state.paths, inst))
}

// Accounts

#[tauri::command]
fn list_accounts(state: State<AppState>) -> Vec<accounts::AccountView> {
    AccountsFile::load(&state.paths).views()
}

#[tauri::command]
async fn set_active_account(state: State<'_, AppState>, id: String) -> R<Vec<accounts::AccountView>> {
    let _g = state.accounts_lock.lock().await;
    let mut f = AccountsFile::load(&state.paths);
    if !f.accounts.iter().any(|a| a.id == id) {
        return Err("Account not found".into());
    }
    f.active = Some(id);
    f.save(&state.paths).map_err(ea)?;
    Ok(f.views())
}

#[tauri::command]
async fn remove_account(state: State<'_, AppState>, id: String) -> R<Vec<accounts::AccountView>> {
    let _g = state.accounts_lock.lock().await;
    let mut f = AccountsFile::load(&state.paths);
    f.remove(&id);
    f.save(&state.paths).map_err(ea)?;
    Ok(f.views())
}

#[tauri::command]
async fn account_cosmetics(state: State<'_, AppState>, id: String) -> R<accounts::AccountCosmetics> {
    let _lock = state.accounts_lock.lock().await;
    let mut f = AccountsFile::load(&state.paths);
    accounts::load_cosmetics(&mut f, &state.paths, &id).await.map_err(ea)
}

#[tauri::command]
async fn set_account_cape(state: State<'_, AppState>, id: String, cape_id: Option<String>) -> R<accounts::AccountCosmetics> {
    let _lock = state.accounts_lock.lock().await;
    let mut f = AccountsFile::load(&state.paths);
    accounts::set_active_cape(&mut f, &state.paths, &id, cape_id.as_deref()).await.map_err(ea)
}

#[tauri::command]
async fn add_offline_account(state: State<'_, AppState>, name: String) -> R<Vec<accounts::AccountView>> {
    let _g = state.accounts_lock.lock().await;
    let mut f = AccountsFile::load(&state.paths);
    accounts::add_offline(&mut f, &name).map_err(ea)?;
    f.save(&state.paths).map_err(ea)?;
    Ok(f.views())
}

/// Open a Microsoft sign-in window (authorization code), like Modrinth App / RexLauncher.
/// Returns the updated account list, or null if the user closed/cancelled the window.
#[tauri::command]
async fn msa_login(app: AppHandle, state: State<'_, AppState>) -> R<Option<Vec<accounts::AccountView>>> {
    use tauri::{UserAttentionType, WebviewUrl, WebviewWindowBuilder};

    let s = Settings::load(&state.paths.settings_file());
    let client_id = s.effective_client_id();
    let auth_url = accounts::authorize_url(&client_id).map_err(ea)?;

    if let Some(w) = app.get_webview_window("signin") {
        let _ = w.close();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    let cancel = Arc::new(AtomicBool::new(false));
    state.sign_ins.lock().unwrap().insert("signin".into(), cancel.clone());

    let window = WebviewWindowBuilder::new(&app, "signin", WebviewUrl::External(auth_url.parse().map_err(e)?))
        .title("Sign in with Microsoft")
        .inner_size(520.0, 720.0)
        .resizable(true)
        .center()
        .build()
        .map_err(e)?;
    let _ = window.request_user_attention(Some(UserAttentionType::Critical));

    let deadline = util::now_ms() + 10 * 60 * 1000;
    let code = loop {
        if cancel.load(Ordering::SeqCst) || util::now_ms() > deadline {
            let _ = window.close();
            state.sign_ins.lock().unwrap().remove("signin");
            return Ok(None);
        }
        if window.title().is_err() {
            state.sign_ins.lock().unwrap().remove("signin");
            return Ok(None);
        }
        if let Ok(url) = window.url() {
            let s = url.as_str();
            if s.starts_with(accounts::AUTH_REDIRECT) {
                let pairs: Vec<_> = url.query_pairs().map(|(k, v)| (k.to_string(), v.to_string())).collect();
                if let Some((_, err)) = pairs.iter().find(|(k, _)| k == "error") {
                    let _ = window.close();
                    state.sign_ins.lock().unwrap().remove("signin");
                    let desc = pairs.iter().find(|(k, _)| k == "error_description").map(|(_, v)| v.as_str()).unwrap_or(err.as_str());
                    return Err(format!("Microsoft sign-in failed: {desc}").into());
                }
                if let Some((_, code)) = pairs.iter().find(|(k, _)| k == "code") {
                    let _ = window.close();
                    break code.clone();
                }
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    };

    state.sign_ins.lock().unwrap().remove("signin");
    let (access, refresh) = accounts::exchange_auth_code(&client_id, &code).await.map_err(ea)?;
    let (token, expires, profile) = accounts::minecraft_login(&access, "live").await.map_err(ea)?;
    let acc = accounts::account_from_login("live", &client_id, refresh, token, expires, &profile);
    let _lock = state.accounts_lock.lock().await;
    let mut f = AccountsFile::load(&state.paths);
    f.upsert(acc);
    f.save(&state.paths).map_err(ea)?;
    Ok(Some(f.views()))
}

#[tauri::command]
fn msa_cancel(app: AppHandle, state: State<AppState>) {
    if let Some(c) = state.sign_ins.lock().unwrap().remove("signin") {
        c.store(true, Ordering::SeqCst);
    }
    if let Some(w) = app.get_webview_window("signin") {
        let _ = w.close();
    }
}

// Java

#[tauri::command]
async fn list_java(state: State<'_, AppState>) -> R<Vec<java::JavaInstall>> {
    let p = state.paths.clone();
    tokio::task::spawn_blocking(move || java::detect(&p)).await.map_err(e)
}

#[tauri::command]
async fn install_java(app: AppHandle, state: State<'_, AppState>, major: u32) -> R<java::JavaInstall> {
    let task = Task::new(&app, format!("Installing Java {major}"));
    let t2 = task.clone();
    task.run(java::install(&state.paths, major, &t2)).await.map_err(ea)
}

/// Open a folder/file in the OS file manager. Only paths inside Cairn's data dir are allowed.
#[tauri::command]
fn open_data_path(app: AppHandle, state: State<AppState>, path: String) -> R<()> {
    use tauri_plugin_opener::OpenerExt;
    let requested = PathBuf::from(&path);
    let target = if requested.exists() {
        std::fs::canonicalize(&requested).map_err(e)?
    } else {
        return Err(format!("That path does not exist: {path}").into());
    };
    let root = std::fs::canonicalize(&state.paths.root).map_err(e)?;
    if !target.starts_with(&root) {
        return Err("Can only open folders inside Cairn's data directory.".into());
    }
    app.opener().open_path(target.to_string_lossy(), None::<&str>).map_err(e)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let paths = Paths::new();
    let data_root = paths.root.clone();
    let settings_file = paths.settings_file();
    let state = AppState {
        paths,
        games: Arc::new(game::Games::default()),
        sign_ins: Mutex::new(HashMap::new()),
        accounts_lock: tokio::sync::Mutex::new(()),
    };
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(state)
        .setup(move |app| {
            // Only Cairn's own data folder (screenshots, icons, world thumbnails) is readable by the webview.
            app.asset_protocol_scope().allow_directory(&data_root, true)?;
            tauri::async_runtime::spawn(async move {
                telemetry::track_app_open(&settings_file).await;
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_info, get_settings, save_settings,
            list_instances, get_instance, create_instance, update_instance, delete_instance, duplicate_instance, set_instance_icon,
            import_mrpack, export_mrpack,
            list_modrinth_app_profiles, import_modrinth_app_profile,
            list_content, toggle_content, remove_content, add_content_files, installed_projects, check_updates, apply_updates,
            list_worlds, list_screenshots, list_log_files, read_instance_file,
            list_servers, save_servers,
            backup_instance, restore_instance_backup, search_library_content,
            launch_instance, kill_instance, game_states, game_log, install_instance,
            mc_versions, mc_version_art, loader_versions,
            modrinth_search, modrinth_project, modrinth_versions, modrinth_tags, modrinth_install, modrinth_install_modpack, modrinth_version_deps,
            list_accounts, set_active_account, remove_account, add_offline_account, account_cosmetics, set_account_cape, msa_login, msa_cancel,
            list_java, install_java, open_data_path
        ])
        .run(tauri::generate_context!())
        .expect("error while running Cairn");
}
