//! Desktop shortcuts that start one instance directly (`cairn --launch <id>`).

use anyhow::{bail, Context, Result};
use std::path::PathBuf;

/// The `--launch <id>` argument, if present.
pub fn launch_arg<I: IntoIterator<Item = String>>(args: I) -> Option<String> {
    let mut args = args.into_iter();
    while let Some(a) = args.next() {
        if a == "--launch" {
            return args.next().filter(|id| safe_id(id));
        }
        if let Some(id) = a.strip_prefix("--launch=") {
            return Some(id.to_string()).filter(|id| safe_id(id));
        }
    }
    None
}

fn safe_id(id: &str) -> bool {
    !id.is_empty() && !id.starts_with('.') && id.chars().all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
}

/// A file name the desktop accepts on every OS.
fn file_stem(name: &str) -> String {
    let clean: String = name
        .chars()
        .map(|c| if c.is_control() || "<>:\"/\\|?*".contains(c) { '_' } else { c })
        .collect();
    let clean = clean.trim().trim_matches('.').to_string();
    if clean.is_empty() { "Minecraft".into() } else { clean }
}

fn desktop_dir() -> Result<PathBuf> {
    dirs::desktop_dir().or_else(dirs::home_dir).context("couldn't find the desktop folder")
}

/// Where a new shortcut ended up, phrased for a toast.
pub fn placed_message(path: Option<&std::path::Path>) -> String {
    match path {
        Some(p) if p.parent().is_some_and(|d| Some(d) == dirs::desktop_dir().as_deref()) => "Shortcut added to your desktop".into(),
        Some(p) => format!("Shortcut added to {}", p.display()),
        None => "Shortcut added to your applications".into(),
    }
}

/// Creates a shortcut that starts the instance. Returns the file it wrote, or `None`
/// when the system placed it (the Flatpak portal).
pub async fn create(id: &str, name: &str, icon: Option<&str>) -> Result<Option<PathBuf>> {
    if !safe_id(id) {
        bail!("This instance id can't be used in a shortcut.");
    }
    #[cfg(target_os = "linux")]
    if std::env::var_os("FLATPAK_ID").is_some() {
        flatpak::create(id, name, icon).await?;
        return Ok(None);
    }
    platform::create(id, &file_stem(name), name, icon).map(Some)
}

/// PNG width and height, read from the IHDR header.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 24 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" || &bytes[12..16] != b"IHDR" {
        return None;
    }
    let be = |b: &[u8]| u32::from_be_bytes([b[0], b[1], b[2], b[3]]);
    Some((be(&bytes[16..20]), be(&bytes[20..24])))
}

/// Inside Flatpak the desktop and the applications folder are outside the sandbox, so
/// the shortcut goes through the DynamicLauncher portal, which asks the user to confirm
/// and rewrites Exec to `flatpak run`.
#[cfg(target_os = "linux")]
mod flatpak {
    use super::*;
    use ashpd::desktop::dynamic_launcher::{DynamicLauncherProxy, PrepareInstallOptions};
    use ashpd::desktop::Icon;

    const FALLBACK_ICON: &[u8] = include_bytes!("../icons/128x128.png");
    /// The portal rejects icons larger than this.
    const MAX_ICON: u32 = 512;

    fn icon_bytes(icon: Option<&str>) -> Vec<u8> {
        icon.and_then(|p| std::fs::read(p).ok())
            .filter(|b| png_size(b).is_some_and(|(w, h)| w <= MAX_ICON && h <= MAX_ICON))
            .unwrap_or_else(|| FALLBACK_ICON.to_vec())
    }

