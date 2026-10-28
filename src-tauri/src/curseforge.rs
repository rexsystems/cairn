//! CurseForge modpack zip import (`manifest.json` + `overrides/`).
//! Downloads require `CAIRN_CURSEFORGE_API_KEY` (Core API key from console.curseforge.com).

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

const API: &str = "https://api.curseforge.com/v1";

/// Built-in Core API key (public in this repo on purpose). Settings / env still override.
const BUILTIN_API_KEY: &str = "$2a$10$.7hi7X13w/SDOW22a10f/.bn/zAyC9nI/Tcai.I/Q1FE/ufUzTrJe";

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

fn api_key() -> Result<String> {
    // Settings → env → compile-time → built-in. Skip shell-corrupted values.
    let settings_key = {
        let root = std::env::var_os("CAIRN_HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join("cairn"));
        crate::settings::Settings::load(&root.join("settings.json")).curseforge_api_key
    };
    let embedded = option_env!("CAIRN_CURSEFORGE_API_KEY").unwrap_or("");
    // Built-in first so a mangled shell env cannot break imports.
    let candidates = [
        BUILTIN_API_KEY.to_string(),
        settings_key,
        std::env::var("CAIRN_CURSEFORGE_API_KEY").unwrap_or_default(),
        embedded.to_string(),
    ];
    for c in &candidates {
        let key = c.trim();
        if !key.is_empty() && key_looks_valid(key) {
            return Ok(key.to_string());
        }
    }
    bail!("No usable CurseForge API key.");
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

fn folder_for_class(class_id: i64) -> &'static str {
    match class_id {
        12 => "resourcepacks",
        17 => "saves",
        6552 => "shaderpacks",
        6945 => "datapacks",
        _ => "mods", // 6 Mods and anything unknown
    }
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
            return Ok(resp.json().await.with_context(|| format!("decoding {path}"))?);
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
            return Ok(resp.json().await.with_context(|| format!("decoding {path}"))?);
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

async fn fetch_mods_batch(key: &str, mod_ids: &[u64]) -> Result<HashMap<u64, i64>> {
    let mut out = HashMap::new();
    for chunk in mod_ids.chunks(50) {
        let body = json!({ "modIds": chunk });
        let data = cf_post("/mods", key, &body).await?;
        for m in data["data"].as_array().cloned().unwrap_or_default() {
            if let (Some(id), Some(class_id)) = (m["id"].as_u64(), m["classId"].as_i64()) {
                out.insert(id, class_id);
            }
        }
    }
    Ok(out)
}

/// CurseForge CDN path: `…/files/{fileId/1000}/{fileId%1000}/{fileName}`.
fn forgecdn_url(file_id: u64, file_name: &str) -> String {
    let a = file_id / 1000;
    let b = file_id % 1000;
    let enc = urlencoding::encode(file_name);
    format!("https://edge.forgecdn.net/files/{a}/{b}/{enc}")
}

async fn resolve_download_url(key: &str, project_id: u64, file_id: u64, file: &Value) -> Result<String> {
    if let Some(u) = file["downloadUrl"].as_str().filter(|s| !s.is_empty()) {
        return Ok(u.to_string());
    }
    let file_name = file["fileName"].as_str().filter(|s| !s.is_empty()).unwrap_or("mod.jar");
    // Many authors disable this endpoint (403) while the jar is still on forgecdn.
    if let Ok(data) = cf_get(&format!("/mods/{project_id}/files/{file_id}/download-url"), key).await {
        if let Some(u) = data["data"].as_str().filter(|s| !s.is_empty()) {
            return Ok(u.to_string());
        }
    }
    Ok(forgecdn_url(file_id, file_name))
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
        let class_by_mod = fetch_mods_batch(&key, &mod_ids).await?;

        let mut dls = Vec::with_capacity(required.len());
        for (i, mf) in required.iter().enumerate() {
            task.stage("Resolving CurseForge files", i as u64 + 1, required.len() as u64);
            let meta = files_by_id
                .get(&mf.file_id)
                .ok_or_else(|| anyhow!("CurseForge file {} (project {}) was not found", mf.file_id, mf.project_id))?;
            let file_name = meta["fileName"]
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or_else(|| anyhow!("CurseForge file {} has no fileName", mf.file_id))?;
            let folder = folder_for_class(class_by_mod.get(&mf.project_id).copied().unwrap_or(6));
            let rel = format!("{folder}/{file_name}");
            let dest = safe_join(&dir, &rel).ok_or_else(|| anyhow!("unsafe path in pack: {rel}"))?;
            let url = resolve_download_url(&key, mf.project_id, mf.file_id, meta).await?;
            dls.push(Download {
                url,
                dest,
                sha1: sha1_from_file(meta),
                size: meta["fileLength"].as_u64(),
            });
        }

        let total = dls.len() as u64;
        let t = task.clone();
        task.stage("Downloading CurseForge files", 0, total);
        net::download_all(dls, concurrency, move |d, n| t.stage("Downloading CurseForge files", d, n)).await?;

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
    fn class_folders() {
        assert_eq!(folder_for_class(6), "mods");
        assert_eq!(folder_for_class(12), "resourcepacks");
        assert_eq!(folder_for_class(6552), "shaderpacks");
    }
}
