use crate::mc::version;
use crate::net::{self, Download};
use crate::paths::Paths;
use crate::tasks::Task;
use anyhow::{anyhow, bail, Context, Result};
use serde::Serialize;
use serde_json::Value;
use std::io::Read;

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LoaderVersion {
    pub version: String,
    pub stable: bool,
}

/// NeoForge versions encode the Minecraft version: 21.1.77 -> 1.21.1, 20.4.237 -> 1.20.4, 26.1.0.5 -> 26.1.
pub fn neoforge_mc(v: &str) -> Option<String> {
    let parts: Vec<&str> = v.split(['.', '-']).collect();
    let major: u32 = parts.first()?.parse().ok()?;
    let minor: u32 = parts.get(1)?.parse().ok()?;
    if major < 25 {
        Some(if minor == 0 { format!("1.{major}") } else { format!("1.{major}.{minor}") })
    } else {
        let patch: u32 = parts.get(2).and_then(|p| p.parse().ok()).unwrap_or(0);
        Some(if patch == 0 { format!("{major}.{minor}") } else { format!("{major}.{minor}.{patch}") })
    }
}

pub async fn versions(loader: &str, mc: &str) -> Result<Vec<LoaderVersion>> {
    Ok(match loader {
        "fabric" => {
            let v: Vec<Value> = net::get_json(&format!("https://meta.fabricmc.net/v2/versions/loader/{mc}")).await?;
            v.iter()
                .filter_map(|e| Some(LoaderVersion { version: e.pointer("/loader/version")?.as_str()?.into(), stable: e.pointer("/loader/stable").and_then(|s| s.as_bool()).unwrap_or(true) }))
                .collect()
        }
        "quilt" => {
            let v: Vec<Value> = net::get_json(&format!("https://meta.quiltmc.org/v3/versions/loader/{mc}")).await?;
            v.iter()
                .filter_map(|e| {
                    let ver = e.pointer("/loader/version")?.as_str()?.to_string();
                    Some(LoaderVersion { stable: !ver.contains('-'), version: ver })
                })
                .collect()
        }
        "forge" => {
            let xml = net::get_text("https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml").await?;
            let promos: Value = net::get_json("https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json").await.unwrap_or(Value::Null);
            let rec = promos.pointer(&format!("/promos/{mc}-recommended")).and_then(|v| v.as_str()).map(|s| s.to_string());
            let re = regex::Regex::new(r"<version>([^<]+)</version>").unwrap();
            let prefix = format!("{mc}-");
            let mut out: Vec<LoaderVersion> = re
                .captures_iter(&xml)
                .filter_map(|c| {
                    let full = c.get(1)?.as_str();
                    let rest = full.strip_prefix(&prefix)?;
                    // Some old builds carry a branch suffix (1.7.10-10.13.4.1614-1.7.10).
                    Some(LoaderVersion { version: rest.to_string(), stable: rec.as_deref() == Some(rest.split('-').next().unwrap_or(rest)) })
                })
                .collect();
            out.sort_by(|a, b| cmp_versions(&b.version, &a.version));
            out
        }
        "neoforge" => {
            let v: Value = net::get_json("https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge").await?;
            let mut out: Vec<LoaderVersion> = v["versions"]
                .as_array()
                .ok_or_else(|| anyhow!("unexpected NeoForge response"))?
                .iter()
                .filter_map(|x| x.as_str())
                .filter(|x| neoforge_mc(x).as_deref() == Some(mc))
                .map(|x| LoaderVersion { version: x.to_string(), stable: !x.contains("beta") && !x.contains("alpha") })
                .collect();
            out.sort_by(|a, b| cmp_versions(&b.version, &a.version));
            out
        }
        _ => vec![],
    })
}

/// Numeric-aware comparison for dotted versions.
pub fn cmp_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let pa: Vec<&str> = a.split(|c: char| !c.is_ascii_alphanumeric()).collect();
    let pb: Vec<&str> = b.split(|c: char| !c.is_ascii_alphanumeric()).collect();
    for i in 0..pa.len().max(pb.len()) {
        let x = pa.get(i).copied().unwrap_or("");
        let y = pb.get(i).copied().unwrap_or("");
        let o = match (x.parse::<u64>(), y.parse::<u64>()) {
            (Ok(m), Ok(n)) => m.cmp(&n),
            _ => x.cmp(y),
        };
        if o != std::cmp::Ordering::Equal {
            return o;
        }
    }
    std::cmp::Ordering::Equal
}

