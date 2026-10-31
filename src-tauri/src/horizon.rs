//! Horizon (Rust Minecraft client) install + launch helpers.
//! https://github.com/horizon-rust/horizon

use crate::instance::HORIZON_MC_VERSION;
use crate::net::{self, Download, CLIENT};
use crate::paths::Paths;
use crate::tasks::Task;
use anyhow::{anyhow, bail, Context, Result};
use flate2::read::GzDecoder;
use serde_json::Value;
use std::fs::File;
use std::path::{Path, PathBuf};

const RELEASES_API: &str = "https://api.github.com/repos/horizon-rust/horizon/releases?per_page=30";

fn platform_asset_name() -> Result<&'static str> {
    if cfg!(target_os = "linux") && cfg!(target_arch = "x86_64") {
        Ok("horizon-linux-x86_64.tar.gz")
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        Ok("horizon-macos-arm64.zip")
    } else if cfg!(target_os = "windows") && cfg!(target_arch = "x86_64") {
        Ok("horizon-windows-x64.zip")
    } else {
        bail!("Horizon has no build for this OS/CPU yet.");
    }
}

fn client_root(paths: &Paths) -> PathBuf {
    paths.root.join("horizon").join("client")
}

fn bin_names() -> &'static [&'static str] {
    if cfg!(windows) {
        &["horizon-client.exe", "horizon_client.exe", "horizon.exe"]
    } else {
        &["horizon-client", "horizon_client", "horizon"]
    }
}

fn find_bin(dir: &Path, names: &[&str]) -> Option<PathBuf> {
    for name in names {
        let p = dir.join(name);
        if p.is_file() {
            return Some(p);
        }
    }
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                if let Some(found) = find_bin(&p, names) {
                    return Some(found);
                }
            } else if let Some(n) = p.file_name().and_then(|s| s.to_str()) {
                if names.iter().any(|want| n.eq_ignore_ascii_case(want)) {
                    return Some(p);
                }
            }
        }
    }
    None
}

fn is_prepared_resources(dir: &Path) -> bool {
    if !dir.is_dir() {
        return false;
    }
    if dir.join("horizon-resources.json").is_file() {
        return true;
    }
    // Older/partial trees: assets + at least one other entry.
    dir.join("assets").is_dir()
        && std::fs::read_dir(dir)
            .ok()
            .map(|rd| rd.filter_map(|e| e.ok()).count() >= 2)
            .unwrap_or(false)
}

fn push_candidate(out: &mut Vec<PathBuf>, p: PathBuf) {
    if !out.iter().any(|x| x == &p) {
        out.push(p);
    }
}

