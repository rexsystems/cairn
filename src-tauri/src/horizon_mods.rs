//! Horizon `.hmod` packages in an instance's `horizon-mods/` folder.
//! Enable flags live in `horizon-mods.txt` as `mod.<id>:true` (see Horizon docs).

use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::{Path, PathBuf};

pub const DIR: &str = "horizon-mods";
pub const FILE: &str = "horizon-mods.txt";

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HorizonModItem {
    pub file_name: String,
    pub id: String,
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub authors: Option<String>,
    pub enabled: bool,
    pub size: u64,
    pub capabilities: Vec<String>,
}

#[derive(Default, Clone)]
struct ModList {
    enabled: BTreeSet<String>,
    /// Preserve keybind lines (`key.<id>.<name>:…`) when rewriting.
    keys: BTreeMap<String, BTreeMap<String, String>>,
}

impl ModList {
    fn load(game_dir: &Path) -> Self {
        std::fs::read_to_string(game_dir.join(FILE))
            .map(|t| Self::parse(&t))
            .unwrap_or_default()
    }

    fn parse(text: &str) -> Self {
        let enabled = text
            .lines()
            .filter_map(|l| l.split_once(':'))
            .filter(|(_, v)| v.trim() == "true")
            .filter_map(|(k, _)| k.trim().strip_prefix("mod."))
            .map(str::to_owned)
            .collect();
        let mut keys: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
        for (k, v) in text.lines().filter_map(|l| l.split_once(':')) {
            if let Some((id, name)) = k.trim().strip_prefix("key.").and_then(|r| r.split_once('.')) {
                keys.entry(id.to_owned()).or_default().insert(name.to_owned(), v.trim().to_owned());
            }
        }
        Self { enabled, keys }
    }

    fn serialize(&self) -> String {
        let on = self.enabled.iter().map(|id| format!("mod.{id}:true\n"));
        let keys = self.keys.iter().flat_map(|(id, keys)| {
            keys.iter().map(move |(name, key)| format!("key.{id}.{name}:{key}\n"))
        });
        on.chain(keys).collect()
    }

    fn save(&self, game_dir: &Path) -> Result<()> {
        let path = game_dir.join(FILE);
        if self.enabled.is_empty() && self.keys.is_empty() {
            if path.exists() {
                std::fs::remove_file(&path)?;
            }
            return Ok(());
        }
        std::fs::write(path, self.serialize())?;
        Ok(())
    }
}

#[derive(Default)]
struct Manifest {
    id: String,
    name: String,
    version: Option<String>,
    description: Option<String>,
    authors: Option<String>,
    capabilities: Vec<String>,
}

fn read_zip_entry(z: &mut zip::ZipArchive<std::fs::File>, name: &str) -> Option<String> {
    let mut e = z.by_name(name).ok()?;
    let mut s = String::new();
    e.read_to_string(&mut s).ok()?;
    Some(s)
}

fn parse_manifest_toml(text: &str) -> Option<Manifest> {
    let v: toml::Value = toml::from_str(text).ok()?;
    let id = v.get("id")?.as_str()?.to_string();
    if id.is_empty() {
        return None;
    }
    let name = v
        .get("name")
        .and_then(|x| x.as_str())
        .unwrap_or(&id)
        .to_string();
    let version = v.get("version").and_then(|x| x.as_str()).map(String::from);
    let description = v.get("description").and_then(|x| x.as_str()).map(String::from);
    let authors = match v.get("authors") {
        Some(toml::Value::Array(a)) => {
            let names: Vec<&str> = a.iter().filter_map(|x| x.as_str()).collect();
            if names.is_empty() {
                None
            } else {
                Some(names.join(", "))
            }
        }
        Some(toml::Value::String(s)) => Some(s.clone()),
        _ => None,
    };
    let capabilities = v
        .get("capabilities")
        .and_then(|x| x.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
        .unwrap_or_default();
    Some(Manifest {
        id,
        name,
        version,
        description,
        authors,
        capabilities,
    })
}

fn read_hmod_manifest(path: &Path) -> Option<Manifest> {
    let f = std::fs::File::open(path).ok()?;
    let mut z = zip::ZipArchive::new(f).ok()?;
    let text = read_zip_entry(&mut z, "horizon-mod.toml")?;
    parse_manifest_toml(&text)
}

fn folder_manifest(dir: &Path) -> Option<Manifest> {
    let text = std::fs::read_to_string(dir.join("horizon-mod.toml")).ok()?;
    parse_manifest_toml(&text)
}

fn mods_dir(game_dir: &Path) -> PathBuf {
    game_dir.join(DIR)
}

fn safe_name(file: &str) -> Result<&str> {
    if file.is_empty() || file.contains('/') || file.contains('\\') || file.contains("..") {
        bail!("invalid mod file name");
    }
    Ok(file)
}

pub fn list(game_dir: &Path) -> Result<Vec<HorizonModItem>> {
    let list = ModList::load(game_dir);
    let dir = mods_dir(game_dir);
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return Ok(out);
    };
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        let path = e.path();
        let meta = if path.is_dir() {
            folder_manifest(&path)
        } else if name.to_lowercase().ends_with(".hmod") {
            read_hmod_manifest(&path)
        } else {
            None
        };
        let Some(m) = meta else { continue };
        out.push(HorizonModItem {
            file_name: name,
            enabled: list.enabled.contains(&m.id),
            id: m.id,
            name: m.name,
            version: m.version,
            description: m.description,
            authors: m.authors,
            size: if path.is_dir() {
                crate::util::dir_size(&path)
            } else {
                e.metadata().map(|x| x.len()).unwrap_or(0)
            },
            capabilities: m.capabilities,
        });
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(out)
}

