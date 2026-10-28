use crate::paths::Paths;
use crate::util;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct PackSource {
    pub project_id: String,
    pub version_id: String,
    pub title: String,
    pub version_number: String,
}

/// Named JVM / launch variant (e.g. "Shaders off").
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct LaunchProfile {
    pub id: String,
    pub name: String,
    pub jvm_args: String,
    pub memory_mb: Option<u32>,
    pub java_path: Option<String>,
    pub disable_shaders: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct Instance {
    pub id: String,
    pub name: String,
    pub mc_version: String,
    pub loader: String,
    pub loader_version: Option<String>,
    pub icon: Option<String>,
    pub memory_mb: Option<u32>,
    pub jvm_args: String,
    pub java_path: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub created: i64,
    pub last_played: Option<i64>,
    pub play_seconds: u64,
    pub group: Option<String>,
    pub pack: Option<PackSource>,
    /// Version id the game launches with once the loader is installed (e.g. fabric-loader-0.16.9-1.21.1).
    pub launch_version: Option<String>,
    pub launch_profiles: Vec<LaunchProfile>,
    pub active_launch_profile: Option<String>,
    /// Launch Horizon (Rust client) instead of the Java game. Vanilla 26.3 only.
    pub use_horizon: bool,
}

pub const LOADERS: [&str; 5] = ["vanilla", "fabric", "quilt", "forge", "neoforge"];
/// Minecraft version Horizon currently targets.
pub const HORIZON_MC_VERSION: &str = "26.3";

impl Instance {
    pub fn dir(&self, paths: &Paths) -> PathBuf {
        paths.instance_dir(&self.id)
    }
    pub fn icon_path(&self, paths: &Paths) -> Option<String> {
        self.icon.as_ref().map(|i| self.dir(paths).join(i)).filter(|p| p.exists()).map(|p| p.to_string_lossy().to_string())
    }
    pub fn horizon_allowed(&self) -> bool {
        self.loader == "vanilla" && self.mc_version == HORIZON_MC_VERSION
    }
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InstanceView {
    #[serde(flatten)]
    pub inst: Instance,
    pub path: String,
    pub icon_path: Option<String>,
    pub mod_count: usize,
    /// Newest in-game screenshot, used as card and hero art.
    pub cover: Option<String>,
}

fn newest_screenshot(dir: &std::path::Path) -> Option<String> {
    std::fs::read_dir(dir.join("screenshots")).ok()?
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().to_lowercase().ends_with(".png"))
        .max_by_key(|e| e.metadata().and_then(|m| m.modified()).ok())
        .map(|e| e.path().to_string_lossy().to_string())
}

pub fn view(paths: &Paths, inst: Instance) -> InstanceView {
    let dir = inst.dir(paths);
    let mod_count = std::fs::read_dir(dir.join("mods"))
        .map(|rd| rd.flatten().filter(|e| {
            let n = e.file_name().to_string_lossy().to_string();
            n.ends_with(".jar") || n.ends_with(".jar.disabled")
        }).count())
        .unwrap_or(0);
    InstanceView { icon_path: inst.icon_path(paths), cover: newest_screenshot(&dir), path: dir.to_string_lossy().to_string(), mod_count, inst }
}

pub fn list(paths: &Paths) -> Vec<Instance> {
    let mut out = vec![];
    if let Ok(rd) = std::fs::read_dir(&paths.instances) {
        for e in rd.flatten() {
            if let Ok(i) = util::read_json::<Instance>(&e.path().join("instance.json")) {
                out.push(i);
            }
        }
    }
    out.sort_by(|a, b| b.last_played.unwrap_or(b.created).cmp(&a.last_played.unwrap_or(a.created)));
    out
}

pub fn get(paths: &Paths, id: &str) -> Result<Instance> {
    if id.contains('/') || id.contains('\\') || id.contains("..") {
        bail!("invalid instance id");
    }
    util::read_json(&paths.instance_dir(id).join("instance.json"))
}

pub fn save(paths: &Paths, inst: &Instance) -> Result<()> {
    let dir = inst.dir(paths);
    std::fs::create_dir_all(&dir)?;
    util::write_json_atomic(&dir.join("instance.json"), inst)
}

pub fn unique_id(paths: &Paths, name: &str) -> String {
    let base = util::slugify(name);
    let mut id = base.clone();
    let mut n = 2;
    while paths.instance_dir(&id).exists() {
        id = format!("{base}-{n}");
        n += 1;
    }
    id
}

pub fn create(paths: &Paths, mut inst: Instance) -> Result<Instance> {
    if inst.name.trim().is_empty() {
        bail!("Give the instance a name");
    }
    if inst.mc_version.trim().is_empty() {
        bail!("Pick a Minecraft version");
    }
    if !LOADERS.contains(&inst.loader.as_str()) {
        bail!("Unknown loader {}", inst.loader);
    }
    if inst.use_horizon && !(inst.loader == "vanilla" && inst.mc_version == HORIZON_MC_VERSION) {
        bail!("Horizon only works with vanilla Minecraft {HORIZON_MC_VERSION}.");
    }
    if !inst.horizon_allowed() {
        inst.use_horizon = false;
    }
    inst.name = inst.name.trim().to_string();
    inst.id = unique_id(paths, &inst.name);
    inst.created = util::now_ms();
    let dir = inst.dir(paths);
    for sub in ["mods", "resourcepacks", "shaderpacks", "datapacks", "saves", "screenshots", "config"] {
        std::fs::create_dir_all(dir.join(sub))?;
    }
    if inst.use_horizon {
        std::fs::create_dir_all(dir.join(crate::horizon_mods::DIR))?;
    }
    save(paths, &inst)?;
    Ok(inst)
}

pub fn duplicate(paths: &Paths, id: &str) -> Result<Instance> {
    let src = get(paths, id)?;
    let mut copy = src.clone();
    copy.name = format!("{} (copy)", src.name);
    copy.id = unique_id(paths, &copy.name);
    copy.created = util::now_ms();
    copy.last_played = None;
    copy.play_seconds = 0;
    util::copy_dir(&src.dir(paths), &copy.dir(paths))?;
    save(paths, &copy)?;
    Ok(copy)
}

pub fn delete(paths: &Paths, id: &str) -> Result<()> {
    let inst = get(paths, id)?;
    std::fs::remove_dir_all(inst.dir(paths))?;
    Ok(())
}
