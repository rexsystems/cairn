//! CurseForge: browse and search, single-project installs through the resolver, and
//! modpack zip import (`manifest.json` + `overrides/`).
//! Needs a Core API key: Settings, `CAIRN_CURSEFORGE_API_KEY` at runtime, or the same var at build time.
//! Files whose authors disabled third-party distribution are never fetched behind the
//! author's back; they become pending manual downloads (see `manual.rs`).

use crate::instance::{self, Instance};
use crate::net::{self, Download, CLIENT};
use crate::paths::{safe_join, Paths};
use crate::tasks::Task;
use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

pub const GAME_ID: u32 = 432;
const API: &str = "https://api.curseforge.com/v1";

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    minecraft: ManifestMinecraft,
    #[serde(default)]
    manifest_type: String,
    name: String,
    #[serde(default)]
    version: String,
    #[serde(default)]
    files: Vec<ManifestFile>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ManifestMinecraft {
    version: String,
    #[serde(default)]
    mod_loaders: Vec<ManifestLoader>,
    #[serde(default)]
    recommended_ram: Option<u32>,
}

#[derive(Deserialize, Debug)]
struct ManifestLoader {
    id: String,
    #[serde(default)]
    primary: bool,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct ManifestFile {
    #[serde(rename = "projectID")]
    project_id: u64,
    #[serde(rename = "fileID")]
    file_id: u64,
    #[serde(default = "default_required")]
    required: bool,
}

fn default_required() -> bool {
    true
}

fn key_looks_valid(key: &str) -> bool {
    let key = key.trim();
    key.len() >= 20 && (!key.contains('$') || key.starts_with('$'))
}

/// Settings, then the runtime env, then the key baked in at build time from the
/// `CAIRN_CURSEFORGE_API_KEY` CI secret. Shell-mangled values are skipped.
pub fn api_key() -> Result<String> {
    let settings_key = crate::settings::Settings::load(&crate::paths::Paths::new().settings_file()).curseforge_api_key;
    let candidates = [
        settings_key,
        std::env::var("CAIRN_CURSEFORGE_API_KEY").unwrap_or_default(),
        option_env!("CAIRN_CURSEFORGE_API_KEY").unwrap_or_default().to_string(),
    ];
    candidates
        .iter()
        .map(|c| c.trim())
        .find(|k| key_looks_valid(k))
        .map(String::from)
        .ok_or_else(|| {
            anyhow!("CurseForge downloads need an API key. Add one under Settings → CurseForge API key (free at console.curseforge.com).")
        })
}

/// `neoforge-21.1.250` → (`neoforge`, `21.1.250`); same for forge / fabric / quilt.
fn parse_loader(id: &str) -> Result<(String, String)> {
    let id = id.trim();
    for prefix in ["neoforge-", "forge-", "fabric-", "quilt-"] {
        if let Some(rest) = id.strip_prefix(prefix) {
            if rest.is_empty() {
                bail!("modpack loader id '{id}' has no version");
            }
            let name = prefix.trim_end_matches('-').to_string();
            return Ok((name, rest.to_string()));
        }
    }
    bail!("unsupported CurseForge loader '{id}' (need forge, neoforge, fabric, or quilt)");
}

/// CurseForge class id to the Modrinth-style project type the rest of Cairn uses.
pub fn class_type(class_id: i64) -> &'static str {
    match class_id {
        6 => "mod",
        12 => "resourcepack",
        6552 => "shader",
        6945 => "datapack",
        4471 => "modpack",
        17 => "world",
        _ => "other",
    }
}

pub fn type_class(project_type: &str) -> Option<u32> {
    Some(match project_type {
        "mod" => 6,
        "resourcepack" => 12,
        "shader" => 6552,
        "datapack" => 6945,
        "modpack" => 4471,
        _ => return None,
    })
}

/// CurseForge `modLoaderType`.
pub fn loader_type(loader: &str) -> Option<u32> {
    Some(match loader {
        "forge" => 1,
        "fabric" => 4,
        "quilt" => 5,
        "neoforge" => 6,
        _ => return None,
    })
}

