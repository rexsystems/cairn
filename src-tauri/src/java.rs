use crate::net::{self, Download};
use crate::paths::Paths;
use crate::tasks::Task;
use anyhow::{anyhow, bail, Context, Result};
use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct JavaInstall {
    pub path: String,
    pub version: String,
    pub major: u32,
    pub managed: bool,
}

fn java_bin_name() -> &'static str {
    if cfg!(windows) { "java.exe" } else { "java" }
}

/// "1.8.0_412" -> 8, "21.0.4" -> 21, "25" -> 25
pub fn parse_major(version: &str) -> Option<u32> {
    let mut it = version.split(|c: char| c == '.' || c == '_' || c == '-' || c == '+');
    let first: u32 = it.next()?.parse().ok()?;
    if first == 1 {
        it.next()?.parse().ok()
    } else {
        Some(first)
    }
}

pub fn probe(java: &Path) -> Option<(String, u32)> {
    // `release` next to bin/ avoids spawning a JVM.
    if let Some(home) = java.parent().and_then(|b| b.parent()) {
        if let Ok(rel) = std::fs::read_to_string(home.join("release")) {
            for line in rel.lines() {
                if let Some(v) = line.strip_prefix("JAVA_VERSION=") {
                    let v = v.trim_matches('"').to_string();
                    if let Some(m) = parse_major(&v) {
                        return Some((v, m));
                    }
                }
            }
        }
    }
    let out = Command::new(java).arg("-version").output().ok()?;
    let text = String::from_utf8_lossy(&out.stderr).to_string() + &String::from_utf8_lossy(&out.stdout);
    let re = regex::Regex::new(r#"version "([^"]+)""#).ok()?;
    let v = re.captures(&text)?.get(1)?.as_str().to_string();
    let m = parse_major(&v)?;
    Some((v, m))
}

fn candidates(paths: &Paths) -> Vec<(PathBuf, bool)> {
    let mut c: Vec<(PathBuf, bool)> = vec![];
    let bin = java_bin_name();
    if let Ok(rd) = std::fs::read_dir(&paths.java) {
        for e in rd.flatten() {
            let direct = e.path().join("bin").join(bin);
            if direct.exists() {
                c.push((direct, true));
            }
            if let Ok(inner) = std::fs::read_dir(e.path()) {
                for i in inner.flatten() {
                    for p in [i.path().join("bin").join(bin), i.path().join("Contents/Home/bin").join(bin)] {
                        if p.exists() {
                            c.push((p, true));
                        }
                    }
                }
            }
        }
    }
    if let Ok(h) = std::env::var("JAVA_HOME") {
        c.push((PathBuf::from(h).join("bin").join(bin), false));
    }
    if let Ok(path) = std::env::var("PATH") {
        for d in std::env::split_paths(&path) {
            let p = d.join(bin);
            if p.exists() {
                c.push((std::fs::canonicalize(&p).unwrap_or(p), false));
            }
        }
    }
    for root in ["/usr/lib/jvm", "/usr/java", "/opt", "/Library/Java/JavaVirtualMachines", "C:\\Program Files\\Java", "C:\\Program Files\\Eclipse Adoptium"] {
        if let Ok(rd) = std::fs::read_dir(root) {
            for e in rd.flatten() {
                for p in [e.path().join("bin").join(bin), e.path().join("Contents/Home/bin").join(bin)] {
                    if p.exists() {
                        c.push((p, false));
                    }
                }
            }
        }
    }
    c
}

pub fn detect(paths: &Paths) -> Vec<JavaInstall> {
    let mut out: Vec<JavaInstall> = vec![];
    for (p, managed) in candidates(paths) {
        let key = std::fs::canonicalize(&p).unwrap_or(p.clone());
        if out.iter().any(|j| std::fs::canonicalize(&j.path).ok().as_ref() == Some(&key)) {
            continue;
        }
        if let Some((version, major)) = probe(&p) {
            out.push(JavaInstall { path: p.to_string_lossy().to_string(), version, major, managed });
        }
    }
    out.sort_by(|a, b| b.major.cmp(&a.major));
    out
}

fn adoptium_os() -> &'static str {
    if cfg!(windows) { "windows" } else if cfg!(target_os = "macos") { "mac" } else { "linux" }
}
fn adoptium_arch() -> &'static str {
    if cfg!(target_arch = "aarch64") { "aarch64" } else { "x64" }
}