    pub async fn create(id: &str, name: &str, icon: Option<&str>) -> Result<()> {
        let app = std::env::var("FLATPAK_ID").unwrap_or_else(|_| "app.cairn.launcher".into());
        let proxy = DynamicLauncherProxy::new()
            .await
            .context("Your desktop doesn't offer the launcher portal, so Cairn can't add shortcuts from Flatpak")?;
        let prepared = proxy
            .prepare_install(None, name, Icon::Bytes(icon_bytes(icon)), PrepareInstallOptions::default().set_editable_name(true))
            .await
            .context("couldn't ask the desktop for a shortcut")?
            .response()
            .map_err(|_| anyhow::anyhow!("Shortcut cancelled."))?;
        // Name and Icon come from the confirmation dialog; Exec is rewritten to `flatpak run`.
        let entry = format!(
            "[Desktop Entry]\nType=Application\nComment=Play in Cairn\nExec=cairn --launch {id}\nTerminal=false\nCategories=Game;\n"
        );
        proxy
            .install(prepared.token(), &format!("{app}.{id}.desktop"), &entry, Default::default())
            .await
            .context("the desktop refused the shortcut")?;
        Ok(())
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    /// Quotes one Exec argument as the Desktop Entry spec asks.
    pub(super) fn quote(arg: &str) -> String {
        let mut out = String::from("\"");
        for c in arg.chars() {
            if matches!(c, '"' | '`' | '$' | '\\') {
                out.push('\\');
            }
            out.push(c);
        }
        out.push('"');
        out
    }

    pub fn create(id: &str, stem: &str, name: &str, icon: Option<&str>) -> Result<PathBuf> {
        // Inside an AppImage, current_exe is a temporary mount; APPIMAGE is the real file.
        let exe = match std::env::var_os("APPIMAGE") {
            Some(p) => PathBuf::from(p),
            None => std::env::current_exe()?,
        };
        let one_line = |s: &str| s.replace(['\n', '\r'], " ");
        let entry = format!(
            "[Desktop Entry]\nType=Application\nName={}\nComment=Play {} in Cairn\nExec={} --launch {}\nIcon={}\nTerminal=false\nCategories=Game;\n",
            one_line(name),
            one_line(name),
            quote(&exe.to_string_lossy()),
            id,
            one_line(icon.unwrap_or("cairn")),
        );
        let path = desktop_dir()?.join(format!("{stem}.desktop"));
        std::fs::write(&path, entry)?;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
        Ok(path)
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use super::*;
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    pub fn create(id: &str, stem: &str, name: &str, _icon: Option<&str>) -> Result<PathBuf> {
        let exe = std::env::current_exe()?;
        let path = desktop_dir()?.join(format!("{stem}.lnk"));
        // Values go through the environment so nothing is spliced into the script.
        let script = "$s = (New-Object -ComObject WScript.Shell).CreateShortcut($env:CAIRN_LNK); \
            $s.TargetPath = $env:CAIRN_EXE; $s.Arguments = $env:CAIRN_ARGS; \
            $s.Description = $env:CAIRN_DESC; $s.IconLocation = $env:CAIRN_EXE + ',0'; $s.Save()";
        let out = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .env("CAIRN_LNK", &path)
            .env("CAIRN_EXE", &exe)
            .env("CAIRN_ARGS", format!("--launch {id}"))
            .env("CAIRN_DESC", format!("Play {name} in Cairn"))
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .context("couldn't run PowerShell")?;
        if !out.status.success() {
            bail!("Couldn't create the shortcut: {}", String::from_utf8_lossy(&out.stderr).trim());
        }
        Ok(path)
    }
}

/// A small app bundle on the desktop whose executable starts Cairn with `--launch`.
/// It shows in Finder and the Dock like any app; if Cairn is already open, the
/// single-instance handoff passes the id to the running window.
#[cfg(target_os = "macos")]
mod platform {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    /// Single-quotes one argument for sh.
    pub(super) fn sh_quote(arg: &str) -> String {
        format!("'{}'", arg.replace('\'', "'\\''"))
    }

    pub(super) fn xml_escape(s: &str) -> String {
        s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
    }

    pub(super) fn info_plist(id: &str, name: &str, has_icon: bool) -> String {
        let icon = if has_icon { "\n  <key>CFBundleIconFile</key><string>icon.icns</string>" } else { "" };
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>{name}</string>
  <key>CFBundleDisplayName</key><string>{name}</string>
  <key>CFBundleIdentifier</key><string>app.cairn.launcher.shortcut.{id}</string>
  <key>CFBundleExecutable</key><string>launch</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>LSUIElement</key><true/>{icon}
</dict>
</plist>
"#,
            name = xml_escape(name),
        )
    }

    pub fn create(id: &str, stem: &str, name: &str, _icon: Option<&str>) -> Result<PathBuf> {
        let exe = std::env::current_exe()?;
        let bundle = desktop_dir()?.join(format!("{stem}.app"));
        let contents = bundle.join("Contents");
        std::fs::create_dir_all(contents.join("MacOS"))?;
        // Cairn.app/Contents/MacOS/cairn -> Cairn.app/Contents/Resources/icon.icns
        let icns = exe.parent().and_then(|d| d.parent()).map(|c| c.join("Resources/icon.icns")).filter(|p| p.is_file());
        if let Some(icns) = &icns {
            std::fs::create_dir_all(contents.join("Resources"))?;
            std::fs::copy(icns, contents.join("Resources/icon.icns"))?;
        }
        std::fs::write(contents.join("Info.plist"), info_plist(id, name, icns.is_some()))?;
        let script = contents.join("MacOS/launch");
        std::fs::write(&script, format!("#!/bin/sh\nexec {} --launch {}\n", sh_quote(&exe.to_string_lossy()), id))?;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))?;
        Ok(bundle)
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
mod platform {
    use super::*;

    pub fn create(_id: &str, _stem: &str, _name: &str, _icon: Option<&str>) -> Result<PathBuf> {
        bail!("Desktop shortcuts aren't supported on this system yet.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn reads_the_launch_argument() {
        assert_eq!(launch_arg(args(&["cairn", "--launch", "my-pack"])), Some("my-pack".into()));
        assert_eq!(launch_arg(args(&["cairn", "--launch=fabric_1.21"])), Some("fabric_1.21".into()));
        assert_eq!(launch_arg(args(&["cairn"])), None);
        assert_eq!(launch_arg(args(&["cairn", "--launch"])), None);
        assert_eq!(launch_arg(args(&["cairn", "--launch", "../etc"])), None);
        assert_eq!(launch_arg(args(&["cairn", "--launch", "a b"])), None);
    }

    #[test]
    fn file_names_are_safe() {
        assert_eq!(file_stem("All the Mods: 10?"), "All the Mods_ 10_");
        assert_eq!(file_stem(" .. "), "Minecraft");
    }

    #[test]
    fn reads_png_dimensions() {
        let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
        png.extend_from_slice(&256u32.to_be_bytes());
        png.extend_from_slice(&128u32.to_be_bytes());
        assert_eq!(png_size(&png), Some((256, 128)));
        assert_eq!(png_size(b"GIF89a"), None);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn mac_bundle_text_is_escaped() {
        assert_eq!(platform::sh_quote("/Apps/It's/cairn"), "'/Apps/It'\\''s/cairn'");
        let plist = platform::info_plist("pack", "A & <B>", true);
        assert!(plist.contains("A &amp; &lt;B&gt;"));
        assert!(plist.contains("icon.icns"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn exec_paths_are_quoted() {
        assert_eq!(platform::quote("/opt/My $App/cairn"), "\"/opt/My \\$App/cairn\"");
    }
}
