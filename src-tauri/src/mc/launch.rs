use super::library::{classifier, main_artifact};
use super::rules::{self, Features};
use crate::paths::Paths;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct LaunchAuth {
    pub name: String,
    pub uuid: String,
    pub access_token: String,
    pub user_type: String,
    pub xuid: String,
}

pub struct LaunchOptions {
    pub memory_mb: u32,
    pub extra_jvm_args: Vec<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub game_dir: PathBuf,
    pub natives_dir: PathBuf,
    pub jar_id: String,
    pub quick_play_world: Option<String>,
    pub quick_play_server: Option<String>,
}

pub fn classpath(paths: &Paths, v: &Value, jar_id: &str) -> Vec<String> {
    let features = Features::new();
    let mut cp: Vec<String> = vec![];
    for lib in v.get("libraries").and_then(|l| l.as_array()).into_iter().flatten() {
        if !rules::allowed(lib.get("rules"), &features) {
            continue;
        }
        if lib.get("clientreq") == Some(&Value::Bool(false)) {
            continue;
        }
        // Old-style native-only entries never go on the classpath.
        if lib.get("natives").is_some() && lib.pointer("/downloads/artifact").is_none() && lib.get("downloads").is_some() {
            continue;
        }
        let name = lib.get("name").and_then(|n| n.as_str()).unwrap_or_default();
        if let Some(a) = main_artifact(lib) {
            let _ = classifier(name);
            let p = paths.libraries().join(&a.path).to_string_lossy().to_string();
            if !cp.contains(&p) {
                cp.push(p);
            }
        }
    }
    cp.push(paths.version_jar(jar_id).to_string_lossy().to_string());
    cp
}

