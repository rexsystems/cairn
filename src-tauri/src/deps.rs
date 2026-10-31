//! Dependency resolver shared by Modrinth and CurseForge installs, updates,
//! performance packs and one-click fixes. It builds a plan first (what will be
//! downloaded, why, and what it replaces) so the UI can show it before anything
//! touches the instance, then applies the plan atomically-ish: every download
//! must succeed before old files are removed and the index is written.

use crate::content::{base_name, ContentIndex, ContentMeta};
use crate::instance::Instance;
use crate::net::{self, Download};
use crate::paths::Paths;
use crate::tasks::Task;
use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    #[default]
    Modrinth,
    Curseforge,
}

impl Source {
    pub fn as_str(self) -> &'static str {
        match self {
            Source::Modrinth => "modrinth",
            Source::Curseforge => "curseforge",
        }
    }
    pub fn parse(s: Option<&str>) -> Source {
        match s {
            Some("curseforge") => Source::Curseforge,
            _ => Source::Modrinth,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Rel {
    Required,
    Optional,
    Incompatible,
    /// Shipped inside the jar already; nothing to do.
    Embedded,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VDep {
    pub project_id: Option<String>,
    pub version_id: Option<String>,
    pub rel: Rel,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ProjectInfo {
    pub id: String,
    pub slug: String,
    pub title: String,
    /// Modrinth project type: mod, resourcepack, shader, datapack, modpack.
    pub project_type: String,
    pub icon: Option<String>,
    pub page_url: Option<String>,
}

/// One concrete file that satisfies a project for the instance.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Candidate {
    pub source: Source,
    pub project: ProjectInfo,
    pub version_id: String,
    pub version_number: String,
    pub file_name: String,
    /// None when the author disabled third-party downloads (CurseForge); the user fetches it.
    pub url: Option<String>,
    /// Where the user can download the file by hand.
    pub download_page: Option<String>,
    pub sha1: Option<String>,
    pub sha512: Option<String>,
    pub size: Option<u64>,
    /// CurseForge murmur2 fingerprint, a fallback when sha1 is missing.
    pub fingerprint: Option<u32>,
    pub deps: Vec<VDep>,
    pub loaders: Vec<String>,
    pub game_versions: Vec<String>,
}

impl Candidate {
    pub fn folder(&self) -> Option<&'static str> {
        crate::modrinth::folder_for(&self.project.project_type)
    }
    pub fn manual(&self) -> bool {
        self.url.is_none()
    }
}

#[derive(Clone, Debug)]
pub struct Target {
    pub loader: String,
    pub mc_version: String,
}

impl Target {
    pub fn of(inst: &Instance) -> Target {
        Target { loader: inst.loader.clone(), mc_version: inst.mc_version.clone() }
    }
    /// Whether a version's declared loaders and game versions fit. Empty lists mean "not declared".
    pub fn accepts(&self, c: &Candidate) -> bool {
        let mc_ok = c.game_versions.is_empty() || c.game_versions.iter().any(|v| v == &self.mc_version);
        if c.project.project_type != "mod" || c.loaders.is_empty() {
            return mc_ok;
        }
        let ok_loaders: &[&str] = match self.loader.as_str() {
            "quilt" => &["quilt", "fabric"],
            "neoforge" if self.mc_version == "1.20.1" => &["neoforge", "forge"],
            l => std::slice::from_ref(match l {
                "fabric" => &"fabric",
                "forge" => &"forge",
                "neoforge" => &"neoforge",
                _ => &"",
            }),
        };
        mc_ok && c.loaders.iter().any(|l| ok_loaders.contains(&l.to_ascii_lowercase().as_str()))
    }
}

/// A source of projects and versions. Futures are `Send` so plans can run inside Tauri commands.
pub trait Repo: Sync {
    fn source(&self) -> Source;
    fn project(&self, id: &str) -> impl Future<Output = Result<ProjectInfo>> + Send;
    /// Best version for the target, or None when there is none.
    fn pick(&self, p: &ProjectInfo, t: &Target) -> impl Future<Output = Result<Option<Candidate>>> + Send;
    fn exact(&self, p: &ProjectInfo, version_id: &str) -> impl Future<Output = Result<Candidate>> + Send;
    /// Project id for a dependency that names only a version.
    fn project_of_version(&self, version_id: &str) -> impl Future<Output = Result<String>> + Send;
}

/// Lowercase alphanumerics only, so `fabric-api`, `fabric_api` and `Fabric API` compare equal.
pub fn norm(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_alphanumeric()).map(|c| c.to_ascii_lowercase()).collect()
}

/// What is already in the instance, from the content index and the jars themselves.
#[derive(Default, Debug, Clone)]
pub struct Have {
    /// (source, project id) -> index keys ("mods/x.jar").
    pub projects: HashMap<(Source, String), Vec<String>>,
    /// Normalized names (mod ids, provided ids, titles) -> index key of the jar.
    pub names: HashMap<String, String>,
}

impl Have {
    pub fn gather(dir: &Path) -> Have {
        let index = ContentIndex::load(dir);
        let mut have = Have::default();
        for (key, m) in &index.files {
            let on_disk = existing_file(dir, key).is_some();
            if !on_disk {
                continue;
            }
            if let Some(pid) = &m.project_id {
                have.projects.entry((Source::parse(m.source.as_deref()), pid.clone())).or_default().push(key.clone());
            }
            if let Some(t) = &m.title {
                if key.starts_with("mods/") {
                    have.names.entry(norm(t)).or_insert_with(|| key.clone());
                }
            }
        }
        for j in crate::modmeta::scan(dir) {
            let key = format!("mods/{}", base_name(&j.file));
            if let Some(info) = &j.info {
                for id in info.ids.iter().chain(info.provides.iter()) {
                    have.names.insert(norm(id), key.clone());
                }
            }
        }
        have
    }

    fn has_project(&self, src: Source, id: &str) -> bool {
        self.projects.contains_key(&(src, id.to_string()))
    }

    /// An installed jar that already provides this project under another source or by hand.
    fn provider_of(&self, p: &ProjectInfo) -> Option<&String> {
        if p.project_type != "mod" {
            return None;
        }
        self.names.get(&norm(&p.slug)).or_else(|| self.names.get(&norm(&p.title)))
    }
}

/// `mods/x.jar` may be on disk as `mods/x.jar.disabled`.
pub fn existing_file(dir: &Path, key: &str) -> Option<std::path::PathBuf> {
    let p = dir.join(key);
    if p.is_file() {
        return Some(p);
    }
    let d = dir.join(format!("{key}.disabled"));
    d.is_file().then_some(d)
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum Reason {
    Requested,
    Update { from: String },
    DependencyOf { title: String },
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PlanItem {
    pub candidate: Candidate,
    pub reason: Reason,
    /// Index keys of files this replaces (older versions of the same project).
    pub replaces: Vec<String>,
    pub optional: bool,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub title: String,
    pub message: String,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub items: Vec<PlanItem>,
    /// Things left alone on purpose (already installed, optional).
    pub skipped: Vec<Note>,
    /// Things that will likely stop the game from starting.
    pub problems: Vec<Note>,
}

impl Plan {
    pub fn manual_count(&self) -> usize {
        self.items.iter().filter(|i| i.candidate.manual()).count()
    }
}

/// A project the user asked for; `version_id` pins an exact version.
#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    pub project_id: String,
    #[serde(default)]
    pub version_id: Option<String>,
}

struct Pending {
    project_id: String,
    version_id: Option<String>,
    rel: Rel,
    parent: Option<String>,
    root: bool,
}

/// Index keys of files from the same project, or for root mods any jar that provides the same id.
fn replaced_keys(have: &Have, src: Source, c: &Candidate, root: bool) -> Vec<String> {
    let folder = c.folder().unwrap_or("mods");
    let new_key = format!("{folder}/{}", c.file_name);
    let mut keys: Vec<String> = have.projects.get(&(src, c.project.id.clone())).cloned().unwrap_or_default();
    if root {
        if let Some(k) = have.provider_of(&c.project) {
            keys.push(k.clone());
        }
    }
    keys.sort();
    keys.dedup();
    keys.retain(|k| k != &new_key);
    keys
}

/// Resolves `roots` and all required (and optionally optional) dependencies for `target`.
pub async fn plan<R: Repo>(repo: &R, target: &Target, have: &Have, roots: &[Root], include_optional: bool) -> Result<Plan> {
    let src = repo.source();
    let mut plan = Plan::default();
    let mut seen: HashSet<String> = HashSet::new();
    let mut incompatible: Vec<(String, String)> = vec![];
    let mut queue: Vec<Pending> = roots
        .iter()
        .map(|r| Pending { project_id: r.project_id.clone(), version_id: r.version_id.clone(), rel: Rel::Required, parent: None, root: true })
        .collect();

    while !queue.is_empty() {
        let level = std::mem::take(&mut queue);
        let fetched = futures::future::join_all(level.into_iter().map(|p| async move {
            let r = fetch(repo, target, &p).await;
            (p, r)
        }))
        .await;
        for (p, r) in fetched {
            let (info, cand) = match r {
                Ok(x) => x,
                Err(e) if p.root => return Err(e),
                Err(e) => {
                    let note = Note { title: p.project_id.clone(), message: format!("Could not look up this dependency: {e:#}") };
                    if p.rel == Rel::Required {
                        plan.problems.push(note);
                    } else {
                        plan.skipped.push(note);
                    }
                    continue;
                }
            };
            if !seen.insert(info.id.clone()) {
                continue;
            }
            if p.root {
                if info.project_type == "modpack" {
                    bail!("{} is a modpack. Install it from Discover as a new instance.", info.title);
                }
                if info.project_type == "mod" && target.loader == "vanilla" {
                    bail!("{} is a mod. Vanilla instances cannot load mods; create a Fabric, Quilt, Forge or NeoForge instance.", info.title);
                }
            } else {
                if have.has_project(src, &info.id) {
                    plan.skipped.push(Note { title: info.title.clone(), message: "Already installed".into() });
                    continue;
                }
                if let Some(key) = have.provider_of(&info) {
                    plan.skipped.push(Note { title: info.title.clone(), message: format!("Already provided by {}", base_name(key)) });
                    continue;
                }
            }
            let Some(cand) = cand else {
                let msg = format!("No version for Minecraft {} with {}", target.mc_version, crate::compat::loader_label(&target.loader));
                if p.root {
                    bail!("{} has no version for Minecraft {} with {}", info.title, target.mc_version, crate::compat::loader_label(&target.loader));
                }
                let parent = p.parent.clone().unwrap_or_default();
                if p.rel == Rel::Required {
                    plan.problems.push(Note { title: info.title.clone(), message: format!("{msg}, but {parent} requires it.") });
                } else {
                    plan.skipped.push(Note { title: info.title.clone(), message: msg });
                }
                continue;
            };
            if cand.folder().is_none() {
                bail!("{} is a {}, which cannot be added to an instance", info.title, info.project_type);
            }
            for d in &cand.deps {
                match d.rel {
                    Rel::Embedded => {}
                    Rel::Incompatible => {
                        if let Some(id) = &d.project_id {
                            incompatible.push((id.clone(), cand.project.title.clone()));
                        }
                    }
                    Rel::Required | Rel::Optional => {
                        if d.rel == Rel::Optional && !include_optional {
                            continue;
                        }
                        let id = d.project_id.clone().unwrap_or_default();
                        if id.is_empty() && d.version_id.is_none() {
                            continue;
                        }
                        if !id.is_empty() && seen.contains(&id) {
                            continue;
                        }
                        queue.push(Pending { project_id: id, version_id: d.version_id.clone(), rel: d.rel, parent: Some(cand.project.title.clone()), root: false });
                    }
                }
            }
            let replaces = replaced_keys(have, src, &cand, p.root);
            let reason = match (&p.parent, p.root, replaces.is_empty()) {
                (Some(parent), _, _) => Reason::DependencyOf { title: parent.clone() },
                (None, true, false) => Reason::Update { from: replaces.iter().map(|k| base_name(k.split_once('/').map(|x| x.1).unwrap_or(k)).to_string()).collect::<Vec<_>>().join(", ") },
                _ => Reason::Requested,
            };
            plan.items.push(PlanItem { candidate: cand, reason, replaces, optional: p.rel == Rel::Optional });
        }
    }

    for (id, by) in incompatible {
        let planned = plan.items.iter().find(|i| i.candidate.project.id == id).map(|i| i.candidate.project.title.clone());
        let installed = have.projects.get(&(src, id.clone())).and_then(|k| k.first()).map(|k| base_name(k).to_string());
        if let Some(other) = planned.or(installed) {
            plan.problems.push(Note { title: by.clone(), message: format!("{by} is marked incompatible with {other}.") });
        }
    }
    Ok(plan)
}

async fn fetch<R: Repo>(repo: &R, target: &Target, p: &Pending) -> Result<(ProjectInfo, Option<Candidate>)> {
    let pid = if p.project_id.is_empty() {
        repo.project_of_version(p.version_id.as_deref().unwrap_or_default()).await?
    } else {
        p.project_id.clone()
    };
    let info = repo.project(&pid).await?;
    if let Some(vid) = &p.version_id {
        let c = repo.exact(&info, vid).await?;
        // Roots are honored as asked. A dependency pinned to a version for another
        // Minecraft or loader is a packaging slip; pick a matching version instead.
        if p.root || target.accepts(&c) {
            return Ok((info, Some(c)));
        }
    }
    let c = repo.pick(&info, target).await?;
    Ok((info, c))
}

/// A CurseForge file the user has to download in a browser, waiting to be picked up.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct PendingDownload {
    pub project_id: String,
    pub file_id: String,
    pub title: String,
    pub version_number: String,
    pub file_name: String,
    pub folder: String,
    pub download_page: String,
    pub size: Option<u64>,
    pub sha1: Option<String>,
    pub fingerprint: Option<u32>,
    pub icon: Option<String>,
    pub replaces: Vec<String>,
}

pub fn pending_path(dir: &Path) -> std::path::PathBuf {
    dir.join(".cairn").join("pending.json")
}

pub fn load_pending(dir: &Path) -> Vec<PendingDownload> {
    crate::util::read_json(&pending_path(dir)).unwrap_or_default()
}

pub fn save_pending(dir: &Path, list: &[PendingDownload]) -> Result<()> {
    if list.is_empty() {
        let _ = std::fs::remove_file(pending_path(dir));
        return Ok(());
    }
    crate::util::write_json_atomic(&pending_path(dir), list)
}

pub fn meta_for(c: &Candidate) -> ContentMeta {
    ContentMeta {
        project_id: Some(c.project.id.clone()),
        version_id: Some(c.version_id.clone()),
        title: Some(c.project.title.clone()),
        version_number: Some(c.version_number.clone()),
        icon_url: c.project.icon.clone(),
        url: c.url.clone(),
        sha1: c.sha1.clone(),
        sha512: c.sha512.clone(),
        size: c.size,
        source: Some(c.source.as_str().into()),
    }
}

fn safe_file_name(n: &str) -> bool {
    !n.is_empty() && !n.contains('/') && !n.contains('\\') && !n.contains("..") && n != "."
}

/// Removes replaced files. A replaced file that was disabled makes the new one disabled too,
/// so updates keep the user's choice.
pub fn swap_in(dir: &Path, index: &mut ContentIndex, new_key: &str, replaces: &[String]) -> Result<()> {
    let mut keep_disabled = false;
    for k in replaces {
        if k == new_key {
            continue;
        }
        let plain = dir.join(k);
        let disabled = dir.join(format!("{k}.disabled"));
        if !plain.exists() && disabled.exists() {
            keep_disabled = true;
        }
        let _ = std::fs::remove_file(plain);
        let _ = std::fs::remove_file(disabled);
        index.files.remove(k);
    }
    if keep_disabled {
        let p = dir.join(new_key);
        if p.exists() {
            std::fs::rename(&p, dir.join(format!("{new_key}.disabled")))?;
        }
    }
    Ok(())
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Applied {
    pub installed: Vec<crate::modrinth::Installed>,
    pub pending: Vec<PendingDownload>,
}

/// Downloads every automatic item, then swaps files and writes the index in one go.
/// If any download fails, files fetched for this plan are removed and nothing else changes.
pub async fn apply(dir: &Path, plan: &Plan, concurrency: usize, task: &Task) -> Result<Applied> {
    let mut dls = vec![];
    let mut fresh = vec![];
    for item in &plan.items {
        let c = &item.candidate;
        if !safe_file_name(&c.file_name) {
            bail!("refusing unsafe file name {}", c.file_name);
        }
        let folder = c.folder().ok_or_else(|| anyhow!("{} cannot be added to an instance", c.project.title))?;
        let Some(url) = &c.url else { continue };
        let dest = dir.join(folder).join(&c.file_name);
        if !dest.exists() {
            fresh.push(dest.clone());
        }
        dls.push(Download { url: url.clone(), dest, sha1: c.sha1.clone(), sha256: None, size: c.size });
    }
    if !dls.is_empty() {
        let total = dls.len() as u64;
        let t = task.clone();
        task.stage("Downloading", 0, total);
        if let Err(e) = net::download_all(dls, concurrency, move |d, n| t.stage("Downloading", d, n)).await {
            for f in fresh {
                let _ = std::fs::remove_file(f);
            }
            return Err(e);
        }
    }

    let mut index = ContentIndex::load(dir);
    let mut pending = load_pending(dir);
    let mut out = Applied::default();
    for item in &plan.items {
        let c = &item.candidate;
        let folder = c.folder().unwrap_or("mods");
        if c.manual() {
            pending.retain(|p| !(p.project_id == c.project.id && p.folder == folder));
            let p = PendingDownload {
                project_id: c.project.id.clone(),
                file_id: c.version_id.clone(),
                title: c.project.title.clone(),
                version_number: c.version_number.clone(),
                file_name: c.file_name.clone(),
                folder: folder.into(),
                download_page: c.download_page.clone().unwrap_or_default(),
                size: c.size,
                sha1: c.sha1.clone(),
                fingerprint: c.fingerprint,
                icon: c.project.icon.clone(),
                replaces: item.replaces.clone(),
            };
            out.pending.push(p.clone());
            pending.push(p);
            continue;
        }
        let key = format!("{folder}/{}", c.file_name);
        swap_in(dir, &mut index, &key, &item.replaces)?;
        index.files.insert(key, meta_for(c));
        out.installed.push(crate::modrinth::Installed { title: c.project.title.clone(), file_name: c.file_name.clone() });
    }
    index.save(dir)?;
    save_pending(dir, &pending)?;
    Ok(out)
}

/// Resolves without changing anything, for the confirmation dialog.
pub async fn preview(paths: &Paths, inst: &Instance, source: Source, roots: &[Root], include_optional: bool) -> Result<Plan> {
    let dir = inst.dir(paths);
    let _ = crate::modrinth::identify(&dir).await;
    let have = tokio::task::spawn_blocking(move || Have::gather(&dir)).await?;
    let target = Target::of(inst);
    match source {
        Source::Modrinth => plan(&crate::modrinth::Repo, &target, &have, roots, include_optional).await,
        Source::Curseforge => plan(&crate::curseforge::Repo::new()?, &target, &have, roots, include_optional).await,
    }
}

/// Shorthand used by commands: gather, plan with the right repo, apply. Problems stop the
/// install unless the user already saw them in the preview and chose to continue.
pub async fn install(paths: &Paths, inst: &Instance, source: Source, roots: &[Root], include_optional: bool, allow_problems: bool, task: &Task) -> Result<Applied> {
    let dir = inst.dir(paths);
    task.stage("Resolving dependencies", 0, 0);
    let plan = preview(paths, inst, source, roots, include_optional).await?;
    if !allow_problems {
        if let Some(p) = plan.problems.first() {
            bail!("{}: {}", p.title, p.message);
        }
    }
    let concurrency = crate::settings::Settings::load(&paths.settings_file()).concurrent_downloads.max(1);
    apply(&dir, &plan, concurrency, task).await
}

#[cfg(test)]
pub mod fake {
    use super::*;

    /// In-memory repository: project id -> (info, versions newest first).
    #[derive(Default)]
    pub struct FakeRepo {
        pub projects: HashMap<String, (ProjectInfo, Vec<Candidate>)>,
        pub src: Source,
    }

    impl FakeRepo {
        pub fn add(&mut self, id: &str, ptype: &str, versions: Vec<Candidate>) {
            let info = ProjectInfo { id: id.into(), slug: id.into(), title: id.to_uppercase(), project_type: ptype.into(), ..Default::default() };
            let versions = versions
                .into_iter()
                .map(|mut v| {
                    v.project = info.clone();
                    v.source = self.src;
                    v
                })
                .collect();
            self.projects.insert(id.into(), (info, versions));
        }
    }

    pub fn ver(id: &str, mc: &str, loader: &str, deps: Vec<VDep>) -> Candidate {
        Candidate {
            version_id: id.into(),
            version_number: id.into(),
            file_name: format!("{id}.jar"),
            url: Some(format!("https://example.invalid/{id}.jar")),
            deps,
            loaders: vec![loader.into()],
            game_versions: vec![mc.into()],
            ..Default::default()
        }
    }

    pub fn dep(project: &str, rel: Rel) -> VDep {
        VDep { project_id: Some(project.into()), version_id: None, rel }
    }

    impl Repo for FakeRepo {
        fn source(&self) -> Source {
            self.src
        }
        async fn project(&self, id: &str) -> Result<ProjectInfo> {
            self.projects.get(id).map(|p| p.0.clone()).ok_or_else(|| anyhow!("404 {id}"))
        }
        async fn pick(&self, p: &ProjectInfo, t: &Target) -> Result<Option<Candidate>> {
            Ok(self.projects[&p.id].1.iter().find(|c| t.accepts(c)).cloned())
        }
        async fn exact(&self, p: &ProjectInfo, version_id: &str) -> Result<Candidate> {
            self.projects[&p.id].1.iter().find(|c| c.version_id == version_id).cloned().ok_or_else(|| anyhow!("no version {version_id}"))
        }
        async fn project_of_version(&self, version_id: &str) -> Result<String> {
            self.projects
                .values()
                .find(|(_, vs)| vs.iter().any(|v| v.version_id == version_id))
                .map(|(p, _)| p.id.clone())
                .ok_or_else(|| anyhow!("no version {version_id}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fake::*;
    use super::*;

    fn target(loader: &str, mc: &str) -> Target {
        Target { loader: loader.into(), mc_version: mc.into() }
    }

    fn root(id: &str) -> Vec<Root> {
        vec![Root { project_id: id.into(), version_id: None }]
    }

    fn titles(p: &Plan) -> Vec<String> {
        p.items.iter().map(|i| i.candidate.project.id.clone()).collect()
    }

    fn block_on<F: Future>(f: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(f)
    }

    fn sample() -> FakeRepo {
        let mut r = FakeRepo::default();
        r.add("api", "mod", vec![ver("api-121", "1.21.1", "fabric", vec![]), ver("api-120", "1.20.1", "fabric", vec![])]);
        r.add("lib", "mod", vec![ver("lib-1", "1.21.1", "fabric", vec![dep("api", Rel::Required)])]);
        r.add("app", "mod", vec![ver("app-1", "1.21.1", "fabric", vec![dep("lib", Rel::Required), dep("api", Rel::Required), dep("extra", Rel::Optional)])]);
        r.add("extra", "mod", vec![ver("extra-1", "1.21.1", "fabric", vec![])]);
        r
    }

    #[test]
    fn recursive_required_dependencies_once_each() {
        let p = block_on(plan(&sample(), &target("fabric", "1.21.1"), &Have::default(), &root("app"), false)).unwrap();
        assert_eq!(titles(&p), vec!["app", "lib", "api"]);
        assert_eq!(p.items[1].reason, Reason::DependencyOf { title: "APP".into() });
        assert_eq!(p.items[2].candidate.version_id, "api-121");
        assert!(p.problems.is_empty());
    }

    #[test]
    fn optional_only_when_asked() {
        let p = block_on(plan(&sample(), &target("fabric", "1.21.1"), &Have::default(), &root("app"), true)).unwrap();
        assert!(titles(&p).contains(&"extra".to_string()));
        assert!(p.items.iter().find(|i| i.candidate.project.id == "extra").unwrap().optional);
    }

    #[test]
    fn installed_dependencies_are_skipped() {
        let mut have = Have::default();
        have.projects.insert((Source::Modrinth, "api".into()), vec!["mods/api-121.jar".into()]);
        let p = block_on(plan(&sample(), &target("fabric", "1.21.1"), &have, &root("app"), false)).unwrap();
        assert_eq!(titles(&p), vec!["app", "lib"]);
        assert_eq!(p.skipped[0].title, "API");
    }

    #[test]
    fn dependency_provided_by_another_jar_is_skipped() {
        let mut have = Have::default();
        have.names.insert("api".into(), "mods/api-from-curseforge.jar".into());
        let p = block_on(plan(&sample(), &target("fabric", "1.21.1"), &have, &root("lib"), false)).unwrap();
        assert_eq!(titles(&p), vec!["lib"]);
        assert!(p.skipped[0].message.contains("api-from-curseforge.jar"));
    }

    #[test]
    fn missing_required_dependency_is_a_problem() {
        let mut r = sample();
        r.add("needs-old", "mod", vec![ver("n-1", "1.21.1", "fabric", vec![dep("oldlib", Rel::Required)])]);
        r.add("oldlib", "mod", vec![ver("old-1", "1.16.5", "fabric", vec![])]);
        let p = block_on(plan(&r, &target("fabric", "1.21.1"), &Have::default(), &root("needs-old"), false)).unwrap();
        assert_eq!(p.problems.len(), 1);
        assert!(p.problems[0].message.contains("NEEDS-OLD requires it"));
    }

    #[test]
    fn root_without_version_fails_clearly() {
        let err = block_on(plan(&sample(), &target("forge", "1.21.1"), &Have::default(), &root("api"), false)).unwrap_err();
        assert!(err.to_string().contains("no version for Minecraft 1.21.1 with Forge"), "{err}");
    }

    #[test]
    fn mods_rejected_on_vanilla_and_modpacks_rejected() {
        let mut r = sample();
        r.add("pack", "modpack", vec![ver("p-1", "1.21.1", "fabric", vec![])]);
        assert!(block_on(plan(&r, &target("vanilla", "1.21.1"), &Have::default(), &root("api"), false)).is_err());
        assert!(block_on(plan(&r, &target("fabric", "1.21.1"), &Have::default(), &root("pack"), false)).unwrap_err().to_string().contains("modpack"));
    }

    #[test]
    fn quilt_accepts_fabric_versions() {
        let p = block_on(plan(&sample(), &target("quilt", "1.21.1"), &Have::default(), &root("api"), false)).unwrap();
        assert_eq!(titles(&p), vec!["api"]);
    }

    #[test]
    fn pinned_dependency_for_wrong_version_falls_back() {
        let mut r = sample();
        r.add("pins", "mod", vec![ver("pins-1", "1.21.1", "fabric", vec![VDep { project_id: Some("api".into()), version_id: Some("api-120".into()), rel: Rel::Required }])]);
        let p = block_on(plan(&r, &target("fabric", "1.21.1"), &Have::default(), &root("pins"), false)).unwrap();
        assert_eq!(p.items[1].candidate.version_id, "api-121");
        // A version-only dependency still resolves its project.
        r.add("vonly", "mod", vec![ver("v-1", "1.21.1", "fabric", vec![VDep { project_id: None, version_id: Some("api-121".into()), rel: Rel::Required }])]);
        let p = block_on(plan(&r, &target("fabric", "1.21.1"), &Have::default(), &root("vonly"), false)).unwrap();
        assert_eq!(titles(&p), vec!["vonly", "api"]);
    }

    #[test]
    fn incompatible_with_installed_or_planned() {
        let mut r = sample();
        r.add("rival", "mod", vec![ver("rival-1", "1.21.1", "fabric", vec![dep("api", Rel::Incompatible)])]);
        let mut have = Have::default();
        have.projects.insert((Source::Modrinth, "api".into()), vec!["mods/api.jar".into()]);
        let p = block_on(plan(&r, &target("fabric", "1.21.1"), &have, &root("rival"), false)).unwrap();
        assert_eq!(p.problems.len(), 1);
        assert!(p.problems[0].message.contains("incompatible"));
    }

    #[test]
    fn cycles_terminate() {
        let mut r = FakeRepo::default();
        r.add("a", "mod", vec![ver("a-1", "1.21.1", "fabric", vec![dep("b", Rel::Required)])]);
        r.add("b", "mod", vec![ver("b-1", "1.21.1", "fabric", vec![dep("a", Rel::Required)])]);
        let p = block_on(plan(&r, &target("fabric", "1.21.1"), &Have::default(), &root("a"), false)).unwrap();
        assert_eq!(titles(&p), vec!["a", "b"]);
    }

    #[test]
    fn update_replaces_old_file() {
        let mut have = Have::default();
        have.projects.insert((Source::Modrinth, "api".into()), vec!["mods/api-old.jar".into()]);
        let p = block_on(plan(&sample(), &target("fabric", "1.21.1"), &have, &root("api"), false)).unwrap();
        assert_eq!(p.items[0].replaces, vec!["mods/api-old.jar"]);
        assert_eq!(p.items[0].reason, Reason::Update { from: "api-old.jar".into() });
    }

    #[test]
    fn swap_keeps_disabled_state() {
        let d = std::env::temp_dir().join(format!("cairn-deps-swap-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("mods")).unwrap();
        std::fs::write(d.join("mods/old.jar.disabled"), b"old").unwrap();
        std::fs::write(d.join("mods/new.jar"), b"new").unwrap();
        let mut index = ContentIndex::default();
        index.files.insert("mods/old.jar".into(), ContentMeta::default());
        swap_in(&d, &mut index, "mods/new.jar", &["mods/old.jar".into()]).unwrap();
        assert!(!d.join("mods/old.jar.disabled").exists());
        assert!(d.join("mods/new.jar.disabled").exists());
        assert!(index.files.is_empty());
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn manual_items_go_to_pending() {
        let d = std::env::temp_dir().join(format!("cairn-deps-pending-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let mut c = ver("blocked-1", "1.21.1", "fabric", vec![]);
        c.url = None;
        c.project = ProjectInfo { id: "77".into(), title: "Blocked".into(), project_type: "mod".into(), ..Default::default() };
        c.download_page = Some("https://www.curseforge.com/minecraft/mc-mods/blocked/download/1".into());
        let p = Plan { items: vec![PlanItem { candidate: c, reason: Reason::Requested, replaces: vec![], optional: false }], ..Default::default() };
        let task = Task::headless("test");
        let applied = block_on(apply(&d, &p, 2, &task)).unwrap();
        assert_eq!(applied.pending.len(), 1);
        assert_eq!(load_pending(&d)[0].project_id, "77");
        std::fs::remove_dir_all(&d).unwrap();
    }
}
