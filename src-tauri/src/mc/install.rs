use super::library::{classifier, main_artifact, native_artifact};
use super::rules::{self, Features};
use super::version;
use crate::net::{self, Download};
use crate::paths::Paths;
use crate::tasks::Task;
use anyhow::{Context, Result};
use serde_json::Value;
use std::path::{Path, PathBuf};

pub struct Installed {
    pub json: Value,
    pub jar_id: String,
    pub natives_dir: PathBuf,
}

fn progress(task: &Task, stage: &'static str) -> impl Fn(u64, u64) + Send + Sync + 'static {
    let t = task.clone();
    move |done, total| {
        let step = (total / 100).max(1);
        if done % step == 0 || done == total {
            t.stage(stage, done, total);
        }
    }
}

/// Downloads everything a version needs: client jar, libraries, natives, assets, log config.
pub async fn install_version(paths: &Paths, id: &str, instance_dir: &Path, task: &Task, concurrency: usize) -> Result<Installed> {
    task.stage("Reading version metadata", 0, 0);
    let original = match version::load_local(paths, id) {
        Ok(v) => v,
        Err(_) => version::ensure_vanilla_json(paths, id).await?,
    };
    let merged = version::resolve(paths, id).await?;
    let jar_id = version::jar_id(&original, &merged);
    let features = Features::new();
    let os = rules::os_name();
    let arch_bits = if cfg!(target_pointer_width = "64") { "64" } else { "32" };

    let mut downloads = vec![];

    // Client jar comes from the vanilla JSON.
    let vanilla = version::ensure_vanilla_json(paths, &jar_id).await.unwrap_or_else(|_| merged.clone());
    if let Some(c) = vanilla.pointer("/downloads/client") {
        downloads.push(Download {
            url: c["url"].as_str().unwrap_or_default().to_string(),
            dest: paths.version_jar(&jar_id),
            sha1: c.get("sha1").and_then(|v| v.as_str()).map(String::from),
            sha256: None,
            size: c.get("size").and_then(|v| v.as_u64()),
        });
    }

    let libs = merged.get("libraries").and_then(|l| l.as_array()).cloned().unwrap_or_default();
    let mut native_jars = vec![];
    for lib in &libs {
        if !rules::allowed(lib.get("rules"), &features) {
            continue;
        }
        if lib.get("clientreq") == Some(&Value::Bool(false)) {
            continue;
        }
        if let Some(a) = main_artifact(lib) {
            let dest = paths.libraries().join(&a.path);
            if !a.url.is_empty() {
                downloads.push(Download { url: a.url.clone(), dest: dest.clone(), sha1: a.sha1.clone(), sha256: None, size: a.size });
            }
            let name = lib.get("name").and_then(|n| n.as_str()).unwrap_or_default();
            if classifier(name).map(|c| c.starts_with("natives-")).unwrap_or(false) {
                native_jars.push((dest, lib.clone()));
            }
        }
        if let Some(n) = native_artifact(lib, os, arch_bits) {
            let dest = paths.libraries().join(&n.path);
            downloads.push(Download { url: n.url, dest: dest.clone(), sha1: n.sha1, sha256: None, size: n.size });
            native_jars.push((dest, lib.clone()));
        }
    }

    if let Some(log) = merged.pointer("/logging/client/file") {
        if let (Some(fid), Some(url)) = (log.get("id").and_then(|v| v.as_str()), log.get("url").and_then(|v| v.as_str())) {
            downloads.push(Download {
                url: url.into(),
                dest: paths.assets().join("log_configs").join(fid),
                sha1: log.get("sha1").and_then(|v| v.as_str()).map(String::from),
                sha256: None,
                size: None,
            });
        }
    }

    downloads.sort_by(|a, b| a.dest.cmp(&b.dest));
    downloads.dedup_by(|a, b| a.dest == b.dest);
    let n = downloads.len() as u64;
    task.stage("Downloading game files", 0, n);
    net::download_all(downloads, concurrency, progress(task, "Downloading game files")).await?;

    let natives_dir = paths.natives(id);
    task.stage("Unpacking native libraries", 0, 0);
    std::fs::create_dir_all(&natives_dir)?;
    for (jar, lib) in &native_jars {
        let excludes: Vec<String> = lib
            .pointer("/extract/exclude")
            .and_then(|e| e.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default();
        extract_natives(jar, &natives_dir, &excludes).with_context(|| format!("extracting {}", jar.display()))?;
    }

    install_assets(paths, &merged, instance_dir, task, concurrency).await?;
    Ok(Installed { json: merged, jar_id, natives_dir })
}

fn extract_natives(jar: &Path, dest: &Path, excludes: &[String]) -> Result<()> {
    let f = std::fs::File::open(jar)?;
    let mut zip = zip::ZipArchive::new(f)?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        let name = entry.name().to_string();
        if entry.is_dir() || name.starts_with("META-INF") || excludes.iter().any(|e| name.starts_with(e.as_str())) {
            continue;
        }
        let lower = name.to_lowercase();
        if !(lower.ends_with(".so") || lower.ends_with(".dll") || lower.ends_with(".dylib") || lower.ends_with(".jnilib")) {
            continue;
        }
        // Flatten: LWJGL 3 natives sit in nested folders like linux/x64/org/lwjgl/.
        let file = Path::new(&name).file_name().unwrap().to_owned();
        let out = dest.join(file);
        let mut w = std::fs::File::create(&out)?;
        std::io::copy(&mut entry, &mut w)?;
    }
    Ok(())
}

async fn install_assets(paths: &Paths, v: &Value, instance_dir: &Path, task: &Task, concurrency: usize) -> Result<()> {
    let Some(idx) = v.get("assetIndex") else { return Ok(()) };
    let id = idx["id"].as_str().unwrap_or("legacy").to_string();
    let index_path = paths.assets().join("indexes").join(format!("{id}.json"));
    task.stage("Fetching asset index", 0, 0);
    net::download(&Download {
        url: idx["url"].as_str().unwrap_or_default().to_string(),
        dest: index_path.clone(),
        sha1: idx.get("sha1").and_then(|x| x.as_str()).map(String::from),
        sha256: None,
        size: None,
    })
    .await?;
    let index: Value = crate::util::read_json(&index_path)?;
    let objects = index.get("objects").and_then(|o| o.as_object()).cloned().unwrap_or_default();
    let mut dls = vec![];
    for (_, obj) in &objects {
        let hash = obj["hash"].as_str().unwrap_or_default();
        if hash.len() < 2 {
            continue;
        }
        dls.push(Download {
            url: format!("https://resources.download.minecraft.net/{}/{}", &hash[..2], hash),
            dest: paths.assets().join("objects").join(&hash[..2]).join(hash),
            sha1: Some(hash.to_string()),
            sha256: None,
            size: obj.get("size").and_then(|s| s.as_u64()),
        });
    }
    dls.sort_by(|a, b| a.dest.cmp(&b.dest));
    dls.dedup_by(|a, b| a.dest == b.dest);
    let total = dls.len() as u64;
    task.stage("Downloading assets", 0, total);
    net::download_all(dls, concurrency, progress(task, "Downloading assets")).await?;

    // Pre-1.7 versions read assets from a flat folder rather than the object store.
    let is_virtual = index.get("virtual").and_then(|x| x.as_bool()).unwrap_or(false);
    let map_resources = index.get("map_to_resources").and_then(|x| x.as_bool()).unwrap_or(false);
    if is_virtual || map_resources {
        let base = if map_resources { instance_dir.join("resources") } else { paths.assets().join("virtual").join(&id) };
        for (name, obj) in &objects {
            let hash = obj["hash"].as_str().unwrap_or_default();
            let src = paths.assets().join("objects").join(&hash[..2]).join(hash);
            if let Some(dst) = crate::paths::safe_join(&base, name) {
                if !dst.exists() {
                    if let Some(p) = dst.parent() {
                        std::fs::create_dir_all(p)?;
                    }
                    std::fs::copy(&src, &dst)?;
                }
            }
        }
    }
    Ok(())
}
