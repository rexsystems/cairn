use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

pub const ROLES: [&str; 4] = ["player", "staff", "admin", "banned"];

pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;
         CREATE TABLE IF NOT EXISTS users (
             uuid TEXT PRIMARY KEY,
             name TEXT NOT NULL,
             role TEXT NOT NULL DEFAULT 'player',
             cape TEXT,
             badge TEXT,
             custom_cape TEXT,
             first_seen INTEGER NOT NULL,
             last_seen INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS capes (
             hash TEXT PRIMARY KEY,
             owner TEXT NOT NULL,
             status TEXT NOT NULL,
             frames INTEGER NOT NULL,
             frame_ms INTEGER NOT NULL,
             width INTEGER NOT NULL,
             height INTEGER NOT NULL,
             created INTEGER NOT NULL,
             reviewed_at INTEGER
         );
         CREATE INDEX IF NOT EXISTS capes_status ON capes(status);
         CREATE TABLE IF NOT EXISTS sessions (
             token_hash TEXT PRIMARY KEY,
             uuid TEXT NOT NULL,
             expires INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS stats (
             key TEXT PRIMARY KEY,
             value INTEGER NOT NULL
         );",
    )?;
    Ok(conn)
}

#[derive(Clone, Debug)]
pub struct User {
    pub uuid: String,
    pub name: String,
    pub role: String,
    pub cape: Option<String>,
    pub badge: Option<String>,
    pub custom_cape: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapeRow {
    pub hash: String,
    pub owner: String,
    pub status: String,
    pub frames: u32,
    pub frame_ms: u32,
    pub width: u32,
    pub height: u32,
    pub created: i64,
}

pub fn user(conn: &Connection, uuid: &str) -> rusqlite::Result<Option<User>> {
    conn.query_row("SELECT uuid, name, role, cape, badge, custom_cape FROM users WHERE uuid = ?1", [uuid], |r| {
        Ok(User { uuid: r.get(0)?, name: r.get(1)?, role: r.get(2)?, cape: r.get(3)?, badge: r.get(4)?, custom_cape: r.get(5)? })
    })
    .optional()
}

pub fn upsert_user(conn: &Connection, uuid: &str, name: &str, now: i64) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO users (uuid, name, first_seen, last_seen) VALUES (?1, ?2, ?3, ?3)
         ON CONFLICT(uuid) DO UPDATE SET name = excluded.name, last_seen = excluded.last_seen",
        params![uuid, name, now],
    )?;
    Ok(())
}

pub fn set_role(conn: &Connection, uuid: &str, role: &str, now: i64) -> rusqlite::Result<()> {
    // Roles can be granted before the player ever signs in; the name fills in on first login.
    conn.execute(
        "INSERT INTO users (uuid, name, role, first_seen, last_seen) VALUES (?1, '', ?2, ?3, ?3)
         ON CONFLICT(uuid) DO UPDATE SET role = excluded.role",
        params![uuid, role, now],
    )?;
    if role == "banned" {
        conn.execute("DELETE FROM sessions WHERE uuid = ?1", [uuid])?;
    }
    Ok(())
}

pub fn cape(conn: &Connection, hash: &str) -> rusqlite::Result<Option<CapeRow>> {
    conn.query_row("SELECT hash, owner, status, frames, frame_ms, width, height, created FROM capes WHERE hash = ?1", [hash], cape_row).optional()
}

pub fn capes_with_status(conn: &Connection, status: &str, limit: u32) -> rusqlite::Result<Vec<CapeRow>> {
    let mut stmt = conn.prepare("SELECT hash, owner, status, frames, frame_ms, width, height, created FROM capes WHERE status = ?1 ORDER BY created LIMIT ?2")?;
    let rows = stmt.query_map(params![status, limit], cape_row)?;
    rows.collect()
}

fn cape_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<CapeRow> {
    Ok(CapeRow { hash: r.get(0)?, owner: r.get(1)?, status: r.get(2)?, frames: r.get(3)?, frame_ms: r.get(4)?, width: r.get(5)?, height: r.get(6)?, created: r.get(7)? })
}

pub fn set_cape_status(conn: &Connection, hash: &str, status: &str, now: i64) -> rusqlite::Result<bool> {
    let changed = conn.execute("UPDATE capes SET status = ?2, reviewed_at = ?3 WHERE hash = ?1", params![hash, status, now])?;
    if status == "rejected" {
        conn.execute("UPDATE users SET custom_cape = NULL, cape = CASE WHEN cape = 'custom' THEN NULL ELSE cape END WHERE custom_cape = ?1", [hash])?;
    }
    Ok(changed > 0)
}

pub fn stat(conn: &Connection, key: &str) -> rusqlite::Result<i64> {
    Ok(conn.query_row("SELECT value FROM stats WHERE key = ?1", [key], |r| r.get(0)).optional()?.unwrap_or(0))
}

pub fn set_stat(conn: &Connection, key: &str, value: i64) -> rusqlite::Result<()> {
    conn.execute("INSERT INTO stats (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value", params![key, value])?;
    Ok(())
}
