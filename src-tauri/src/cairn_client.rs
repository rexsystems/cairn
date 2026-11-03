use crate::{instance::Instance, util};
use anyhow::{bail, Context, Result};
use std::io::Read;
use std::path::Path;

const MANAGED_FILE: &str = "cairn-client-managed.jar";
const MARKER: &str = "META-INF/cairn-client-managed";

pub fn artifact(inst: &Instance) -> Option<&'static [u8]> {
    if inst.use_horizon { return None; }
    match (inst.loader.as_str(), inst.mc_version.as_str()) {
        ("forge", "1.8") => Some(include_bytes!("../client-mods/cairn-client-forge-1.8.jar")),
        ("forge", "1.8.8") => Some(include_bytes!("../client-mods/cairn-client-forge-1.8.8.jar")),
        ("forge", "1.8.9") => Some(include_bytes!("../client-mods/cairn-client-forge-1.8.9.jar")),
        ("fabric", "1.20.1") => Some(include_bytes!("../client-mods/cairn-client-fabric-1.20.1.jar")),
        ("fabric", "1.21.1") => Some(include_bytes!("../client-mods/cairn-client-fabric-1.21.1.jar")),
        ("fabric", "26.3") => Some(include_bytes!("../client-mods/cairn-client-fabric-26.3.jar")),
        _ => None,
    }
}

pub fn validate(inst: &Instance) -> Result<()> {
    if inst.use_cairn && artifact(inst).is_none() {
        bail!("Cairn Client has builds for Forge 1.8, 1.8.8, 1.8.9 and Fabric 1.20.1, 1.21.1, 26.3. Disable Cairn Client before changing to this version or loader.");
    }
    if inst.use_cairn && inst.loader == "fabric" {
        if let Some(version) = &inst.loader_version {
            if crate::versioning::fabric_any(&[">=0.20.1".into()], version) == Some(false) {
                bail!("Cairn Client needs Fabric Loader 0.20.1 or newer. Update the loader in Options first.");
            }
        }
    }
    Ok(())
}

fn owned_file(path: &Path) -> bool {
    (|| -> Result<bool> {
        if !std::fs::symlink_metadata(path)?.file_type().is_file() { return Ok(false); }
        let mut zip = zip::ZipArchive::new(std::fs::File::open(path)?)?;
        let mut marker = String::new();
        zip.by_name(MARKER)?.take(64).read_to_string(&mut marker)?;
        Ok(marker.trim() == "app.cairn.client")
    })().unwrap_or(false)
}

pub fn planned_mods(dir: &Path, inst: &Instance, mut mods: Vec<crate::modmeta::JarMod>) -> Vec<crate::modmeta::JarMod> {
    if owned_file(&dir.join("mods").join(MANAGED_FILE)) { mods.retain(|item| item.file != MANAGED_FILE); }
    let manual = mods.iter().any(|item| item.enabled && item.info.as_ref().is_some_and(|info| info.ids.iter().any(|id| id == "cairn_client")));
    if inst.use_cairn && !manual {
        if let Some(info) = artifact(inst).and_then(crate::modmeta::read_bytes) {
            mods.push(crate::modmeta::JarMod { file: MANAGED_FILE.into(), enabled: true, info: Some(info) });
        }
    }
    mods
}

