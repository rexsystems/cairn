//! Import instances from Modrinth App / Theseus / RexLauncher (same schema, different data dirs),
//! plus Prism Launcher, PolyMC and MultiMC instances found by `crate::prism`.
use crate::instance::{self, Instance, PackSource};
use crate::paths::Paths;
use crate::tasks::Task;
use crate::util;
use anyhow::{anyhow, bail, Context, Result};
use rusqlite::{Connection, OpenFlags};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ModrinthAppProfile {
    /// Stable key for import (absolute game dir).
    pub key: String,
    pub name: String,
    pub mc_version: String,
    pub loader: String,
    pub loader_version: Option<String>,
    pub path: String,
    pub icon_path: Option<String>,
    pub source_app: String,
    pub created: i64,
    pub last_played: Option<i64>,
    pub play_seconds: u64,
    pub memory_mb: Option<u32>,
    pub pack_project_id: Option<String>,
    pub pack_version_id: Option<String>,
}

const COPY_ENTRIES: &[&str] = &[
    "mods",
    "config",
    "resourcepacks",
    "shaderpacks",
    "saves",
    "screenshots",
    "datapacks",
    "defaultconfigs",
    "data",
    ".fabric",
    "options.txt",
    "optionsof.txt",
    "optionsshaders.txt",
    "servers.dat",
    "servers.dat_old",
    "hotbar.nbt",
    "usercache.json",
];

/// Settings dir names used by stock Modrinth and the RexLauncher fork.
const APP_ROOTS: &[(&str, &str)] = &[
    ("Modrinth App", "ModrinthApp"),
    ("RexLauncher", "rexlauncher"),
];

fn candidate_settings_dirs() -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    let Some(data) = dirs::data_dir() else {
        return out;
    };
    for (label, name) in APP_ROOTS {
        let p = data.join(name);
        if p.join("app.db").is_file() {
            out.push(((*label).into(), p));
        }
    }
    out
}

