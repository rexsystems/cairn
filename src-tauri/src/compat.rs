//! Pre-launch compatibility check over the jars in `mods/`: wrong loader, wrong
//! Minecraft version, missing or outdated dependencies, declared incompatibilities
//! and duplicates. Pure over scanned metadata so it is cheap to test.

use crate::content::{base_name, ContentIndex};
use crate::modmeta::{DepKind, JarMod, ModInfo, Side};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

/// A one-click remedy. The UI sends the same object back to `apply_fix`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Fix {
    /// Install a mod by loader mod id, or by a known project.
    InstallMod { mod_id: String, project_id: Option<String>, source: Option<String> },
    EnableMod { file: String },
    DisableMod { file: String },
    SetMemory { mb: u32 },
    /// Drop a custom Java path so Cairn picks the right Java for this version.
    UseManagedJava,
    /// Open Discover with this search (when no project can be matched automatically).
    SearchMod { query: String },
    /// Show the pending manual CurseForge downloads.
    ManualDownloads,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FixAction {
    pub label: String,
    #[serde(flatten)]
    pub fix: Fix,
}

pub fn action(label: impl Into<String>, fix: Fix) -> FixAction {
    FixAction { label: label.into(), fix }
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    /// Stable key so the UI can remember dismissals.
    pub key: String,
    pub severity: Severity,
    pub title: String,
    pub detail: String,
    pub files: Vec<String>,
    pub fixes: Vec<FixAction>,
}

pub struct Ctx<'a> {
    pub loader: &'a str,
    pub mc_version: &'a str,
    pub loader_version: Option<&'a str>,
}

/// Ids the loader itself satisfies.
fn builtins(loader: &str) -> &'static [&'static str] {
    match loader {
        "fabric" => &["minecraft", "java", "fabricloader", "fabric-loader", "mixinextras"],
        "quilt" => &["minecraft", "java", "fabricloader", "fabric-loader", "quilt_loader", "mixinextras"],
        "forge" => &["minecraft", "java", "forge", "javafml", "lowcodefml", "mcp"],
        "neoforge" => &["minecraft", "java", "neoforge", "javafml", "lowcodefml", "mixinextras", "fml"],
        _ => &["minecraft", "java"],
    }
}

fn loader_ids(loader: &str) -> &'static [&'static str] {
    match loader {
        "fabric" => &["fabricloader", "fabric-loader"],
        "quilt" => &["quilt_loader"],
        "forge" => &["forge"],
        "neoforge" => &["neoforge"],
        _ => &[],
    }
}

pub fn loader_label(l: &str) -> &'static str {
    match l {
        "fabric" => "Fabric",
        "quilt" => "Quilt",
        "forge" => "Forge",
        "neoforge" => "NeoForge",
        _ => "Vanilla",
    }
}

/// Which loaders this jar was built for, as display names.
fn built_for(i: &ModInfo) -> Vec<&'static str> {
    let mut v = vec![];
    if i.formats.fabric {
        v.push("Fabric");
    }
    if i.formats.quilt {
        v.push("Quilt");
    }
    if i.formats.neoforge_toml || (i.formats.mods_toml && i.targets_neoforge) {
        v.push("NeoForge");
    }
    if (i.formats.mods_toml && !i.targets_neoforge) || i.formats.legacy_forge {
        v.push("Forge");
    }
    v
}

/// Whether the instance's loader can load this jar. `None` when unknown.
pub fn loads_on(i: &ModInfo, loader: &str, mc: &str) -> Option<bool> {
    let f = &i.formats;
    if !f.any() {
        return None;
    }
    Some(match loader {
        "fabric" => f.fabric,
        "quilt" => f.quilt || f.fabric,
        "forge" => (f.mods_toml && !i.targets_neoforge) || f.legacy_forge,
        // NeoForge 1.20.1 is a Forge fork and loads Forge mods; later versions do not.
        "neoforge" => f.neoforge_toml || (f.mods_toml && (i.targets_neoforge || mc == "1.20.1")),
        _ => false,
    })
}

fn display(j: &JarMod) -> String {
    j.info.as_ref().map(|i| i.name.clone()).filter(|n| !n.is_empty()).unwrap_or_else(|| base_name(&j.file).trim_end_matches(".jar").to_string())
}

