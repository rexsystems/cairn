use crate::net;
use crate::paths::Paths;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ManifestVersion {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub url: String,
    pub release_time: String,
    pub sha1: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Manifest {
    pub latest: Value,
    pub versions: Vec<ManifestVersion>,
}

pub async fn manifest(paths: &Paths) -> Result<Manifest> {
    let cache = paths.cache.join("version_manifest_v2.json");
    match net::get_json::<Manifest>(MANIFEST_URL).await {
        Ok(m) => {
            let _ = crate::util::write_json_atomic(&cache, &m);
            Ok(m)
        }
        // Offline: fall back to the last copy we saw.
        Err(e) => crate::util::read_json(&cache).map_err(|_| e),
    }
}

/// Makes sure the vanilla version JSON exists locally and returns it.
pub async fn ensure_vanilla_json(paths: &Paths, id: &str) -> Result<Value> {
    let p = paths.version_json(id);
    if p.exists() {
        if let Ok(v) = crate::util::read_json::<Value>(&p) {
            return Ok(v);
        }
    }
    let m = manifest(paths).await?;
    let entry = m.versions.iter().find(|v| v.id == id).ok_or_else(|| anyhow!("Minecraft {id} is not in Mojang's version list"))?;
    net::download(&net::Download { url: entry.url.clone(), dest: p.clone(), sha1: entry.sha1.clone(), size: None }).await?;
    crate::util::read_json(&p)
}

pub fn load_local(paths: &Paths, id: &str) -> Result<Value> {
    crate::util::read_json(&paths.version_json(id)).with_context(|| format!("version {id} is not installed"))
}

/// Merges a child version JSON (Fabric, Forge...) over its parent.
pub fn merge(parent: &Value, child: &Value) -> Value {
    let mut out = parent.clone();
    let o = out.as_object_mut().unwrap();
    for (k, v) in child.as_object().unwrap() {
        match k.as_str() {
            "libraries" => {
                let mut libs: Vec<Value> = v.as_array().cloned().unwrap_or_default();
                let seen: std::collections::HashSet<String> = libs
                    .iter()
                    .filter_map(|l| l.get("name").and_then(|n| n.as_str()).map(super::library::library_key))
                    .collect();
                for l in parent.get("libraries").and_then(|l| l.as_array()).cloned().unwrap_or_default() {
                    let key = l.get("name").and_then(|n| n.as_str()).map(super::library::library_key).unwrap_or_default();
                    if !seen.contains(&key) {
                        libs.push(l);
                    }
                }
                o.insert("libraries".into(), Value::Array(libs));
            }
            "arguments" => {
                let mut args = parent.get("arguments").cloned().unwrap_or_else(|| json!({}));
                for kind in ["game", "jvm"] {
                    if let Some(extra) = v.get(kind).and_then(|x| x.as_array()) {
                        let list = args.as_object_mut().unwrap().entry(kind).or_insert_with(|| json!([]));
                        if let Some(arr) = list.as_array_mut() {
                            arr.extend(extra.iter().cloned());
                        }
                    }
                }
                o.insert("arguments".into(), args);
            }
            "inheritsFrom" => {}
            _ => {
                o.insert(k.clone(), v.clone());
            }
        }
    }
    out
}

/// Loads a version and walks `inheritsFrom`, returning the fully merged JSON.
pub async fn resolve(paths: &Paths, id: &str) -> Result<Value> {
    let mut chain = vec![];
    let mut cur = id.to_string();
    for _ in 0..8 {
        let v = match load_local(paths, &cur) {
            Ok(v) => v,
            Err(_) => ensure_vanilla_json(paths, &cur).await?,
        };
        let parent = v.get("inheritsFrom").and_then(|p| p.as_str()).map(String::from);
        chain.push(v);
        match parent {
            Some(p) => cur = p,
            None => break,
        }
    }
    let mut merged = chain.pop().unwrap();
    while let Some(child) = chain.pop() {
        merged = merge(&merged, &child);
    }
    if let Some(o) = merged.as_object_mut() {
        o.insert("id".into(), Value::String(id.to_string()));
    }
    Ok(merged)
}

/// The vanilla version whose client jar this version runs on.
pub fn jar_id(original: &Value, merged: &Value) -> String {
    original
        .get("jar")
        .or_else(|| original.get("inheritsFrom"))
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_else(|| merged.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string())
}

pub fn java_major(v: &Value) -> u32 {
    v.pointer("/javaVersion/majorVersion").and_then(|x| x.as_u64()).map(|x| x as u32).unwrap_or(8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_prefers_child_libraries_and_appends_args() {
        let parent = json!({
            "id":"1.21","mainClass":"net.minecraft.client.main.Main",
            "libraries":[{"name":"org.ow2.asm:asm:9.3"},{"name":"com.mojang:brigadier:1.0"}],
            "arguments":{"game":["--username","${auth_player_name}"],"jvm":["-cp","${classpath}"]},
            "assetIndex":{"id":"17"}
        });
        let child = json!({
            "id":"fabric-loader-0.16-1.21","inheritsFrom":"1.21","mainClass":"net.fabricmc.loader.impl.launch.knot.KnotClient",
            "libraries":[{"name":"org.ow2.asm:asm:9.7"},{"name":"net.fabricmc:fabric-loader:0.16"}],
            "arguments":{"jvm":["-DFabricMcEmu= net.minecraft.client.main.Main "]}
        });
        let m = merge(&parent, &child);
        assert_eq!(m["mainClass"], "net.fabricmc.loader.impl.launch.knot.KnotClient");
        let names: Vec<&str> = m["libraries"].as_array().unwrap().iter().map(|l| l["name"].as_str().unwrap()).collect();
        assert_eq!(names, vec!["org.ow2.asm:asm:9.7", "net.fabricmc:fabric-loader:0.16", "com.mojang:brigadier:1.0"]);
        assert_eq!(m["arguments"]["jvm"].as_array().unwrap().len(), 3);
        assert_eq!(m["arguments"]["game"].as_array().unwrap().len(), 2);
        assert_eq!(m["assetIndex"]["id"], "17");
        assert!(m.get("inheritsFrom").is_none());
    }

    #[test]
    fn java_major_defaults_to_8() {
        assert_eq!(java_major(&json!({})), 8);
        assert_eq!(java_major(&json!({"javaVersion":{"majorVersion":21}})), 21);
    }
}
