//! Read/write Minecraft `servers.dat` (gzip NBT).
use anyhow::{bail, Context, Result};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ServerEntry {
    pub name: String,
    pub ip: String,
    pub icon: Option<String>,
    pub accept_textures: Option<i8>,
}

#[derive(Serialize, Deserialize, Default)]
struct Root {
    #[serde(default)]
    servers: Vec<NbtServer>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
struct NbtServer {
    #[serde(default)]
    name: String,
    #[serde(default)]
    ip: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    icon: Option<String>,
    #[serde(default, rename = "acceptTextures", skip_serializing_if = "Option::is_none")]
    accept_textures: Option<i8>,
}

fn path(dir: &Path) -> std::path::PathBuf {
    dir.join("servers.dat")
}

pub fn list(dir: &Path) -> Result<Vec<ServerEntry>> {
    let p = path(dir);
    if !p.is_file() {
        return Ok(vec![]);
    }
    let bytes = std::fs::read(&p).context("reading servers.dat")?;
    let root = decode(&bytes)?;
    Ok(root
        .servers
        .into_iter()
        .map(|s| ServerEntry {
            name: if s.name.is_empty() { s.ip.clone() } else { s.name },
            ip: s.ip,
            icon: s.icon,
            accept_textures: s.accept_textures,
        })
        .collect())
}

pub fn save(dir: &Path, servers: &[ServerEntry]) -> Result<()> {
    for s in servers {
        if s.ip.trim().is_empty() {
            bail!("Server address cannot be empty");
        }
        if s.ip.contains('\n') || s.name.contains('\n') {
            bail!("Invalid server entry");
        }
    }
    let root = Root {
        servers: servers
            .iter()
            .map(|s| NbtServer {
                name: s.name.trim().to_string(),
                ip: s.ip.trim().to_string(),
                icon: s.icon.clone(),
                accept_textures: s.accept_textures,
            })
            .collect(),
    };
    let raw = fastnbt::to_bytes(&root).context("encoding servers.dat")?;
    let mut enc = GzEncoder::new(Vec::new(), Compression::default());
    enc.write_all(&raw)?;
    let gz = enc.finish()?;
    std::fs::create_dir_all(dir)?;
    crate::util::write_bytes_atomic(&path(dir), &gz)?;
    Ok(())
}

fn decode(bytes: &[u8]) -> Result<Root> {
    // Prefer gzip; fall back to raw NBT for odd copies.
    let mut raw = Vec::new();
    match GzDecoder::new(bytes).read_to_end(&mut raw) {
        Ok(_) if !raw.is_empty() => {}
        _ => raw = bytes.to_vec(),
    }
    fastnbt::from_bytes(&raw).context("parsing servers.dat")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip() {
        let dir = std::env::temp_dir().join(format!("cairn-servers-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let list = vec![ServerEntry {
            name: "Hypixel".into(),
            ip: "mc.hypixel.net".into(),
            icon: None,
            accept_textures: Some(1),
        }];
        save(&dir, &list).unwrap();
        let back = super::list(&dir).unwrap();
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].ip, "mc.hypixel.net");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
