//! Manual CurseForge downloads. Some authors turn off third-party distribution, so the
//! file has to come from their CurseForge page. We open that page, watch the Downloads
//! folder, and move the file into the instance once its hash matches what CurseForge
//! reports. Nothing is accepted on file name alone.

use crate::content::ContentIndex;
use crate::deps::{self, PendingDownload};
use crate::paths::Paths;
use anyhow::{anyhow, bail, Result};
use once_cell::sync::Lazy;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

/// How long a watcher keeps looking after the last page was opened.
const WATCH_FOR: Duration = Duration::from_secs(30 * 60);
const POLL: Duration = Duration::from_secs(2);

/// Whether a file on disk is the pending download, by size and then sha1 or fingerprint.
pub fn matches(path: &Path, p: &PendingDownload) -> bool {
    let Ok(md) = std::fs::metadata(path) else { return false };
    if !md.is_file() {
        return false;
    }
    if let Some(size) = p.size {
        if md.len() != size {
            return false;
        }
    }
    if let Some(want) = &p.sha1 {
        return crate::util::sha1_file(path).map(|h| h.eq_ignore_ascii_case(want)).unwrap_or(false);
    }
    if let Some(fp) = p.fingerprint {
        return crate::curseforge::fingerprint_file(path).map(|h| h == fp).unwrap_or(false);
    }
    false
}

/// Copies a verified file into the instance, replaces older versions and records it.
pub fn adopt(dir: &Path, p: &PendingDownload, src: &Path) -> Result<()> {
    if p.file_name.contains(['/', '\\']) || p.file_name.contains("..") || p.folder.contains("..") {
        bail!("refusing unsafe file name {}", p.file_name);
    }
    let key = format!("{}/{}", p.folder, p.file_name);
    let dest = dir.join(&key);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let part = dest.with_extension("cairn-part");
    std::fs::copy(src, &part)?;
    std::fs::rename(&part, &dest)?;
    let mut index = ContentIndex::load(dir);
    deps::swap_in(dir, &mut index, &key, &p.replaces)?;
    index.files.insert(
        key,
        crate::content::ContentMeta {
            project_id: Some(p.project_id.clone()),
            version_id: Some(p.file_id.clone()),
            title: Some(p.title.clone()),
            version_number: Some(p.version_number.clone()),
            icon_url: p.icon.clone(),
            url: None,
            sha1: p.sha1.clone(),
            sha512: None,
            size: p.size,
            source: Some("curseforge".into()),
        },
    );
    index.save(dir)?;
    let mut list = deps::load_pending(dir);
    list.retain(|x| !(x.project_id == p.project_id && x.file_id == p.file_id));
    deps::save_pending(dir, &list)
}

/// Files in `folder` worth hashing: right size for some pending item, not a partial download.
fn candidates(folder: &Path, pending: &[PendingDownload]) -> Vec<PathBuf> {
    let sizes: HashSet<u64> = pending.iter().filter_map(|p| p.size).collect();
    let names: HashSet<&str> = pending.iter().filter(|p| p.size.is_none()).map(|p| p.file_name.as_str()).collect();
    let Ok(rd) = std::fs::read_dir(folder) else { return vec![] };
    rd.flatten()
        .map(|e| e.path())
        .filter(|p| {
            let n = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if n.ends_with(".part") || n.ends_with(".crdownload") || n.ends_with(".download") || n.starts_with('.') {
                return false;
            }
            let len = std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
            sizes.contains(&len) || names.contains(n)
        })
        .collect()
}

/// One pass over `folder`. `seen` remembers files (path, mtime) that matched nothing so
/// they are not hashed again every poll. Returns the titles that were picked up.
pub fn sweep(dir: &Path, folder: &Path, seen: &mut HashSet<(PathBuf, i64)>) -> Vec<String> {
    let pending = deps::load_pending(dir);
    if pending.is_empty() {
        return vec![];
    }
    let mut got = vec![];
    for f in candidates(folder, &pending) {
        let stamp = (f.clone(), crate::util::mtime_ms(&f));
        if seen.contains(&stamp) {
            continue;
        }
        match pending.iter().find(|p| matches(&f, p)) {
            Some(p) if adopt(dir, p, &f).is_ok() => got.push(p.title.clone()),
            _ => {
                seen.insert(stamp);
            }
        }
    }
    got
}

