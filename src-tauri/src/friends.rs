//! People you played with through an invite, kept on this computer only.
//!
//! Discord's public RPC has no friends list or online status, so this is what Cairn can offer:
//! who joined you after you accepted their request, and the sessions you joined.

use crate::paths::Paths;
use crate::util;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const KEEP: usize = 50;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Friend {
    pub id: String,
    /// "joinedYou" or "youJoined".
    pub kind: String,
    pub name: String,
    pub user_id: Option<String>,
    pub avatar_url: Option<String>,
    pub instance_id: String,
    pub instance_name: String,
    /// The invite used to join, so "Join again" works without a fresh one while the host is up.
    pub secret: Option<String>,
    pub code: Option<String>,
    pub at: i64,
}

fn file(paths: &Paths) -> PathBuf {
    paths.root.join("friends.json")
}

pub fn list(paths: &Paths) -> Vec<Friend> {
    util::read_json(&file(paths)).unwrap_or_default()
}

fn same(a: &Friend, b: &Friend) -> bool {
    a.kind == b.kind
        && match (&a.user_id, &b.user_id) {
            (Some(x), Some(y)) => x == y,
            _ => a.secret.is_some() && a.secret == b.secret || a.code.is_some() && a.code == b.code,
        }
}

/// Adds or refreshes an entry; the newest comes first.
pub fn record(paths: &Paths, mut f: Friend) -> Result<Vec<Friend>> {
    let mut all = list(paths);
    all.retain(|x| !same(x, &f));
    if f.id.is_empty() {
        f.id = uuid::Uuid::new_v4().to_string();
    }
    f.at = util::now_ms();
    all.insert(0, f);
    all.truncate(KEEP);
    util::write_json_atomic(&file(paths), &all)?;
    Ok(all)
}

pub fn forget(paths: &Paths, id: &str) -> Result<Vec<Friend>> {
    let mut all = list(paths);
    all.retain(|x| x.id != id);
    util::write_json_atomic(&file(paths), &all)?;
    Ok(all)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_dedupes_and_forgets() {
        let root = std::env::temp_dir().join(format!("cairn-friends-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let paths = Paths::at(&root);
        let alex = Friend { kind: "joinedYou".into(), name: "Alex".into(), user_id: Some("1".into()), ..Default::default() };
        record(&paths, alex.clone()).unwrap();
        record(&paths, Friend { name: "Alex R".into(), ..alex.clone() }).unwrap();
        let joined = Friend { kind: "youJoined".into(), name: "SMP".into(), secret: Some("c1;x".into()), ..Default::default() };
        let all = record(&paths, joined).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].name, "SMP");
        assert_eq!(all[1].name, "Alex R");
        let left = forget(&paths, &all[0].id).unwrap();
        assert_eq!(left.len(), 1);
        assert_eq!(list(&paths), left);
        std::fs::remove_dir_all(&root).unwrap();
    }
}
