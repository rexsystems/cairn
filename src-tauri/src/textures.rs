//! Skin and cape images from textures.minecraft.net, cached on disk and handed to the UI as data URLs.
//! Mojang serves them over plain http without CORS, which the webview's CSP and WebGL both refuse.

use crate::paths::Paths;
use anyhow::{bail, Context, Result};
use base64::Engine;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

const HOST: &str = "textures.minecraft.net/texture/";
const MAX_BYTES: usize = 1 << 20;
const PNG_MAGIC: &[u8] = b"\x89PNG\r\n\x1a\n";

/// The texture hash when `url` is a Mojang texture link, http or https; anything else is refused.
pub fn texture_id(url: &str) -> Option<&str> {
    let rest = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://"))?;
    let id = rest.strip_prefix(HOST)?;
    (!id.is_empty() && id.len() <= 128 && id.bytes().all(|b| b.is_ascii_alphanumeric())).then_some(id)
}

fn cache_file(paths: &Paths, id: &str) -> PathBuf {
    paths.cache.join("textures").join(format!("{id}.png"))
}

fn as_data_url(bytes: &[u8]) -> Result<String> {
    if bytes.len() > MAX_BYTES || !bytes.starts_with(PNG_MAGIC) {
        bail!("not a skin texture");
    }
    Ok(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
}

/// Texture hashes are content addresses, so a cached file never goes stale.
pub async fn data_url(paths: &Paths, url: &str) -> Result<String> {
    let id = texture_id(url).context("not a Minecraft texture URL")?;
    let file = cache_file(paths, id);
    if let Ok(bytes) = std::fs::read(&file) {
        if let Ok(data) = as_data_url(&bytes) {
            return Ok(data);
        }
    }
    let r = crate::net::CLIENT.get(format!("https://{HOST}{id}")).send().await?;
    if !r.status().is_success() {
        bail!("texture download returned {}", r.status());
    }
    let bytes = r.bytes().await?;
    let data = as_data_url(&bytes)?;
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let tmp = file.with_extension(format!("{}.part", NEXT.fetch_add(1, Ordering::Relaxed)));
    std::fs::write(&tmp, &bytes)?;
    std::fs::rename(&tmp, &file)?;
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_mojang_texture_links_are_fetched() {
        let id = "1a4af718455d4aab528e7a61f86fa25e6a369d1768dcb13f7df319a713eb810b";
        assert_eq!(texture_id(&format!("http://textures.minecraft.net/texture/{id}")), Some(id));
        assert_eq!(texture_id(&format!("https://textures.minecraft.net/texture/{id}")), Some(id));
        for bad in [
            "https://textures.minecraft.net/texture/",
            "https://textures.minecraft.net/texture/../../etc",
            "https://textures.minecraft.net/texture/abc?x=1",
            "https://textures.minecraft.net.evil.example/texture/abc",
            "https://evil.example/textures.minecraft.net/texture/abc",
            "file:///textures.minecraft.net/texture/abc",
            "C:\\skins\\me.png",
        ] {
            assert_eq!(texture_id(bad), None, "{bad}");
        }
    }

    #[test]
    fn only_pngs_become_data_urls() {
        let mut png = PNG_MAGIC.to_vec();
        png.extend_from_slice(b"rest");
        assert!(as_data_url(&png).unwrap().starts_with("data:image/png;base64,iVBORw0KG"));
        assert!(as_data_url(b"<html>").is_err());
        let mut huge = PNG_MAGIC.to_vec();
        huge.resize(MAX_BYTES + 1, 0);
        assert!(as_data_url(&huge).is_err());
    }

    #[tokio::test]
    async fn cached_textures_load_without_the_network() {
        let root = std::env::temp_dir().join(format!("cairn-textures-{}", std::process::id()));
        let paths = Paths::at(&root);
        let file = cache_file(&paths, "abc123");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, [PNG_MAGIC, b"x"].concat()).unwrap();
        let data = data_url(&paths, "http://textures.minecraft.net/texture/abc123").await.unwrap();
        assert!(data.starts_with("data:image/png;base64,"));
        assert!(data_url(&paths, "https://example.org/texture/abc123").await.is_err());
        std::fs::remove_dir_all(&root).unwrap();
    }
}