async fn cf_get(path: &str, key: &str) -> Result<Value> {
    let url = format!("{API}{path}");
    let mut last = None;
    for attempt in 0..3 {
        let resp = CLIENT
            .get(&url)
            .header("x-api-key", key)
            .header("Accept", "application/json")
            .send()
            .await
            .with_context(|| format!("CurseForge GET {path}"))?;
        let status = resp.status();
        if status.is_success() {
            return resp.json().await.with_context(|| format!("decoding {path}"));
        }
        if status.as_u16() == 401 || status.as_u16() == 403 {
            let body = resp.text().await.unwrap_or_default();
            bail!("CurseForge {path} returned {status}: {body}");
        }
        if status.is_client_error() {
            let body = resp.text().await.unwrap_or_default();
            bail!("CurseForge {path} returned {status}: {body}");
        }
        last = Some(anyhow!("CurseForge {path} returned {status}"));
        tokio::time::sleep(std::time::Duration::from_millis(400 * (attempt + 1))).await;
    }
    Err(last.unwrap())
}

async fn cf_post(path: &str, key: &str, body: &Value) -> Result<Value> {
    let url = format!("{API}{path}");
    let mut last = None;
    for attempt in 0..3 {
        let resp = CLIENT
            .post(&url)
            .header("x-api-key", key)
            .header("Accept", "application/json")
            .json(body)
            .send()
            .await
            .with_context(|| format!("CurseForge POST {path}"))?;
        let status = resp.status();
        if status.is_success() {
            return resp.json().await.with_context(|| format!("decoding {path}"));
        }
        if status.as_u16() == 401 || status.as_u16() == 403 {
            let body = resp.text().await.unwrap_or_default();
            bail!("CurseForge {path} returned {status}: {body}");
        }
        if status.is_client_error() {
            let body = resp.text().await.unwrap_or_default();
            bail!("CurseForge {path} returned {status}: {body}");
        }
        last = Some(anyhow!("CurseForge {path} returned {status}"));
        tokio::time::sleep(std::time::Duration::from_millis(400 * (attempt + 1))).await;
    }
    Err(last.unwrap())
}

async fn fetch_files_batch(key: &str, file_ids: &[u64]) -> Result<HashMap<u64, Value>> {
    let mut out = HashMap::new();
    for chunk in file_ids.chunks(50) {
        let body = json!({ "fileIds": chunk });
        let data = cf_post("/mods/files", key, &body).await?;
        for f in data["data"].as_array().cloned().unwrap_or_default() {
            if let Some(id) = f["id"].as_u64() {
                out.insert(id, f);
            }
        }
    }
    Ok(out)
}

async fn fetch_mods_batch(key: &str, mod_ids: &[u64]) -> Result<HashMap<u64, Value>> {
    let mut out = HashMap::new();
    for chunk in mod_ids.chunks(50) {
        let body = json!({ "modIds": chunk });
        let data = cf_post("/mods", key, &body).await?;
        for m in data["data"].as_array().cloned().unwrap_or_default() {
            if let Some(id) = m["id"].as_u64() {
                out.insert(id, m);
            }
        }
    }
    Ok(out)
}

/// The official download URL, or None when the author disabled third-party downloads.
async fn resolve_download_url(key: &str, project_id: u64, file_id: u64, file: &Value) -> Option<String> {
    if let Some(u) = file["downloadUrl"].as_str().filter(|s| !s.is_empty()) {
        return Some(u.to_string());
    }
    let data = cf_get(&format!("/mods/{project_id}/files/{file_id}/download-url"), key).await.ok()?;
    data["data"].as_str().filter(|s| !s.is_empty()).map(String::from)
}

fn sha1_from_file(file: &Value) -> Option<String> {
    file["hashes"]
        .as_array()?
        .iter()
        .find(|h| h["algo"].as_i64() == Some(1)) // 1 = Sha1 on CF
        .and_then(|h| h["value"].as_str())
        .map(str::to_string)
}

pub fn is_curseforge_zip(path: &Path) -> bool {
    let Ok(f) = std::fs::File::open(path) else { return false };
    let Ok(mut zip) = zip::ZipArchive::new(f) else { return false };
    let Ok(mut entry) = zip.by_name("manifest.json") else { return false };
    let mut s = String::new();
    if entry.read_to_string(&mut s).is_err() {
        return false;
    }
    serde_json::from_str::<Value>(&s)
        .ok()
        .and_then(|v| v.get("manifestType")?.as_str().map(|t| t == "minecraftModpack"))
        .unwrap_or(false)
}