pub fn set_enabled(game_dir: &Path, mod_id: &str, enabled: bool) -> Result<()> {
    if mod_id.is_empty() || mod_id.contains('.') || mod_id.contains('/') {
        bail!("invalid mod id");
    }
    let mut list = ModList::load(game_dir);
    if enabled {
        list.enabled.insert(mod_id.to_string());
    } else {
        list.enabled.remove(mod_id);
    }
    list.save(game_dir)
}

pub fn toggle(game_dir: &Path, mod_id: &str) -> Result<bool> {
    let list = ModList::load(game_dir);
    let on = !list.enabled.contains(mod_id);
    set_enabled(game_dir, mod_id, on)?;
    Ok(on)
}

pub fn remove(game_dir: &Path, file: &str) -> Result<()> {
    let file = safe_name(file)?;
    let path = mods_dir(game_dir).join(file);
    if !path.exists() {
        bail!("mod not found");
    }
    let id = if path.is_dir() {
        folder_manifest(&path).map(|m| m.id)
    } else {
        read_hmod_manifest(&path).map(|m| m.id)
    };
    if path.is_dir() {
        std::fs::remove_dir_all(&path)?;
    } else {
        std::fs::remove_file(&path)?;
    }
    if let Some(id) = id {
        let mut list = ModList::load(game_dir);
        list.enabled.remove(&id);
        list.keys.remove(&id);
        list.save(game_dir)?;
    }
    Ok(())
}

pub fn add_local(game_dir: &Path, src: &Path) -> Result<HorizonModItem> {
    let name = src
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    if !name.to_lowercase().ends_with(".hmod") {
        bail!("{name} is not a .hmod file");
    }
    safe_name(&name)?;
    let dest_dir = mods_dir(game_dir);
    std::fs::create_dir_all(&dest_dir)?;
    let dest = dest_dir.join(&name);
    std::fs::copy(src, &dest).with_context(|| format!("copying {}", src.display()))?;
    let m = read_hmod_manifest(&dest).ok_or_else(|| anyhow::anyhow!("not a valid .hmod (missing horizon-mod.toml)"))?;
    Ok(HorizonModItem {
        file_name: name,
        enabled: ModList::load(game_dir).enabled.contains(&m.id),
        id: m.id,
        name: m.name,
        version: m.version,
        description: m.description,
        authors: m.authors,
        size: dest.metadata().map(|x| x.len()).unwrap_or(0),
        capabilities: m.capabilities,
    })
}

/// Copy a bundled sample `.hmod` into the instance and turn it on.
pub fn install_bundled(game_dir: &Path, bundled: &Path, enable: bool) -> Result<HorizonModItem> {
    let item = add_local(game_dir, bundled)?;
    if enable {
        set_enabled(game_dir, &item.id, true)?;
    }
    Ok(list(game_dir)?
        .into_iter()
        .find(|m| m.id == item.id)
        .unwrap_or(item))
}

/// Cairn-shipped sample mods (embedded under `resources/horizon-mods/`).
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BundledHorizonMod {
    pub id: String,
    pub name: String,
    pub description: String,
    pub file_name: String,
}

pub fn bundled_catalog() -> Vec<BundledHorizonMod> {
    vec![
        BundledHorizonMod {
            id: "cairn_zoom".into(),
            name: "Cairn Zoom".into(),
            description: "Hold Zoom key (rebind in Options → Mods). Scroll to change level.".into(),
            file_name: "cairn_zoom.hmod".into(),
        },
        BundledHorizonMod {
            id: "cairn_map".into(),
            name: "Cairn Map".into(),
            description: "Minimap + checkpoints: B opens New (type name, color, icon, place). N = list. Options → Mods.".into(),
            file_name: "cairn_map.hmod".into(),
        },
        BundledHorizonMod {
            id: "cairn_vitals".into(),
            name: "Cairn Vitals".into(),
            description: "Health + hunger on the HUD (AppleSkin-lite).".into(),
            file_name: "cairn_vitals.hmod".into(),
        },
    ]
}

pub fn bundled_bytes(file_name: &str) -> Option<&'static [u8]> {
    match file_name {
        "cairn_zoom.hmod" => Some(include_bytes!("../resources/horizon-mods/cairn_zoom.hmod")),
        "cairn_map.hmod" => Some(include_bytes!("../resources/horizon-mods/cairn_map.hmod")),
        "cairn_vitals.hmod" => Some(include_bytes!("../resources/horizon-mods/cairn_vitals.hmod")),
        _ => None,
    }
}