/// Find Horizon prepared resources for Minecraft 26.3.
pub fn find_resources(paths: &Paths) -> Option<PathBuf> {
    let mut cands: Vec<PathBuf> = Vec::new();
    push_candidate(&mut cands, paths.root.join("horizon").join("resources").join(HORIZON_MC_VERSION));

    // Resources prepared next to a Cairn-installed client release.
    let root = client_root(paths);
    if root.is_dir() {
        if let Ok(rd) = std::fs::read_dir(&root) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    push_candidate(&mut cands, p.join(".local").join("horizon-client").join(HORIZON_MC_VERSION));
                    // Nested archive folder (horizon-linux-x86_64/.local/...)
                    if let Ok(inner) = std::fs::read_dir(&p) {
                        for ie in inner.flatten() {
                            let ip = ie.path();
                            if ip.is_dir() {
                                push_candidate(
                                    &mut cands,
                                    ip.join(".local").join("horizon-client").join(HORIZON_MC_VERSION),
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    if let Some(data) = dirs::data_dir() {
        // Official Horizon launcher installs: versions/<tag>/.local/horizon-client/26.3
        let versions = data.join("horizon-launcher").join("versions");
        if versions.is_dir() {
            if let Ok(rd) = std::fs::read_dir(&versions) {
                for e in rd.flatten() {
                    let p = e.path();
                    if p.is_dir() {
                        push_candidate(
                            &mut cands,
                            p.join(".local").join("horizon-client").join(HORIZON_MC_VERSION),
                        );
                    }
                }
            }
        }
        push_candidate(
            &mut cands,
            data.join("horizon").join("resources").join(HORIZON_MC_VERSION),
        );
    }

    if let Some(home) = dirs::home_dir() {
        push_candidate(
            &mut cands,
            home.join(".local").join("horizon-client").join(HORIZON_MC_VERSION),
        );
    }

    cands.into_iter().find(|c| is_prepared_resources(c))
}

async fn latest_stable_asset() -> Result<(String, String, u64, Option<String>)> {
    let want = platform_asset_name()?;
    let releases: Value = CLIENT
        .get(RELEASES_API)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await
        .context("listing Horizon releases")?
        .error_for_status()
        .context("GitHub releases API")?
        .json()
        .await?;
    let arr = releases.as_array().ok_or_else(|| anyhow!("bad GitHub releases payload"))?;
    for rel in arr {
        let tag = rel["tag_name"].as_str().unwrap_or("");
        if tag.is_empty() || tag.starts_with("launcher") || tag.starts_with("nightly") {
            continue;
        }
        if !(tag.starts_with('v') || tag.chars().next().is_some_and(|c| c.is_ascii_digit())) {
            continue;
        }
        let assets = rel["assets"].as_array().cloned().unwrap_or_default();
        if let Some(a) = assets.iter().find(|a| a["name"].as_str() == Some(want)) {
            let url = a["browser_download_url"]
                .as_str()
                .ok_or_else(|| anyhow!("Horizon asset missing download URL"))?
                .to_string();
            let size = a["size"].as_u64().unwrap_or(0);
            let sha256 = a["digest"].as_str().filter(|d| d.starts_with("sha256:")).and_then(|d| net::parse_checksum(d, 64));
            return Ok((tag.to_string(), url, size, sha256));
        }
    }
    bail!("No Horizon {want} asset found on recent GitHub releases.");
}

fn extract_archive(archive: &Path, dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    let name = archive.file_name().and_then(|s| s.to_str()).unwrap_or("");
    if name.ends_with(".zip") {
        let mut z = zip::ZipArchive::new(File::open(archive)?)?;
        z.extract(dest)?;
    } else {
        let gz = GzDecoder::new(File::open(archive)?);
        let mut t = tar::Archive::new(gz);
        t.unpack(dest)?;
    }
    Ok(())
}

/// Download (if needed) and return path to `horizon-client` binary.
pub async fn ensure_client(paths: &Paths, task: &Task) -> Result<PathBuf> {
    let names = bin_names();
    let root = client_root(paths);
    if root.is_dir() {
        if let Some(bin) = find_bin(&root, names) {
            return Ok(bin);
        }
    }

    task.stage("Finding Horizon release", 0, 0);
    let (tag, url, size, sha256) = latest_stable_asset().await?;
    let dest_dir = root.join(&tag);
    if let Some(bin) = find_bin(&dest_dir, names) {
        return Ok(bin);
    }

    let archive_name = url.rsplit('/').next().unwrap_or("horizon.bin");
    let archive = paths.cache.join("horizon").join(archive_name);
    task.stage("Downloading Horizon", 0, 1);
    net::download(&Download {
        url,
        dest: archive.clone(),
        sha1: None,
        sha256,
        size: if size > 0 { Some(size) } else { None },
    })
    .await
    .context("downloading Horizon")?;
    task.stage("Installing Horizon", 1, 1);
    if dest_dir.exists() {
        let _ = std::fs::remove_dir_all(&dest_dir);
    }
    std::fs::create_dir_all(&dest_dir)?;
    extract_archive(&archive, &dest_dir).context("extracting Horizon")?;

    let bin = find_bin(&dest_dir, names).ok_or_else(|| anyhow!("horizon-client missing after extract"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&bin)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&bin, perms)?;
        // Make sibling tools runnable too (prepare is out of scope for auto-run, but handy).
        if let Some(parent) = bin.parent() {
            for sibling in ["horizon-prepare", "horizon-ctl", "horizon-server"] {
                let p = parent.join(sibling);
                if p.is_file() {
                    let mut perms = std::fs::metadata(&p)?.permissions();
                    perms.set_mode(0o755);
                    let _ = std::fs::set_permissions(&p, perms);
                }
            }
        }
    }
    Ok(bin)
}

pub fn validate_instance(inst: &crate::instance::Instance) -> Result<()> {
    if !inst.use_horizon {
        return Ok(());
    }
    if !inst.horizon_allowed() {
        bail!("Horizon only works with vanilla Minecraft {HORIZON_MC_VERSION}.");
    }
    Ok(())
}

/// Persist offline display name so Horizon does not invent `Player#####`.
pub fn write_offline_player_name(game_home: &Path, name: &str) -> Result<()> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 16 {
        bail!("Player name must be 1–16 characters");
    }
    std::fs::write(game_home.join("horizon-player.txt"), format!("name:{name}\n"))?;
    Ok(())
}

/// One-shot Microsoft session file for Horizon (`--session-file`). Mode 0600; Horizon deletes it after read.
pub fn write_ms_session_file(game_home: &Path, acc: &crate::accounts::Account) -> Result<PathBuf> {
    if acc.kind != "microsoft" {
        bail!("not a Microsoft account");
    }
    if acc.mc_token.is_empty() {
        bail!("Sign in to {} again from Accounts.", acc.username);
    }
    let dir = game_home.join(".cairn");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("horizon-session.json");
    let body = serde_json::json!({
        "name": acc.username,
        "uuid": acc.uuid,
        "access_token": acc.mc_token,
        "user_type": "msa",
    });
    std::fs::write(&path, serde_json::to_vec(&body)?)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&path)?.permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(&path, perms)?;
    }
    Ok(path)
}

/// Binary, args, and game home (`HORIZON_HOME`) for spawning the client.
pub async fn prepare_launch(
    paths: &Paths,
    inst: &crate::instance::Instance,
    task: &Task,
    world: Option<&str>,
    server: Option<&str>,
    width: u32,
    height: u32,
) -> Result<(PathBuf, Vec<String>, PathBuf)> {
    validate_instance(inst)?;
    let bin = ensure_client(paths, task).await?;
    let resources = find_resources(paths).ok_or_else(|| {
        anyhow!(
            "Horizon needs prepared Minecraft {HORIZON_MC_VERSION} resources. \
Play {HORIZON_MC_VERSION} once in the official launcher, then run Prepare in the Horizon launcher \
(or run horizon-prepare next to the client). Or place prepared resources at {}.",
            paths.root.join("horizon").join("resources").join(HORIZON_MC_VERSION).display()
        )
    })?;

    let game_home = inst.dir(paths);
    std::fs::create_dir_all(game_home.join("saves"))?;

    let mut args = vec![
        "--resources".into(),
        resources.display().to_string(),
        "--width".into(),
        width.to_string(),
        "--height".into(),
        height.to_string(),
    ];
    if let Some(w) = world {
        args.push("--world".into());
        args.push(w.to_string());
    }
    if let Some(s) = server {
        args.push("--server".into());
        args.push(s.to_string());
    }
    Ok((bin, args, game_home))
}
