fn main() {
    // Baked in with option_env! (curseforge.rs, msa_client.rs); rebuild when they change.
    println!("cargo:rerun-if-env-changed=CAIRN_CURSEFORGE_API_KEY");
    println!("cargo:rerun-if-env-changed=CAIRN_MSA_CLIENT_ID");
    tauri_build::build()
}