/// Imports a file the user picked by hand. It must match one of the pending downloads.
pub fn import_file(dir: &Path, file: &Path) -> Result<String> {
    let pending = deps::load_pending(dir);
    let p = pending
        .iter()
        .find(|p| matches(file, p))
        .ok_or_else(|| anyhow!("This file does not match any download Cairn is waiting for. Check that it is the exact version listed."))?;
    adopt(dir, p, file)?;
    Ok(p.title.clone())
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PickedUp {
    pub instance_id: String,
    pub titles: Vec<String>,
    pub remaining: usize,
}

/// Instance id -> when its watcher should give up. Restarting extends the deadline.
static WATCHERS: Lazy<Mutex<HashMap<String, std::time::Instant>>> = Lazy::new(|| Mutex::new(HashMap::new()));

pub fn downloads_dir() -> Option<PathBuf> {
    dirs::download_dir().or_else(|| dirs::home_dir().map(|h| h.join("Downloads"))).filter(|d| d.is_dir())
}

/// Starts (or extends) a background watcher for one instance.
pub fn watch(app: AppHandle, paths: Paths, instance_id: String) -> Result<()> {
    let folder = downloads_dir().ok_or_else(|| anyhow!("Could not find your Downloads folder. Use \"Choose file\" instead."))?;
    let deadline = std::time::Instant::now() + WATCH_FOR;
    {
        let mut w = WATCHERS.lock().unwrap_or_else(|e| e.into_inner());
        let running = w.contains_key(&instance_id);
        w.insert(instance_id.clone(), deadline);
        if running {
            return Ok(());
        }
    }
    tauri::async_runtime::spawn(async move {
        let mut seen = HashSet::new();
        loop {
            tokio::time::sleep(POLL).await;
            let Ok(inst) = crate::instance::get(&paths, &instance_id) else { break };
            let dir = inst.dir(&paths);
            let (d2, f2, mut s2) = (dir.clone(), folder.clone(), std::mem::take(&mut seen));
            let (titles, s2) = tokio::task::spawn_blocking(move || {
                let t = sweep(&d2, &f2, &mut s2);
                (t, s2)
            })
            .await
            .unwrap_or_default();
            seen = s2;
            let remaining = deps::load_pending(&dir).len();
            if !titles.is_empty() {
                let _ = app.emit("manual-download", PickedUp { instance_id: instance_id.clone(), titles, remaining });
            }
            let expired = WATCHERS.lock().unwrap_or_else(|e| e.into_inner()).get(&instance_id).map(|d| std::time::Instant::now() > *d).unwrap_or(true);
            if remaining == 0 || expired {
                break;
            }
        }
        WATCHERS.lock().unwrap_or_else(|e| e.into_inner()).remove(&instance_id);
    });
    Ok(())
}

pub fn is_watching(instance_id: &str) -> bool {
    WATCHERS.lock().unwrap_or_else(|e| e.into_inner()).contains_key(instance_id)
}

/// Only CurseForge pages are opened from pending entries.
pub fn safe_page(url: &str) -> bool {
    url.starts_with("https://www.curseforge.com/") || url.starts_with("https://curseforge.com/")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("cairn-manual-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn pending_for(bytes: &[u8], name: &str) -> PendingDownload {
        use sha1::{Digest, Sha1};
        let sha1 = format!("{:x}", Sha1::digest(bytes));
        PendingDownload {
            project_id: "42".into(),
            file_id: "7".into(),
            title: "Blocked Mod".into(),
            file_name: name.into(),
            folder: "mods".into(),
            download_page: "https://www.curseforge.com/minecraft/mc-mods/blocked/download/7".into(),
            size: Some(bytes.len() as u64),
            sha1: Some(sha1),
            ..Default::default()
        }
    }

    #[test]
    fn picks_up_renamed_download_by_hash_only() {
        let inst = tmp("inst");
        let dl = tmp("dl");
        let p = pending_for(b"the real jar", "blocked-1.0.jar");
        deps::save_pending(&inst, &[p]).unwrap();
        // Same size, different bytes: must be ignored.
        std::fs::write(dl.join("blocked-1.0.jar"), b"the fake jar").unwrap();
        std::fs::write(dl.join("blocked-1.0 (1).jar"), b"the real jar").unwrap();
        std::fs::write(dl.join("blocked-1.0.jar.part"), b"the real jar").unwrap();
        let mut seen = HashSet::new();
        let got = sweep(&inst, &dl, &mut seen);
        assert_eq!(got, vec!["Blocked Mod"]);
        assert_eq!(std::fs::read(inst.join("mods/blocked-1.0.jar")).unwrap(), b"the real jar");
        assert!(deps::load_pending(&inst).is_empty());
        let index = ContentIndex::load(&inst);
        assert_eq!(index.files["mods/blocked-1.0.jar"].source.as_deref(), Some("curseforge"));
        assert_eq!(seen.len(), 1, "the fake file is remembered, not rehashed");
        std::fs::remove_dir_all(&inst).unwrap();
        std::fs::remove_dir_all(&dl).unwrap();
    }

    #[test]
    fn fingerprint_used_without_sha1() {
        let d = tmp("fp");
        std::fs::write(d.join("a.jar"), b"abc def").unwrap();
        let mut p = pending_for(b"abc def", "a.jar");
        p.sha1 = None;
        p.fingerprint = Some(crate::curseforge::fingerprint(b"abc def"));
        assert!(matches(&d.join("a.jar"), &p));
        p.fingerprint = Some(1);
        assert!(!matches(&d.join("a.jar"), &p));
        p.fingerprint = None;
        assert!(!matches(&d.join("a.jar"), &p), "no hash means no match");
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn import_rejects_wrong_file_and_replaces_old_version() {
        let inst = tmp("imp");
        std::fs::create_dir_all(inst.join("mods")).unwrap();
        std::fs::write(inst.join("mods/blocked-0.9.jar"), b"old").unwrap();
        let mut p = pending_for(b"new bytes", "blocked-1.0.jar");
        p.replaces = vec!["mods/blocked-0.9.jar".into()];
        deps::save_pending(&inst, &[p]).unwrap();
        let wrong = inst.join("wrong.jar");
        std::fs::write(&wrong, b"other").unwrap();
        assert!(import_file(&inst, &wrong).is_err());
        let right = inst.join("right.jar");
        std::fs::write(&right, b"new bytes").unwrap();
        assert_eq!(import_file(&inst, &right).unwrap(), "Blocked Mod");
        assert!(!inst.join("mods/blocked-0.9.jar").exists());
        assert!(inst.join("mods/blocked-1.0.jar").exists());
        std::fs::remove_dir_all(&inst).unwrap();
    }

    #[test]
    fn only_curseforge_pages_open() {
        assert!(safe_page("https://www.curseforge.com/minecraft/mc-mods/x/download/1"));
        assert!(!safe_page("http://www.curseforge.com/x"));
        assert!(!safe_page("https://evil.example/curseforge.com"));
        assert!(!safe_page("file:///etc/passwd"));
    }
}
