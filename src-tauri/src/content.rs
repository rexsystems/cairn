use crate::paths::Paths;
use crate::util;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Read;
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

#[derive(Default)]
struct JarMeta {
    name: Option<String>,
    version: Option<String>,
    description: Option<String>,
    authors: Option<String>,
    icon: Option<String>,
}

fn read_entry(z: &mut zip::ZipArchive<std::fs::File>, name: &str) -> Option<Vec<u8>> {
    let mut e = z.by_name(name).ok()?;
    let mut buf = vec![];
    e.read_to_end(&mut buf).ok()?;
    Some(buf)
}

fn people(v: &serde_json::Value) -> Option<String> {
    let names: Vec<String> = match v {
        serde_json::Value::Array(a) => a.iter().filter_map(|p| p.as_str().map(String::from).or_else(|| p.get("name").and_then(|n| n.as_str()).map(String::from))).collect(),
        serde_json::Value::Object(o) => o.keys().cloned().collect(),
        _ => vec![],
    };
    if names.is_empty() { None } else { Some(names.join(", ")) }
}

/// Reads name/version/icon from fabric.mod.json, quilt.mod.json, (neo)forge mods.toml or mcmod.info.
fn jar_meta(path: &Path, cache: &Path) -> JarMeta {
    let mut m = JarMeta::default();
    let Ok(f) = std::fs::File::open(path) else { return m };
    let Ok(mut z) = zip::ZipArchive::new(f) else { return m };
    let mut icon_entry: Option<String> = None;
    if let Some(b) = read_entry(&mut z, "fabric.mod.json") {
        if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&b) {
            m.name = v["name"].as_str().or(v["id"].as_str()).map(String::from);
            m.version = v["version"].as_str().map(String::from);
            m.description = v["description"].as_str().map(String::from);
            m.authors = people(&v["authors"]);
            icon_entry = match &v["icon"] {
                serde_json::Value::String(s) => Some(s.clone()),
                serde_json::Value::Object(o) => o.values().last().and_then(|x| x.as_str()).map(String::from),
                _ => None,
            };
        }
    } else if let Some(b) = read_entry(&mut z, "quilt.mod.json") {
        if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&b) {
            let ql = &v["quilt_loader"];
            m.name = ql.pointer("/metadata/name").and_then(|x| x.as_str()).or(ql["id"].as_str()).map(String::from);
            m.version = ql["version"].as_str().map(String::from);
            m.description = ql.pointer("/metadata/description").and_then(|x| x.as_str()).map(String::from);
            m.authors = ql.pointer("/metadata/contributors").and_then(people);
            icon_entry = ql.pointer("/metadata/icon").and_then(|x| x.as_str()).map(String::from);
        }
    } else if let Some(b) = read_entry(&mut z, "META-INF/neoforge.mods.toml").or_else(|| read_entry(&mut z, "META-INF/mods.toml")) {
        if let Ok(v) = toml::from_str::<toml::Value>(&String::from_utf8_lossy(&b)) {
            if let Some(first) = v.get("mods").and_then(|x| x.as_array()).and_then(|a| a.first()) {
                m.name = first.get("displayName").and_then(|x| x.as_str()).map(String::from);
                m.version = first.get("version").and_then(|x| x.as_str()).filter(|s| !s.contains("${")).map(String::from);
                m.description = first.get("description").and_then(|x| x.as_str()).map(|s| s.trim().to_string());
                m.authors = first.get("authors").and_then(|x| x.as_str()).map(String::from);
                icon_entry = first.get("logoFile").or(v.get("logoFile")).and_then(|x| x.as_str()).map(String::from);
            }
        }
    } else if let Some(b) = read_entry(&mut z, "mcmod.info") {
        if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&b) {
            let first = if v.is_array() { v[0].clone() } else { v["modList"][0].clone() };
            m.name = first["name"].as_str().map(String::from);
            m.version = first["version"].as_str().map(String::from);
            m.description = first["description"].as_str().map(String::from);
        }
    } else if let Some(b) = read_entry(&mut z, "pack.mcmeta") {
        if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&b) {
            m.description = v.pointer("/pack/description").and_then(|x| x.as_str()).map(String::from);
        }
        icon_entry = Some("pack.png".into());
    }
    if let Some(entry) = icon_entry {
        let key = format!("{:x}", md5_of(path.to_string_lossy().as_bytes(), util::mtime_ms(path)));
        let out = cache.join("icons").join(format!("{key}.png"));
        if out.exists() {
            m.icon = Some(out.to_string_lossy().to_string());
        } else if let Some(bytes) = read_entry(&mut z, entry.trim_start_matches('/')) {
            let _ = std::fs::create_dir_all(out.parent().unwrap());
            if std::fs::write(&out, bytes).is_ok() {
                m.icon = Some(out.to_string_lossy().to_string());
            }
        }
    }
    m
}

fn md5_of(b: &[u8], salt: i64) -> u128 {
    use md5::{Digest, Md5};
    let mut h = Md5::new();
    h.update(b);
    h.update(salt.to_le_bytes());
    u128::from_be_bytes(h.finalize().into())
}

fn accepted(kind: &str, name: &str) -> bool {
    let n = base_name(name).to_lowercase();
    match kind {
        "mods" => n.ends_with(".jar"),
        _ => n.ends_with(".zip") || n.ends_with(".jar"),
    }
}

pub fn list(paths: &Paths, dir: &Path, kind: &str) -> Result<Vec<ContentItem>> {
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
        let jm = if is_dir { JarMeta::default() } else { jar_meta(&path, &paths.cache) };
        out.push(ContentItem {
            title: meta.title.clone().or(jm.name).unwrap_or_else(|| base_name(&name).trim_end_matches(".jar").trim_end_matches(".zip").to_string()),
            version: meta.version_number.clone().or(jm.version),
            authors: jm.authors,
            description: jm.description,
            icon: meta.icon_url.clone().or(jm.icon),
            project_id: meta.project_id.clone(),
            size: if is_dir { util::dir_size(&path) } else { e.metadata().map(|m| m.len()).unwrap_or(0) },
            file_name: name,
            kind: kind.into(),
            enabled,
        });
    }
    out.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
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
    out.sort_by(|a, b| b.last_played.cmp(&a.last_played));
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
    out.sort_by(|a, b| b.modified.cmp(&a.modified));
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