fn project_of(index: &ContentIndex, file: &str) -> Option<(String, Option<String>)> {
    let m = index.files.get(&format!("mods/{}", base_name(file)))?;
    Some((m.project_id.clone()?, m.source.clone()))
}

fn install_fix(label: String, mod_id: &str, project: Option<(String, Option<String>)>) -> FixAction {
    let (project_id, source) = match project {
        Some((p, s)) => (Some(p), s),
        None => (None, None),
    };
    action(label, Fix::InstallMod { mod_id: mod_id.to_string(), project_id, source })
}

pub fn analyze(mods: &[JarMod], index: &ContentIndex, ctx: &Ctx) -> Vec<Issue> {
    let mut issues = vec![];
    if ctx.loader == "vanilla" {
        let n = mods.iter().filter(|m| m.enabled).count();
        if n > 0 {
            issues.push(Issue {
                key: "vanilla-mods".into(),
                severity: Severity::Warning,
                title: format!("{n} mod {} will not load", if n == 1 { "file" } else { "files" }),
                detail: "This is a vanilla instance, so files in the mods folder are ignored.".into(),
                files: mods.iter().filter(|m| m.enabled).map(|m| m.file.clone()).collect(),
                fixes: vec![],
            });
        }
        return issues;
    }
    let enabled: Vec<&JarMod> = mods.iter().filter(|m| m.enabled && m.info.is_some()).collect();
    let loaded: Vec<&JarMod> = enabled
        .iter()
        .copied()
        .filter(|m| loads_on(m.info.as_ref().unwrap(), ctx.loader, ctx.mc_version) != Some(false))
        .collect();

    for m in &enabled {
        let info = m.info.as_ref().unwrap();
        if loads_on(info, ctx.loader, ctx.mc_version) == Some(false) {
            let name = display(m);
            let mut fixes = vec![action(format!("Disable {name}"), Fix::DisableMod { file: m.file.clone() })];
            if let Some(p) = project_of(index, &m.file) {
                let id = info.ids.first().cloned().unwrap_or_default();
                fixes.insert(0, install_fix(format!("Get the {} version", loader_label(ctx.loader)), &id, Some(p)));
            }
            issues.push(Issue {
                key: format!("loader:{}", base_name(&m.file)),
                severity: Severity::Error,
                title: format!("{name} is not a {} mod", loader_label(ctx.loader)),
                detail: format!("{} is built for {}. It will not load on {}.", m.file, built_for(info).join(" / "), loader_label(ctx.loader)),
                files: vec![m.file.clone()],
                fixes,
            });
        }
        if info.side == Side::Server {
            issues.push(Issue {
                key: format!("side:{}", base_name(&m.file)),
                severity: Severity::Warning,
                title: format!("{} is a server-only mod", display(m)),
                detail: "It is meant for dedicated servers and usually does nothing, or crashes, in the client.".into(),
                files: vec![m.file.clone()],
                fixes: vec![action(format!("Disable {}", display(m)), Fix::DisableMod { file: m.file.clone() })],
            });
        }
    }

    // id -> (version, file) for everything loadable, including provided and nested ids.
    let mut provided: HashMap<&str, (&str, &str)> = HashMap::new();
    for m in &loaded {
        let info = m.info.as_ref().unwrap();
        for id in &info.provides {
            provided.entry(id.as_str()).or_insert((info.version.as_str(), m.file.as_str()));
        }
    }
    let disabled: HashMap<&str, &str> = mods
        .iter()
        .filter(|m| !m.enabled)
        .filter_map(|m| m.info.as_ref().map(|i| (i, m.file.as_str())))
        .flat_map(|(i, f)| i.provides.iter().map(move |id| (id.as_str(), f)))
        .collect();
    let builtin = builtins(ctx.loader);

    // Duplicates: the same top-level id in two loadable jars.
    let mut owners: BTreeMap<&str, Vec<&JarMod>> = BTreeMap::new();
    for m in &loaded {
        for id in &m.info.as_ref().unwrap().ids {
            owners.entry(id.as_str()).or_default().push(m);
        }
    }
    for (id, jars) in &owners {
        if jars.len() < 2 {
            continue;
        }
        let mut sorted = jars.clone();
        sorted.sort_by(|a, b| crate::versioning::compare(&b.info.as_ref().unwrap().version, &a.info.as_ref().unwrap().version));
        let keep = sorted[0];
        let older: Vec<&&JarMod> = sorted[1..].iter().collect();
        issues.push(Issue {
            key: format!("dup:{id}"),
            severity: Severity::Error,
            title: format!("{} is installed {} times", display(keep), jars.len()),
            detail: format!(
                "Mod id \"{id}\" comes from {}. The game stops when one mod is loaded twice.",
                jars.iter().map(|j| j.file.as_str()).collect::<Vec<_>>().join(", ")
            ),
            files: jars.iter().map(|j| j.file.clone()).collect(),
            fixes: older
                .iter()
                .map(|o| action(format!("Disable {} ({})", o.file, o.info.as_ref().unwrap().version), Fix::DisableMod { file: o.file.clone() }))
                .collect(),
        });
    }

    // Missing and outdated dependencies, grouped by the dependency id.
    let mut missing: BTreeMap<&str, Vec<(&JarMod, String)>> = BTreeMap::new();
    for m in &loaded {
        let info = m.info.as_ref().unwrap();
        for d in &info.depends {
            let id = d.id.as_str();
            match d.kind {
                DepKind::Required => {
                    if id == "minecraft" {
                        if crate::versioning::is_release_mc(ctx.mc_version) && d.range.matches(ctx.mc_version) == Some(false) {
                            let name = display(m);
                            let mut fixes = vec![];
                            if let Some(p) = project_of(index, &m.file) {
                                fixes.push(install_fix(format!("Get the {} version", ctx.mc_version), info.ids.first().map(String::as_str).unwrap_or(""), Some(p)));
                            }
                            fixes.push(action(format!("Disable {name}"), Fix::DisableMod { file: m.file.clone() }));
                            issues.push(Issue {
                                key: format!("mc:{}", base_name(&m.file)),
                                severity: Severity::Error,
                                title: format!("{name} is for a different Minecraft version"),
                                detail: format!("It needs Minecraft {}, this instance runs {}.", d.range.describe(), ctx.mc_version),
                                files: vec![m.file.clone()],
                                fixes,
                            });
                        }
                        continue;
                    }
                    if loader_ids(ctx.loader).contains(&id) {
                        if let Some(lv) = ctx.loader_version {
                            if d.range.matches(lv) == Some(false) {
                                issues.push(Issue {
                                    key: format!("loaderver:{}", base_name(&m.file)),
                                    severity: Severity::Error,
                                    title: format!("{} needs a newer {}", display(m), loader_label(ctx.loader)),
                                    detail: format!(
                                        "It asks for {} {}, this instance has {lv}. Pick a newer loader version in Options.",
                                        loader_label(ctx.loader),
                                        d.range.describe()
                                    ),
                                    files: vec![m.file.clone()],
                                    fixes: vec![],
                                });
                            }
                        }
                        continue;
                    }
                    if builtin.contains(&id) {
                        continue;
                    }
                    match provided.get(id) {
                        None => missing.entry(id).or_default().push((m, d.range.describe())),
                        Some((ver, file)) => {
                            if !ver.is_empty() && d.range.matches(ver) == Some(false) {
                                let dep_name = loaded
                                    .iter()
                                    .find(|j| j.file == *file)
                                    .map(|j| display(j))
                                    .unwrap_or_else(|| id.to_string());
                                issues.push(Issue {
                                    key: format!("depver:{}:{id}", base_name(&m.file)),
                                    severity: Severity::Error,
                                    title: format!("{} needs a different {dep_name}", display(m)),
                                    detail: format!("It asks for {dep_name} {}, but {ver} is installed.", d.range.describe()),
                                    files: vec![m.file.clone(), file.to_string()],
                                    fixes: vec![install_fix(format!("Update {dep_name}"), id, project_of(index, file))],
                                });
                            }
                        }
                    }
                }
                DepKind::Breaks | DepKind::Conflicts => {
                    let Some((ver, file)) = provided.get(id) else { continue };
                    if *file == m.file.as_str() || d.range.matches(ver) != Some(true) {
                        continue;
                    }
                    let other = loaded.iter().find(|j| j.file == *file).map(|j| display(j)).unwrap_or_else(|| id.to_string());
                    let breaks = d.kind == DepKind::Breaks;
                    issues.push(Issue {
                        key: format!("breaks:{}:{id}", base_name(&m.file)),
                        severity: if breaks { Severity::Error } else { Severity::Warning },
                        title: format!("{} and {other} {}", display(m), if breaks { "cannot be used together" } else { "do not work well together" }),
                        detail: format!(
                            "{} marks {other}{} as {}.",
                            display(m),
                            if matches!(d.range, crate::modmeta::Range::Any) { String::new() } else { format!(" {}", d.range.describe()) },
                            if breaks { "incompatible" } else { "conflicting" }
                        ),
                        files: vec![m.file.clone(), file.to_string()],
                        fixes: vec![
                            action(format!("Disable {other}"), Fix::DisableMod { file: file.to_string() }),
                            action(format!("Disable {}", display(m)), Fix::DisableMod { file: m.file.clone() }),
                        ],
                    });
                }
                DepKind::Optional => {}
            }
        }
    }
    for (id, requesters) in missing {
        let names: Vec<String> = requesters.iter().map(|(m, _)| display(m)).collect();
        let range = &requesters[0].1;
        let mut fixes = vec![];
        if let Some(file) = disabled.get(id) {
            fixes.push(action(format!("Enable {file}"), Fix::EnableMod { file: file.to_string() }));
        }
        fixes.push(install_fix(format!("Install {}", pretty_id(id)), id, None));
        issues.push(Issue {
            key: format!("missing:{id}"),
            severity: Severity::Error,
            title: format!("Missing dependency: {}", pretty_id(id)),
            detail: format!(
                "{} {} {} ({range}).",
                list_names(&names),
                if names.len() == 1 { "needs" } else { "need" },
                pretty_id(id)
            ),
            files: requesters.iter().map(|(m, _)| m.file.clone()).collect(),
            fixes,
        });
    }

    issues.extend(known_conflicts(&loaded, mods));
    issues.sort_by(|a, b| a.severity.cmp(&b.severity).then(a.title.cmp(&b.title)));
    issues
}

