use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::Instant;

/// Token buckets per client and route class. Small and in memory: one instance
/// serves one community, and a restart simply forgets the counters.
type Buckets = HashMap<(IpAddr, &'static str), (f64, Instant)>;

pub struct RateLimiter {
    buckets: Mutex<Buckets>,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self { buckets: Mutex::new(HashMap::new()) }
    }
}

impl RateLimiter {
    /// Allows `burst` requests at once, refilled at `per_minute`.
    pub fn check(&self, ip: IpAddr, class: &'static str, per_minute: u32, burst: u32) -> bool {
        let now = Instant::now();
        let mut buckets = self.buckets.lock().unwrap_or_else(|e| e.into_inner());
        if buckets.len() > 100_000 {
            buckets.retain(|_, (_, at)| now.duration_since(*at).as_secs() < 600);
        }
        let (tokens, at) = buckets.entry((ip, class)).or_insert((burst as f64, now));
        *tokens = (*tokens + now.duration_since(*at).as_secs_f64() * per_minute as f64 / 60.0).min(burst as f64);
        *at = now;
        if *tokens < 1.0 {
            return false;
        }
        *tokens -= 1.0;
        true
    }
}
