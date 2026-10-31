//! Runs the compatibility check for an instance and applies the one-click fixes it suggests.

use crate::compat::{self, Fix, Issue};
use crate::content::ContentIndex;
use crate::deps::{self, norm, PendingDownload, Repo as _, Root, Source, Target};
use crate::instance::{self, Instance};
use crate::paths::Paths;
use crate::tasks::Task;
use anyhow::{anyhow, bail, Result};
use serde::Serialize;

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub issues: Vec<Issue>,
    pub pending: Vec<PendingDownload>,
    pub mods: usize,
}

impl Report {
    pub fn has_errors(&self) -> bool {
        self.issues.iter().any(|i| i.severity == compat::Severity::Error) || !self.pending.is_empty()
    }
}

/// Reads every jar (cached) and reports problems. Blocking; call from spawn_blocking.
pub fn check(paths: &Paths, inst: &Instance) -> Report {
    let dir = inst.dir(paths);
    let mods = crate::modmeta::scan(&dir);
    let index = ContentIndex::load(&dir);
    let ctx = compat::Ctx { loader: &inst.loader, mc_version: &inst.mc_version, loader_version: inst.loader_version.as_deref() };
    let mut issues = compat::analyze(&mods, &index, &ctx);
    let pending = deps::load_pending(&dir);
    if !pending.is_empty() {
        issues.insert(
            0,
            Issue {
                key: "manual-downloads".into(),
                severity: compat::Severity::Error,
                title: format!("{} file(s) still need to be downloaded from CurseForge", pending.len()),
                detail: "Their authors only allow downloads from the CurseForge website. Cairn picks them up from your Downloads folder once you download them.".into(),
                files: pending.iter().map(|p| p.file_name.clone()).collect(),
                fixes: vec![compat::action("Show downloads", Fix::ManualDownloads)],
            },
        );
    }
    Report { issues, pending, mods: mods.len() }
}

/// Loader mod ids whose Modrinth slug differs from the id.
const SLUGS: &[(&str, &str)] = &[
    ("fabric", "fabric-api"),
    ("fabric-api", "fabric-api"),
    ("fabric_api", "fabric-api"),
    ("quilted_fabric_api", "qsl"),
    ("ferritecore", "ferrite-core"),
    ("cloth_config", "cloth-config"),
    ("cloth-config", "cloth-config"),
    ("cloth-config2", "cloth-config"),
    ("forgeconfigapiport", "forge-config-api-port"),
    ("kotlinforforge", "kotlin-for-forge"),
    ("fabric_language_kotlin", "fabric-language-kotlin"),
    ("fabric-language-kotlin", "fabric-language-kotlin"),
    ("yet_another_config_lib_v3", "yacl"),
    ("yet-another-config-lib", "yacl"),
    ("puzzleslib", "puzzles-lib"),
    ("resourcefullib", "resourceful-lib"),
    ("owo", "owo-lib"),
    ("roughlyenoughitems", "rei"),
    ("connectormod", "connector"),
    ("fzzy_config", "fzzy-config"),
    ("architectury", "architectury-api"),
    ("geckolib", "geckolib"),
    ("modmenu", "modmenu"),
    ("sodium", "sodium"),
    ("iris", "iris"),
    ("indium", "indium"),
    ("balm", "balm"),
    ("curios", "curios"),
    ("trinkets", "trinkets"),
    ("terrablender", "terrablender"),
    ("bookshelf", "bookshelf-lib"),
    ("citadel", "citadel"),
    ("creativecore", "creativecore"),
    ("collective", "collective"),
    ("moonlight", "moonlight"),
    ("lithostitched", "lithostitched"),
    ("placebo", "placebo"),
    ("supermartijn642corelib", "supermartijn642s-core-lib"),
    ("supermartijn642configlib", "supermartijn642s-config-lib"),
    ("playeranimator", "playeranimator"),
    ("bettercombat", "better-combat"),
    ("jei", "jei"),
    ("midnightlib", "midnightlib"),
    ("libipn", "libipn"),
    ("cristellib", "cristel-lib"),
    ("zeta", "zeta"),
    ("kotlin", "kotlin-for-forge"),
];

/// Modrinth slugs to try for a loader mod id, most likely first.
pub fn slug_guesses(mod_id: &str) -> Vec<String> {
    let id = mod_id.to_ascii_lowercase();
    let mut out = vec![];
    if let Some((_, s)) = SLUGS.iter().find(|(k, _)| *k == id) {
        out.push(s.to_string());
    }
    for s in [id.clone(), id.replace('_', "-")] {
        if !out.contains(&s) {
            out.push(s);
        }
    }
    out
}

