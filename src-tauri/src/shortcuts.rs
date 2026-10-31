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

/// Creates a shortcut on the desktop and returns its path.
pub fn create(id: &str, name: &str, icon: Option<&str>) -> Result<PathBuf> {
    if !safe_id(id) {
        bail!("This instance id can't be used in a shortcut.");
    }
    if std::env::var_os("FLATPAK_ID").is_some() {
        bail!("Shortcuts aren't available in the Flatpak build yet.");
    }
    platform::create(id, &file_stem(name), name, icon)
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

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
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

    #[cfg(target_os = "linux")]
    #[test]
    fn exec_paths_are_quoted() {
        assert_eq!(platform::quote("/opt/My $App/cairn"), "\"/opt/My \\$App/cairn\"");
    }
}
