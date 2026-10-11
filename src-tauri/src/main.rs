#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(target_os = "linux")]
    {
        // WebKitGTK Error 71 on NVIDIA Wayland — dmabuf off is enough.
        // Stripped from the game process via isolate_game_env.
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
    cairn_lib::run()
}
