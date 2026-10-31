use crate::paths::Paths;
use crate::util;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub const KINDS: [&str; 4] = ["mods", "resourcepacks", "shaderpacks", "datapacks"];

/// Where a file came from, so updates and exports can find it again.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct ContentMeta {
    pub project_id: Option<String>,
    pub version_id: Option<String>,
    pub title: Option<String>,
    pub version_number: Option<String>,
    pub icon_url: Option<String>,
    pub url: Option<String>,
    pub sha1: Option<String>,
    pub sha512: Option<String>,
    pub size: Option<u64>,
    /// "modrinth" or "curseforge"; missing means Modrinth (older indexes).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct ContentIndex {
    /// Keyed by "<kind>/<file name without .disabled>".
    pub files: HashMap<String, ContentMeta>,
}

fn index_path(dir: &Path) -> PathBuf {
    dir.join(".cairn").join("content.json")
}

impl ContentIndex {
    pub fn load(dir: &Path) -> Self {
        util::read_json(&index_path(dir)).unwrap_or_default()
    }
    pub fn save(&self, dir: &Path) -> Result<()> {
        util::write_json_atomic(&index_path(dir), self)
    }
}

pub fn base_name(file: &str) -> &str {
    file.strip_suffix(".disabled").unwrap_or(file)
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ContentItem {
    pub file_name: String,
    pub kind: String,
    pub enabled: bool,
    pub title: String,
    pub version: Option<String>,
    pub authors: Option<String>,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub project_id: Option<String>,
    pub size: u64,
}

fn accepted(kind: &str, name: &str) -> bool {
    let n = base_name(name).to_lowercase();
    match kind {
        "mods" => n.ends_with(".jar"),
        _ => n.ends_with(".zip") || n.ends_with(".jar"),
    }
}

fn display_name(file: &str) -> String {
    base_name(file)
        .trim_end_matches(".jar")
        .trim_end_matches(".zip")
        .to_string()
}

/// List installed content without opening every jar/zip (that freezes big packs).
/// Titles/icons come from `.cairn/content.json` when known; otherwise the file name.
pub fn list(paths: &Paths, dir: &Path, kind: &str) -> Result<Vec<ContentItem>> {
    let _ = paths;
    if !KINDS.contains(&kind) {
        bail!("unknown content kind");
    }
    let index = ContentIndex::load(dir);
    let mut out = vec![];
    let folder = dir.join(kind);
    let Ok(rd) = std::fs::read_dir(&folder) else { return Ok(out) };
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        let path = e.path();
        let is_dir = path.is_dir();
        if !is_dir && !accepted(kind, &name) {
            continue;
        }
        let enabled = !name.ends_with(".disabled");
        let meta = index.files.get(&format!("{kind}/{}", base_name(&name))).cloned().unwrap_or_default();
        out.push(ContentItem {
            title: meta.title.clone().unwrap_or_else(|| display_name(&name)),
            version: meta.version_number.clone(),
            authors: None,
            description: None,
            icon: meta.icon_url.clone(),
            project_id: meta.project_id.clone(),
            size: if is_dir {
                // Folder size walks the tree; skip for list speed (datapack folders).
                e.metadata().map(|m| m.len()).unwrap_or(0)
            } else {
                e.metadata().map(|m| m.len()).unwrap_or(0)
            },
            file_name: name,
            kind: kind.into(),
            enabled,
        });
    }
    out.sort_by_key(|a| a.title.to_lowercase());
    Ok(out)
}

fn checked(dir: &Path, kind: &str, file: &str) -> Result<PathBuf> {
    if !KINDS.contains(&kind) || file.contains('/') || file.contains('\\') || file.contains("..") {
        bail!("invalid file");
    }
    Ok(dir.join(kind).join(file))
}

pub fn toggle(dir: &Path, kind: &str, file: &str) -> Result<String> {
    let p = checked(dir, kind, file)?;
    let new_name = if file.ends_with(".disabled") { base_name(file).to_string() } else { format!("{file}.disabled") };
    std::fs::rename(&p, dir.join(kind).join(&new_name))?;
    Ok(new_name)
}

