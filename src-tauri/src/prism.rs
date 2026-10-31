//! Lists Prism Launcher, PolyMC and MultiMC instances so they can be imported
//! the same way as Modrinth App profiles.
//!
//! Each instance folder has an `instance.cfg` (INI) and an `mmc-pack.json`
//! that names the Minecraft version and the loader as components.
use crate::modrinth_app::ModrinthAppProfile;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// (label, data folder name, Flatpak app id).
const LAUNCHERS: &[(&str, &str, Option<&str>)] = &[
    ("Prism Launcher", "PrismLauncher", Some("org.prismlauncher.PrismLauncher")),
    ("PolyMC", "PolyMC", Some("org.polymc.PolyMC")),
    // MultiMC is portable; only a copy unpacked into the data folder is found.
    ("MultiMC", "MultiMC", None),
];

#[derive(Deserialize)]
struct Pack {
    #[serde(default)]
    components: Vec<Component>,
}

#[derive(Deserialize)]
struct Component {
    uid: String,
    #[serde(default)]
    version: Option<String>,
}

/// Reads the `[General]` keys of an instance.cfg. QSettings quotes values with special characters.
fn parse_cfg(text: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            let v = v.trim();
            let v = v.strip_prefix('"').and_then(|v| v.strip_suffix('"')).unwrap_or(v);
            out.insert(k.trim().to_string(), v.replace("\\\"", "\"").replace("\\\\", "\\"));
        }
    }
    out
}

/// Minecraft version, Cairn loader name and loader version from mmc-pack.json.
fn parse_pack(text: &str) -> Option<(String, String, Option<String>)> {
    let pack: Pack = serde_json::from_str(text).ok()?;
    let mut mc = None;
    let mut loader = ("vanilla", None);
    for c in pack.components {
        match c.uid.as_str() {
            "net.minecraft" => mc = c.version,
            "net.fabricmc.fabric-loader" => loader = ("fabric", c.version),
            "org.quiltmc.quilt-loader" => loader = ("quilt", c.version),
            "net.minecraftforge" => loader = ("forge", c.version),
            "net.neoforged" => loader = ("neoforge", c.version),
            _ => {}
        }
    }
    Some((mc.filter(|v| !v.is_empty())?, loader.0.to_string(), loader.1))
}

fn roots() -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    let data = dirs::data_dir();
    let home = dirs::home_dir();
    for (label, dir, flatpak) in LAUNCHERS {
        if let Some(d) = &data {
            out.push((label.to_string(), d.join(dir)));
        }
        if let (Some(h), Some(app)) = (&home, flatpak) {
            out.push((label.to_string(), h.join(".var/app").join(app).join("data").join(dir)));
        }
    }
    out
}

/// The instances folder: `InstanceDir` from the launcher config, else `instances`.
fn instances_dir(root: &Path) -> PathBuf {
    let cfg = ["prismlauncher.cfg", "polymc.cfg", "multimc.cfg"]
        .iter()
        .find_map(|f| std::fs::read_to_string(root.join(f)).ok())
        .map(|t| parse_cfg(&t))
        .unwrap_or_default();
    match cfg.get("InstanceDir").filter(|d| !d.is_empty()) {
        Some(d) => root.join(d), // join keeps absolute paths as they are
        None => root.join("instances"),
    }
}