pub fn sync(dir: &Path, inst: &Instance) -> Result<()> {
    validate(inst)?;
    let destination = dir.join("mods").join(MANAGED_FILE);
    if let Ok(metadata) = std::fs::symlink_metadata(&destination) {
        if !metadata.file_type().is_file() { bail!("{} must be a regular file", destination.display()); }
        if !owned_file(&destination) { bail!("{} already exists and was not created by Cairn Client. Move or rename it first.", destination.display()); }
    }
    if !inst.use_cairn {
        if destination.exists() { std::fs::remove_file(&destination).context("disabling Cairn Client")?; }
        return Ok(());
    }
    let bytes = artifact(inst).context("Cairn Client build is unavailable")?;
    let manual_cairn = crate::modmeta::scan(dir).into_iter().any(|item| {
        item.enabled && item.file != MANAGED_FILE
            && item.info.is_some_and(|info| info.ids.iter().any(|id| id == "cairn_client"))
    });
    if manual_cairn {
        if destination.exists() { std::fs::remove_file(&destination)?; }
        return Ok(());
    }
    if std::fs::read(&destination).ok().as_deref() != Some(bytes) {
        util::write_bytes_atomic(&destination, bytes).context("installing Cairn Client")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn instance() -> Instance { Instance { loader: "fabric".into(), mc_version: "26.3".into(), use_cairn: true, ..Default::default() } }
    fn directory() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("cairn-client-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(dir.join("mods")).unwrap();
        dir
    }
    #[test]
    fn enabling_and_disabling_preserves_other_mods_and_configuration() {
        let dir = directory();
        let mut inst = instance();
        std::fs::write(dir.join("mods/user.jar"), b"user mod").unwrap();
        std::fs::create_dir_all(dir.join("config")).unwrap();
        std::fs::write(dir.join("config/cairn-client.properties"), b"FPS.enabled=false").unwrap();
        sync(&dir, &inst).unwrap();
        assert!(dir.join("mods").join(MANAGED_FILE).exists());
        sync(&dir, &inst).unwrap();
        inst.use_cairn = false;
        sync(&dir, &inst).unwrap();
        assert!(!dir.join("mods").join(MANAGED_FILE).exists());
        assert_eq!(std::fs::read(dir.join("mods/user.jar")).unwrap(), b"user mod");
        assert_eq!(std::fs::read(dir.join("config/cairn-client.properties")).unwrap(), b"FPS.enabled=false");
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn refuses_to_overwrite_an_unmanaged_file() {
        let dir = directory();
        std::fs::write(dir.join("mods").join(MANAGED_FILE), b"user file").unwrap();
        assert!(sync(&dir, &instance()).is_err());
        let mut disabled = instance(); disabled.use_cairn = false;
        assert!(sync(&dir, &disabled).is_err());
        assert_eq!(std::fs::read(dir.join("mods").join(MANAGED_FILE)).unwrap(), b"user file");
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn reuses_a_manually_installed_cairn_without_changing_it() {
        let dir = directory();
        let mut inst = instance();
        let manual = dir.join("mods/my-cairn.jar");
        std::fs::write(&manual, artifact(&inst).unwrap()).unwrap();
        sync(&dir, &inst).unwrap();
        assert!(!dir.join("mods").join(MANAGED_FILE).exists());
        inst.use_cairn = false;
        sync(&dir, &inst).unwrap();
        assert_eq!(std::fs::read(&manual).unwrap(), artifact(&inst).unwrap());
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn older_instances_default_to_normal_mode() {
        let inst: Instance = serde_json::from_str(r#"{"name":"Existing instance"}"#).unwrap();
        assert!(!inst.use_cairn);
    }
    #[test]
    fn bundled_builds_match_their_game_versions_and_use_the_normal_font() {
        for (loader, version, major) in [("forge", "1.8", 52), ("forge", "1.8.8", 52), ("forge", "1.8.9", 52), ("fabric", "1.20.1", 61), ("fabric", "1.21.1", 65), ("fabric", "26.3", 69)] {
            let inst = Instance { loader: loader.into(), mc_version: version.into(), use_cairn: true, ..Default::default() };
            let mut zip = zip::ZipArchive::new(std::io::Cursor::new(artifact(&inst).unwrap())).unwrap();
            let mut header = [0; 8];
            zip.by_name("app/cairn/client/CairnClient.class").unwrap().read_exact(&mut header).unwrap();
            assert_eq!(u16::from_be_bytes([header[6], header[7]]), major);
            assert!(zip.by_name("app/cairn/client/CairnMenu.class").is_ok());
            for name in zip.file_names() {
                assert!(!name.ends_with(".ttf") && !name.ends_with(".woff") && !name.contains("CairnFont") && !name.contains("fabric-resource-loader"), "unexpected custom font dependency: {name}");
            }
            let mut metadata = String::new();
            zip.by_name(if loader == "fabric" { "fabric.mod.json" } else { "mcmod.info" }).unwrap().read_to_string(&mut metadata).unwrap();
            let metadata: serde_json::Value = serde_json::from_str(&metadata).unwrap();
            assert_eq!(if loader == "fabric" { metadata["depends"]["minecraft"].as_str() } else { metadata[0]["mcversion"].as_str() }, Some(version));
        }
    }
    #[test]
    fn rejects_an_old_fabric_loader() {
        let mut inst = instance();
        inst.loader_version = Some("0.19.5".into());
        assert!(validate(&inst).is_err());
        inst.loader_version = Some("0.20.1".into());
        assert!(validate(&inst).is_ok());
    }
    #[test]
    fn prelaunch_checks_use_the_next_launch_mode_without_changing_files() {
        let dir = directory();
        let mut inst = instance();
        let mods = planned_mods(&dir, &inst, vec![]);
        assert_eq!(mods.len(), 1);
        assert!(mods[0].info.as_ref().unwrap().ids.iter().any(|id| id == "cairn_client"));
        assert!(!dir.join("mods").join(MANAGED_FILE).exists());
        sync(&dir, &inst).unwrap();
        let bytes = std::fs::read(dir.join("mods").join(MANAGED_FILE)).unwrap();
        inst.mc_version = "1.20.1".into();
        let mods = planned_mods(&dir, &inst, crate::modmeta::scan(&dir));
        let ctx = crate::compat::Ctx { loader: "fabric", mc_version: "1.20.1", loader_version: Some("0.20.1") };
        assert!(crate::compat::analyze(&mods, &crate::content::ContentIndex::default(), &ctx).is_empty());
        inst.use_cairn = false;
        assert!(planned_mods(&dir, &inst, crate::modmeta::scan(&dir)).is_empty());
        assert_eq!(std::fs::read(dir.join("mods").join(MANAGED_FILE)).unwrap(), bytes);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn rejects_unsupported_versions_and_loaders_before_changing_files() {
        let mut inst = instance();
        inst.mc_version = "1.8.9".into();
        assert!(validate(&inst).is_err());
        inst.loader = "forge".into();
        assert!(validate(&inst).is_ok());
        inst.use_horizon = true;
        assert!(validate(&inst).is_err());
        inst.use_cairn = false;
        assert!(validate(&inst).is_ok());
    }
}
