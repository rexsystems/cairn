//! Tells Discord how to start Cairn (where discord-rpc registers games), so Join works while Cairn is closed.
//! Best effort: without it Cairn just has to be open already.

use anyhow::Result;

#[cfg(target_os = "linux")]
pub fn register(app_id: &str) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    // The sandbox can't write the host's applications folder; Flatpak users keep Cairn open.
    if std::env::var_os("FLATPAK_ID").is_some() {
        return Ok(());
    }
    let exe = match std::env::var_os("APPIMAGE") {
        Some(p) => std::path::PathBuf::from(p),
        None => std::env::current_exe()?,
    };
    let data = std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".local/share")))
        .ok_or_else(|| anyhow::anyhow!("no home folder"))?;
    let dir = data.join("applications");
    std::fs::create_dir_all(&dir)?;
    let name = format!("discord-{app_id}.desktop");
    let entry = format!(
        "[Desktop Entry]\nName=Cairn\nExec={} %u\nType=Application\nNoDisplay=true\nCategories=Discord;Games;\nMimeType=x-scheme-handler/discord-{app_id};\n",
        crate::shortcuts::desktop_quote(&exe.to_string_lossy())
    );
    let path = dir.join(&name);
    if std::fs::read_to_string(&path).ok().as_deref() == Some(entry.as_str()) {
        return Ok(());
    }
    std::fs::write(&path, entry)?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))?;
    let _ = std::process::Command::new("xdg-mime").args(["default", &name, &format!("x-scheme-handler/discord-{app_id}")]).status();
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn register(app_id: &str) -> Result<()> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let exe = std::env::current_exe()?.to_string_lossy().into_owned();
    let key = format!(r"HKCU\Software\Classes\discord-{app_id}");
    let command = format!("\"{exe}\"");
    let entries: [(String, Option<&str>, String); 4] = [
        (key.clone(), None, format!("URL:Run game {app_id} protocol")),
        (key.clone(), Some("URL Protocol"), String::new()),
        (format!(r"{key}\DefaultIcon"), None, exe.clone()),
        (format!(r"{key}\shell\open\command"), None, command),
    ];
    for (k, name, value) in entries {
        let mut cmd = std::process::Command::new("reg");
        cmd.args(["add", &k]);
        match name {
            Some(n) => cmd.args(["/v", n]),
            None => cmd.arg("/ve"),
        };
        cmd.args(["/d", &value, "/f"]).creation_flags(CREATE_NO_WINDOW);
        let out = cmd.output()?;
        if !out.status.success() {
            anyhow::bail!("reg add failed for {k}");
        }
    }
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn register(app_id: &str) -> Result<()> {
    let home = std::env::var_os("HOME").ok_or_else(|| anyhow::anyhow!("no home folder"))?;
    let dir = std::path::PathBuf::from(home).join("Library/Application Support/discord/games");
    std::fs::create_dir_all(&dir)?;
    let exe = std::env::current_exe()?;
    crate::util::write_json_atomic(&dir.join(format!("{app_id}.json")), &serde_json::json!({ "command": exe.to_string_lossy() }))
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
pub fn register(_app_id: &str) -> Result<()> {
    Ok(())
}
