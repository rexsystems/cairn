use anyhow::{Context, Result};
use serde::Serialize;
use sha1::{Digest, Sha1};
use std::io::{Read, Write};
use std::path::Path;

pub fn write_json_atomic<T: Serialize + ?Sized>(path: &Path, value: &T) -> Result<()> {
    write_bytes_atomic(path, &serde_json::to_vec_pretty(value)?)
}

pub fn write_bytes_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    let tmp = path.with_extension(format!("{}.tmp-write", uuid::Uuid::new_v4()));
    let result = (|| -> Result<()> {
        let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&tmp)?;
        file.write_all(bytes)?;
        drop(file);
        std::fs::rename(&tmp, path).with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    })();
    if result.is_err() { let _ = std::fs::remove_file(&tmp); }
    result
}

pub fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let s = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_slice(&s).with_context(|| format!("parsing {}", path.display()))
}

pub fn sha256_file(path: &Path) -> Result<String> {
    let mut f = std::fs::File::open(path)?;
    let mut h = sha2::Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(hex::encode(h.finalize()))
}

pub fn sha1_file(path: &Path) -> Result<String> {
    let mut f = std::fs::File::open(path)?;
    let mut h = Sha1::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(hex::encode(h.finalize()))
}

pub fn sha512_file(path: &Path) -> Result<String> {
    use sha2::Sha512;
    let mut f = std::fs::File::open(path)?;
    let mut h = Sha512::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(hex::encode(h.finalize()))
}

pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

pub fn mtime_ms(path: &Path) -> i64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn dir_size(path: &Path) -> u64 {
    let mut total = 0;
    if let Ok(rd) = std::fs::read_dir(path) {
        for e in rd.flatten() {
            match e.file_type() {
                Ok(t) if t.is_dir() => total += dir_size(&e.path()),
                Ok(_) => total += e.metadata().map(|m| m.len()).unwrap_or(0),
                _ => {}
            }
        }
    }
    total
}

pub fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for e in std::fs::read_dir(src)? {
        let e = e?;
        let to = dst.join(e.file_name());
        if e.file_type()?.is_dir() {
            copy_dir(&e.path(), &to)?;
        } else {
            std::fs::copy(e.path(), to)?;
        }
    }
    Ok(())
}

pub fn slugify(name: &str) -> String {
    let mut s: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect();
    while s.contains("--") {
        s = s.replace("--", "-");
    }
    let s = s.trim_matches('-').to_string();
    if s.is_empty() { "instance".into() } else { s.chars().take(40).collect() }
}

/// Keep console-subsystem children (java.exe, etc.) from opening a terminal window.
#[cfg(windows)]
pub fn hide_console(cmd: &mut tokio::process::Command) {
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
}

#[cfg(not(windows))]
pub fn hide_console(_: &mut tokio::process::Command) {}

#[cfg(windows)]
pub fn hide_console_std(cmd: &mut std::process::Command) {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(0x0800_0000);
}

#[cfg(not(windows))]
pub fn hide_console_std(_: &mut std::process::Command) {}

/// `Mutex::lock` that survives poisoning. The guarded data here is plain maps and
/// flags, so a panic in another thread should not take the whole launcher down.
pub trait LockExt<T> {
    fn locked(&self) -> std::sync::MutexGuard<'_, T>;
}

impl<T> LockExt<T> for std::sync::Mutex<T> {
    fn locked(&self) -> std::sync::MutexGuard<'_, T> {
        self.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn concurrent_atomic_writes_do_not_share_temporary_files() {
        let dir = std::env::temp_dir().join(format!("cairn-atomic-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("profile.json");
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
        let writers: Vec<_> = (0..8).map(|n| {
            let target = target.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let body = vec![n; 4096];
                barrier.wait();
                for _ in 0..40 { write_bytes_atomic(&target, &body).unwrap(); }
            })
        }).collect();
        for writer in writers { writer.join().unwrap(); }
        let body = std::fs::read(&target).unwrap();
        assert_eq!(body.len(), 4096);
        assert!(body.iter().all(|b| *b == body[0]));
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn slug() {
        assert_eq!(slugify("My Cool Pack!! 1.21"), "my-cool-pack-1-21");
        assert_eq!(slugify("???"), "instance");
    }
}
