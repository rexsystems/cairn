//! Reads what a mod jar declares about itself: ids, version, dependencies, breaks,
//! environment and which loaders it targets. Results are cached per file so the
//! pre-launch check stays fast on large packs.

use crate::content::base_name;
use crate::util;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DepKind {
    Required,
    Optional,
    /// Fabric `breaks`, NeoForge `incompatible`: the game will refuse to start.
    Breaks,
    /// Fabric `conflicts`, NeoForge `discouraged`: loads, but known to misbehave.
    Conflicts,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "syntax", content = "value", rename_all = "lowercase")]
pub enum Range {
    Any,
    /// Any of these Fabric predicates.
    Fabric(Vec<String>),
    Maven(String),
}

impl Range {
    pub fn matches(&self, v: &str) -> Option<bool> {
        match self {
            Range::Any => Some(true),
            Range::Fabric(p) => crate::versioning::fabric_any(p, v),
            Range::Maven(r) => crate::versioning::maven_matches(r, v),
        }
    }
    pub fn describe(&self) -> String {
        match self {
            Range::Any => "any version".into(),
            Range::Fabric(p) => p.join(" or "),
            Range::Maven(r) => r.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DepReq {
    pub id: String,
    pub range: Range,
    pub kind: DepKind,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    #[default]
    Both,
    Client,
    Server,
}

/// Which metadata formats a jar ships. A jar can carry several (multi-loader builds).
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(default)]
pub struct Formats {
    pub fabric: bool,
    pub quilt: bool,
    /// `META-INF/mods.toml`
    pub mods_toml: bool,
    /// `META-INF/neoforge.mods.toml`
    pub neoforge_toml: bool,
    /// Pre-1.13 Forge `mcmod.info`.
    pub legacy_forge: bool,
}

impl Formats {
    pub fn any(&self) -> bool {
        self.fabric || self.quilt || self.mods_toml || self.neoforge_toml || self.legacy_forge
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct ModInfo {
    /// Mod ids this jar defines at the top level (Forge jars may hold several).
    pub ids: Vec<String>,
    /// Ids satisfied by this jar: own ids, `provides`, and jar-in-jar mods.
    pub provides: Vec<String>,
    pub name: String,
    pub version: String,
    pub depends: Vec<DepReq>,
    pub side: Side,
    pub formats: Formats,
    /// mods.toml jars that depend on `neoforge` rather than `forge`.
    pub targets_neoforge: bool,
    /// mods.toml jars that depend on `forge`.
    pub targets_forge: bool,
}

const NESTED_LIMIT: u64 = 32 * 1024 * 1024;

fn read_entry<R: Read + std::io::Seek>(zip: &mut zip::ZipArchive<R>, name: &str) -> Option<Vec<u8>> {
    let mut e = zip.by_name(name).ok()?;
    if e.size() > NESTED_LIMIT {
        return None;
    }
    let mut buf = Vec::with_capacity(e.size() as usize);
    e.read_to_end(&mut buf).ok()?;
    Some(buf)
}

/// Fabric tolerates raw control characters inside strings; serde does not.
fn lenient_json(bytes: &[u8]) -> Option<Value> {
    let s = String::from_utf8_lossy(bytes);
    let s = s.trim_start_matches('\u{feff}');
    serde_json::from_str(s).ok().or_else(|| {
        let cleaned: String = s.chars().map(|c| if c.is_control() && c != '\n' { ' ' } else { c }).collect();
        let cleaned = cleaned.replace('\n', " ");
        serde_json::from_str(&cleaned).ok()
    })
}

fn fabric_range(v: &Value) -> Range {
    match v {
        Value::String(s) if s.trim() == "*" || s.trim().is_empty() => Range::Any,
        Value::String(s) => Range::Fabric(vec![s.clone()]),
        Value::Array(a) => {
            let preds: Vec<String> = a.iter().filter_map(|x| x.as_str().map(String::from)).collect();
            if preds.is_empty() || preds.iter().any(|p| p.trim() == "*") {
                Range::Any
            } else {
                Range::Fabric(preds)
            }
        }
        _ => Range::Any,
    }
}

fn parse_fabric(v: &Value, info: &mut ModInfo) {
    let Some(id) = v["id"].as_str() else { return };
    info.formats.fabric = true;
    info.ids.push(id.to_string());
    info.provides.push(id.to_string());
    if info.name.is_empty() {
        info.name = v["name"].as_str().unwrap_or(id).to_string();
    }
    if info.version.is_empty() {
        info.version = v["version"].as_str().unwrap_or_default().to_string();
    }
    if let Some(p) = v["provides"].as_array() {
        info.provides.extend(p.iter().filter_map(|x| x.as_str().map(String::from)));
    }
    info.side = match v["environment"].as_str() {
        Some("client") => Side::Client,
        Some("server") => Side::Server,
        _ => Side::Both,
    };
    for (key, kind) in [("depends", DepKind::Required), ("breaks", DepKind::Breaks), ("conflicts", DepKind::Conflicts)] {
        if let Some(obj) = v[key].as_object() {
            for (dep, range) in obj {
                info.depends.push(DepReq { id: dep.clone(), range: fabric_range(range), kind });
            }
        }
    }
}

fn quilt_dep(d: &Value, kind: DepKind, out: &mut Vec<DepReq>) {
    match d {
        Value::String(s) => {
            let (id, range) = match s.split_once('@') {
                Some((id, r)) => (id, Range::Fabric(vec![r.to_string()])),
                None => (s.as_str(), Range::Any),
            };
            out.push(DepReq { id: id.split(':').next_back().unwrap_or(id).to_string(), range, kind });
        }
        Value::Object(o) => {
            let Some(id) = o.get("id").and_then(|x| x.as_str()) else { return };
            // Conditional and optional deps are not enforced by Quilt either.
            if o.get("unless").is_some() {
                return;
            }
            let kind = if o.get("optional").and_then(|x| x.as_bool()) == Some(true) { DepKind::Optional } else { kind };
            let range = o.get("versions").map(fabric_range).unwrap_or(Range::Any);
            out.push(DepReq { id: id.split(':').next_back().unwrap_or(id).to_string(), range, kind });
        }
        _ => {}
    }
}

fn parse_quilt(v: &Value, info: &mut ModInfo) {
    let ql = &v["quilt_loader"];
    let Some(id) = ql["id"].as_str() else { return };
    info.formats.quilt = true;
    info.ids.push(id.to_string());
    info.provides.push(id.to_string());
    if info.name.is_empty() {
        info.name = ql.pointer("/metadata/name").and_then(|x| x.as_str()).unwrap_or(id).to_string();
    }
    if info.version.is_empty() {
        info.version = ql["version"].as_str().unwrap_or_default().to_string();
    }
    if let Some(p) = ql["provides"].as_array() {
        for x in p {
            let pid = x.as_str().or_else(|| x["id"].as_str());
            if let Some(pid) = pid {
                info.provides.push(pid.to_string());
            }
        }
    }
    info.side = match v.pointer("/minecraft/environment").and_then(|x| x.as_str()) {
        Some("client") => Side::Client,
        Some("dedicated_server") => Side::Server,
        _ => info.side,
    };
    for (key, kind) in [("depends", DepKind::Required), ("breaks", DepKind::Breaks)] {
        if let Some(a) = ql[key].as_array() {
            for d in a {
                quilt_dep(d, kind, &mut info.depends);
            }
        }
    }
}

fn manifest_version(zip: &mut zip::ZipArchive<impl Read + std::io::Seek>) -> Option<String> {
    let bytes = read_entry(zip, "META-INF/MANIFEST.MF")?;
    let s = String::from_utf8_lossy(&bytes);
    s.lines()
        .find_map(|l| l.strip_prefix("Implementation-Version:"))
        .map(|v| v.trim().to_string())
}

fn parse_toml(text: &str, neoforge_file: bool, jar_version: Option<&str>, info: &mut ModInfo) {
    let Ok(doc) = text.parse::<toml::Table>() else { return };
    let mods = doc.get("mods").and_then(|m| m.as_array()).cloned().unwrap_or_default();
    for m in &mods {
        let Some(id) = m.get("modId").and_then(|x| x.as_str()) else { continue };
        info.ids.push(id.to_string());
        info.provides.push(id.to_string());
        if info.name.is_empty() {
            info.name = m.get("displayName").and_then(|x| x.as_str()).unwrap_or(id).to_string();
        }
        if info.version.is_empty() {
            let v = m.get("version").and_then(|x| x.as_str()).unwrap_or_default();
            info.version = if v.contains("${") { jar_version.unwrap_or_default().to_string() } else { v.to_string() };
        }
    }
    if let Some(deps) = doc.get("dependencies").and_then(|d| d.as_table()) {
        for list in deps.values() {
            let Some(list) = list.as_array() else { continue };
            for d in list {
                let Some(id) = d.get("modId").and_then(|x| x.as_str()) else { continue };
                let kind = match d.get("type").and_then(|x| x.as_str()).map(|s| s.to_ascii_lowercase()) {
                    Some(t) if t == "required" => DepKind::Required,
                    Some(t) if t == "optional" => DepKind::Optional,
                    Some(t) if t == "incompatible" => DepKind::Breaks,
                    Some(t) if t == "discouraged" => DepKind::Conflicts,
                    _ => match d.get("mandatory").and_then(|x| x.as_bool()) {
                        Some(true) => DepKind::Required,
                        Some(false) => DepKind::Optional,
                        // NeoForge defaults `type` to required.
                        None if neoforge_file => DepKind::Required,
                        None => DepKind::Optional,
                    },
                };
                let range = match d.get("versionRange").and_then(|x| x.as_str()) {
                    Some(r) if !r.trim().is_empty() && r.trim() != "*" => Range::Maven(r.to_string()),
                    _ => Range::Any,
                };
                if kind == DepKind::Required {
                    match id {
                        "forge" => info.targets_forge = true,
                        "neoforge" => info.targets_neoforge = true,
                        _ => {}
                    }
                }
                info.depends.push(DepReq { id: id.to_string(), range, kind });
            }
        }
    }
    if doc.get("clientSideOnly").and_then(|x| x.as_bool()) == Some(true) {
        info.side = Side::Client;
    }
}

fn nested_ids(bytes: Vec<u8>, depth: u8) -> Vec<String> {
    let Ok(mut zip) = zip::ZipArchive::new(std::io::Cursor::new(bytes)) else { return vec![] };
    let info = read_zip(&mut zip, depth);
    info.provides
}

fn read_zip<R: Read + std::io::Seek>(zip: &mut zip::ZipArchive<R>, depth: u8) -> ModInfo {
    let mut info = ModInfo::default();
    if let Some(b) = read_entry(zip, "fabric.mod.json") {
        if let Some(v) = lenient_json(&b) {
            parse_fabric(&v, &mut info);
            if depth < 2 {
                for j in v["jars"].as_array().cloned().unwrap_or_default() {
                    if let Some(path) = j["file"].as_str() {
                        if let Some(bytes) = read_entry(zip, path) {
                            info.provides.extend(nested_ids(bytes, depth + 1));
                        }
                    }
                }
            }
        }
    }
    if let Some(b) = read_entry(zip, "quilt.mod.json") {
        if let Some(v) = lenient_json(&b) {
            parse_quilt(&v, &mut info);
            if depth < 2 {
                for j in v.pointer("/quilt_loader/jars").and_then(|x| x.as_array()).cloned().unwrap_or_default() {
                    if let Some(path) = j.as_str() {
                        if let Some(bytes) = read_entry(zip, path) {
                            info.provides.extend(nested_ids(bytes, depth + 1));
                        }
                    }
                }
            }
        }
    }
    let jar_version = manifest_version(zip);
    for (path, neo) in [("META-INF/neoforge.mods.toml", true), ("META-INF/mods.toml", false)] {
        if let Some(b) = read_entry(zip, path) {
            if neo {
                info.formats.neoforge_toml = true;
            } else {
                info.formats.mods_toml = true;
            }
            parse_toml(&String::from_utf8_lossy(&b), neo, jar_version.as_deref(), &mut info);
        }
    }
    if !info.formats.any() && zip.by_name("mcmod.info").is_ok() {
        info.formats.legacy_forge = true;
    }
    if depth < 2 {
        let nested: Vec<String> = (0..zip.len())
            .filter_map(|i| zip.by_index(i).ok().map(|e| e.name().to_string()))
            .filter(|n| n.starts_with("META-INF/jarjar/") && n.ends_with(".jar"))
            .collect();
        for path in nested {
            if let Some(bytes) = read_entry(zip, &path) {
                info.provides.extend(nested_ids(bytes, depth + 1));
            }
        }
    }
    info.ids.dedup();
    info.provides.sort();
    info.provides.dedup();
    info
}

pub fn read_jar(path: &Path) -> Option<ModInfo> {
    let f = std::fs::File::open(path).ok()?;
    let mut zip = zip::ZipArchive::new(std::io::BufReader::new(f)).ok()?;
    Some(read_zip(&mut zip, 0))
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct CacheEntry {
    size: u64,
    mtime: i64,
    info: Option<ModInfo>,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct Cache {
    version: u32,
    files: HashMap<String, CacheEntry>,
}

const CACHE_VERSION: u32 = 1;

/// A mod jar in the instance with its metadata (None when the jar has no mod metadata).
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct JarMod {
    pub file: String,
    pub enabled: bool,
    pub info: Option<ModInfo>,
}

/// Every jar in `mods/`, enabled or not. Unchanged jars come from `.cairn/modmeta.json`.
pub fn scan(dir: &Path) -> Vec<JarMod> {
    let cache_path = dir.join(".cairn").join("modmeta.json");
    let mut cache: Cache = util::read_json(&cache_path).unwrap_or_default();
    if cache.version != CACHE_VERSION {
        cache = Cache { version: CACHE_VERSION, files: HashMap::new() };
    }
    let mut out = vec![];
    let mut seen = std::collections::HashSet::new();
    let mut dirty = false;
    let Ok(rd) = std::fs::read_dir(dir.join("mods")) else { return out };
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if !base_name(&name).to_lowercase().ends_with(".jar") || !e.path().is_file() {
            continue;
        }
        let size = e.metadata().map(|m| m.len()).unwrap_or(0);
        let mtime = util::mtime_ms(&e.path());
        let key = base_name(&name).to_string();
        seen.insert(key.clone());
        let info = match cache.files.get(&key) {
            Some(c) if c.size == size && c.mtime == mtime => c.info.clone(),
            _ => {
                let info = read_jar(&e.path()).filter(|i| i.formats.any());
                cache.files.insert(key, CacheEntry { size, mtime, info: info.clone() });
                dirty = true;
                info
            }
        };
        out.push(JarMod { enabled: !name.ends_with(".disabled"), file: name, info });
    }
    let before = cache.files.len();
    cache.files.retain(|k, _| seen.contains(k));
    if dirty || cache.files.len() != before {
        let _ = util::write_json_atomic(&cache_path, &cache);
    }
    out.sort_by(|a, b| a.file.cmp(&b.file));
    out
}

#[cfg(test)]
pub mod testjar {
    use std::io::Write;
    use std::path::Path;

    /// Builds a jar from (path, bytes) entries.
    pub fn build(entries: &[(&str, Vec<u8>)]) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut z = zip::ZipWriter::new(&mut buf);
            let opts = zip::write::SimpleFileOptions::default();
            for (name, bytes) in entries {
                z.start_file(*name, opts).unwrap();
                z.write_all(bytes).unwrap();
            }
            z.finish().unwrap();
        }
        buf.into_inner()
    }

    pub fn write(path: &Path, entries: &[(&str, Vec<u8>)]) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, build(entries)).unwrap();
    }

    pub fn fabric(id: &str, version: &str, depends: &str) -> Vec<u8> {
        build(&[("fabric.mod.json", format!(r#"{{"schemaVersion":1,"id":"{id}","version":"{version}","depends":{depends}}}"#).into_bytes())])
    }
}

#[cfg(test)]
mod tests {
    use super::testjar::*;
    use super::*;

    fn tmp(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("cairn-modmeta-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn fabric_metadata_with_nested_jars() {
        let d = tmp("fabric");
        let nested = fabric("fabric-api-base", "0.4.1", "{}");
        let fmj = r#"{"schemaVersion":1,"id":"fabric-api","name":"Fabric API","version":"0.92.2+1.20.1",
            "environment":"*","provides":["fabric"],"depends":{"fabricloader":">=0.14.22","minecraft":["1.20.1"]},
            "breaks":{"optifabric":"<1.13.0"},"jars":[{"file":"META-INF/jars/base.jar"}]}"#;
        write(&d.join("api.jar"), &[("fabric.mod.json", fmj.as_bytes().to_vec()), ("META-INF/jars/base.jar", nested)]);
        let info = read_jar(&d.join("api.jar")).unwrap();
        assert_eq!(info.ids, vec!["fabric-api"]);
        assert!(info.provides.contains(&"fabric".to_string()));
        assert!(info.provides.contains(&"fabric-api-base".to_string()));
        assert_eq!(info.version, "0.92.2+1.20.1");
        assert!(info.formats.fabric);
        let mc = info.depends.iter().find(|d| d.id == "minecraft").unwrap();
        assert_eq!(mc.range.matches("1.20.1"), Some(true));
        assert_eq!(mc.range.matches("1.20.4"), Some(false));
        assert!(info.depends.iter().any(|d| d.id == "optifabric" && d.kind == DepKind::Breaks));
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn fabric_json_with_raw_newline_in_string() {
        let raw = b"{\"schemaVersion\":1,\"id\":\"odd\",\"version\":\"1.0\",\"description\":\"line one\nline two\"}".to_vec();
        let d = tmp("lenient");
        write(&d.join("odd.jar"), &[("fabric.mod.json", raw)]);
        assert_eq!(read_jar(&d.join("odd.jar")).unwrap().ids, vec!["odd"]);
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn neoforge_toml() {
        let toml = r#"
modLoader="javafml"
loaderVersion="[4,)"
[[mods]]
modId="examplemod"
version="${file.jarVersion}"
displayName="Example Mod"
[[dependencies.examplemod]]
modId="neoforge"
type="required"
versionRange="[21.1,)"
[[dependencies.examplemod]]
modId="architectury"
versionRange="[13.0,)"
[[dependencies.examplemod]]
modId="optifine"
type="incompatible"
[[dependencies.examplemod]]
modId="jei"
type="optional"
"#;
        let d = tmp("neo");
        write(&d.join("ex.jar"), &[("META-INF/neoforge.mods.toml", toml.as_bytes().to_vec()), ("META-INF/MANIFEST.MF", b"Manifest-Version: 1.0\r\nImplementation-Version: 2.3.4\r\n".to_vec())]);
        let info = read_jar(&d.join("ex.jar")).unwrap();
        assert_eq!(info.ids, vec!["examplemod"]);
        assert_eq!(info.version, "2.3.4");
        assert!(info.formats.neoforge_toml && info.targets_neoforge);
        let arch = info.depends.iter().find(|d| d.id == "architectury").unwrap();
        assert_eq!(arch.kind, DepKind::Required);
        assert_eq!(arch.range, Range::Maven("[13.0,)".into()));
        assert!(info.depends.iter().any(|d| d.id == "optifine" && d.kind == DepKind::Breaks));
        assert!(info.depends.iter().any(|d| d.id == "jei" && d.kind == DepKind::Optional));
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn forge_toml_mandatory_flag_and_jarjar() {
        let toml = r#"
modLoader="javafml"
[[mods]]
modId="create"
version="0.5.1"
[[dependencies.create]]
modId="forge"
mandatory=true
versionRange="[47.1,)"
[[dependencies.create]]
modId="flywheel"
mandatory=true
[[dependencies.create]]
modId="jei"
mandatory=false
"#;
        let inner = build(&[("META-INF/mods.toml", b"[[mods]]\nmodId=\"flywheel\"\nversion=\"0.6.10\"\n".to_vec())]);
        let d = tmp("forge");
        write(&d.join("create.jar"), &[("META-INF/mods.toml", toml.as_bytes().to_vec()), ("META-INF/jarjar/flywheel.jar", inner)]);
        let info = read_jar(&d.join("create.jar")).unwrap();
        assert!(info.targets_forge && !info.targets_neoforge);
        assert!(info.provides.contains(&"flywheel".to_string()));
        assert!(info.depends.iter().any(|d| d.id == "flywheel" && d.kind == DepKind::Required));
        assert!(info.depends.iter().any(|d| d.id == "jei" && d.kind == DepKind::Optional));
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn quilt_metadata() {
        let qmj = r#"{"schema_version":1,"quilt_loader":{"id":"qmod","version":"1.0.0","metadata":{"name":"Q Mod"},
            "depends":["quilt_loader",{"id":"qsl","versions":">=6.0"},{"id":"emi","optional":true}],
            "provides":["qmod_compat"]},"minecraft":{"environment":"client"}}"#;
        let d = tmp("quilt");
        write(&d.join("q.jar"), &[("quilt.mod.json", qmj.as_bytes().to_vec())]);
        let info = read_jar(&d.join("q.jar")).unwrap();
        assert!(info.formats.quilt);
        assert_eq!(info.side, Side::Client);
        assert!(info.provides.contains(&"qmod_compat".to_string()));
        assert!(info.depends.iter().any(|d| d.id == "qsl" && d.kind == DepKind::Required));
        assert!(info.depends.iter().any(|d| d.id == "emi" && d.kind == DepKind::Optional));
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn scan_uses_cache_and_tracks_disabled() {
        let d = tmp("scan");
        std::fs::create_dir_all(d.join("mods")).unwrap();
        std::fs::write(d.join("mods/a.jar"), fabric("a", "1.0", "{}")).unwrap();
        std::fs::write(d.join("mods/b.jar.disabled"), fabric("b", "1.0", "{}")).unwrap();
        std::fs::write(d.join("mods/notes.txt"), b"x").unwrap();
        std::fs::write(d.join("mods/lib.jar"), build(&[("x.class", vec![0])])).unwrap();
        let first = scan(&d);
        assert_eq!(first.len(), 3);
        assert!(first.iter().any(|m| m.file == "b.jar.disabled" && !m.enabled));
        assert!(first.iter().any(|m| m.file == "lib.jar" && m.info.is_none()));
        assert!(d.join(".cairn/modmeta.json").exists());
        let again = scan(&d);
        assert_eq!(again.iter().filter(|m| m.info.is_some()).count(), 2);
        std::fs::remove_dir_all(&d).unwrap();
    }
}
