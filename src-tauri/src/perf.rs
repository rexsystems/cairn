//! Memory and JVM presets sized to the machine, and the performance mod pack for each loader.

use crate::deps::Root;
use crate::versioning::compare;
use serde::Serialize;
use std::cmp::Ordering;

/// Above this, more heap only makes garbage collection pauses longer.
const MAX_USEFUL_MB: u32 = 16384;
const MIN_MB: u32 = 1024;

#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryPreset {
    pub id: &'static str,
    pub label: &'static str,
    pub mb: u32,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryAdvice {
    pub total_mb: u32,
    /// The most Cairn suggests giving the game while leaving room for the OS and browser.
    pub max_safe_mb: u32,
    pub recommended_mb: u32,
    pub presets: Vec<MemoryPreset>,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct JvmPreset {
    pub id: &'static str,
    pub label: &'static str,
    pub detail: &'static str,
    pub args: &'static str,
}

/// Physical memory in MB, or 0 when the OS does not say.
pub fn system_memory_mb() -> u32 {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    u32::try_from(sys.total_memory() / (1024 * 1024)).unwrap_or(u32::MAX)
}

fn round_512(mb: u32) -> u32 {
    (mb + 256) / 512 * 512
}

/// Heap sizes for an instance on a machine with `total_mb` of RAM. `mods` is the number of
/// enabled mods; vanilla and Horizon instances pass `modded = false`.
pub fn advise(total_mb: u32, modded: bool, mods: usize) -> MemoryAdvice {
    let total = if total_mb == 0 { 8192 } else { total_mb };
    let reserve = (total / 4).clamp(2048, 8192);
    let max_safe = total.saturating_sub(reserve).clamp(MIN_MB, MAX_USEFUL_MB);
    let want: u32 = match (modded, mods) {
        (false, _) => 3072,
        (true, 0..=60) => 4096,
        (true, 61..=180) => 6144,
        (true, _) => 8192,
    };
    let fit = |mb: u32| round_512(mb).clamp(MIN_MB, max_safe);
    let presets = vec![
        MemoryPreset { id: "light", label: "Light", mb: fit(want * 2 / 3) },
        MemoryPreset { id: "balanced", label: "Balanced", mb: fit(want) },
        MemoryPreset { id: "heavy", label: "Heavy", mb: fit(want * 3 / 2) },
    ];
    MemoryAdvice { total_mb, max_safe_mb: max_safe, recommended_mb: fit(want), presets }
}

/// Garbage collector tuning based on the widely used Aikar flags, minus `AlwaysPreTouch`,
/// which makes a client start slowly with a large heap.
pub const TUNED_G1: &str = "-XX:+UseG1GC -XX:+ParallelRefProcEnabled -XX:MaxGCPauseMillis=200 -XX:+UnlockExperimentalVMOptions -XX:+DisableExplicitGC -XX:G1NewSizePercent=30 -XX:G1MaxNewSizePercent=40 -XX:G1HeapRegionSize=8M -XX:G1ReservePercent=20 -XX:G1HeapWastePercent=5 -XX:G1MixedGCCountTarget=4 -XX:InitiatingHeapOccupancyPercent=15 -XX:G1MixedGCLiveThresholdPercent=90 -XX:G1RSetUpdatingPauseTimePercent=5 -XX:SurvivorRatio=32 -XX:+PerfDisableSharedMem -XX:MaxTenuringThreshold=1";

pub fn jvm_presets() -> Vec<JvmPreset> {
    vec![
        JvmPreset { id: "default", label: "Java defaults", detail: "No extra flags. Safest choice.", args: "" },
        JvmPreset { id: "smooth", label: "Smooth (tuned G1)", detail: "Shorter garbage collection pauses, fewer stutters with big modpacks.", args: TUNED_G1 },
    ]
}

/// Which preset an argument string matches, so the UI can highlight it.
pub fn jvm_preset_of(args: &str) -> Option<&'static str> {
    let words: Vec<&str> = args.split_whitespace().collect();
    jvm_presets().into_iter().find(|p| p.args.split_whitespace().collect::<Vec<_>>() == words).map(|p| p.id)
}