pub async fn install_curseforge_zip(paths: &Paths, file: &Path, concurrency: usize, task: &Task) -> Result<Instance> {
    let key = api_key()?;
    task.stage("Reading CurseForge pack", 0, 0);
    let mut zip = zip::ZipArchive::new(std::fs::File::open(file).with_context(|| format!("opening {}", file.display()))?)
        .context("this file is not a valid zip")?;
    let mut manifest_s = String::new();
    zip.by_name("manifest.json")
        .context("manifest.json is missing — not a CurseForge modpack")?
        .read_to_string(&mut manifest_s)?;
    let manifest: Manifest = serde_json::from_str(&manifest_s).context("invalid manifest.json")?;
    if !manifest.manifest_type.is_empty() && manifest.manifest_type != "minecraftModpack" {
        bail!("unsupported manifest type '{}'", manifest.manifest_type);
    }
    let loader_id = manifest
        .minecraft
        .mod_loaders
        .iter()
        .find(|l| l.primary)
        .or_else(|| manifest.minecraft.mod_loaders.first())
        .ok_or_else(|| anyhow!("modpack does not list a mod loader"))?;
    let (loader, loader_version) = parse_loader(&loader_id.id)?;
    let settings = crate::settings::Settings::load(&paths.settings_file());
    let memory = manifest
        .minecraft
        .recommended_ram
        .unwrap_or_else(|| settings.default_memory_mb.max(4096));
    let name = if manifest.version.is_empty() {
        manifest.name.clone()
    } else {
        format!("{} {}", manifest.name, manifest.version)
    };
    let inst = instance::create(
        paths,
        Instance {
            name,
            mc_version: manifest.minecraft.version.clone(),
            loader,
            loader_version: Some(loader_version),
            memory_mb: Some(memory),
            ..Default::default()
        },
    )?;
    let dir = inst.dir(paths);
    let result: Result<()> = async {
        let required: Vec<ManifestFile> = manifest.files.into_iter().filter(|f| f.required).collect();
        if required.is_empty() {
            bail!("modpack lists no required files");
        }
        task.stage("Resolving CurseForge files", 0, required.len() as u64);
        let file_ids: Vec<u64> = required.iter().map(|f| f.file_id).collect();
        let mod_ids: Vec<u64> = {
            let mut ids: Vec<u64> = required.iter().map(|f| f.project_id).collect();
            ids.sort_unstable();
            ids.dedup();
            ids
        };
        let files_by_id = fetch_files_batch(&key, &file_ids).await?;
        let mods_by_id = fetch_mods_batch(&key, &mod_ids).await?;

        let mut items = Vec::with_capacity(required.len());
        for (i, mf) in required.iter().enumerate() {
            task.stage("Resolving CurseForge files", i as u64 + 1, required.len() as u64);
            let meta = files_by_id
                .get(&mf.file_id)
                .ok_or_else(|| anyhow!("CurseForge file {} (project {}) was not found", mf.file_id, mf.project_id))?;
            let m = mods_by_id.get(&mf.project_id).cloned().unwrap_or_else(|| json!({ "id": mf.project_id, "classId": 6 }));
            let info = project_info(&m);
            let mut c = candidate(&info, meta).ok_or_else(|| anyhow!("CurseForge file {} has no file name", mf.file_id))?;
            if c.url.is_none() {
                c.url = resolve_download_url(&key, mf.project_id, mf.file_id, meta).await;
            }
            if c.folder().is_none() {
                // Worlds and other classes: keep them out of mods/ rather than guessing.
                continue;
            }
            items.push(crate::deps::PlanItem { candidate: c, reason: crate::deps::Reason::Requested, replaces: vec![], optional: false });
        }
        let plan = crate::deps::Plan { items, ..Default::default() };
        let applied = crate::deps::apply(&dir, &plan, concurrency, task).await?;
        if !applied.pending.is_empty() {
            task.stage(&format!("{} file(s) need a manual download from CurseForge", applied.pending.len()), 0, 0);
        }

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
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut w = std::fs::File::create(&dest)?;
                std::io::copy(&mut e, &mut w)?;
            }
        }
        Ok(())
    }
    .await;
    if let Err(e) = result {
        let _ = std::fs::remove_dir_all(&dir);
        return Err(e);
    }
    instance::get(paths, &inst.id)
}

/// CurseForge file fingerprint: MurmurHash2 (seed 1) over the bytes with tabs, newlines,
/// carriage returns and spaces removed.
pub fn fingerprint(bytes: &[u8]) -> u32 {
    let data: Vec<u8> = bytes.iter().copied().filter(|b| !matches!(b, 9 | 10 | 13 | 32)).collect();
    murmur2(&data, 1)
}

