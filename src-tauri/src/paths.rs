use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Paths {
    pub root: PathBuf,
    pub meta: PathBuf,
    pub instances: PathBuf,
    pub java: PathBuf,
    pub cache: PathBuf,
}

impl Paths {
    pub fn new() -> Self {
        let root = std::env::var_os("CAIRN_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("cairn"));
        Self::at(&root)
    }

    pub fn at(root: &Path) -> Self {
        // `meta` mirrors a .minecraft layout so the Forge/NeoForge installers can write into it.
        let p = Paths {
            root: root.to_path_buf(),
            meta: root.join("meta"),
            instances: root.join("instances"),
            java: root.join("java"),
            cache: root.join("cache"),
        };
        for d in [&p.root, &p.meta, &p.instances, &p.java, &p.cache] {
            let _ = std::fs::create_dir_all(d);
        }
        p
    }

    pub fn versions(&self) -> PathBuf { self.meta.join("versions") }
    pub fn libraries(&self) -> PathBuf { self.meta.join("libraries") }
    pub fn assets(&self) -> PathBuf { self.meta.join("assets") }
    pub fn natives(&self, version: &str) -> PathBuf { self.meta.join("natives").join(version) }
    pub fn version_json(&self, id: &str) -> PathBuf { self.versions().join(id).join(format!("{id}.json")) }
    pub fn version_jar(&self, id: &str) -> PathBuf { self.versions().join(id).join(format!("{id}.jar")) }
    pub fn instance_dir(&self, id: &str) -> PathBuf { self.instances.join(id) }
    pub fn settings_file(&self) -> PathBuf { self.root.join("settings.json") }
    pub fn accounts_file(&self) -> PathBuf { self.root.join("accounts.json") }
}

impl Default for Paths {
    fn default() -> Self {
        Self::new()
    }
}

/// Joins a relative path from untrusted input (modpack index, zip entry) onto a base,
/// refusing absolute paths and `..` components.
pub fn safe_join(base: &Path, rel: &str) -> Option<PathBuf> {
    let rel = rel.replace('\\', "/");
    if rel.starts_with('/') || rel.contains(':') {
        return None;
    }
    let mut out = base.to_path_buf();
    for part in rel.split('/') {
        match part {
            "" | "." => continue,
            ".." => return None,
            p => out.push(p),
        }
    }
    if out == base { None } else { Some(out) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn safe_join_blocks_traversal() {
        let b = Path::new("/tmp/x");
        assert_eq!(safe_join(b, "mods/a.jar"), Some(PathBuf::from("/tmp/x/mods/a.jar")));
        assert_eq!(safe_join(b, "../etc/passwd"), None);
        assert_eq!(safe_join(b, "mods/../../a"), None);
        assert_eq!(safe_join(b, "/etc/passwd"), None);
        assert_eq!(safe_join(b, "C:\\Windows"), None);
        assert_eq!(safe_join(b, ""), None);
    }
}
