//! Host-side session state for invites: which server the game is on, and who has joined.
//! Kept in memory only, so a restart never leaves a session joinable.

use crate::util::LockExt;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Seen {
    /// host:port of the last multiplayer server the game connected to.
    pub server: Option<String>,
    /// Port of a world opened to LAN.
    pub lan_port: Option<u16>,
}

static SEEN: Lazy<Mutex<HashMap<String, Seen>>> = Lazy::new(Default::default);

static CONNECTING: Lazy<Regex> = Lazy::new(|| Regex::new(r"Connecting to ([A-Za-z0-9.\-]{1,253}), (\d{1,5})\s*$").unwrap());
static LAN_OPEN: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?:Started serving on|Started on|Local game hosted on port) (\d{1,5})\s*$").unwrap());
static LAN_STOP: Lazy<Regex> = Lazy::new(|| Regex::new(r"Stopping (?:singleplayer )?server\s*$").unwrap());
static JOINED: Lazy<Regex> = Lazy::new(|| Regex::new(r"\]: ([A-Za-z0-9_]{3,16}) joined the game\s*$").unwrap());
static LEFT: Lazy<Regex> = Lazy::new(|| Regex::new(r"\]: ([A-Za-z0-9_]{3,16}) left the game\s*$").unwrap());

/// Reads one game log line. Returns true when the session's address changed.
pub fn observe_game_line(id: &str, line: &str) -> bool {
    let update = |f: &dyn Fn(&mut Seen)| {
        let mut all = SEEN.locked();
        let seen = all.entry(id.to_string()).or_default();
        let before = seen.clone();
        f(seen);
        *seen != before
    };
    if let Some(c) = CONNECTING.captures(line) {
        let addr = format!("{}:{}", &c[1], &c[2]);
        return update(&|s| {
            s.server = Some(addr.clone());
            s.lan_port = None;
        });
    }
    if let Some(port) = LAN_OPEN.captures(line).and_then(|c| c[1].parse::<u16>().ok()).filter(|p| *p > 0) {
        return update(&|s| {
            s.lan_port = Some(port);
            s.server = None;
        });
    }
    if LAN_STOP.is_match(line) {
        return update(&|s| s.lan_port = None);
    }
    false
}

pub fn seen(id: &str) -> Seen {
    SEEN.locked().get(id).cloned().unwrap_or_default()
}

pub fn forget(id: &str) {
    SEEN.locked().remove(id);
}

/// Player count from a server console line, given the current count.
pub fn players_after(line: &str, online: u32) -> u32 {
    if JOINED.is_match(line) {
        online + 1
    } else if LEFT.is_match(line) {
        online.saturating_sub(1)
    } else {
        online
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn follows_servers_and_lan_worlds_from_the_game_log() {
        let id = "party-test";
        forget(id);
        assert!(observe_game_line(id, "[12:00:01] [Render thread/INFO]: Connecting to play.example.org, 25565"));
        assert_eq!(seen(id).server.as_deref(), Some("play.example.org:25565"));
        assert!(!observe_game_line(id, "[12:00:01] [Render thread/INFO]: Connecting to play.example.org, 25565"));
        assert!(observe_game_line(id, "[12:05:00] [Server thread/INFO]: Started serving on 51234"));
        assert_eq!(seen(id), Seen { server: None, lan_port: Some(51234) });
        assert!(observe_game_line(id, "[12:09:00] [Server thread/INFO]: Stopping server"));
        assert_eq!(seen(id).lan_port, None);
        assert!(!observe_game_line(id, "[12:09:00] [Render thread/INFO]: Connecting to evil host; rm, 1"));
        forget(id);
    }

    #[test]
    fn counts_players_from_console_lines() {
        let n = players_after("[12:00:00] [Server thread/INFO]: Steve joined the game", 0);
        assert_eq!(n, 1);
        assert_eq!(players_after("[12:00:00] [Server thread/INFO]: Steve left the game", n), 0);
        assert_eq!(players_after("[12:00:00] [Server thread/INFO]: Steve left the game", 0), 0);
        assert_eq!(players_after("[12:00:00] [Server thread/INFO]: <Steve> I joined the game", 3), 3);
    }
}