/// Downloads an Eclipse Temurin runtime from the Adoptium API into the managed java folder.
pub async fn install(paths: &Paths, major: u32, task: &Task) -> Result<JavaInstall> {
    let mut chosen: Option<Value> = None;
    for image in ["jre", "jdk"] {
        let url = format!(
            "https://api.adoptium.net/v3/assets/latest/{major}/hotspot?architecture={}&image_type={image}&os={}&vendor=eclipse",
            adoptium_arch(),
            adoptium_os()
        );
        task.stage(&format!("Looking up Java {major}"), 0, 0);
        if let Ok(list) = net::get_json::<Vec<Value>>(&url).await {
            if let Some(first) = list.into_iter().next() {
                chosen = Some(first);
                break;
            }
        }
    }
    let asset = chosen.ok_or_else(|| anyhow!("Adoptium has no Java {major} build for this system"))?;
    let pkg = asset.pointer("/binary/package").ok_or_else(|| anyhow!("unexpected Adoptium response"))?;
    let link = pkg["link"].as_str().unwrap_or_default().to_string();
    let name = pkg["name"].as_str().unwrap_or("java.tar.gz").to_string();
    let sha256 = pkg["checksum"].as_str().unwrap_or_default().to_lowercase();
    let archive = paths.cache.join(&name);
    task.stage(&format!("Downloading Java {major}"), 0, 1);
    net::download(&Download { url: link, dest: archive.clone(), sha1: None, size: pkg.get("size").and_then(|s| s.as_u64()) }).await?;
    if !sha256.is_empty() {
        use sha2::Digest;
        let bytes = std::fs::read(&archive)?;
        let got = hex::encode(sha2::Sha256::digest(&bytes));
        if got != sha256 {
            let _ = std::fs::remove_file(&archive);
            bail!("Java download failed its checksum check");
        }
    }
    task.stage(&format!("Unpacking Java {major}"), 0, 0);
    let dest = paths.java.join(format!("temurin-{major}"));
    if dest.exists() {
        std::fs::remove_dir_all(&dest)?;
    }
    std::fs::create_dir_all(&dest)?;
    let a2 = archive.clone();
    let d2 = dest.clone();
    tokio::task::spawn_blocking(move || extract_archive(&a2, &d2)).await??;
    let _ = std::fs::remove_file(&archive);
    let found = detect(paths)
        .into_iter()
        .find(|j| j.managed && j.major == major && Path::new(&j.path).starts_with(&dest))
        .ok_or_else(|| anyhow!("Java {major} unpacked but no java binary was found"))?;
    Ok(found)
}

fn extract_archive(archive: &Path, dest: &Path) -> Result<()> {
    let name = archive.to_string_lossy().to_lowercase();
    if name.ends_with(".zip") {
        let mut z = zip::ZipArchive::new(std::fs::File::open(archive)?)?;
        z.extract(dest)?;
    } else {
        let gz = flate2::read::GzDecoder::new(std::fs::File::open(archive)?);
        let mut t = tar::Archive::new(gz);
        t.set_preserve_permissions(true);
        t.unpack(dest).context("unpacking Java archive")?;
    }
    Ok(())
}

/// Picks the Java for a game version: user override, then an exact major match, then a download.
pub async fn ensure(paths: &Paths, settings: &crate::settings::Settings, major: u32, task: &Task) -> Result<String> {
    if let Some(p) = settings.java_overrides.get(&major.to_string()) {
        if Path::new(p).exists() {
            return Ok(p.clone());
        }
    }
    let all = detect(paths);
    if let Some(j) = all.iter().find(|j| j.major == major && j.managed).or_else(|| all.iter().find(|j| j.major == major)) {
        return Ok(j.path.clone());
    }
    if !settings.auto_install_java {
        bail!("Java {major} is required and automatic Java downloads are off. Install Java {major} or set its path in Settings.");
    }
    Ok(install(paths, major, task).await?.path)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn majors() {
        assert_eq!(parse_major("1.8.0_412"), Some(8));
        assert_eq!(parse_major("17.0.12"), Some(17));
        assert_eq!(parse_major("21"), Some(21));
        assert_eq!(parse_major("25-ea"), Some(25));
        assert_eq!(parse_major("x"), None);
    }
}