pub fn remove(dir: &Path, kind: &str, file: &str) -> Result<()> {
    let p = checked(dir, kind, file)?;
    if p.is_dir() { std::fs::remove_dir_all(&p)? } else { std::fs::remove_file(&p)? }
    let mut idx = ContentIndex::load(dir);
    idx.files.remove(&format!("{kind}/{}", base_name(file)));
    idx.save(dir)?;
    Ok(())
}

pub fn add_local(dir: &Path, kind: &str, src: &Path) -> Result<String> {
    let name = src.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    if !accepted(kind, &name) {
        bail!("{name} is not a {} file", if kind == "mods" { ".jar" } else { ".zip" });
    }
    let dest = checked(dir, kind, &name)?;
    std::fs::create_dir_all(dest.parent().unwrap())?;
    std::fs::copy(src, &dest)?;
    Ok(name)
}

/// Project ids installed in an instance (for "Installed" markers in Discover).
pub fn installed_projects(dir: &Path) -> Vec<String> {
    ContentIndex::load(dir).files.values().filter_map(|m| m.project_id.clone()).collect()
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ContentHit {
    pub instance_id: String,
    pub instance_name: String,
    pub kind: String,
    pub file_name: String,
    pub title: String,
    pub project_id: Option<String>,
}

/// Find installed content across all instances by title, file name, or project id.
pub fn search_all(paths: &Paths, query: &str) -> Result<Vec<ContentHit>> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Ok(vec![]);
    }
    let mut out = Vec::new();
    for inst in crate::instance::list(paths) {
        let dir = inst.dir(paths);
        for kind in KINDS {
            for item in list(paths, &dir, kind)? {
                let hay = format!(
                    "{} {} {}",
                    item.title,
                    item.file_name,
                    item.project_id.clone().unwrap_or_default()
                )
                .to_lowercase();
                if hay.contains(&q) {
                    out.push(ContentHit {
                        instance_id: inst.id.clone(),
                        instance_name: inst.name.clone(),
                        kind: kind.into(),
                        file_name: item.file_name,
                        title: item.title,
                        project_id: item.project_id,
                    });
                }
            }
        }
    }
    out.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()).then(a.instance_name.cmp(&b.instance_name)));
    Ok(out)
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct World {
    pub folder: String,
    pub icon: Option<String>,
    pub last_played: i64,
    pub size: u64,
}

pub fn worlds(dir: &Path) -> Vec<World> {
    let mut out = vec![];
    if let Ok(rd) = std::fs::read_dir(dir.join("saves")) {
        for e in rd.flatten() {
            let p = e.path();
            if !p.join("level.dat").exists() {
                continue;
            }
            let icon = p.join("icon.png");
            out.push(World {
                folder: e.file_name().to_string_lossy().to_string(),
                icon: icon.exists().then(|| icon.to_string_lossy().to_string()),
                last_played: util::mtime_ms(&p.join("level.dat")),
                size: util::dir_size(&p),
            });
        }
    }
    out.sort_by_key(|a| std::cmp::Reverse(a.last_played));
    out
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub modified: i64,
    pub size: u64,
}

pub fn files_in(dir: &Path, exts: &[&str]) -> Vec<FileEntry> {
    let mut out = vec![];
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if !exts.iter().any(|x| name.to_lowercase().ends_with(x)) {
                continue;
            }
            out.push(FileEntry {
                path: e.path().to_string_lossy().to_string(),
                modified: util::mtime_ms(&e.path()),
                size: e.metadata().map(|m| m.len()).unwrap_or(0),
                name,
            });
        }
    }
    out.sort_by_key(|a| std::cmp::Reverse(a.modified));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn toggle_and_index() {
        let dir = std::env::temp_dir().join(format!("cairn-content-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("mods")).unwrap();
        std::fs::write(dir.join("mods/a.jar"), b"x").unwrap();
        let n = toggle(&dir, "mods", "a.jar").unwrap();
        assert_eq!(n, "a.jar.disabled");
        assert!(dir.join("mods/a.jar.disabled").exists());
        assert_eq!(toggle(&dir, "mods", &n).unwrap(), "a.jar");
        assert!(toggle(&dir, "mods", "../x").is_err());
        let paths = Paths::at(&dir.join("root"));
        let l = list(&paths, &dir, "mods").unwrap();
        assert_eq!(l.len(), 1);
        assert!(l[0].enabled);
        remove(&dir, "mods", "a.jar").unwrap();
        assert!(list(&paths, &dir, "mods").unwrap().is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
