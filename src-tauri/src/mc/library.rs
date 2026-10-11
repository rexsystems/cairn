use serde_json::Value;

/// Maven coordinate `group:artifact:version[:classifier][@ext]` to a repository path.
pub fn maven_path(name: &str) -> Option<String> {
    let (coords, ext) = match name.split_once('@') {
        Some((c, e)) => (c, e),
        None => (name, "jar"),
    };
    let parts: Vec<&str> = coords.split(':').collect();
    if parts.len() < 3 {
        return None;
    }
    let group = parts[0].replace('.', "/");
    let artifact = parts[1];
    let version = parts[2];
    let file = match parts.get(3) {
        Some(cls) => format!("{artifact}-{version}-{cls}.{ext}"),
        None => format!("{artifact}-{version}.{ext}"),
    };
    Some(format!("{group}/{artifact}/{version}/{file}"))
}

/// Key used to dedupe libraries across inherited version JSONs: group:artifact[:classifier].
pub fn library_key(name: &str) -> String {
    let coords = name.split('@').next().unwrap_or(name);
    let p: Vec<&str> = coords.split(':').collect();
    match p.len() {
        0..=2 => coords.to_string(),
        3 => format!("{}:{}", p[0], p[1]),
        _ => format!("{}:{}:{}", p[0], p[1], p[3]),
    }
}

pub fn classifier(name: &str) -> Option<String> {
    let coords = name.split('@').next().unwrap_or(name);
    coords.split(':').nth(3).map(|s| s.to_string())
}

#[derive(Debug, Clone, PartialEq)]
pub struct Artifact {
    pub path: String,
    pub url: String,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

/// Main artifact of a library entry, handling both `downloads.artifact` and the bare `url` style.
pub fn main_artifact(lib: &Value) -> Option<Artifact> {
    let name = lib.get("name")?.as_str()?;
    if let Some(a) = lib.pointer("/downloads/artifact") {
        let path = a.get("path").and_then(|v| v.as_str()).map(String::from).or_else(|| maven_path(name))?;
        return Some(Artifact {
            path,
            url: a.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            sha1: a.get("sha1").and_then(|v| v.as_str()).map(String::from),
            size: a.get("size").and_then(|v| v.as_u64()),
        });
    }
    if lib.get("downloads").is_some() {
        // Has only classifiers (old natives-only entries).
        return None;
    }
    let path = maven_path(name)?;
    let mut base = lib.get("url").and_then(|v| v.as_str()).unwrap_or("https://libraries.minecraft.net/").to_string();
    if !base.ends_with('/') {
        base.push('/');
    }
    Some(Artifact { url: format!("{base}{path}"), path, sha1: lib.get("sha1").and_then(|v| v.as_str()).map(String::from), size: None })
}

/// Old-style natives (pre 1.19): `natives.linux` names a classifier inside `downloads.classifiers`.
pub fn native_artifact(lib: &Value, os: &str, arch_bits: &str) -> Option<Artifact> {
    let key = lib.get("natives")?.get(os)?.as_str()?.replace("${arch}", arch_bits);
    let c = lib.pointer("/downloads/classifiers")?.get(&key)?;
    Some(Artifact {
        path: c.get("path")?.as_str()?.to_string(),
        url: c.get("url")?.as_str()?.to_string(),
        sha1: c.get("sha1").and_then(|v| v.as_str()).map(String::from),
        size: c.get("size").and_then(|v| v.as_u64()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn paths() {
        assert_eq!(maven_path("org.ow2.asm:asm:9.6").unwrap(), "org/ow2/asm/asm/9.6/asm-9.6.jar");
        assert_eq!(
            maven_path("org.lwjgl:lwjgl:3.3.3:natives-linux").unwrap(),
            "org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-linux.jar"
        );
        assert_eq!(maven_path("de.oceanlabs.mcp:mcp_config:1.20.1@zip").unwrap(), "de/oceanlabs/mcp/mcp_config/1.20.1/mcp_config-1.20.1.zip");
        assert!(maven_path("bad").is_none());
    }

    #[test]
    fn keys() {
        assert_eq!(library_key("org.ow2.asm:asm:9.6"), "org.ow2.asm:asm");
        assert_eq!(library_key("org.lwjgl:lwjgl:3.3.3:natives-linux"), "org.lwjgl:lwjgl:natives-linux");
    }

    #[test]
    fn fabric_style_library() {
        let l = json!({"name":"net.fabricmc:fabric-loader:0.16.0","url":"https://maven.fabricmc.net/"});
        let a = main_artifact(&l).unwrap();
        assert_eq!(a.url, "https://maven.fabricmc.net/net/fabricmc/fabric-loader/0.16.0/fabric-loader-0.16.0.jar");
    }

    #[test]
    fn old_natives() {
        let l = json!({"name":"org.lwjgl.lwjgl:lwjgl-platform:2.9.4","natives":{"linux":"natives-linux","windows":"natives-windows-${arch}"},
            "downloads":{"classifiers":{"natives-linux":{"path":"a/b.jar","url":"https://x/b.jar","sha1":"aa","size":3},
            "natives-windows-64":{"path":"w.jar","url":"https://x/w.jar"}}}});
        assert_eq!(native_artifact(&l, "linux", "64").unwrap().path, "a/b.jar");
        assert_eq!(native_artifact(&l, "windows", "64").unwrap().path, "w.jar");
        assert!(main_artifact(&l).is_none());
    }
}
