use crate::content::{base_name, ContentIndex, ContentMeta};
use crate::instance::{self, Instance, PackSource};
use crate::net::{self, Download, CLIENT};
use crate::paths::{safe_join, Paths};
use crate::tasks::Task;
use crate::util;
use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub const API: &str = "https://api.modrinth.com/v2";

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
pub struct SearchParams {
    pub query: String,
    pub facets: Vec<Vec<String>>,
    pub index: String,
    pub offset: u32,
    pub limit: u32,
}

pub fn search_url(p: &SearchParams) -> String {
    let mut url = format!(
        "{API}/search?limit={}&offset={}&index={}",
        p.limit.clamp(1, 100),
        p.offset,
        if p.index.is_empty() { "relevance" } else { &p.index }
    );
    if !p.query.trim().is_empty() {
        url.push_str(&format!("&query={}", urlencoding::encode(p.query.trim())));
    }
    let facets: Vec<&Vec<String>> = p.facets.iter().filter(|f| !f.is_empty()).collect();
    if !facets.is_empty() {
        url.push_str(&format!("&facets={}", urlencoding::encode(&serde_json::to_string(&facets).unwrap())));
    }
    url
}

pub async fn search(p: &SearchParams) -> Result<Value> {
    net::get_json(&search_url(p)).await
}

pub async fn project(id: &str) -> Result<Value> {
    net::get_json(&format!("{API}/project/{}", urlencoding::encode(id))).await
}

pub async fn versions(id: &str, loaders: &[String], game_versions: &[String]) -> Result<Vec<Value>> {
    let mut url = format!("{API}/project/{}/version", urlencoding::encode(id));
    let mut q = vec![];
    if !loaders.is_empty() {
        q.push(format!("loaders={}", urlencoding::encode(&serde_json::to_string(loaders)?)));
    }
    if !game_versions.is_empty() {
        q.push(format!("game_versions={}", urlencoding::encode(&serde_json::to_string(game_versions)?)));
    }
    if !q.is_empty() {
        url.push('?');
        url.push_str(&q.join("&"));
    }
    net::get_json(&url).await
}

pub async fn version(id: &str) -> Result<Value> {
    net::get_json(&format!("{API}/version/{}", urlencoding::encode(id))).await
}

pub fn folder_for(project_type: &str) -> Option<&'static str> {
    match project_type {
        "mod" => Some("mods"),
        "resourcepack" => Some("resourcepacks"),
        "shader" => Some("shaderpacks"),
        "datapack" => Some("datapacks"),
        _ => None,
    }
}

/// Loader filter for a content type in an instance. Quilt runs most Fabric mods.
pub fn loader_filter(project_type: &str, inst_loader: &str) -> Vec<String> {
    match project_type {
        "mod" => match inst_loader {
            "quilt" => vec!["quilt".into(), "fabric".into()],
            l => vec![l.to_string()],
        },
        "resourcepack" | "datapack" => vec!["minecraft".into()],
        _ => vec![],
    }
}

