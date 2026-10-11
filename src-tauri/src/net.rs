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
            Ok(r) if r.status().is_success() => return Ok(r.json::<T>().await.with_context(|| format!("decoding {url}"))?),
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

#[derive(Clone, Debug)]
pub struct Download {
    pub url: String,
    pub dest: PathBuf,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

fn already_ok(d: &Download) -> bool {
    let Ok(meta) = std::fs::metadata(&d.dest) else { return false };
    if let Some(s) = d.size {
        if meta.len() != s {
            return false;
        }
    }
    match &d.sha1 {
        Some(h) => crate::util::sha1_file(&d.dest).map(|x| x.eq_ignore_ascii_case(h)).unwrap_or(false),
        None => meta.len() > 0,
    }
}

pub async fn download(d: &Download) -> Result<()> {
    if already_ok(d) {
        return Ok(());
    }
    let mut last = None;
    for attempt in 0..4 {
        match download_once(d).await {
            Ok(()) => return Ok(()),
            Err(e) => last = Some(e),
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
    let mut hasher = sha1::Sha1::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
    }
    file.flush().await?;
    drop(file);
    if let Some(expected) = &d.sha1 {
        let got = hex::encode(hasher.finalize());
        if !got.eq_ignore_ascii_case(expected) {
            let _ = tokio::fs::remove_file(&tmp).await;
            bail!("checksum mismatch for {} (expected {expected}, got {got})", d.url);
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
