//! Share codes: a compact text form of an instance (version, loader and the exact content
//! versions) that another Cairn can rebuild by downloading the same files.

use crate::content::{ContentIndex, KINDS};
use crate::instance::{Instance, LOADERS};
use anyhow::{anyhow, bail, Context, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::Path;

const PREFIX: &str = "CAIRN1:";
/// Inflated manifests above this are refused, so a pasted code cannot exhaust memory.
const MAX_JSON: u64 = 256 * 1024;
const MAX_ITEMS: usize = 2000;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Item {
    /// "m" for Modrinth, "c" for CurseForge.
    pub s: String,
    pub p: String,
    pub v: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Manifest {
    pub name: String,
    pub mc: String,
    pub loader: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lv: Option<String>,
    #[serde(default)]
    pub items: Vec<Item>,
}

/// What a share code holds, plus the files that could not go in it.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Shared {
    pub code: String,
    pub items: usize,
    /// Enabled files with no Modrinth or CurseForge source; the receiver will not get them.
    pub left_out: Vec<String>,
}

pub fn manifest(dir: &Path, inst: &Instance) -> (Manifest, Vec<String>) {
    let index = ContentIndex::load(dir);
    let mut items = vec![];
    let mut left_out = vec![];
    for kind in KINDS {
        let Ok(rd) = std::fs::read_dir(dir.join(kind)) else { continue };
        let mut names: Vec<String> = rd.flatten().filter(|e| e.path().is_file()).map(|e| e.file_name().to_string_lossy().into_owned()).collect();
        names.sort();
        for name in names {
            if name.ends_with(".disabled") {
                continue;
            }
            let meta = index.files.get(&format!("{kind}/{name}"));
            let ids = meta.and_then(|m| Some((m.project_id.clone()?, m.version_id.clone()?, m.source.as_deref() == Some("curseforge"))));
            match ids {
                Some((p, v, cf)) => items.push(Item { s: if cf { "c" } else { "m" }.into(), p, v }),
                None => left_out.push(format!("{kind}/{name}")),
            }
        }
    }
    let m = Manifest { name: inst.name.clone(), mc: inst.mc_version.clone(), loader: inst.loader.clone(), lv: inst.loader_version.clone(), items };
    (m, left_out)
}

pub fn encode(m: &Manifest) -> Result<String> {
    let json = serde_json::to_vec(m)?;
    let mut z = flate2::write::DeflateEncoder::new(vec![], flate2::Compression::best());
    z.write_all(&json)?;
    Ok(format!("{PREFIX}{}", URL_SAFE_NO_PAD.encode(z.finish()?)))
}

fn plain(s: &str, max: usize) -> bool {
    !s.is_empty() && s.len() <= max && s.chars().all(|c| c.is_ascii_alphanumeric() || "._-+".contains(c))
}

/// Parses and checks a pasted code. Whitespace and line breaks from chat apps are ignored.
pub fn decode(code: &str) -> Result<Manifest> {
    let code: String = code.chars().filter(|c| !c.is_whitespace()).collect();
    let body = code.strip_prefix(PREFIX).ok_or_else(|| anyhow!("This is not a Cairn share code. They start with {PREFIX}"))?;
    let raw = URL_SAFE_NO_PAD.decode(body.trim_end_matches('=')).context("The share code is damaged or incomplete")?;
    let mut json = vec![];
    flate2::read::DeflateDecoder::new(raw.as_slice()).take(MAX_JSON + 1).read_to_end(&mut json).context("The share code is damaged or incomplete")?;
    if json.len() as u64 > MAX_JSON {
        bail!("The share code is too large");
    }
    let mut m: Manifest = serde_json::from_slice(&json).context("The share code is damaged or incomplete")?;
    m.name = m.name.trim().chars().filter(|c| !c.is_control()).take(64).collect();
    if m.name.is_empty() {
        m.name = "Shared instance".into();
    }
    if !plain(&m.mc, 32) {
        bail!("The share code has an invalid Minecraft version");
    }
    if !LOADERS.contains(&m.loader.as_str()) {
        bail!("The share code uses an unknown loader: {}", m.loader);
    }
    if m.lv.as_deref().is_some_and(|v| !plain(v, 64)) {
        bail!("The share code has an invalid loader version");
    }
    if m.items.len() > MAX_ITEMS {
        bail!("The share code lists too many files");
    }
    if m.items.iter().any(|i| !(i.s == "m" || i.s == "c") || !plain(&i.p, 64) || !plain(&i.v, 64)) {
        bail!("The share code lists an invalid file");
    }
    Ok(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Manifest {
        Manifest {
            name: "Friends".into(),
            mc: "1.21.1".into(),
            loader: "fabric".into(),
            lv: Some("0.16.9".into()),
            items: vec![Item { s: "m".into(), p: "AANobbMI".into(), v: "abc123".into() }, Item { s: "c".into(), p: "238222".into(), v: "5101366".into() }],
        }
    }

    #[test]
    fn round_trips_and_ignores_whitespace() {
        let code = encode(&sample()).unwrap();
        assert!(code.starts_with(PREFIX));
        let wrapped = format!("  {}\n{}  ", &code[..20], &code[20..]);
        assert_eq!(decode(&wrapped).unwrap(), sample());
    }

    #[test]
    fn rejects_foreign_or_damaged_codes() {
        assert!(decode("hello").unwrap_err().to_string().contains("not a Cairn share code"));
        assert!(decode("CAIRN1:!!!!").is_err());
        let code = encode(&sample()).unwrap();
        assert!(decode(&code[..code.len() - 6]).is_err());
    }

    #[test]
    fn rejects_unsafe_values() {
        let mut m = sample();
        m.items[0].v = "../../evil".into();
        assert!(decode(&encode(&m).unwrap()).is_err());
        let mut m = sample();
        m.loader = "rift".into();
        assert!(decode(&encode(&m).unwrap()).is_err());
        let mut m = sample();
        m.name = "\u{7}  ".into();
        assert_eq!(decode(&encode(&m).unwrap()).unwrap().name, "Shared instance");
    }

    #[test]
    fn refuses_inflation_bombs() {
        let json = format!("{{\"name\":\"{}\",\"mc\":\"1\",\"loader\":\"vanilla\"}}", "a".repeat(MAX_JSON as usize * 2));
        let mut z = flate2::write::DeflateEncoder::new(vec![], flate2::Compression::best());
        z.write_all(json.as_bytes()).unwrap();
        let code = format!("{PREFIX}{}", URL_SAFE_NO_PAD.encode(z.finish().unwrap()));
        assert!(decode(&code).unwrap_err().to_string().contains("too large"));
    }

    #[test]
    fn manifest_lists_sourced_enabled_files() {
        let dir = std::env::temp_dir().join(format!("cairn-share-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("mods")).unwrap();
        for f in ["sodium.jar", "local.jar", "off.jar.disabled"] {
            std::fs::write(dir.join("mods").join(f), b"x").unwrap();
        }
        let mut index = ContentIndex::default();
        index.files.insert(
            "mods/sodium.jar".into(),
            crate::content::ContentMeta { project_id: Some("AANobbMI".into()), version_id: Some("v1".into()), ..Default::default() },
        );
        index.save(&dir).unwrap();
        let inst = Instance { name: "Pack".into(), mc_version: "1.21.1".into(), loader: "fabric".into(), ..Default::default() };
        let (m, left) = manifest(&dir, &inst);
        assert_eq!(m.items, vec![Item { s: "m".into(), p: "AANobbMI".into(), v: "v1".into() }]);
        assert_eq!(left, vec!["mods/local.jar".to_string()]);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
