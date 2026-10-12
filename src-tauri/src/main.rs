#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(target_os = "linux")]
    {
        // WebKitGTK Error 71 on NVIDIA Wayland — dmabuf off is enough.
        // Stripped from the game process via isolate_game_env.
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    // Keep _guard alive for the whole process so events flush on exit.
    // sentry 0.49: ClientOptions is non-exhaustive — use the builder.
    let _guard = sentry::init(
        sentry::ClientOptions::new()
            .dsn("https://bad5c2806df24b6d846b5ddff0effd62@error.mythical.systems/6")
            .release(format!(
                "{}@{}",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            ))
            .traces_sample_rate(0.01),
    );

    #[cfg(debug_assertions)]
    sentry::capture_message("Test GlitchTip error", sentry::Level::Error);

    cairn_lib::run()
}