fn open_db(settings_dir: &Path) -> Result<Connection> {
    let db = settings_dir.join("app.db");
    if !db.is_file() {
        bail!("no app.db in {}", settings_dir.display());
    }
    let uri = format!("file:{}?mode=ro", db.to_string_lossy().replace('\\', "/"));
    match Connection::open_with_flags(
        &uri,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
    ) {
        Ok(c) => Ok(c),
        Err(_) => {
            let tmp = std::env::temp_dir().join(format!("cairn-modrinth-db-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&tmp);
            std::fs::create_dir_all(&tmp)?;
            std::fs::copy(&db, tmp.join("app.db"))?;
            for side in ["app.db-wal", "app.db-shm"] {
                let s = settings_dir.join(side);
                if s.is_file() {
                    let _ = std::fs::copy(&s, tmp.join(side));
                }
            }
            Connection::open(tmp.join("app.db")).context("opening Modrinth app.db snapshot")
        }
    }
}

fn config_dir(conn: &Connection, settings_dir: &Path) -> PathBuf {
    // Theseus: custom_dir is the instance/meta root; falls back to settings dir.
    conn.query_row("SELECT custom_dir FROM settings WHERE id = 0", [], |r| r.get::<_, Option<String>>(0))
        .ok()
        .flatten()
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
        .unwrap_or_else(|| settings_dir.to_path_buf())
}

fn normalize_loader(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "fabric" => "fabric".into(),
        "quilt" => "quilt".into(),
        "forge" => "forge".into(),
        "neoforge" | "neo" => "neoforge".into(),
        "vanilla" | "minecraft" | "" => "vanilla".into(),
        other => other.into(),
    }
}

fn to_ms(ts: i64) -> i64 {
    if ts > 10_000_000_000 {
        ts
    } else {
        ts.saturating_mul(1000)
    }
}

fn resolve_game_dir(config_dir: &Path, path: &str) -> PathBuf {
    let p = PathBuf::from(path);
    if p.is_absolute() && p.is_dir() {
        return p;
    }
    let under = config_dir.join("profiles").join(path);
    if under.is_dir() {
        return under;
    }
    p
}

fn table_exists(conn: &Connection, name: &str) -> bool {
    conn.prepare("SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1")
        .ok()
        .and_then(|mut s| s.exists([name]).ok())
        .unwrap_or(false)
}

fn read_profiles_legacy(
    conn: &Connection,
    config_dir: &Path,
    source_app: &str,
) -> Result<Vec<ModrinthAppProfile>> {
    let mut stmt = conn.prepare(
        "SELECT path, name, game_version, mod_loader, mod_loader_version, icon_path,
                created, last_played, submitted_time_played, recent_time_played,
                linked_project_id, linked_version_id, override_mc_memory_max
         FROM profiles",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, Option<String>>(4)?,
            r.get::<_, Option<String>>(5)?,
            r.get::<_, i64>(6)?,
            r.get::<_, Option<i64>>(7)?,
            r.get::<_, i64>(8).unwrap_or(0),
            r.get::<_, i64>(9).unwrap_or(0),
            r.get::<_, Option<String>>(10)?,
            r.get::<_, Option<String>>(11)?,
            r.get::<_, Option<i64>>(12)?,
        ))
    })?;
    let mut out = Vec::new();
    for row in rows.flatten() {
        let (path, name, mc, loader, loader_ver, icon, created, last, submitted, recent, proj, ver, mem) =
            row;
        let dir = resolve_game_dir(config_dir, &path);
        if !dir.is_dir() {
            continue;
        }
        out.push(ModrinthAppProfile {
            key: dir.to_string_lossy().into(),
            name,
            mc_version: mc,
            loader: normalize_loader(&loader),
            loader_version: loader_ver.filter(|s| !s.is_empty()),
            path: dir.to_string_lossy().into(),
            icon_path: icon.filter(|p| Path::new(p).is_file()),
            source_app: source_app.into(),
            created: to_ms(created),
            last_played: last.map(to_ms),
            play_seconds: (submitted.max(0) + recent.max(0)) as u64,
            memory_mb: mem.map(|m| m as u32),
            pack_project_id: proj.filter(|s| !s.is_empty()),
            pack_version_id: ver.filter(|s| !s.is_empty()),
        });
    }
    Ok(out)
}

fn read_instances_modern(
    conn: &Connection,
    config_dir: &Path,
    source_app: &str,
) -> Result<Vec<ModrinthAppProfile>> {
    let mut stmt = conn.prepare(
        "SELECT i.path, i.name, i.icon_path, i.created, i.last_played, i.recent_time_played,
                COALESCE(c.game_version, ''), COALESCE(c.loader, 'vanilla'), c.loader_version
         FROM instances i
         LEFT JOIN instance_content_sets c ON c.id = i.applied_content_set_id",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, Option<String>>(2)?,
            r.get::<_, i64>(3)?,
            r.get::<_, Option<i64>>(4)?,
            r.get::<_, i64>(5).unwrap_or(0),
            r.get::<_, String>(6)?,
            r.get::<_, String>(7)?,
            r.get::<_, Option<String>>(8)?,
        ))
    })?;
    let mut out = Vec::new();
    for row in rows.flatten() {
        let (path, name, icon, created, last, recent, mc, loader, loader_ver) = row;
        let dir = resolve_game_dir(config_dir, &path);
        if !dir.is_dir() || mc.is_empty() {
            continue;
        }
        let icon_path = icon.and_then(|p| {
            let abs = PathBuf::from(&p);
            if abs.is_file() {
                return Some(p);
            }
            let joined = config_dir.join(&p);
            if joined.is_file() {
                Some(joined.to_string_lossy().into())
            } else {
                None
            }
        });
        out.push(ModrinthAppProfile {
            key: dir.to_string_lossy().into(),
            name,
            mc_version: mc,
            loader: normalize_loader(&loader),
            loader_version: loader_ver.filter(|s| !s.is_empty()),
            path: dir.to_string_lossy().into(),
            icon_path,
            source_app: source_app.into(),
            created: to_ms(created),
            last_played: last.map(to_ms),
            play_seconds: recent.max(0) as u64,
            memory_mb: None,
            pack_project_id: None,
            pack_version_id: None,
        });
    }
    Ok(out)
}