pub fn fingerprint_file(path: &Path) -> Result<u32> {
    Ok(fingerprint(&std::fs::read(path)?))
}

fn murmur2(data: &[u8], seed: u32) -> u32 {
    const M: u32 = 0x5bd1_e995;
    let mut h = seed ^ data.len() as u32;
    // Plain `chunks` instead of `as_chunks`, which needs a newer Rust than some packaging toolchains.
    let whole = data.len() / 4 * 4;
    for c in data[..whole].chunks(4) {
        let mut k = u32::from_le_bytes([c[0], c[1], c[2], c[3]]);
        k = k.wrapping_mul(M);
        k ^= k >> 24;
        k = k.wrapping_mul(M);
        h = h.wrapping_mul(M) ^ k;
    }
    let tail = &data[whole..];
    if !tail.is_empty() {
        for (i, b) in tail.iter().enumerate() {
            h ^= (*b as u32) << (8 * i);
        }
        h = h.wrapping_mul(M);
    }
    h ^= h >> 13;
    h = h.wrapping_mul(M);
    h ^ (h >> 15)
}

pub fn project_info(m: &Value) -> crate::deps::ProjectInfo {
    crate::deps::ProjectInfo {
        id: m["id"].as_u64().map(|x| x.to_string()).unwrap_or_default(),
        slug: m["slug"].as_str().unwrap_or_default().into(),
        title: m["name"].as_str().unwrap_or_default().into(),
        project_type: class_type(m["classId"].as_i64().unwrap_or(6)).into(),
        icon: m.pointer("/logo/thumbnailUrl").and_then(|x| x.as_str()).map(String::from),
        page_url: m.pointer("/links/websiteUrl").and_then(|x| x.as_str()).map(String::from),
    }
}

const LOADER_TAGS: [&str; 4] = ["forge", "fabric", "quilt", "neoforge"];

pub fn candidate(p: &crate::deps::ProjectInfo, f: &Value) -> Option<crate::deps::Candidate> {
    use crate::deps::{Candidate, Rel, Source, VDep};
    let id = f["id"].as_u64()?;
    let tags: Vec<String> = f["gameVersions"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
    let loaders = tags.iter().map(|t| t.to_ascii_lowercase()).filter(|t| LOADER_TAGS.contains(&t.as_str())).collect();
    let game_versions = tags.iter().filter(|t| t.chars().next().is_some_and(|c| c.is_ascii_digit())).cloned().collect();
    let deps = f["dependencies"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|d| {
                    let rel = match d["relationType"].as_i64()? {
                        3 => Rel::Required,
                        2 => Rel::Optional,
                        5 => Rel::Incompatible,
                        1 | 6 => Rel::Embedded,
                        _ => return None,
                    };
                    Some(VDep { project_id: d["modId"].as_u64().map(|x| x.to_string()), version_id: None, rel })
                })
                .collect()
        })
        .unwrap_or_default();
    Some(Candidate {
        source: Source::Curseforge,
        project: p.clone(),
        version_id: id.to_string(),
        version_number: f["displayName"].as_str().unwrap_or_default().into(),
        file_name: f["fileName"].as_str().filter(|s| !s.is_empty())?.into(),
        url: f["downloadUrl"].as_str().filter(|s| !s.is_empty()).map(String::from),
        download_page: p.page_url.as_ref().map(|u| format!("{}/download/{id}", u.trim_end_matches('/'))),
        sha1: sha1_from_file(f),
        sha512: None,
        size: f["fileLength"].as_u64(),
        fingerprint: f["fileFingerprint"].as_u64().map(|x| x as u32),
        deps,
        loaders,
        game_versions,
    })
}

/// Newest file first, releases before betas before alphas.
fn best_file<'a>(files: &'a [Value], p: &crate::deps::ProjectInfo, t: &crate::deps::Target) -> Option<&'a Value> {
    let mut ok: Vec<&Value> = files
        .iter()
        .filter(|f| f["isAvailable"].as_bool() != Some(false) && f["isServerPack"].as_bool() != Some(true))
        .filter(|f| candidate(p, f).map(|c| t.accepts(&c)).unwrap_or(false))
        .collect();
    ok.sort_by(|a, b| b["fileDate"].as_str().unwrap_or("").cmp(a["fileDate"].as_str().unwrap_or("")));
    ok.iter().find(|f| f["releaseType"].as_i64() == Some(1)).or(ok.first()).copied()
}

