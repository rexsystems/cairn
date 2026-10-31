//! Shares a log on mclo.gs, the paste site most Minecraft support channels ask for.
//! mclo.gs masks IP addresses, home folder names and access tokens on its side.

use anyhow::{bail, Context, Result};
use serde::Deserialize;

const API: &str = "https://api.mclo.gs/1/log";
/// mclo.gs keeps at most 25,000 lines; the end of a log is what people need.
const MAX_LINES: usize = 25_000;
/// And at most 10 MiB.
const MAX_BYTES: usize = 10 * 1024 * 1024;

#[derive(Deserialize)]
struct Reply {
    success: bool,
    url: Option<String>,
    error: Option<String>,
}

/// Keeps the last lines that fit mclo.gs limits.
pub fn tail(content: &str) -> &str {
    let lines: Vec<&str> = content.split_inclusive('\n').collect();
    let (mut kept, mut bytes) = (0, 0);
    for line in lines.iter().rev() {
        if kept == MAX_LINES || bytes + line.len() > MAX_BYTES {
            break;
        }
        kept += 1;
        bytes += line.len();
    }
    &content[content.len() - bytes..]
}

/// Uploads the log and returns its public URL.
pub async fn upload(content: &str) -> Result<String> {
    let body = tail(content);
    if body.trim().is_empty() {
        bail!("This log is empty.");
    }
    let reply: Reply = crate::net::CLIENT
        .post(API)
        .json(&serde_json::json!({ "content": body, "source": "Cairn" }))
        .send()
        .await
        .context("couldn't reach mclo.gs")?
        .json()
        .await
        .context("mclo.gs sent an unexpected reply")?;
    match (reply.success, reply.url) {
        (true, Some(url)) => Ok(url),
        _ => bail!("mclo.gs refused the log: {}", reply.error.unwrap_or_else(|| "no reason given".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_logs_are_kept_whole() {
        assert_eq!(tail("a\nb\nc"), "a\nb\nc");
    }

    #[test]
    fn long_logs_keep_the_end() {
        let log: String = (0..30_000).map(|i| format!("line {i}\n")).collect();
        let kept = tail(&log);
        assert_eq!(kept.lines().count(), MAX_LINES);
        assert!(kept.ends_with("line 29999\n"));
        assert!(kept.starts_with("line 5000\n"));
    }
}
