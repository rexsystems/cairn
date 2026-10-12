//! Cover art for vanilla / version-only instances: Minecraft Wiki page image for that release.

use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;
use urlencoding::encode;

static CACHE: Lazy<Mutex<HashMap<String, Option<String>>>> = Lazy::new(|| Mutex::new(HashMap::new()));
static SNAPSHOT_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^(\d+\.\d+(?:\.\d+)?)-snapshot-(\d+)$").expect("snapshot re"));

/// Map launcher version ids onto wiki article titles.
/// `1.21.1` → `Java Edition 1.21.1`, `26.4-snapshot-3` → `Java Edition 26.4 Snapshot 3`.
pub fn wiki_title(mc_version: &str) -> String {
    let v = mc_version.trim();
    if let Some(caps) = SNAPSHOT_RE.captures(v) {
        return format!(
            "Java Edition {} Snapshot {}",
            &caps[1],
            &caps[2]
        );
    }
    format!("Java Edition {v}")
}

pub async fn image_url(mc_version: &str) -> Option<String> {
    let key = mc_version.trim().to_string();
    if key.is_empty() {
        return None;
    }
    if let Ok(cache) = CACHE.lock() {
        if let Some(hit) = cache.get(&key) {
            return hit.clone();
        }
    }

    let title = wiki_title(&key);
    let api = format!(
        "https://minecraft.wiki/api.php?action=query&titles={}&prop=pageimages&format=json&pithumbsize=800&pilicense=any",
        encode(&title)
    );
    let resolved = match crate::net::get_json::<Value>(&api).await {
        Ok(body) => body
            .pointer("/query/pages")
            .and_then(|p| p.as_object())
            .and_then(|pages| {
                pages.values().find_map(|page| {
                    if page.get("missing").is_some() {
                        return None;
                    }
                    page.pointer("/thumbnail/source")
                        .and_then(|s| s.as_str())
                        .map(|s| s.to_string())
                })
            }),
        Err(_) => None,
    };

    if let Ok(mut cache) = CACHE.lock() {
        cache.insert(key, resolved.clone());
    }
    resolved
}