/// CurseForge as a dependency source for the resolver.
pub struct Repo {
    key: String,
}

impl Repo {
    pub fn new() -> Result<Repo> {
        Ok(Repo { key: api_key()? })
    }

    async fn files(&self, id: &str, mc: &str, loader: Option<u32>) -> Result<Vec<Value>> {
        let mut path = format!("/mods/{id}/files?pageSize=50&gameVersion={}", urlencoding::encode(mc));
        if let Some(l) = loader {
            path.push_str(&format!("&modLoaderType={l}"));
        }
        Ok(cf_get(&path, &self.key).await?["data"].as_array().cloned().unwrap_or_default())
    }
}

impl crate::deps::Repo for Repo {
    fn source(&self) -> crate::deps::Source {
        crate::deps::Source::Curseforge
    }
    async fn project(&self, id: &str) -> Result<crate::deps::ProjectInfo> {
        let m = cf_get(&format!("/mods/{}", urlencoding::encode(id)), &self.key).await?;
        Ok(project_info(&m["data"]))
    }
    async fn pick(&self, p: &crate::deps::ProjectInfo, t: &crate::deps::Target) -> Result<Option<crate::deps::Candidate>> {
        // Loader filters only apply to mods. Quilt falls back to Fabric builds, and
        // NeoForge 1.20.1 falls back to Forge builds, matching what those loaders accept.
        let mut tries: Vec<Option<u32>> = vec![];
        if p.project_type == "mod" {
            tries.push(loader_type(&t.loader));
            match t.loader.as_str() {
                "quilt" => tries.push(loader_type("fabric")),
                "neoforge" if t.mc_version == "1.20.1" => tries.push(loader_type("forge")),
                _ => {}
            }
        } else {
            tries.push(None);
        }
        for l in tries {
            let files = self.files(&p.id, &t.mc_version, l).await?;
            if let Some(f) = best_file(&files, p, t) {
                return Ok(candidate(p, f));
            }
        }
        Ok(None)
    }
    async fn exact(&self, p: &crate::deps::ProjectInfo, version_id: &str) -> Result<crate::deps::Candidate> {
        let f = cf_get(&format!("/mods/{}/files/{}", p.id, urlencoding::encode(version_id)), &self.key).await?;
        candidate(p, &f["data"]).ok_or_else(|| anyhow!("{} file {version_id} has no file name", p.title))
    }
    async fn project_of_version(&self, version_id: &str) -> Result<String> {
        bail!("CurseForge dependency {version_id} does not name a project")
    }
}

#[derive(Deserialize, Debug, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct SearchParams {
    pub query: String,
    pub project_type: String,
    pub game_version: String,
    pub loader: String,
    /// relevance, downloads, follows, updated, newest, name
    pub sort: String,
    pub offset: u32,
    pub limit: u32,
}

pub fn search_path(p: &SearchParams) -> Result<String> {
    let class = type_class(&p.project_type).ok_or_else(|| anyhow!("CurseForge has no {} category", p.project_type))?;
    // CurseForge rejects index + pageSize above 10,000.
    let limit = p.limit.clamp(1, 50);
    let offset = p.offset.min(10_000 - limit);
    let (field, order) = match p.sort.as_str() {
        "downloads" => (6, "desc"),
        "follows" => (12, "desc"),
        "updated" => (3, "desc"),
        "newest" => (11, "desc"),
        "name" => (4, "asc"),
        _ => (if p.query.trim().is_empty() { 2 } else { 1 }, "desc"),
    };
    let mut path = format!("/mods/search?gameId={GAME_ID}&classId={class}&index={offset}&pageSize={limit}&sortField={field}&sortOrder={order}");
    if !p.query.trim().is_empty() {
        path.push_str(&format!("&searchFilter={}", urlencoding::encode(p.query.trim())));
    }
    if !p.game_version.is_empty() {
        path.push_str(&format!("&gameVersion={}", urlencoding::encode(&p.game_version)));
    }
    if matches!(p.project_type.as_str(), "mod" | "modpack") {
        if let Some(l) = loader_type(&p.loader) {
            path.push_str(&format!("&modLoaderType={l}"));
        }
    }
    Ok(path)
}