/// Finds a Modrinth project for a loader mod id that has a version for this instance.
pub async fn resolve_mod_id(mod_id: &str, target: &Target) -> Result<String> {
    let repo = crate::modrinth::Repo;
    for slug in slug_guesses(mod_id) {
        let Ok(p) = repo.project(&slug).await else { continue };
        if p.project_type == "mod" && repo.pick(&p, target).await?.is_some() {
            return Ok(p.id);
        }
    }
    let mut facets = vec![vec!["project_type:mod".to_string()], vec![format!("versions:{}", target.mc_version)]];
    if target.loader != "vanilla" {
        facets.push(vec![format!("categories:{}", target.loader)]);
    }
    let params = crate::modrinth::SearchParams { query: mod_id.replace(['_', '-'], " "), facets, index: "relevance".into(), offset: 0, limit: 10 };
    let res = crate::modrinth::search(&params).await?;
    let want = norm(mod_id);
    let hits = res["hits"].as_array().cloned().unwrap_or_default();
    hits.iter()
        .find(|h| norm(h["slug"].as_str().unwrap_or("")) == want || norm(h["title"].as_str().unwrap_or("")) == want)
        .and_then(|h| h["project_id"].as_str().map(String::from))
        .ok_or_else(|| anyhow!("Could not find {mod_id} on Modrinth for Minecraft {} with {}. Search for it in Discover.", target.mc_version, compat::loader_label(&target.loader)))
}

fn mods_file(dir: &std::path::Path, file: &str) -> Result<()> {
    if file.contains(['/', '\\']) || file.contains("..") {
        bail!("refusing unsafe file name {file}");
    }
    if !dir.join("mods").join(file).is_file() {
        bail!("{file} is no longer in the mods folder");
    }
    Ok(())
}

/// Applies one fix and returns a short message for the UI.
pub async fn apply(paths: &Paths, inst: &Instance, fix: &Fix, task: &Task) -> Result<String> {
    let dir = inst.dir(paths);
    match fix {
        Fix::EnableMod { file } | Fix::DisableMod { file } => {
            mods_file(&dir, file)?;
            let enable = matches!(fix, Fix::EnableMod { .. });
            if enable != file.ends_with(".disabled") {
                return Ok("Already done".into());
            }
            let n = crate::content::toggle(&dir, "mods", file)?;
            Ok(format!("{} {}", if enable { "Enabled" } else { "Disabled" }, crate::content::base_name(&n)))
        }
        Fix::InstallMod { mod_id, project_id, source } => {
            let target = Target::of(inst);
            let (src, pid) = match project_id {
                Some(p) => (Source::parse(source.as_deref()), p.clone()),
                None => (Source::Modrinth, resolve_mod_id(mod_id, &target).await?),
            };
            let applied = deps::install(paths, inst, src, &[Root { project_id: pid, version_id: None }], false, false, task).await?;
            let mut names: Vec<String> = applied.installed.iter().map(|i| i.title.clone()).collect();
            names.extend(applied.pending.iter().map(|p| format!("{} (download from CurseForge)", p.title)));
            Ok(format!("Installed {}", names.join(", ")))
        }
        Fix::SetMemory { mb } => {
            let mut i = instance::get(paths, &inst.id)?;
            i.memory_mb = Some(*mb);
            instance::save(paths, &i)?;
            Ok(format!("Memory set to {} GB", *mb as f32 / 1024.0))
        }
        Fix::UseManagedJava => {
            let mut i = instance::get(paths, &inst.id)?;
            i.java_path = None;
            instance::save(paths, &i)?;
            Ok("This instance now uses the Java Cairn picks for it".into())
        }
        // Handled by the UI (navigation), nothing to change here.
        Fix::SearchMod { .. } | Fix::ManualDownloads => Ok(String::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_guesses_cover_known_and_plain_ids() {
        assert_eq!(slug_guesses("fabric"), vec!["fabric-api", "fabric"]);
        assert_eq!(slug_guesses("cloth_config"), vec!["cloth-config", "cloth_config"]);
        assert_eq!(slug_guesses("some_lib"), vec!["some_lib", "some-lib"]);
        assert_eq!(slug_guesses("jei"), vec!["jei"]);
    }

    #[test]
    fn toggles_only_files_inside_mods() {
        let d = std::env::temp_dir().join(format!("cairn-fixes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("mods")).unwrap();
        std::fs::write(d.join("mods/a.jar"), b"x").unwrap();
        assert!(mods_file(&d, "../a.jar").is_err());
        assert!(mods_file(&d, "missing.jar").is_err());
        assert!(mods_file(&d, "a.jar").is_ok());
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn pending_downloads_are_errors() {
        let r = Report { pending: vec![PendingDownload::default()], ..Default::default() };
        assert!(r.has_errors());
        assert!(!Report::default().has_errors());
    }
}
