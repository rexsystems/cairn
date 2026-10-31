//! GlitchTip (Sentry protocol) error reporting, gated on the user's privacy choice.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

const DSN: &str = "https://bad5c2806df24b6d846b5ddff0effd62@error.mythical.systems/6";

static ALLOWED: AtomicBool = AtomicBool::new(false);

/// Called whenever settings are saved so a change applies without a restart.
pub fn set_allowed(allowed: bool) {
    ALLOWED.store(allowed, Ordering::Relaxed);
}

pub fn allowed() -> bool {
    ALLOWED.load(Ordering::Relaxed)
}

/// Install the client; nothing leaves the machine while reports are not allowed.
/// Keep the guard alive for the whole process so queued events flush on exit.
pub fn init(settings_file: &Path) -> sentry::ClientInitGuard {
    let settings = crate::settings::Settings::load(settings_file);
    set_allowed(settings.crash_reports_allowed());
    let dsn = std::env::var("CAIRN_SENTRY_DSN")
        .ok()
        .and_then(|v| v.parse().ok())
        .or_else(|| DSN.parse().ok());
    let mut options = sentry::ClientOptions::new()
        .release(format!("{}@{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")))
        .traces_sample_rate(0.0)
        .before_send(|event| if allowed() { Some(event) } else { None });
    options.dsn = dsn;
    sentry::init(options)
}