/// One CurseForge project in the same shape as a Modrinth search hit, so the UI shares one card.
pub fn hit(m: &Value) -> Value {
    let info = project_info(m);
    let cats: Vec<String> = m["categories"].as_array().map(|a| a.iter().filter_map(|c| c["name"].as_str().map(|n| n.to_lowercase())).collect()).unwrap_or_default();
    let shots: Vec<String> = m["screenshots"].as_array().map(|a| a.iter().filter_map(|c| c["url"].as_str().map(String::from)).collect()).unwrap_or_default();
    json!({
        "project_id": info.id,
        "slug": info.slug,
        "title": info.title,
        "description": m["summary"],
        "author": m.pointer("/authors/0/name").and_then(|x| x.as_str()).unwrap_or(""),
        "icon_url": info.icon,
        "downloads": m["downloadCount"].as_f64().unwrap_or(0.0) as u64,
        "follows": m["thumbsUpCount"].as_u64().unwrap_or(0),
        "date_modified": m["dateModified"],
        "categories": cats,
        "display_categories": cats,
        "gallery": shots,
        "featured_gallery": shots.first(),
        "project_type": info.project_type,
        "source": "curseforge",
        "page_url": info.page_url,
        "distribution": m["allowModDistribution"].as_bool().unwrap_or(true),
    })
}

pub async fn search(p: &SearchParams) -> Result<Value> {
    let key = api_key()?;
    let data = cf_get(&search_path(p)?, &key).await?;
    let hits: Vec<Value> = data["data"].as_array().map(|a| a.iter().map(hit).collect()).unwrap_or_default();
    let total = data.pointer("/pagination/totalCount").and_then(|x| x.as_u64()).unwrap_or(hits.len() as u64).min(10_000);
    Ok(json!({ "hits": hits, "total_hits": total, "offset": p.offset, "limit": p.limit }))
}

/// Project details for the modal: the hit fields plus description HTML and links.
pub async fn project(id: &str) -> Result<Value> {
    let key = api_key()?;
    let id = urlencoding::encode(id).into_owned();
    let (mp, dp) = (format!("/mods/{id}"), format!("/mods/{id}/description"));
    let (m, desc) = futures::join!(cf_get(&mp, &key), cf_get(&dp, &key));
    let m = m?["data"].clone();
    let mut out = hit(&m);
    out["body_html"] = desc.map(|d| d["data"].clone()).unwrap_or(Value::Null);
    out["source_url"] = m.pointer("/links/sourceUrl").cloned().unwrap_or(Value::Null);
    out["issues_url"] = m.pointer("/links/issuesUrl").cloned().unwrap_or(Value::Null);
    out["wiki_url"] = m.pointer("/links/wikiUrl").cloned().unwrap_or(Value::Null);
    Ok(out)
}