fn at_least(mc: &str, v: &str) -> bool {
    compare(mc, v) != Ordering::Less
}

/// Modrinth slugs of the performance pack for a loader and Minecraft version. Every root is
/// soft: mods without a build for the instance, or already installed, are skipped.
pub fn pack(loader: &str, mc: &str) -> Vec<Root> {
    let slugs: Vec<&str> = match loader {
        "fabric" | "quilt" => vec!["sodium", "lithium", "ferrite-core", "modernfix", "entityculling", "immediatelyfast", "dynamic-fps"],
        // Sodium ships NeoForge builds from 1.21; older versions use the Embeddium port.
        "neoforge" if at_least(mc, "1.21") => vec!["sodium", "ferrite-core", "modernfix", "entityculling", "immediatelyfast"],
        "neoforge" | "forge" => vec!["embeddium", "ferrite-core", "modernfix", "entityculling", "immediatelyfast"],
        _ => vec![],
    };
    slugs.into_iter().map(|s| Root { project_id: s.into(), version_id: None, soft: true }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(r: &[Root]) -> Vec<&str> {
        r.iter().map(|r| r.project_id.as_str()).collect()
    }

    #[test]
    fn leaves_room_for_the_os() {
        let a = advise(8192, true, 100);
        assert_eq!(a.max_safe_mb, 6144);
        assert_eq!(a.recommended_mb, 6144);
        assert!(a.presets.iter().all(|p| p.mb <= 6144 && p.mb >= MIN_MB));
    }

    #[test]
    fn small_machines_still_get_a_usable_heap() {
        let a = advise(4096, true, 300);
        assert_eq!(a.max_safe_mb, 2048);
        assert_eq!(a.presets.iter().map(|p| p.mb).collect::<Vec<_>>(), vec![2048, 2048, 2048]);
        assert_eq!(advise(2048, false, 0).max_safe_mb, MIN_MB);
    }

    #[test]
    fn big_machines_cap_at_useful_size() {
        let a = advise(65536, true, 400);
        assert_eq!(a.max_safe_mb, MAX_USEFUL_MB);
        assert_eq!(a.recommended_mb, 8192);
        assert_eq!(a.presets[2].mb, 12288);
    }

    #[test]
    fn vanilla_needs_less_than_modded() {
        let v = advise(16384, false, 0);
        let m = advise(16384, true, 30);
        assert_eq!(v.recommended_mb, 3072);
        assert_eq!(m.recommended_mb, 4096);
        assert!(v.presets.iter().all(|p| p.mb % 512 == 0));
    }

    #[test]
    fn unknown_total_assumes_eight_gigabytes() {
        let a = advise(0, true, 10);
        assert_eq!(a.total_mb, 0);
        assert_eq!(a.max_safe_mb, 6144);
    }

    #[test]
    fn pack_matches_the_loader() {
        assert!(ids(&pack("fabric", "1.21.1")).contains(&"sodium"));
        assert!(ids(&pack("quilt", "1.20.1")).contains(&"lithium"));
        assert_eq!(ids(&pack("neoforge", "1.21.1"))[0], "sodium");
        assert_eq!(ids(&pack("neoforge", "1.20.1"))[0], "embeddium");
        assert_eq!(ids(&pack("forge", "1.20.1"))[0], "embeddium");
        assert!(!ids(&pack("forge", "1.20.1")).contains(&"sodium"));
        assert!(pack("vanilla", "1.21.1").is_empty());
        assert!(pack("fabric", "1.21.1").iter().all(|r| r.soft));
    }

    #[test]
    fn recognises_presets() {
        assert_eq!(jvm_preset_of(""), Some("default"));
        assert_eq!(jvm_preset_of(&format!("  {TUNED_G1} ")), Some("smooth"));
        assert_eq!(jvm_preset_of("-XX:+UseZGC"), None);
    }
}