/// Installs the loader profile into the shared versions folder and returns the version id to launch.
pub async fn install(paths: &Paths, loader: &str, mc: &str, loader_version: Option<&str>, java: Option<&str>, task: &Task) -> Result<String> {
    match loader {
        "vanilla" => {
            version::ensure_vanilla_json(paths, mc).await?;
            Ok(mc.to_string())
        }
        "fabric" | "quilt" => {
            let lv = match loader_version {
                Some(v) if !v.is_empty() => v.to_string(),
                _ => versions(loader, mc).await?.into_iter().find(|v| v.stable).ok_or_else(|| anyhow!("No {loader} loader for Minecraft {mc}"))?.version,
            };
            let url = if loader == "fabric" {
                format!("https://meta.fabricmc.net/v2/versions/loader/{mc}/{lv}/profile/json")
            } else {
                format!("https://meta.quiltmc.org/v3/versions/loader/{mc}/{lv}/profile/json")
            };
            task.stage(&format!("Installing {loader} {lv}"), 0, 0);
            let profile: Value = net::get_json(&url).await?;
            let id = profile["id"].as_str().ok_or_else(|| anyhow!("loader profile has no id"))?.to_string();
            crate::util::write_json_atomic(&paths.version_json(&id), &profile)?;
            version::ensure_vanilla_json(paths, mc).await?;
            Ok(id)
        }
        "forge" | "neoforge" => {
            let lv = loader_version.filter(|v| !v.is_empty()).map(String::from);
            let lv = match lv {
                Some(v) => v,
                None => versions(loader, mc).await?.first().ok_or_else(|| anyhow!("No {loader} build for Minecraft {mc}"))?.version.clone(),
            };
            install_forge_like(paths, loader, mc, &lv, java.ok_or_else(|| anyhow!("Java is needed to run the {loader} installer"))?, task).await
        }
        other => bail!("Unknown loader {other}"),
    }
}

/// Maven repos publish `<file>.sha256` / `<file>.sha1` next to artifacts; older Forge builds may lack the sha256.
async fn maven_checksums(url: &str) -> (Option<String>, Option<String>) {
    let sha256 = net::get_text(&format!("{url}.sha256")).await.ok().and_then(|t| net::parse_checksum(&t, 64));
    if sha256.is_some() {
        return (None, sha256);
    }
    let sha1 = net::get_text(&format!("{url}.sha1")).await.ok().and_then(|t| net::parse_checksum(&t, 40));
    (sha1, None)
}