/// Files of a project for the version picker, newest first, in a Modrinth-like shape.
pub async fn files(id: &str, game_version: &str, loader: &str) -> Result<Vec<Value>> {
    let key = api_key()?;
    let mut path = format!("/mods/{}/files?pageSize=50", urlencoding::encode(id));
    if !game_version.is_empty() {
        path.push_str(&format!("&gameVersion={}", urlencoding::encode(game_version)));
    }
    if let Some(l) = loader_type(loader) {
        path.push_str(&format!("&modLoaderType={l}"));
    }
    let data = cf_get(&path, &key).await?;
    let mut out: Vec<Value> = data["data"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|f| f["isServerPack"].as_bool() != Some(true))
        .map(|f| {
            let tags: Vec<String> = f["gameVersions"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
            json!({
                "id": f["id"].as_u64().map(|x| x.to_string()),
                "name": f["displayName"],
                "version_number": f["displayName"],
                "version_type": match f["releaseType"].as_i64() { Some(2) => "beta", Some(3) => "alpha", _ => "release" },
                "date_published": f["fileDate"],
                "downloads": f["downloadCount"],
                "loaders": tags.iter().map(|t| t.to_ascii_lowercase()).filter(|t| LOADER_TAGS.contains(&t.as_str())).collect::<Vec<_>>(),
                "game_versions": tags.iter().filter(|t| t.chars().next().is_some_and(|c| c.is_ascii_digit())).collect::<Vec<_>>(),
                "manual": f["downloadUrl"].as_str().map(|s| s.is_empty()).unwrap_or(true),
            })
        })
        .collect();
    out.sort_by(|a, b| b["date_published"].as_str().unwrap_or("").cmp(a["date_published"].as_str().unwrap_or("")));
    Ok(out)
}

/// Turns a CurseForge modpack project into a new instance by downloading its zip.
pub async fn install_modpack_project(paths: &Paths, project_id: &str, file_id: Option<&str>, concurrency: usize, task: &Task) -> Result<Instance> {
    let key = api_key()?;
    task.stage("Finding the modpack file", 0, 0);
    let m = cf_get(&format!("/mods/{}", urlencoding::encode(project_id)), &key).await?["data"].clone();
    let info = project_info(&m);
    if info.project_type != "modpack" {
        bail!("{} is not a modpack", info.title);
    }
    let file = match file_id {
        Some(f) => cf_get(&format!("/mods/{}/files/{}", info.id, urlencoding::encode(f)), &key).await?["data"].clone(),
        None => {
            let files = cf_get(&format!("/mods/{}/files?pageSize=50", info.id), &key).await?["data"].as_array().cloned().unwrap_or_default();
            let mut ok: Vec<Value> = files.into_iter().filter(|f| f["isServerPack"].as_bool() != Some(true) && f["isAvailable"].as_bool() != Some(false)).collect();
            ok.sort_by(|a, b| b["fileDate"].as_str().unwrap_or("").cmp(a["fileDate"].as_str().unwrap_or("")));
            ok.iter().find(|f| f["releaseType"].as_i64() == Some(1)).or(ok.first()).cloned().ok_or_else(|| anyhow!("{} has no files", info.title))?
        }
    };
    let fid = file["id"].as_u64().ok_or_else(|| anyhow!("CurseForge returned a file without an id"))?;
    let Some(url) = resolve_download_url(&key, m["id"].as_u64().unwrap_or_default(), fid, &file).await else {
        bail!("The author of {} only allows downloads from the CurseForge website. Download the zip there and import it with Add instance.", info.title);
    };
    let tmp = paths.cache.join("modpacks").join(format!("curseforge-{fid}.zip"));
    task.stage(&format!("Downloading {}", info.title), 0, 1);
    net::download(&Download { url, dest: tmp.clone(), sha1: sha1_from_file(&file), sha256: None, size: file["fileLength"].as_u64() }).await?;
    let r = install_curseforge_zip(paths, &tmp, concurrency, task).await;
    let _ = std::fs::remove_file(&tmp);
    let mut inst = r?;
    inst.name = info.title.clone();
    if let Some(icon) = info.icon {
        if net::download(&Download { url: icon, dest: inst.dir(paths).join("icon.png"), ..Default::default() }).await.is_ok() {
            inst.icon = Some("icon.png".into());
        }
    }
    instance::save(paths, &inst)?;
    Ok(inst)
}

/// Newer files for CurseForge-installed content. CurseForge file ids only grow, so a
/// higher id than the installed one is newer.
pub async fn check_updates(paths: &Paths, inst: &Instance) -> Result<Vec<crate::modrinth::UpdateInfo>> {
    use crate::deps::Repo as _;
    use futures::StreamExt;
    let dir = inst.dir(paths);
    let index = crate::content::ContentIndex::load(&dir);
    let entries: Vec<(String, crate::content::ContentMeta, String)> = index
        .files
        .iter()
        .filter(|(_, m)| m.source.as_deref() == Some("curseforge") && m.project_id.is_some())
        .filter_map(|(k, m)| {
            let on_disk = crate::deps::existing_file(&dir, k)?;
            let name = on_disk.file_name()?.to_string_lossy().to_string();
            Some((k.clone(), m.clone(), name))
        })
        .collect();
    if entries.is_empty() {
        return Ok(vec![]);
    }
    let repo = Repo::new()?;
    let target = crate::deps::Target::of(inst);
    let repo = &repo;
    let target = &target;
    let mut out: Vec<crate::modrinth::UpdateInfo> = futures::stream::iter(entries)
        .map(|(key, meta, name)| async move {
            let pid = meta.project_id.clone()?;
            let info = repo.project(&pid).await.ok()?;
            let c = repo.pick(&info, target).await.ok()??;
            let cur: u64 = meta.version_id.as_deref().and_then(|v| v.parse().ok()).unwrap_or(0);
            let new: u64 = c.version_id.parse().ok()?;
            (new > cur).then(|| crate::modrinth::UpdateInfo {
                kind: key.split('/').next().unwrap_or("mods").into(),
                file_name: name,
                title: info.title.clone(),
                icon: info.icon.clone(),
                project_id: pid,
                current_version: meta.version_number.clone().unwrap_or_else(|| "installed".into()),
                new_version: c.version_number.clone(),
                new_version_id: c.version_id.clone(),
                source: "curseforge".into(),
            })
        })
        .buffer_unordered(8)
        .filter_map(|x| async { x })
        .collect()
        .await;
    out.sort_by_key(|a| a.title.to_lowercase());
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loader_ids() {
        assert_eq!(parse_loader("neoforge-21.1.250").unwrap(), ("neoforge".into(), "21.1.250".into()));
        assert_eq!(parse_loader("forge-47.2.0").unwrap(), ("forge".into(), "47.2.0".into()));
        assert_eq!(parse_loader("fabric-0.16.9").unwrap(), ("fabric".into(), "0.16.9".into()));
        assert!(parse_loader("quilt").is_err());
    }

    #[test]
    fn class_types() {
        assert_eq!(class_type(6), "mod");
        assert_eq!(class_type(12), "resourcepack");
        assert_eq!(class_type(6552), "shader");
        assert_eq!(type_class("modpack"), Some(4471));
    }

    #[test]
    fn murmur2_matches_reference() {
        // Reference values from the `murmurhash2` Python package, seed 1.
        assert_eq!(murmur2(b"", 1), 1540447798);
        assert_eq!(murmur2(b"a", 1), 626045324);
        assert_eq!(murmur2(b"hello", 1), 2788266382);
        assert_eq!(murmur2(b"abcdefg", 1), 184182053);
        assert_eq!(fingerprint(b"The quick brown fox jumps over the lazy dog"), 3751777527);
        assert_eq!(fingerprint(b"a b\tc\r\nd"), fingerprint(b"abcd"));
    }

    #[test]
    fn search_paths() {
        let p = SearchParams { query: "jei".into(), project_type: "mod".into(), game_version: "1.20.1".into(), loader: "neoforge".into(), sort: "downloads".into(), offset: 20, limit: 20 };
        let path = search_path(&p).unwrap();
        assert!(path.contains("classId=6") && path.contains("searchFilter=jei") && path.contains("modLoaderType=6") && path.contains("sortField=6") && path.contains("index=20"));
        let deep = SearchParams { project_type: "shader".into(), offset: 50_000, limit: 20, loader: "fabric".into(), ..Default::default() };
        let path = search_path(&deep).unwrap();
        assert!(path.contains("index=9980") && !path.contains("modLoaderType"));
        assert!(search_path(&SearchParams { project_type: "world".into(), ..Default::default() }).is_err());
    }

    #[test]
    fn file_to_candidate() {
        let info = project_info(&json!({"id": 238222, "slug": "jei", "name": "JEI", "classId": 6, "links": {"websiteUrl": "https://www.curseforge.com/minecraft/mc-mods/jei"}}));
        let f = json!({"id": 5101366, "displayName": "jei-1.20.1-forge-15.3.0.4", "fileName": "jei-1.20.1-forge-15.3.0.4.jar",
            "downloadUrl": null, "fileLength": 1234, "fileFingerprint": 4012345678u64,
            "hashes": [{"value": "abc", "algo": 1}, {"value": "def", "algo": 2}],
            "gameVersions": ["1.20.1", "Forge", "NeoForge", "Client"],
            "dependencies": [{"modId": 1, "relationType": 3}, {"modId": 2, "relationType": 2}, {"modId": 3, "relationType": 4}]});
        let c = candidate(&info, &f).unwrap();
        assert!(c.manual());
        assert_eq!(c.download_page.as_deref(), Some("https://www.curseforge.com/minecraft/mc-mods/jei/download/5101366"));
        assert_eq!(c.sha1.as_deref(), Some("abc"));
        assert_eq!(c.loaders, vec!["forge", "neoforge"]);
        assert_eq!(c.game_versions, vec!["1.20.1"]);
        assert_eq!(c.deps.len(), 2);
        assert_eq!(c.fingerprint, Some(4012345678));
        let t = crate::deps::Target { loader: "neoforge".into(), mc_version: "1.20.1".into() };
        assert!(t.accepts(&c));
        let t = crate::deps::Target { loader: "fabric".into(), mc_version: "1.20.1".into() };
        assert!(!t.accepts(&c));
    }
}
