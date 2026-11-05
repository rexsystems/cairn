use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Moderation {
    /// Custom capes stay hidden from other players until an admin approves them.
    Approval,
    /// Custom capes show right away; admins can still reject them later.
    Instant,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub bind: SocketAddr,
    pub data_dir: PathBuf,
    pub admin_token: Option<String>,
    pub cors_origins: Vec<String>,
    pub moderation: Moderation,
    pub trust_proxy: bool,
    pub session_server: String,
    pub dev_insecure_auth: bool,
    pub token_ttl_secs: i64,
    pub online_window_secs: u64,
    pub rate_per_minute: u32,
    /// Uploads allowed back to back per IP before the one-per-minute refill applies.
    pub upload_burst: u32,
}

impl Config {
    pub fn for_tests(data_dir: PathBuf) -> Self {
        Self {
            bind: "127.0.0.1:0".parse().unwrap(),
            data_dir,
            admin_token: None,
            cors_origins: Vec::new(),
            moderation: Moderation::Approval,
            trust_proxy: false,
            session_server: "http://127.0.0.1:9".into(),
            dev_insecure_auth: false,
            token_ttl_secs: 3600,
            online_window_secs: 90,
            rate_per_minute: 1_000_000,
            upload_burst: 1_000,
        }
    }

    pub fn from_env() -> Result<Self, String> {
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<Self, String> {
        let get = |key: &str| get(key).map(|v| v.trim().to_owned()).filter(|v| !v.is_empty());
        let bind: SocketAddr = get("COSMETICS_BIND").unwrap_or_else(|| "127.0.0.1:9437".into()).parse().map_err(|e| format!("COSMETICS_BIND: {e}"))?;
        let admin_token = get("COSMETICS_ADMIN_TOKEN");
        if let Some(token) = &admin_token {
            if token.len() < 32 {
                return Err("COSMETICS_ADMIN_TOKEN must be at least 32 characters (try `openssl rand -hex 32`).".into());
            }
        }
        let moderation = match get("COSMETICS_CAPE_MODERATION").as_deref() {
            None | Some("approval") => Moderation::Approval,
            Some("instant") => Moderation::Instant,
            Some(other) => return Err(format!("COSMETICS_CAPE_MODERATION must be `approval` or `instant`, not `{other}`")),
        };
        let flag = |key: &str| matches!(get(key).as_deref(), Some("1" | "true" | "yes"));
        let dev_insecure_auth = flag("COSMETICS_DEV_INSECURE_AUTH");
        // The mock trusts any name, so it must never be reachable from another machine.
        if dev_insecure_auth && !bind.ip().is_loopback() {
            return Err("COSMETICS_DEV_INSECURE_AUTH only works with a loopback COSMETICS_BIND.".into());
        }
        let number = |key: &str, default: u64| -> Result<u64, String> { get(key).map(|v| v.parse::<u64>().map_err(|e| format!("{key}: {e}"))).unwrap_or(Ok(default)) };
        let session_server = get("COSMETICS_SESSION_SERVER").unwrap_or_else(|| "https://sessionserver.mojang.com".into());
        if !session_server.starts_with("https://") && !session_server.starts_with("http://127.0.0.1") && !session_server.starts_with("http://localhost") {
            return Err("COSMETICS_SESSION_SERVER must use https.".into());
        }
        Ok(Self {
            bind,
            data_dir: get("COSMETICS_DATA_DIR").unwrap_or_else(|| "data".into()).into(),
            admin_token,
            cors_origins: get("COSMETICS_CORS_ORIGINS")
                .map(|v| v.split(',').map(|o| o.trim().trim_end_matches('/').to_owned()).filter(|o| !o.is_empty()).collect())
                .unwrap_or_default(),
            moderation,
            trust_proxy: flag("COSMETICS_TRUST_PROXY"),
            session_server: session_server.trim_end_matches('/').to_owned(),
            dev_insecure_auth,
            token_ttl_secs: number("COSMETICS_TOKEN_TTL_SECS", 3600)?.clamp(300, 86_400) as i64,
            online_window_secs: 90,
            rate_per_minute: number("COSMETICS_RATE_PER_MINUTE", 120)?.clamp(10, 100_000) as u32,
            upload_burst: 5,
        })
    }
}