fn profiles_from_settings_dir(label: &str, settings_dir: &Path) -> Vec<ModrinthAppProfile> {
    let Ok(conn) = open_db(settings_dir) else {
        return vec![];
    };
    let config = config_dir(&conn, settings_dir);
    let mut out = Vec::new();
    if table_exists(&conn, "profiles") {
        if let Ok(mut v) = read_profiles_legacy(&conn, &config, label) {
            out.append(&mut v);
        }
    }
    if table_exists(&conn, "instances") {
        if let Ok(mut v) = read_instances_modern(&conn, &config, label) {
            out.append(&mut v);
        }
    }
    out
}

pub fn list_profiles() -> Result<Vec<ModrinthAppProfile>> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    let modrinth = candidate_settings_dirs()
        .into_iter()
        .flat_map(|(label, root)| profiles_from_settings_dir(&label, &root));
    for p in modrinth.chain(crate::prism::list()) {
        if seen.insert(p.key.clone()) {
            out.push(p);
        }
    }
    out.sort_by_key(|a| std::cmp::Reverse(a.last_played.unwrap_or(a.created)));
    Ok(out)
}

fn copy_profile_files(src: &Path, dst: &Path, task: &Task) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    let total = COPY_ENTRIES.len() as u64;
    for (i, name) in COPY_ENTRIES.iter().enumerate() {
        task.stage("Copying files", i as u64, total);
        let from = src.join(name);
        if !from.exists() {
            continue;
        }
        let to = dst.join(name);
        if from.is_dir() {
            util::copy_dir(&from, &to)?;
        } else {
            if let Some(parent) = to.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&from, &to)?;
        }
    }
    task.stage("Copying files", total, total);
    Ok(())
}

pub fn import_profile(paths: &Paths, key: &str, task: &Task) -> Result<Instance> {
    let src = list_profiles()?
        .into_iter()
        .find(|p| p.key == key)
        .ok_or_else(|| anyhow!("That profile is no longer available"))?;
    let src_dir = PathBuf::from(&src.path);
    if !src_dir.is_dir() {
        bail!("Profile folder missing: {}", src.path);
    }
    if !instance::LOADERS.contains(&src.loader.as_str()) {
        bail!("Unsupported loader {}", src.loader);
    }

    task.stage("Creating instance", 0, 0);
    let mut inst = Instance {
        id: String::new(),
        name: src.name.clone(),
        mc_version: src.mc_version.clone(),
        loader: src.loader.clone(),
        loader_version: src.loader_version.clone(),
        icon: None,
        memory_mb: src.memory_mb,
        jvm_args: String::new(),
        java_path: None,
        width: None,
        height: None,
        created: src.created,
        last_played: src.last_played,
        play_seconds: src.play_seconds,
        group: Some(format!("Imported from {}", src.source_app)),
        pack: match (&src.pack_project_id, &src.pack_version_id) {
            (Some(p), Some(v)) => Some(PackSource {
                project_id: p.clone(),
                version_id: v.clone(),
                title: src.name.clone(),
                version_number: String::new(),
            }),
            _ => None,
        },
        launch_version: None,
        launch_profiles: vec![],
        active_launch_profile: None,
        use_horizon: false,
    };
    inst = instance::create(paths, inst)?;

    task.stage("Copying files", 0, 0);
    copy_profile_files(&src_dir, &inst.dir(paths), task)?;

    if let Some(icon) = &src.icon_path {
        let icon_src = PathBuf::from(icon);
        if icon_src.is_file() {
            let ext = icon_src.extension().and_then(|e| e.to_str()).unwrap_or("png");
            let dest_name = format!("icon.{ext}");
            let dest = inst.dir(paths).join(&dest_name);
            if std::fs::copy(&icon_src, &dest).is_ok() {
                inst.icon = Some(dest_name);
                instance::save(paths, &inst)?;
            }
        }
    }

    Ok(inst)
}