/// Friendly name for a dependency id we only know by id.
pub fn pretty_id(id: &str) -> String {
    match id {
        "fabric-api" | "fabric" => "Fabric API".into(),
        "qsl" | "quilted_fabric_api" => "Quilted Fabric API".into(),
        "cloth-config" | "cloth-config2" | "cloth_config" => "Cloth Config".into(),
        "architectury" => "Architectury API".into(),
        "fabric-language-kotlin" => "Fabric Language Kotlin".into(),
        "kotlinforforge" => "Kotlin for Forge".into(),
        "geckolib" => "GeckoLib".into(),
        _ => id.to_string(),
    }
}

fn list_names(names: &[String]) -> String {
    match names.len() {
        0 => String::new(),
        1 => names[0].clone(),
        2 => format!("{} and {}", names[0], names[1]),
        n if n <= 4 => format!("{} and {}", names[..n - 1].join(", "), names[n - 1]),
        n => format!("{}, {} and {} more", names[0], names[1], n - 2),
    }
}

/// OptiFine has no loader metadata, so declared conflicts never catch it.
fn known_conflicts(loaded: &[&JarMod], all: &[JarMod]) -> Vec<Issue> {
    let optifine = all.iter().find(|m| m.enabled && base_name(&m.file).to_lowercase().contains("optifine") && !base_name(&m.file).to_lowercase().contains("optifabric"));
    let Some(of) = optifine else { return vec![] };
    let renderers = ["sodium", "embeddium", "rubidium", "iris", "oculus"];
    let hits: Vec<&&JarMod> = loaded
        .iter()
        .filter(|m| m.info.as_ref().unwrap().ids.iter().any(|id| renderers.contains(&id.as_str())))
        .collect();
    if hits.is_empty() {
        return vec![];
    }
    vec![Issue {
        key: "optifine-renderer".into(),
        severity: Severity::Error,
        title: "OptiFine conflicts with your rendering mods".into(),
        detail: format!("{} replaces the same renderer as OptiFine. Keep one of them.", hits.iter().map(|m| display(m)).collect::<Vec<_>>().join(", ")),
        files: std::iter::once(of.file.clone()).chain(hits.iter().map(|m| m.file.clone())).collect(),
        fixes: vec![action("Disable OptiFine", Fix::DisableMod { file: of.file.clone() })],
    }]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modmeta::{DepReq, Formats, Range};

    fn fabric(file: &str, id: &str, version: &str, deps: Vec<DepReq>) -> JarMod {
        JarMod {
            file: file.into(),
            enabled: true,
            info: Some(ModInfo {
                ids: vec![id.into()],
                provides: vec![id.into()],
                name: id.into(),
                version: version.into(),
                depends: deps,
                formats: Formats { fabric: true, ..Default::default() },
                ..Default::default()
            }),
        }
    }

    fn req(id: &str, range: Range) -> DepReq {
        DepReq { id: id.into(), range, kind: DepKind::Required }
    }

    fn ctx<'a>(loader: &'a str, mc: &'a str) -> Ctx<'a> {
        Ctx { loader, mc_version: mc, loader_version: Some("0.16.9") }
    }

    fn keys(issues: &[Issue]) -> Vec<String> {
        issues.iter().map(|i| i.key.clone()).collect()
    }

    #[test]
    fn clean_pack_has_no_issues() {
        let mods = vec![
            fabric("api.jar", "fabric-api", "0.92.2", vec![req("fabricloader", Range::Fabric(vec![">=0.14".into()])), req("minecraft", Range::Fabric(vec!["~1.20.1".into()]))]),
            fabric("sodium.jar", "sodium", "0.5.8", vec![req("fabric-api", Range::Any), req("java", Range::Any)]),
        ];
        assert!(analyze(&mods, &ContentIndex::default(), &ctx("fabric", "1.20.1")).is_empty());
    }

    #[test]
    fn missing_dependency_grouped_with_install_fix() {
        let mods = vec![
            fabric("a.jar", "a", "1.0", vec![req("fabric-api", Range::Any)]),
            fabric("b.jar", "b", "1.0", vec![req("fabric-api", Range::Fabric(vec![">=0.90".into()]))]),
        ];
        let issues = analyze(&mods, &ContentIndex::default(), &ctx("fabric", "1.20.1"));
        assert_eq!(keys(&issues), vec!["missing:fabric-api"]);
        assert_eq!(issues[0].title, "Missing dependency: Fabric API");
        assert_eq!(issues[0].files, vec!["a.jar", "b.jar"]);
        assert_eq!(issues[0].fixes[0].fix, Fix::InstallMod { mod_id: "fabric-api".into(), project_id: None, source: None });
    }

    #[test]
    fn disabled_dependency_offers_enable() {
        let mut dep = fabric("api.jar.disabled", "fabric-api", "0.92", vec![]);
        dep.enabled = false;
        let mods = vec![fabric("a.jar", "a", "1.0", vec![req("fabric-api", Range::Any)]), dep];
        let issues = analyze(&mods, &ContentIndex::default(), &ctx("fabric", "1.20.1"));
        assert_eq!(issues[0].fixes[0].fix, Fix::EnableMod { file: "api.jar.disabled".into() });
    }

    #[test]
    fn nested_jar_satisfies_dependency() {
        let mut api = fabric("api.jar", "fabric-api", "0.92", vec![]);
        api.info.as_mut().unwrap().provides.push("fabric-api-base".into());
        let mods = vec![api, fabric("a.jar", "a", "1.0", vec![req("fabric-api-base", Range::Any)])];
        assert!(analyze(&mods, &ContentIndex::default(), &ctx("fabric", "1.20.1")).is_empty());
    }

    #[test]
    fn outdated_dependency() {
        let mods = vec![
            fabric("api.jar", "fabric-api", "0.80.0", vec![]),
            fabric("a.jar", "a", "1.0", vec![req("fabric-api", Range::Fabric(vec![">=0.90.0".into()]))]),
        ];
        let issues = analyze(&mods, &ContentIndex::default(), &ctx("fabric", "1.20.1"));
        assert_eq!(keys(&issues), vec!["depver:a.jar:fabric-api"]);
        assert!(issues[0].detail.contains("0.80.0"));
    }

    #[test]
    fn wrong_minecraft_and_loader_version() {
        let mods = vec![fabric(
            "a.jar",
            "a",
            "1.0",
            vec![req("minecraft", Range::Fabric(vec![">=1.21".into()])), req("fabricloader", Range::Fabric(vec![">=0.17".into()]))],
        )];
        let k = keys(&analyze(&mods, &ContentIndex::default(), &ctx("fabric", "1.20.1")));
        assert!(k.contains(&"mc:a.jar".to_string()));
        assert!(k.contains(&"loaderver:a.jar".to_string()));
        // Snapshots are never flagged; their ordering is unknown.
        assert!(!keys(&analyze(&mods, &ContentIndex::default(), &ctx("fabric", "24w14a"))).contains(&"mc:a.jar".to_string()));
    }

    #[test]
    fn wrong_loader_detected_and_quilt_runs_fabric() {
        let forge = JarMod {
            file: "create.jar".into(),
            enabled: true,
            info: Some(ModInfo { ids: vec!["create".into()], provides: vec!["create".into()], name: "Create".into(), formats: Formats { mods_toml: true, ..Default::default() }, targets_forge: true, ..Default::default() }),
        };
        let issues = analyze(std::slice::from_ref(&forge), &ContentIndex::default(), &ctx("fabric", "1.20.1"));
        assert_eq!(keys(&issues), vec!["loader:create.jar"]);
        assert!(issues[0].detail.contains("Forge"));
        assert!(analyze(&[fabric("a.jar", "a", "1", vec![])], &ContentIndex::default(), &ctx("quilt", "1.20.1")).is_empty());
        // NeoForge 1.20.1 still loads Forge mods; later versions do not.
        assert!(analyze(std::slice::from_ref(&forge), &ContentIndex::default(), &ctx("neoforge", "1.20.1")).is_empty());
        assert_eq!(keys(&analyze(&[forge], &ContentIndex::default(), &ctx("neoforge", "1.21.1"))), vec!["loader:create.jar"]);
    }

    #[test]
    fn duplicates_keep_newest() {
        let mods = vec![fabric("sodium-0.5.jar", "sodium", "0.5.8", vec![]), fabric("sodium-0.6.jar", "sodium", "0.6.0", vec![])];
        let issues = analyze(&mods, &ContentIndex::default(), &ctx("fabric", "1.21.1"));
        assert_eq!(keys(&issues), vec!["dup:sodium"]);
        assert_eq!(issues[0].fixes, vec![action("Disable sodium-0.5.jar (0.5.8)", Fix::DisableMod { file: "sodium-0.5.jar".into() })]);
    }

    #[test]
    fn breaks_and_conflicts() {
        let mods = vec![
            fabric("a.jar", "a", "1.0", vec![DepReq { id: "b".into(), range: Range::Fabric(vec!["<2.0".into()]), kind: DepKind::Breaks }]),
            fabric("b.jar", "b", "1.5", vec![]),
            fabric("c.jar", "c", "1.0", vec![DepReq { id: "b".into(), range: Range::Any, kind: DepKind::Conflicts }]),
        ];
        let issues = analyze(&mods, &ContentIndex::default(), &ctx("fabric", "1.21.1"));
        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].severity, Severity::Error);
        assert_eq!(issues[1].severity, Severity::Warning);
        // A breaks range that does not match the installed version is fine.
        let ok = vec![
            fabric("a.jar", "a", "1.0", vec![DepReq { id: "b".into(), range: Range::Fabric(vec!["<1.0".into()]), kind: DepKind::Breaks }]),
            fabric("b.jar", "b", "1.5", vec![]),
        ];
        assert!(analyze(&ok, &ContentIndex::default(), &ctx("fabric", "1.21.1")).is_empty());
    }

    #[test]
    fn optifine_with_sodium() {
        let of = JarMod { file: "OptiFine_1.20.1_HD_U_I6.jar".into(), enabled: true, info: None };
        let mods = vec![of, fabric("sodium.jar", "sodium", "0.5.8", vec![])];
        assert_eq!(keys(&analyze(&mods, &ContentIndex::default(), &ctx("fabric", "1.20.1"))), vec!["optifine-renderer"]);
    }

    #[test]
    fn vanilla_ignores_mods() {
        let issues = analyze(&[fabric("a.jar", "a", "1", vec![])], &ContentIndex::default(), &ctx("vanilla", "1.21.1"));
        assert_eq!(issues[0].severity, Severity::Warning);
    }
}
