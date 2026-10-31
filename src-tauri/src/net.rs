use anyhow::{anyhow, bail, Context, Result};
use futures::StreamExt;
use once_cell::sync::Lazy;
use serde::de::DeserializeOwned;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncWriteExt;

pub const USER_AGENT: &str = concat!("cairn-launcher/", env!("CARGO_PKG_VERSION"), " (local build)");

pub static CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(Duration::from_secs(20))
        .read_timeout(Duration::from_secs(60))
        .build()
        .expect("http client")
});

pub async fn get_json<T: DeserializeOwned>(url: &str) -> Result<T> {
    let mut last = None;
    for attempt in 0..3 {
        match CLIENT.get(url).send().await {
            Ok(r) if r.status().is_success() => return r.json::<T>().await.with_context(|| format!("decoding {url}")),
            Ok(r) if r.status().is_client_error() => bail!("{} returned {}", url, r.status()),
            Ok(r) => last = Some(anyhow!("{} returned {}", url, r.status())),
            Err(e) => last = Some(anyhow!(e).context(format!("requesting {url}"))),
        }
        tokio::time::sleep(Duration::from_millis(400 * (attempt + 1))).await;
    }
    Err(last.unwrap())
}

pub async fn get_text(url: &str) -> Result<String> {
    let r = CLIENT.get(url).send().await?;
    if !r.status().is_success() {
        bail!("{} returned {}", url, r.status());
    }
    Ok(r.text().await?)
}

#[derive(Clone, Debug, Default)]
pub struct Download {
    pub url: String,
    pub dest: PathBuf,
    pub sha1: Option<String>,
    /// Checked in addition to `sha1` when the source publishes it (Adoptium, Maven, GitHub).
    pub sha256: Option<String>,
    pub size: Option<u64>,
}

fn already_ok(d: &Download) -> bool {
    let Ok(meta) = std::fs::metadata(&d.dest) else { return false };
    if let Some(s) = d.size {
        if meta.len() != s {
            return false;
        }
    }
    if meta.len() == 0 {
        return false;
    }
    if let Some(h) = &d.sha256 {
        if !crate::util::sha256_file(&d.dest).map(|x| x.eq_ignore_ascii_case(h)).unwrap_or(false) {
            return false;
        }
    }
    match &d.sha1 {
        Some(h) => crate::util::sha1_file(&d.dest).map(|x| x.eq_ignore_ascii_case(h)).unwrap_or(false),
        None => true,
    }
}

/// A file arrived but its hash does not match what the source published.
#[derive(Debug)]
pub struct ChecksumMismatch {
    pub url: String,
    pub algo: &'static str,
    pub expected: String,
    pub got: String,
}

impl std::fmt::Display for ChecksumMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} checksum mismatch for {} (expected {}, got {}). The file was deleted; try again, and report it if it keeps happening.",
            self.algo, self.url, self.expected, self.got
        )
    }
}

impl std::error::Error for ChecksumMismatch {}

/// Pull the hex out of a published checksum file or GitHub digest ("sha256:<hex>", "<hex>  name").
pub fn parse_checksum(text: &str, hex_len: usize) -> Option<String> {
    let token = text.split_whitespace().next()?;
    let token = token.rsplit(':').next()?;
    (token.len() == hex_len && token.bytes().all(|b| b.is_ascii_hexdigit())).then(|| token.to_ascii_lowercase())
}

pub async fn download(d: &Download) -> Result<()> {
    if already_ok(d) {
        return Ok(());
    }
    let mut last = None;
    for attempt in 0..4 {
        match download_once(d).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                // A second mismatch means the source itself is wrong; retrying will not help.
                let mismatch = e.downcast_ref::<ChecksumMismatch>().is_some();
                last = Some(e);
                if mismatch && attempt >= 1 {
                    break;
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(500 * (attempt + 1))).await;
    }
    Err(last.unwrap())
}

async fn download_once(d: &Download) -> Result<()> {
    if let Some(p) = d.dest.parent() {
        tokio::fs::create_dir_all(p).await?;
    }
    let resp = CLIENT.get(&d.url).send().await.with_context(|| format!("downloading {}", d.url))?;
    if !resp.status().is_success() {
        bail!("{} returned {}", d.url, resp.status());
    }
    let tmp = part_path(&d.dest);
    let mut file = tokio::fs::File::create(&tmp).await?;
    let mut stream = resp.bytes_stream();
    use sha1::Digest;
    let mut h1 = sha1::Sha1::new();
    let mut h256 = sha2::Sha256::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        h1.update(&chunk);
        h256.update(&chunk);
        file.write_all(&chunk).await?;
    }
    file.flush().await?;
    drop(file);
    let checks = [("SHA-1", &d.sha1, hex::encode(h1.finalize())), ("SHA-256", &d.sha256, hex::encode(h256.finalize()))];
    for (algo, expected, got) in checks {
        if let Some(expected) = expected {
            if !got.eq_ignore_ascii_case(expected) {
                let _ = tokio::fs::remove_file(&tmp).await;
                return Err(ChecksumMismatch { url: d.url.clone(), algo, expected: expected.clone(), got }.into());
            }
        }
    }
    tokio::fs::rename(&tmp, &d.dest).await?;
    Ok(())
}

fn part_path(p: &Path) -> PathBuf {
    let mut s = p.as_os_str().to_owned();
    s.push(".part");
    PathBuf::from(s)
}

/// Downloads many files with bounded concurrency. `on_progress(done, total)` fires after each file.
pub async fn download_all<F>(items: Vec<Download>, concurrency: usize, on_progress: F) -> Result<()>
where
    F: Fn(u64, u64) + Send + Sync + 'static,
{
    let total = items.len() as u64;
    let done = Arc::new(AtomicU64::new(0));
    let cb = Arc::new(on_progress);
    let results: Vec<Result<()>> = futures::stream::iter(items.into_iter().map(|d| {
        let done = done.clone();
        let cb = cb.clone();
        async move {
            let r = download(&d).await;
            let n = done.fetch_add(1, Ordering::SeqCst) + 1;
            cb(n, total);
            r
        }
    }))
    .buffer_unordered(concurrency.max(1))
    .collect()
    .await;
    let errors: Vec<String> = results.into_iter().filter_map(|r| r.err().map(|e| format!("{e:#}"))).collect();
    if !errors.is_empty() {
        bail!("{} download(s) failed. First error: {}", errors.len(), errors[0]);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_checksum;

    #[test]
    fn parses_published_checksums() {
        let hex = "6018abce95cc058768e365865fd7cf3bd31b5c553231daf885dc385fcd6d875a";
        assert_eq!(parse_checksum(hex, 64).as_deref(), Some(hex));
        assert_eq!(parse_checksum(&format!("sha256:{hex}"), 64).as_deref(), Some(hex));
        assert_eq!(parse_checksum(&format!("{}  forge.jar\n", hex.to_uppercase()), 64).as_deref(), Some(hex));
        assert_eq!(parse_checksum("0deb8e547a9e3f098a4c9f082903499703c52f5d", 64), None);
        assert_eq!(parse_checksum("<html>404</html>", 40), None);
        assert_eq!(parse_checksum("", 40), None);
    }
}