fn primary_file(v: &Value) -> Option<&Value> {
    let files = v["files"].as_array()?;
    files.iter().find(|f| f["primary"].as_bool() == Some(true)).or(files.first())
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Installed {
    pub title: String,
    pub file_name: String,
}

pub async fn install_project(
    paths: &Paths,
    inst: &Instance,
    project_id: &str,
    version_id: Option<&str>,
    include_optional: bool,
    task: &Task,
) -> Result<Vec<Installed>> {
    let mut seen = HashSet::new();
    let mut out = vec![];
    install_rec(
        paths,
        inst,
        project_id.to_string(),
        version_id.map(String::from),
        include_optional,
        task,
        &mut seen,
        &mut out,
        true,
    )
    .await?;
    Ok(out)
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DepInfo {
    pub project_id: Option<String>,
    pub version_id: Option<String>,
    pub dependency_type: String,
    pub title: Option<String>,
}

/// Required + optional dependencies for a Modrinth version (titles resolved when possible).
pub async fn version_dependencies(version_id: &str) -> Result<Vec<DepInfo>> {
    let ver = version(version_id).await?;
    let mut out = vec![];
    for dep in ver["dependencies"].as_array().cloned().unwrap_or_default() {
        let dtype = dep["dependency_type"].as_str().unwrap_or("required").to_string();
        if dtype != "required" && dtype != "optional" {
            continue;
        }
        let project_id = dep["project_id"].as_str().map(String::from);
        let version_id = dep["version_id"].as_str().map(String::from);
        let mut title = None;
        if let Some(pid) = &project_id {
            if let Ok(p) = project(pid).await {
                title = p["title"].as_str().map(String::from);
            }
        }
        out.push(DepInfo { project_id, version_id, dependency_type: dtype, title });
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
fn install_rec<'a>(
    paths: &'a Paths,
    inst: &'a Instance,
    project_id: String,
    version_id: Option<String>,
    include_optional: bool,
    task: &'a Task,
    seen: &'a mut HashSet<String>,
    out: &'a mut Vec<Installed>,
    top: bool,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
    Box::pin(async move {
        let dir = inst.dir(paths);
        // A dependency may name only a version; resolve its project first.
        let ver_from_id = match &version_id {
            Some(vid) => Some(version(vid).await?),
            None => None,
        };
        let pid = if project_id.is_empty() {
            ver_from_id.as_ref().and_then(|v| v["project_id"].as_str()).unwrap_or_default().to_string()
        } else {
            project_id
        };
        if !seen.insert(pid.clone()) {
            return Ok(());
        }
        let proj = project(&pid).await?;
        // Callers may pass a slug; the index and dependency checks use the canonical id.
        let pid = proj["id"].as_str().map(String::from).unwrap_or(pid);
        if !seen.insert(pid.clone()) && !top {
            return Ok(());
        }
        let title = proj["title"].as_str().unwrap_or(&pid).to_string();
        let ptype = proj["project_type"].as_str().unwrap_or("mod");
        let folder = folder_for(ptype).ok_or_else(|| anyhow!("{title} is a {ptype}; install modpacks from Discover as a new instance"))?;
        if ptype == "mod" && inst.loader == "vanilla" {
            bail!("{} is a mod. Vanilla instances cannot load mods; create a Fabric, Quilt, Forge or NeoForge instance.", title);
        }
        let mut index = ContentIndex::load(&dir);
        if !top && index.files.values().any(|m| m.project_id.as_deref() == Some(&pid)) {
            return Ok(());
        }
        task.stage(&format!("Finding a compatible version of {title}"), 0, 0);
        let ver = match ver_from_id {
            Some(v) => v,
            None => versions(&pid, &loader_filter(ptype, &inst.loader), &[inst.mc_version.clone()])
                .await?
                .into_iter()
                .next()
                .ok_or_else(|| anyhow!("{title} has no version for Minecraft {} with {}", inst.mc_version, inst.loader))?,
        };
        let file = primary_file(&ver).ok_or_else(|| anyhow!("{title} has no downloadable file"))?.clone();
        let file_name = file["filename"].as_str().unwrap_or("download.jar").to_string();
        if file_name.contains('/') || file_name.contains('\\') || file_name.contains("..") {
            bail!("refusing unsafe file name {file_name}");
        }
        let dest = dir.join(folder).join(&file_name);
        task.stage(&format!("Downloading {title}"), 0, 1);
        net::download(&Download {
            url: file["url"].as_str().unwrap_or_default().into(),
            dest: dest.clone(),
            sha1: file.pointer("/hashes/sha1").and_then(|h| h.as_str()).map(String::from),
            size: file["size"].as_u64(),
        })
        .await?;

        // Replace any older file from the same project.
        let old: Vec<String> = index
            .files
            .iter()
            .filter(|(k, m)| m.project_id.as_deref() == Some(&pid) && k.as_str() != format!("{folder}/{file_name}"))
            .map(|(k, _)| k.clone())
            .collect();
        for k in old {
            if let Some((kind, f)) = k.split_once('/') {
                for cand in [dir.join(kind).join(f), dir.join(kind).join(format!("{f}.disabled"))] {
                    let _ = std::fs::remove_file(cand);
                }
            }
            index.files.remove(&k);
        }
        index.files.insert(
            format!("{folder}/{file_name}"),
            ContentMeta {
                project_id: Some(pid.clone()),
                version_id: ver["id"].as_str().map(String::from),
                title: Some(title.clone()),
                version_number: ver["version_number"].as_str().map(String::from),
                icon_url: proj["icon_url"].as_str().map(String::from),
                url: file["url"].as_str().map(String::from),
                sha1: file.pointer("/hashes/sha1").and_then(|h| h.as_str()).map(String::from),
                sha512: file.pointer("/hashes/sha512").and_then(|h| h.as_str()).map(String::from),
                size: file["size"].as_u64(),
            },
        );
        index.save(&dir)?;
        out.push(Installed { title, file_name });

        for dep in ver["dependencies"].as_array().cloned().unwrap_or_default() {
            let dtype = dep["dependency_type"].as_str().unwrap_or("");
            let want = dtype == "required" || (include_optional && dtype == "optional");
            if !want {
                continue;
            }
            let dpid = dep["project_id"].as_str().unwrap_or_default().to_string();
            let dvid = dep["version_id"].as_str().map(String::from);
            if dpid.is_empty() && dvid.is_none() {
                continue;
            }
            install_rec(paths, inst, dpid, dvid, include_optional, task, seen, out, false).await?;
        }
        Ok(())
    })
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub kind: String,
    pub file_name: String,
    pub title: String,
    pub icon: Option<String>,
    pub project_id: String,
    pub current_version: String,
    pub new_version: String,
    pub new_version_id: String,
}

fn local_hashes(dir: &Path, kind: &str) -> HashMap<String, String> {
    let mut m = HashMap::new();
    if let Ok(rd) = std::fs::read_dir(dir.join(kind)) {
        for e in rd.flatten() {
            if e.path().is_file() {
                if let Ok(h) = util::sha1_file(&e.path()) {
                    m.insert(h, e.file_name().to_string_lossy().to_string());
                }
            }
        }
    }
    m
}

async fn post_json(url: &str, body: &Value) -> Result<Value> {
    let r = CLIENT.post(url).json(body).send().await?;
    if !r.status().is_success() {
        bail!("{url} returned {}", r.status());
    }
    Ok(r.json().await?)
}

/// Fills the content index for files Modrinth recognises by hash (manually added mods, modpack files).
/// Modrinth ids are 8 base62 chars; slugs are lowercase. Older indexes stored slugs, which get re-identified.
pub fn looks_like_id(s: &str) -> bool {
    s.len() == 8 && s.chars().all(|c| c.is_ascii_alphanumeric()) && s.chars().any(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
}

pub async fn identify(dir: &Path) -> Result<()> {
    let mut index = ContentIndex::load(dir);
    let mut changed = false;
    for kind in crate::content::KINDS {
        let hashes = local_hashes(dir, kind);
        let unknown: Vec<String> = hashes
            .iter()
            .filter(|(_, f)| !index.files.get(&format!("{kind}/{}", base_name(f))).and_then(|m| m.project_id.clone()).map(|p| looks_like_id(&p)).unwrap_or(false))
            .map(|(h, _)| h.clone())
            .collect();
        if unknown.is_empty() {
            continue;
        }
        let current = post_json(&format!("{API}/version_files"), &json!({"hashes": unknown, "algorithm": "sha1"})).await?;
        let Some(map) = current.as_object() else { continue };
        let pids: Vec<String> = map.values().filter_map(|v| v["project_id"].as_str().map(String::from)).collect();
        let projects: Vec<Value> = if pids.is_empty() {
            vec![]
        } else {
            net::get_json(&format!("{API}/projects?ids={}", urlencoding::encode(&serde_json::to_string(&pids)?))).await.unwrap_or_default()
        };
        for (hash, v) in map {
            let Some(fname) = hashes.get(hash) else { continue };
            let pid = v["project_id"].as_str().unwrap_or_default();
            let proj = projects.iter().find(|p| p["id"].as_str() == Some(pid));
            let file = v["files"].as_array().and_then(|fs| fs.iter().find(|f| f.pointer("/hashes/sha1").and_then(|x| x.as_str()) == Some(hash.as_str())));
            index.files.insert(
                format!("{kind}/{}", base_name(fname)),
                ContentMeta {
                    project_id: Some(pid.to_string()),
                    version_id: v["id"].as_str().map(String::from),
                    title: proj.and_then(|p| p["title"].as_str()).map(String::from),
                    version_number: v["version_number"].as_str().map(String::from),
                    icon_url: proj.and_then(|p| p["icon_url"].as_str()).map(String::from),
                    url: file.and_then(|f| f["url"].as_str()).map(String::from),
                    sha1: Some(hash.clone()),
                    sha512: file.and_then(|f| f.pointer("/hashes/sha512")).and_then(|x| x.as_str()).map(String::from),
                    size: file.and_then(|f| f["size"].as_u64()),
                },
            );
            changed = true;
        }
    }
    if changed {
        index.save(dir)?;
    }
    Ok(())
}

pub async fn check_updates(paths: &Paths, inst: &Instance) -> Result<Vec<UpdateInfo>> {
    let dir = inst.dir(paths);
    let _ = identify(&dir).await;
    let index = ContentIndex::load(&dir);
    let mut out = vec![];
    for (kind, ptype) in [("mods", "mod"), ("resourcepacks", "resourcepack"), ("shaderpacks", "shader")] {
        if kind == "mods" && inst.loader == "vanilla" {
            continue;
        }
        let hashes = local_hashes(&dir, kind);
        if hashes.is_empty() {
            continue;
        }
        let mut body = json!({"hashes": hashes.keys().collect::<Vec<_>>(), "algorithm": "sha1", "game_versions": [inst.mc_version]});
        let loaders = loader_filter(ptype, &inst.loader);
        if !loaders.is_empty() {
            body["loaders"] = json!(loaders);
        }
        let latest = post_json(&format!("{API}/version_files/update"), &body).await?;
        let Some(map) = latest.as_object() else { continue };
        for (hash, v) in map {
            let Some(fname) = hashes.get(hash) else { continue };
            let still_same = v["files"].as_array().map(|fs| fs.iter().any(|f| f.pointer("/hashes/sha1").and_then(|x| x.as_str()) == Some(hash.as_str()))).unwrap_or(true);
            if still_same {
                continue;
            }
            let meta = index.files.get(&format!("{kind}/{}", base_name(fname))).cloned().unwrap_or_default();
            out.push(UpdateInfo {
                kind: kind.into(),
                file_name: fname.clone(),
                title: meta.title.clone().unwrap_or_else(|| base_name(fname).to_string()),
                icon: meta.icon_url.clone(),
                project_id: v["project_id"].as_str().unwrap_or_default().into(),
                current_version: meta.version_number.clone().unwrap_or_else(|| "installed".into()),
                new_version: v["version_number"].as_str().unwrap_or_default().into(),
                new_version_id: v["id"].as_str().unwrap_or_default().into(),
            });
        }
    }
    out.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    Ok(out)
}

pub async fn apply_update(paths: &Paths, inst: &Instance, u: &UpdateInfo, task: &Task) -> Result<()> {
    let dir = inst.dir(paths);
    let was_disabled = u.file_name.ends_with(".disabled");
    let old = dir.join(&u.kind).join(&u.file_name);
    let mut seen = HashSet::new();
    let mut out = vec![];
    // install_rec replaces the old file when the project id is already indexed.
    install_rec(paths, inst, u.project_id.clone(), Some(u.new_version_id.clone()), false, task, &mut seen, &mut out, true).await?;
    if old.exists() && out.first().map(|i| i.file_name != base_name(&u.file_name)).unwrap_or(false) {
        std::fs::remove_file(&old)?;
    }
    if was_disabled {
        if let Some(i) = out.first() {
            let _ = std::fs::rename(dir.join(&u.kind).join(&i.file_name), dir.join(&u.kind).join(format!("{}.disabled", i.file_name)));
        }
    }
    Ok(())
}

// Modpacks (.mrpack)

fn loader_from_deps(deps: &Value) -> (String, Option<String>) {
    for (key, name) in [("fabric-loader", "fabric"), ("quilt-loader", "quilt"), ("neoforge", "neoforge"), ("forge", "forge")] {
        if let Some(v) = deps.get(key).and_then(|v| v.as_str()) {
            return (name.into(), Some(v.into()));
        }
    }
    ("vanilla".into(), None)
}

pub async fn install_mrpack(
    paths: &Paths,
    file: &Path,
    pack: Option<PackSource>,
    icon_url: Option<String>,
    concurrency: usize,
    task: &Task,
) -> Result<Instance> {
    task.stage("Reading modpack", 0, 0);
    let mut zip = zip::ZipArchive::new(std::fs::File::open(file)?).context("this file is not a valid .mrpack")?;
    let mut s = String::new();
    zip.by_name("modrinth.index.json").context("modrinth.index.json is missing")?.read_to_string(&mut s)?;
    let index: Value = serde_json::from_str(&s)?;
    let deps = &index["dependencies"];
    let mc = deps["minecraft"].as_str().ok_or_else(|| anyhow!("modpack does not say which Minecraft version it needs"))?;
    let (loader, loader_version) = loader_from_deps(deps);
    let settings = crate::settings::Settings::load(&paths.settings_file());
    let inst = instance::create(
        paths,
        Instance {
            name: index["name"].as_str().unwrap_or("Modpack").into(),
            mc_version: mc.into(),
            loader,
            loader_version,
            pack,
            memory_mb: Some(settings.default_memory_mb.max(4096)),
            ..Default::default()
        },
    )?;
    let dir = inst.dir(paths);
    let result: Result<()> = async {
        let mut dls = vec![];
        for f in index["files"].as_array().cloned().unwrap_or_default() {
            if f.pointer("/env/client").and_then(|x| x.as_str()) == Some("unsupported") {
                continue;
            }
            let rel = f["path"].as_str().unwrap_or_default();
            let dest = safe_join(&dir, rel).ok_or_else(|| anyhow!("modpack contains an unsafe path: {rel}"))?;
            let url = f["downloads"].as_array().and_then(|d| d.first()).and_then(|u| u.as_str()).ok_or_else(|| anyhow!("no download for {rel}"))?;
            dls.push(Download { url: url.into(), dest, sha1: f.pointer("/hashes/sha1").and_then(|x| x.as_str()).map(String::from), size: f["fileSize"].as_u64() });
        }
        let total = dls.len() as u64;
        let t = task.clone();
        task.stage("Downloading modpack files", 0, total);
        net::download_all(dls, concurrency, move |d, n| t.stage("Downloading modpack files", d, n)).await?;

        task.stage("Applying overrides", 0, 0);
        for prefix in ["overrides/", "client-overrides/"] {
            for i in 0..zip.len() {
                let mut e = zip.by_index(i)?;
                let name = e.name().to_string();
                let Some(rel) = name.strip_prefix(prefix) else { continue };
                if e.is_dir() || rel.is_empty() {
                    continue;
                }
                let Some(dest) = safe_join(&dir, rel) else { continue };
                std::fs::create_dir_all(dest.parent().unwrap())?;
                let mut w = std::fs::File::create(&dest)?;
                std::io::copy(&mut e, &mut w)?;
            }
        }
        if let Some(icon) = icon_url {
            if net::download(&Download { url: icon, dest: dir.join("icon.png"), sha1: None, size: None }).await.is_ok() {
                let mut i = instance::get(paths, &inst.id)?;
                i.icon = Some("icon.png".into());
                instance::save(paths, &i)?;
            }
        }
        task.stage("Matching files with Modrinth", 0, 0);
        let _ = identify(&dir).await;
        Ok(())
    }
    .await;
    if let Err(e) = result {
        let _ = std::fs::remove_dir_all(&dir);
        return Err(e);
    }
    instance::get(paths, &inst.id)
}

pub async fn install_modpack_project(paths: &Paths, project_id: &str, version_id: Option<&str>, concurrency: usize, task: &Task) -> Result<Instance> {
    let proj = project(project_id).await?;
    let ver = match version_id {
        Some(v) => version(v).await?,
        None => versions(project_id, &[], &[]).await?.into_iter().next().ok_or_else(|| anyhow!("this modpack has no versions"))?,
    };
    let file = primary_file(&ver).ok_or_else(|| anyhow!("modpack version has no file"))?;
    let dest = paths.cache.join("modpacks").join(file["filename"].as_str().unwrap_or("pack.mrpack").replace(['/', '\\'], "_"));
    task.stage("Downloading modpack", 0, 0);
    net::download(&Download {
        url: file["url"].as_str().unwrap_or_default().into(),
        dest: dest.clone(),
        sha1: file.pointer("/hashes/sha1").and_then(|h| h.as_str()).map(String::from),
        size: None,
    })
    .await?;
    let pack = PackSource {
        project_id: project_id.into(),
        version_id: ver["id"].as_str().unwrap_or_default().into(),
        title: proj["title"].as_str().unwrap_or_default().into(),
        version_number: ver["version_number"].as_str().unwrap_or_default().into(),
    };
    let mut inst = install_mrpack(paths, &dest, Some(pack), proj["icon_url"].as_str().map(String::from), concurrency, task).await?;
    if let Some(t) = proj["title"].as_str() {
        inst.name = t.to_string();
        instance::save(paths, &inst)?;
    }
    Ok(inst)
}

/// Writes an .mrpack: Modrinth-hosted files by reference, everything else as overrides.
pub fn export_mrpack(paths: &Paths, inst: &Instance, dest: &Path, extra: &[String]) -> Result<PathBuf> {
    let dir = inst.dir(paths);
    let index = ContentIndex::load(&dir);
    let f = std::fs::File::create(dest)?;
    let mut z = zip::ZipWriter::new(f);
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut files = vec![];
    for kind in crate::content::KINDS {
        let Ok(rd) = std::fs::read_dir(dir.join(kind)) else { continue };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.ends_with(".disabled") || !e.path().is_file() {
                continue;
            }
            let meta = index.files.get(&format!("{kind}/{name}"));
            let hosted = meta.and_then(|m| m.url.as_ref()).filter(|u| u.starts_with("https://cdn.modrinth.com/"));
            if let (Some(url), Some(m)) = (hosted, meta) {
                let sha1 = util::sha1_file(&e.path())?;
                if m.sha1.as_deref().map(|s| s == sha1).unwrap_or(true) {
                    files.push(json!({
                        "path": format!("{kind}/{name}"),
                        "hashes": {"sha1": sha1, "sha512": util::sha512_file(&e.path())?},
                        "env": {"client": "required", "server": if kind == "mods" { "required" } else { "unsupported" }},
                        "downloads": [url],
                        "fileSize": e.metadata()?.len()
                    }));
                    continue;
                }
            }
            z.start_file(format!("overrides/{kind}/{name}"), opts)?;
            z.write_all(&std::fs::read(e.path())?)?;
        }
    }
    for item in extra {
        let Some(src) = safe_join(&dir, item) else { continue };
        if src.is_file() {
            z.start_file(format!("overrides/{item}"), opts)?;
            z.write_all(&std::fs::read(&src)?)?;
        } else if src.is_dir() {
            add_dir(&mut z, &src, &format!("overrides/{item}"), opts)?;
        }
    }
    let mut deps = json!({"minecraft": inst.mc_version});
    if let Some(lv) = &inst.loader_version {
        let key = match inst.loader.as_str() {
            "fabric" => Some("fabric-loader"),
            "quilt" => Some("quilt-loader"),
            "forge" => Some("forge"),
            "neoforge" => Some("neoforge"),
            _ => None,
        };
        if let Some(k) = key {
            deps[k] = json!(lv);
        }
    }
    let idx = json!({
        "formatVersion": 1, "game": "minecraft", "versionId": "1.0.0",
        "name": inst.name, "files": files, "dependencies": deps
    });
    z.start_file("modrinth.index.json", opts)?;
    z.write_all(serde_json::to_string_pretty(&idx)?.as_bytes())?;
    z.finish()?;
    Ok(dest.to_path_buf())
}

fn add_dir(z: &mut zip::ZipWriter<std::fs::File>, src: &Path, prefix: &str, opts: zip::write::SimpleFileOptions) -> Result<()> {
    for e in std::fs::read_dir(src)?.flatten() {
        let name = format!("{prefix}/{}", e.file_name().to_string_lossy());
        if e.path().is_dir() {
            add_dir(z, &e.path(), &name, opts)?;
        } else {
            z.start_file(name, opts)?;
            z.write_all(&std::fs::read(e.path())?)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_vs_slug() {
        assert!(looks_like_id("AANobbMI"));
        assert!(looks_like_id("P7dR8mSH"));
        assert!(!looks_like_id("sodium"));
        assert!(!looks_like_id("modmenu"));
        assert!(!looks_like_id("lithium-"));
    }

    #[test]
    fn search_url_encodes_facets() {
        let p = SearchParams {
            query: "sodium extra".into(),
            facets: vec![vec!["project_type:mod".into()], vec!["versions:1.21.1".into()], vec![]],
            index: "downloads".into(),
            offset: 20,
            limit: 20,
        };
        let u = search_url(&p);
        assert!(u.starts_with("https://api.modrinth.com/v2/search?limit=20&offset=20&index=downloads"));
        assert!(u.contains("query=sodium%20extra"));
        assert!(u.contains(&urlencoding::encode(r#"[["project_type:mod"],["versions:1.21.1"]]"#).to_string()));
    }

    #[test]
    fn loader_filters() {
        assert_eq!(loader_filter("mod", "quilt"), vec!["quilt", "fabric"]);
        assert_eq!(loader_filter("mod", "fabric"), vec!["fabric"]);
        assert_eq!(loader_filter("resourcepack", "fabric"), vec!["minecraft"]);
        assert!(loader_filter("shader", "fabric").is_empty());
    }

    #[test]
    fn pack_loader() {
        assert_eq!(loader_from_deps(&json!({"minecraft":"1.20.1","fabric-loader":"0.15.0"})), ("fabric".into(), Some("0.15.0".into())));
        assert_eq!(loader_from_deps(&json!({"minecraft":"1.20.1"})).0, "vanilla");
    }

    #[test]
    fn export_roundtrip_structure() {
        let root = std::env::temp_dir().join(format!("cairn-export-{}", std::process::id()));
        let paths = Paths::at(&root);
        let inst = instance::create(&paths, Instance { name: "Export Test".into(), mc_version: "1.21.1".into(), loader: "fabric".into(), loader_version: Some("0.16.9".into()), ..Default::default() }).unwrap();
        let dir = inst.dir(&paths);
        std::fs::write(dir.join("mods/local.jar"), b"jar").unwrap();
        std::fs::write(dir.join("config/a.toml"), b"x=1").unwrap();
        let out = root.join("out.mrpack");
        export_mrpack(&paths, &inst, &out, &["config".into()]).unwrap();
        let mut z = zip::ZipArchive::new(std::fs::File::open(&out).unwrap()).unwrap();
        let names: Vec<String> = (0..z.len()).map(|i| z.by_index(i).unwrap().name().to_string()).collect();
        assert!(names.contains(&"overrides/mods/local.jar".to_string()));
        assert!(names.contains(&"overrides/config/a.toml".to_string()));
        let mut s = String::new();
        z.by_name("modrinth.index.json").unwrap().read_to_string(&mut s).unwrap();
        let v: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["dependencies"]["fabric-loader"], "0.16.9");
        std::fs::remove_dir_all(&root).unwrap();
    }
}
