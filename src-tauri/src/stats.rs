//! Play sessions per instance (`.cairn/sessions.json`): playtime, history and crashes.

use crate::util;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const KEEP: usize = 300;
const DAY_MS: i64 = 86_400_000;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Session {
    pub started: i64,
    pub ended: i64,
    pub seconds: u64,
    pub exit_code: Option<i32>,
    pub crashed: bool,
    /// Why it crashed, when the log said.
    pub reason: Option<String>,
    /// Log or crash report to diagnose, relative to the instance folder.
    pub log: Option<String>,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    /// Total playtime, including play from before sessions were recorded.
    pub total_seconds: u64,
    pub sessions: usize,
    pub crashes: usize,
    pub longest_seconds: u64,
    pub week_seconds: u64,
    /// Seconds played on each of the last 14 days, oldest first.
    pub days: Vec<u64>,
    /// Newest first.
    pub recent: Vec<Session>,
}

fn file(dir: &Path) -> PathBuf {
    dir.join(".cairn").join("sessions.json")
}

pub fn load(dir: &Path) -> Vec<Session> {
    util::read_json(&file(dir)).unwrap_or_default()
}

pub fn record(dir: &Path, s: Session) -> anyhow::Result<()> {
    let mut all = load(dir);
    all.push(s);
    if all.len() > KEEP {
        all.drain(..all.len() - KEEP);
    }
    std::fs::create_dir_all(dir.join(".cairn"))?;
    util::write_json_atomic(&file(dir), &all)
}

/// Whether an exit counts as a crash. Stopping the game from Cairn never does.
pub fn is_crash(exit_code: Option<i32>, stopped_by_user: bool, new_crash_report: bool) -> bool {
    if stopped_by_user {
        return false;
    }
    new_crash_report || exit_code != Some(0)
}

/// `local_midnight_ms` is the start of today in local time, so days line up with the user's calendar.
pub fn summarize(all: &[Session], total_seconds: u64, now_ms: i64, local_midnight_ms: i64) -> Stats {
    let mut days = vec![0u64; 14];
    for s in all {
        if s.started >= local_midnight_ms {
            days[13] += s.seconds;
        } else {
            let back = ((local_midnight_ms - s.started - 1) / DAY_MS + 1) as usize;
            if back < 14 {
                days[13 - back] += s.seconds;
            }
        }
    }
    Stats {
        total_seconds,
        sessions: all.len(),
        crashes: all.iter().filter(|s| s.crashed).count(),
        longest_seconds: all.iter().map(|s| s.seconds).max().unwrap_or(0),
        week_seconds: all.iter().filter(|s| s.started > now_ms - 7 * DAY_MS).map(|s| s.seconds).sum(),
        days,
        recent: all.iter().rev().take(40).cloned().collect(),
    }
}

/// Newest crash report written at or after `since_ms`, relative to the instance folder.
pub fn crash_report_since(dir: &Path, since_ms: i64) -> Option<String> {
    let rd = std::fs::read_dir(dir.join("crash-reports")).ok()?;
    rd.flatten()
        .filter(|e| e.file_name().to_string_lossy().ends_with(".txt"))
        .map(|e| (util::mtime_ms(&e.path()), e.file_name().to_string_lossy().to_string()))
        .filter(|(t, _)| *t >= since_ms)
        .max()
        .map(|(_, n)| format!("crash-reports/{n}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(started: i64, seconds: u64, crashed: bool) -> Session {
        Session { started, ended: started + seconds as i64 * 1000, seconds, crashed, ..Default::default() }
    }

    #[test]
    fn crashes_exclude_user_stops() {
        assert!(!is_crash(Some(0), false, false));
        assert!(is_crash(Some(1), false, false));
        assert!(is_crash(None, false, false));
        assert!(is_crash(Some(0), false, true));
        assert!(!is_crash(Some(137), true, false));
    }

    #[test]
    fn summary_buckets_days() {
        let midnight = 100 * DAY_MS;
        let now = midnight + 3_600_000;
        let all = [s(midnight + 1000, 600, false), s(midnight - 1000, 300, true), s(midnight - 3 * DAY_MS + 5, 120, false), s(midnight - 30 * DAY_MS, 50, false)];
        let st = summarize(&all, 2000, now, midnight);
        assert_eq!(st.days[13], 600);
        assert_eq!(st.days[12], 300);
        assert_eq!(st.days[10], 120);
        assert_eq!(st.days.iter().sum::<u64>(), 1020);
        assert_eq!(st.week_seconds, 1020);
        assert_eq!((st.sessions, st.crashes, st.longest_seconds, st.total_seconds), (4, 1, 600, 2000));
        assert_eq!(st.recent[0], all[3]);
    }

    #[test]
    fn history_is_capped_and_persisted() {
        let dir = std::env::temp_dir().join(format!("cairn-stats-{}", uuid::Uuid::new_v4()));
        for i in 0..(KEEP as i64 + 5) {
            record(&dir, s(i, 1, false)).unwrap();
        }
        let all = load(&dir);
        assert_eq!(all.len(), KEEP);
        assert_eq!(all[0].started, 5);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn finds_new_crash_reports() {
        let dir = std::env::temp_dir().join(format!("cairn-crash-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(dir.join("crash-reports")).unwrap();
        assert_eq!(crash_report_since(&dir, 0), None);
        std::fs::write(dir.join("crash-reports/crash-2026-10-31_20.00.00-client.txt"), "x").unwrap();
        assert_eq!(crash_report_since(&dir, 0).as_deref(), Some("crash-reports/crash-2026-10-31_20.00.00-client.txt"));
        assert_eq!(crash_report_since(&dir, util::now_ms() + 60_000), None);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