fn read_instance(label: &str, root: &Path, dir: &Path) -> Option<ModrinthAppProfile> {
    let cfg = parse_cfg(&std::fs::read_to_string(dir.join("instance.cfg")).ok()?);
    let (mc_version, loader, loader_version) = parse_pack(&std::fs::read_to_string(dir.join("mmc-pack.json")).ok()?)?;
    let game_dir = [".minecraft", "minecraft"].iter().map(|d| dir.join(d)).find(|d| d.is_dir())?;
    let num = |k: &str| cfg.get(k).and_then(|v| v.parse::<i64>().ok());
    let icon_path = cfg
        .get("iconKey")
        .filter(|k| !k.is_empty() && *k != "default")
        .and_then(|k| {
            ["png", "jpg", "jpeg"]
                .iter()
                .flat_map(|ext| [root.join("icons").join(format!("{k}.{ext}")), dir.join(format!("{k}.{ext}"))])
                .find(|p| p.is_file())
        })
        .map(|p| p.to_string_lossy().to_string());
    let created = std::fs::metadata(dir.join("instance.cfg"))
        .and_then(|m| m.created().or_else(|_| m.modified()))
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let memory_mb = (cfg.get("OverrideMemory").map(String::as_str) == Some("true"))
        .then(|| num("MaxMemAlloc"))
        .flatten()
        .and_then(|m| u32::try_from(m).ok());
    Some(ModrinthAppProfile {
        key: game_dir.to_string_lossy().into(),
        name: cfg.get("name").cloned().unwrap_or_else(|| dir.file_name().unwrap_or_default().to_string_lossy().into()),
        mc_version,
        loader,
        loader_version,
        path: game_dir.to_string_lossy().into(),
        icon_path,
        source_app: label.into(),
        created,
        last_played: num("lastLaunchTime").filter(|t| *t > 0),
        play_seconds: num("totalTimePlayed").unwrap_or(0).max(0) as u64,
        memory_mb,
        pack_project_id: None,
        pack_version_id: None,
    })
}

pub fn list_in(label: &str, root: &Path) -> Vec<ModrinthAppProfile> {
    let Ok(entries) = std::fs::read_dir(instances_dir(root)) else {
        return vec![];
    };
    entries.flatten().filter_map(|e| read_instance(label, root, &e.path())).collect()
}

pub fn list() -> Vec<ModrinthAppProfile> {
    roots().iter().flat_map(|(label, root)| list_in(label, root)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_components() {
        let pack = r#"{"formatVersion":1,"components":[
            {"uid":"org.lwjgl3","version":"3.3.3"},
            {"uid":"net.minecraft","version":"1.21.1"},
            {"uid":"net.fabricmc.intermediary","version":"1.21.1"},
            {"uid":"net.fabricmc.fabric-loader","version":"0.16.9"}]}"#;
        assert_eq!(parse_pack(pack), Some(("1.21.1".into(), "fabric".into(), Some("0.16.9".into()))));
        assert_eq!(parse_pack(r#"{"components":[{"uid":"net.minecraft","version":"1.8.9"}]}"#), Some(("1.8.9".into(), "vanilla".into(), None)));
        assert_eq!(parse_pack(r#"{"components":[]}"#), None);
    }

    #[test]
    fn imports_an_instance_folder() {
        let root = std::env::temp_dir().join(format!("cairn-prism-test-{}", std::process::id()));
        let inst = root.join("instances").join("Better MC");
        std::fs::create_dir_all(inst.join(".minecraft/mods")).unwrap();
        std::fs::create_dir_all(root.join("icons")).unwrap();
        std::fs::write(root.join("icons/bmc.png"), b"png").unwrap();
        std::fs::write(
            inst.join("instance.cfg"),
            "[General]\nname=\"Better MC, 1.20\"\niconKey=bmc\nlastLaunchTime=1700000000000\ntotalTimePlayed=3600\nOverrideMemory=true\nMaxMemAlloc=6144\n",
        )
        .unwrap();
        std::fs::write(inst.join("mmc-pack.json"), r#"{"components":[{"uid":"net.minecraft","version":"1.20.1"},{"uid":"net.minecraftforge","version":"47.3.0"}]}"#).unwrap();
        std::fs::create_dir_all(root.join("instances/broken")).unwrap();

        let found = list_in("Prism Launcher", &root);
        std::fs::remove_dir_all(&root).ok();
        assert_eq!(found.len(), 1);
        let p = &found[0];
        assert_eq!(p.name, "Better MC, 1.20");
        assert_eq!((p.mc_version.as_str(), p.loader.as_str()), ("1.20.1", "forge"));
        assert_eq!(p.loader_version.as_deref(), Some("47.3.0"));
        assert!(p.path.ends_with(".minecraft"));
        assert!(p.icon_path.as_deref().unwrap().ends_with("bmc.png"));
        assert_eq!((p.last_played, p.play_seconds, p.memory_mb), (Some(1700000000000), 3600, Some(6144)));
    }
}