fn substitute(arg: &str, vars: &HashMap<&str, String>) -> String {
    let mut out = String::with_capacity(arg.len());
    let mut rest = arg;
    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match after.find('}') {
            Some(end) => {
                let key = &after[..end];
                match vars.get(key) {
                    Some(v) => out.push_str(v),
                    None => {
                        out.push_str("${");
                        out.push_str(key);
                        out.push('}');
                    }
                }
                rest = &after[end + 1..];
            }
            None => {
                out.push_str(&rest[start..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

fn collect_args(list: Option<&Value>, features: &Features, vars: &HashMap<&str, String>) -> Vec<String> {
    let mut out = vec![];
    for a in list.and_then(|l| l.as_array()).into_iter().flatten() {
        match a {
            Value::String(s) => out.push(substitute(s, vars)),
            Value::Object(_) => {
                if !rules::allowed(a.get("rules"), features) {
                    continue;
                }
                match a.get("value") {
                    Some(Value::String(s)) => out.push(substitute(s, vars)),
                    Some(Value::Array(arr)) => {
                        for s in arr.iter().filter_map(|x| x.as_str()) {
                            out.push(substitute(s, vars));
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    out
}

/// Builds the JVM + game argument list (everything after the java executable).
pub fn build_args(paths: &Paths, v: &Value, auth: &LaunchAuth, opts: &LaunchOptions) -> Vec<String> {
    let sep = if cfg!(windows) { ";" } else { ":" };
    let cp = classpath(paths, v, &opts.jar_id).join(sep);
    let asset_index = v.pointer("/assetIndex/id").and_then(|x| x.as_str()).unwrap_or("legacy").to_string();
    let version_id = v.get("id").and_then(|x| x.as_str()).unwrap_or_default().to_string();
    let mut vars: HashMap<&str, String> = HashMap::new();
    vars.insert("auth_player_name", auth.name.clone());
    vars.insert("version_name", version_id.clone());
    vars.insert("game_directory", opts.game_dir.to_string_lossy().to_string());
    vars.insert("assets_root", paths.assets().to_string_lossy().to_string());
    vars.insert("game_assets", paths.assets().join("virtual").join(&asset_index).to_string_lossy().to_string());
    vars.insert("assets_index_name", asset_index);
    vars.insert("auth_uuid", auth.uuid.clone());
    vars.insert("auth_access_token", auth.access_token.clone());
    vars.insert("auth_session", format!("token:{}:{}", auth.access_token, auth.uuid));
    vars.insert("auth_xuid", auth.xuid.clone());
    vars.insert("clientid", String::new());
    vars.insert("user_type", auth.user_type.clone());
    vars.insert("user_properties", "{}".into());
    vars.insert("version_type", v.get("type").and_then(|x| x.as_str()).unwrap_or("release").to_string());
    vars.insert("natives_directory", opts.natives_dir.to_string_lossy().to_string());
    vars.insert("launcher_name", "cairn".into());
    vars.insert("launcher_version", env!("CARGO_PKG_VERSION").into());
    vars.insert("classpath", cp.clone());
    vars.insert("classpath_separator", sep.into());
    vars.insert("library_directory", paths.libraries().to_string_lossy().to_string());
    vars.insert("resolution_width", opts.width.unwrap_or(854).to_string());
    vars.insert("resolution_height", opts.height.unwrap_or(480).to_string());
    if let Some(w) = &opts.quick_play_world {
        vars.insert("quickPlaySingleplayer", w.clone());
    }
    if let Some(s) = &opts.quick_play_server {
        vars.insert("quickPlayMultiplayer", s.clone());
    }

    let mut features = Features::new();
    features.insert("has_custom_resolution".into(), opts.width.is_some() && opts.height.is_some());
    features.insert("is_quick_play_singleplayer".into(), opts.quick_play_world.is_some());
    features.insert("is_quick_play_multiplayer".into(), opts.quick_play_server.is_some());

    let mut args: Vec<String> = vec![format!("-Xmx{}M", opts.memory_mb), format!("-Xms{}M", (opts.memory_mb / 4).max(256))];

    if let Some(log) = v.pointer("/logging/client") {
        if let (Some(arg), Some(fid)) = (log.get("argument").and_then(|x| x.as_str()), log.pointer("/file/id").and_then(|x| x.as_str())) {
            let p = paths.assets().join("log_configs").join(fid);
            args.push(arg.replace("${path}", &p.to_string_lossy()));
        }
    }

    let modern = v.pointer("/arguments/jvm").is_some();
    if modern {
        args.extend(collect_args(v.pointer("/arguments/jvm"), &features, &vars));
    } else {
        args.push(format!("-Djava.library.path={}", opts.natives_dir.to_string_lossy()));
        args.push("-cp".into());
        args.push(cp);
    }
    args.extend(opts.extra_jvm_args.iter().cloned());
    args.push(v.get("mainClass").and_then(|x| x.as_str()).unwrap_or("net.minecraft.client.main.Main").to_string());

    if let Some(game) = v.pointer("/arguments/game") {
        args.extend(collect_args(Some(game), &features, &vars));
    } else if let Some(legacy) = v.get("minecraftArguments").and_then(|x| x.as_str()) {
        args.extend(legacy.split_whitespace().map(|a| substitute(a, &vars)));
        if let (Some(w), Some(h)) = (opts.width, opts.height) {
            args.extend(["--width".into(), w.to_string(), "--height".into(), h.to_string()]);
        }
    }
    args
}

/// Java's UUID.nameUUIDFromBytes("OfflinePlayer:" + name), which servers expect for offline players.
pub fn offline_uuid(name: &str) -> String {
    use md5::{Digest, Md5};
    let mut h: [u8; 16] = Md5::digest(format!("OfflinePlayer:{name}").as_bytes()).into();
    h[6] = (h[6] & 0x0f) | 0x30;
    h[8] = (h[8] & 0x3f) | 0x80;
    let s = hex::encode(h);
    format!("{}-{}-{}-{}-{}", &s[0..8], &s[8..12], &s[12..16], &s[16..20], &s[20..32])
}

pub fn redact(args: &[String], token: &str) -> Vec<String> {
    args.iter().map(|a| if !token.is_empty() && a.contains(token) { a.replace(token, "<token hidden>") } else { a.clone() }).collect()
}

#[allow(dead_code)]
pub fn exists(p: &Path) -> bool {
    p.exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn offline_uuid_matches_java() {
        // Value produced by Java's UUID.nameUUIDFromBytes("OfflinePlayer:Notch".getBytes()).
        assert_eq!(offline_uuid("Notch"), "b50ad385-829d-3141-a216-7e7d7539ba7f");
    }

    #[test]
    fn substitution() {
        let mut vars = HashMap::new();
        vars.insert("a", "1".to_string());
        assert_eq!(substitute("x${a}y${b}", &vars), "x1y${b}");
        assert_eq!(substitute("${a", &vars), "${a");
    }

    #[test]
    fn modern_args() {
        let dir = std::env::temp_dir().join("cairn-test-args");
        let paths = Paths::at(&dir);
        let v = json!({
            "id":"1.21.1","type":"release","mainClass":"net.minecraft.client.main.Main",
            "assetIndex":{"id":"17"},
            "libraries":[
                {"name":"com.mojang:brigadier:1.3.10","downloads":{"artifact":{"path":"com/mojang/brigadier/1.3.10/brigadier-1.3.10.jar","url":"u","sha1":"s"}}},
                {"name":"ca.weblite:java-objc-bridge:1.1","rules":[{"action":"allow","os":{"name":"osx"}}],"downloads":{"artifact":{"path":"x.jar","url":"u"}}}
            ],
            "arguments":{
                "game":["--username","${auth_player_name}","--assetIndex","${assets_index_name}",
                    {"rules":[{"action":"allow","features":{"is_demo_user":true}}],"value":"--demo"},
                    {"rules":[{"action":"allow","features":{"has_custom_resolution":true}}],"value":["--width","${resolution_width}","--height","${resolution_height}"]}],
                "jvm":["-Djava.library.path=${natives_directory}","-cp","${classpath}"]
            }
        });
        let auth = LaunchAuth { name: "Steve".into(), uuid: "u".into(), access_token: "t".into(), user_type: "legacy".into(), xuid: "0".into() };
        let opts = LaunchOptions {
            memory_mb: 2048, extra_jvm_args: vec!["-Dfoo=1".into()], width: Some(1280), height: Some(720),
            game_dir: dir.join("g"), natives_dir: dir.join("n"), jar_id: "1.21.1".into(), quick_play_world: None, quick_play_server: None,
        };
        let a = build_args(&paths, &v, &auth, &opts);
        assert_eq!(a[0], "-Xmx2048M");
        assert!(a.iter().any(|x| x.starts_with("-Djava.library.path=")));
        let main = a.iter().position(|x| x == "net.minecraft.client.main.Main").unwrap();
        assert_eq!(a[main - 1], "-Dfoo=1");
        assert!(!a.contains(&"--demo".to_string()));
        assert!(a.contains(&"1280".to_string()));
        let cp = &a[a.iter().position(|x| x == "-cp").unwrap() + 1];
        assert!(cp.contains("brigadier"));
        assert!(!cp.contains("x.jar"));
        assert!(cp.ends_with("1.21.1.jar"));
        assert!(a.contains(&"Steve".to_string()));
    }
}