async fn install_forge_like(paths: &Paths, loader: &str, mc: &str, lv: &str, java: &str, task: &Task) -> Result<String> {
    let (url, file) = if loader == "forge" {
        let full = format!("{mc}-{lv}");
        (format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{full}/forge-{full}-installer.jar"), format!("forge-{full}-installer.jar"))
    } else {
        (format!("https://maven.neoforged.net/releases/net/neoforged/neoforge/{lv}/neoforge-{lv}-installer.jar"), format!("neoforge-{lv}-installer.jar"))
    };
    let installer = paths.cache.join("installers").join(&file);
    task.stage(&format!("Downloading the {loader} installer"), 0, 0);
    let (sha1, sha256) = maven_checksums(&url).await;
    net::download(&Download { url, dest: installer.clone(), sha1, sha256, size: None })
        .await
        .with_context(|| format!("downloading the {loader} installer"))?;

    let (profile, version_json) = read_installer(&installer)?;
    // Legacy installers (Forge 1.12.2 and older) embed the full version JSON.
    if let Some(info) = profile.get("versionInfo") {
        let id = info["id"].as_str().ok_or_else(|| anyhow!("installer profile has no id"))?.to_string();
        let path = profile.pointer("/install/path").and_then(|v| v.as_str()).unwrap_or_default();
        let file_path = profile.pointer("/install/filePath").and_then(|v| v.as_str()).unwrap_or_default();
        if let Some(rel) = crate::mc::library::maven_path(path) {
            let dest = paths.libraries().join(rel);
            if !dest.exists() {
                let mut z = zip::ZipArchive::new(std::fs::File::open(&installer)?)?;
                let mut e = z.by_name(file_path).context("installer is missing the universal jar")?;
                std::fs::create_dir_all(dest.parent().unwrap())?;
                let mut out = std::fs::File::create(&dest)?;
                std::io::copy(&mut e, &mut out)?;
            }
        }
        let mut info = info.clone();
        if info.get("inheritsFrom").is_none() {
            info["inheritsFrom"] = Value::String(mc.to_string());
            info["jar"] = Value::String(mc.to_string());
        }
        // The forge universal jar itself has no download URL; it was extracted above.
        if let Some(libs) = info.get_mut("libraries").and_then(|l| l.as_array_mut()) {
            for l in libs.iter_mut() {
                if l["name"].as_str() == Some(path) {
                    l["downloads"] = serde_json::json!({"artifact": {"path": crate::mc::library::maven_path(path), "url": ""}});
                }
            }
        }
        crate::util::write_json_atomic(&paths.version_json(&id), &info)?;
        version::ensure_vanilla_json(paths, mc).await?;
        return Ok(id);
    }

    let vj = version_json.ok_or_else(|| anyhow!("installer has no version.json"))?;
    let id = vj["id"].as_str().ok_or_else(|| anyhow!("version.json has no id"))?.to_string();
    let marker = paths.versions().join(&id).join(".cairn-installed");
    if marker.exists() && paths.version_json(&id).exists() {
        return Ok(id);
    }

    // The installer validates the vanilla client jar and writes into a .minecraft-shaped folder.
    let vanilla = version::ensure_vanilla_json(paths, mc).await?;
    if let Some(c) = vanilla.pointer("/downloads/client") {
        task.stage("Downloading the Minecraft client", 0, 0);
        net::download(&Download {
            url: c["url"].as_str().unwrap_or_default().into(),
            dest: paths.version_jar(mc),
            sha1: c["sha1"].as_str().map(String::from),
            sha256: None,
            size: None,
        })
        .await?;
    }
    let lp = paths.meta.join("launcher_profiles.json");
    if !lp.exists() {
        std::fs::write(&lp, r#"{"profiles":{},"selectedProfile":null}"#)?;
    }
    let pretty = match loader { "neoforge" => "NeoForge", "forge" => "Forge", other => other };
    task.stage(&format!("Running the {pretty} installer (this can take a few minutes)"), 0, 0);
    let out = tokio::process::Command::new(java)
        .arg("-jar")
        .arg(&installer)
        .arg("--installClient")
        .arg(&paths.meta)
        .current_dir(&paths.cache)
        .output()
        .await
        .context("starting the installer")?;
    let log = String::from_utf8_lossy(&out.stdout).to_string() + &String::from_utf8_lossy(&out.stderr);
    let _ = std::fs::write(paths.cache.join(format!("{file}.log")), &log);
    if !out.status.success() || !paths.version_json(&id).exists() {
        let tail: Vec<&str> = log.lines().rev().take(6).collect();
        bail!(
            "The {pretty} installer failed. Older {pretty} builds cannot install from the command line. Last output: {}",
            tail.into_iter().rev().collect::<Vec<_>>().join(" | ")
        );
    }
    std::fs::write(&marker, "ok")?;
    Ok(id)
}

fn read_installer(path: &std::path::Path) -> Result<(Value, Option<Value>)> {
    let mut z = zip::ZipArchive::new(std::fs::File::open(path)?)?;
    let mut s = String::new();
    z.by_name("install_profile.json").context("not a Forge installer")?.read_to_string(&mut s)?;
    let profile: Value = serde_json::from_str(&s)?;
    let vj = match z.by_name("version.json") {
        Ok(mut f) => {
            let mut s = String::new();
            f.read_to_string(&mut s)?;
            Some(serde_json::from_str(&s)?)
        }
        Err(_) => None,
    };
    Ok((profile, vj))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn neoforge_mapping() {
        assert_eq!(neoforge_mc("21.1.77").as_deref(), Some("1.21.1"));
        assert_eq!(neoforge_mc("20.4.237").as_deref(), Some("1.20.4"));
        assert_eq!(neoforge_mc("21.0.10-beta").as_deref(), Some("1.21"));
        assert_eq!(neoforge_mc("26.1.0.5-beta").as_deref(), Some("26.1"));
    }
    #[test]
    fn version_order() {
        use std::cmp::Ordering::*;
        assert_eq!(cmp_versions("47.3.10", "47.3.9"), Greater);
        assert_eq!(cmp_versions("1.20.1", "1.20.1"), Equal);
        assert_eq!(cmp_versions("0.16.9", "0.16.10"), Less);
    }
}
