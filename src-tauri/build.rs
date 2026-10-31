fn main() {
    // curseforge.rs bakes this in with option_env!; rebuild when the CI secret changes.
    println!("cargo:rerun-if-env-changed=CAIRN_CURSEFORGE_API_KEY");
    tauri_build::build()
}
