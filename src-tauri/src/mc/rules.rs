use serde_json::Value;
use std::collections::HashMap;

pub fn os_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "linux"
    }
}

pub fn os_arch() -> &'static str {
    if cfg!(target_arch = "aarch64") {
        "arm64"
    } else if cfg!(target_arch = "x86") {
        "x86"
    } else {
        "x86_64"
    }
}

pub type Features = HashMap<String, bool>;

fn rule_matches(rule: &Value, features: &Features, os: &str, arch: &str) -> bool {
    if let Some(o) = rule.get("os") {
        if let Some(name) = o.get("name").and_then(|v| v.as_str()) {
            if name != os {
                return false;
            }
        }
        if let Some(a) = o.get("arch").and_then(|v| v.as_str()) {
            // Mojang uses "x86" to mean a 32-bit JVM.
            let ok = match a {
                "x86" => arch == "x86",
                "arm64" => arch == "arm64",
                other => other == arch,
            };
            if !ok {
                return false;
            }
        }
    }
    if let Some(f) = rule.get("features").and_then(|v| v.as_object()) {
        for (k, v) in f {
            let want = v.as_bool().unwrap_or(false);
            if features.get(k).copied().unwrap_or(false) != want {
                return false;
            }
        }
    }
    true
}

pub fn allowed_for(rules: Option<&Value>, features: &Features, os: &str, arch: &str) -> bool {
    let Some(rules) = rules.and_then(|r| r.as_array()) else { return true };
    if rules.is_empty() {
        return true;
    }
    let mut allowed = false;
    for r in rules {
        if rule_matches(r, features, os, arch) {
            allowed = r.get("action").and_then(|v| v.as_str()) == Some("allow");
        }
    }
    allowed
}

pub fn allowed(rules: Option<&Value>, features: &Features) -> bool {
    allowed_for(rules, features, os_name(), os_arch())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn no_rules_allows() {
        assert!(allowed_for(None, &Features::new(), "linux", "x86_64"));
    }

    #[test]
    fn osx_only_lib_is_skipped_on_linux() {
        let r = json!([{"action":"allow","os":{"name":"osx"}}]);
        assert!(!allowed_for(Some(&r), &Features::new(), "linux", "x86_64"));
        assert!(allowed_for(Some(&r), &Features::new(), "osx", "x86_64"));
    }

    #[test]
    fn allow_then_disallow() {
        let r = json!([{"action":"allow"},{"action":"disallow","os":{"name":"osx"}}]);
        assert!(allowed_for(Some(&r), &Features::new(), "linux", "x86_64"));
        assert!(!allowed_for(Some(&r), &Features::new(), "osx", "x86_64"));
    }

    #[test]
    fn features_gate_arguments() {
        let r = json!([{"action":"allow","features":{"has_custom_resolution":true}}]);
        let mut f = Features::new();
        assert!(!allowed_for(Some(&r), &f, "linux", "x86_64"));
        f.insert("has_custom_resolution".into(), true);
        assert!(allowed_for(Some(&r), &f, "linux", "x86_64"));
    }

    #[test]
    fn x86_means_32_bit() {
        let r = json!([{"action":"allow","os":{"arch":"x86"}}]);
        assert!(!allowed_for(Some(&r), &Features::new(), "windows", "x86_64"));
    }
}
