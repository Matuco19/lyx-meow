use std::path::PathBuf;

///Shortcut name exposed on the shell after registering.
pub const SHORTCUT: &str = "mlyx";

///Subdir of the user profile holding the `mlyx` shim on Windows.
fn shim_dir() -> color_eyre::Result<PathBuf> {
    // prefer USERPROFILE, fall back to home
    if let Ok(profile) = std::env::var("USERPROFILE") {
        Ok(PathBuf::from(profile).join(".lyx-meow").join("bin"))
    } else {
        let home = std::env::home_dir().ok_or_else(|| {
            color_eyre::eyre::eyre!("could not find a home dir for the mlyx shim")
        })?;
        Ok(home.join(".lyx-meow").join("bin"))
    }
}

///Absolute path of this binary, the shim forwards to it.
fn target_exe() -> color_eyre::Result<PathBuf> {
    Ok(std::env::current_exe()?)
}

///Shim file name per platform.
fn shim_file_name() -> &'static str {
    if cfg!(windows) { "mlyx.cmd" } else { "mlyx" }
}

///Batch content forwarding all args to the real binary.
fn shim_content(exe: &str) -> String {
    format!("@echo off\r\n\"{exe}\" %*\r\n")
}

///Splits a `;`-joined PATH into trimmed entries.
fn split_entries(raw: &str) -> Vec<String> {
    raw.split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

///Appends `dir` to PATH text unless already present (case-insensitive).
fn with_entry(raw: &str, dir: &str) -> String {
    let mut entries = split_entries(raw);
    if !entries.iter().any(|e| e.eq_ignore_ascii_case(dir)) {
        entries.push(dir.to_string());
    }
    entries.join(";")
}

///Removes all `dir` entries from PATH text (case-insensitive).
fn without_entry(raw: &str, dir: &str) -> String {
    split_entries(raw)
        .into_iter()
        .filter(|e| {
            let trimmed = e.trim_end_matches(['\\', '/']);
            let wanted = dir.trim_end_matches(['\\', '/']);
            !trimmed.eq_ignore_ascii_case(wanted)
        })
        .collect::<Vec<_>>()
        .join(";")
}

///Registers `mlyx` for the current user.
pub fn register() -> color_eyre::Result<RegisterOutcome> {
    let exe = target_exe()?;
    let dir = shim_dir()?;
    std::fs::create_dir_all(&dir)?;
    if cfg!(windows) {
        let shim = dir.join(shim_file_name());
        std::fs::write(&shim, shim_content(&exe.to_string_lossy()))?;
        let added_to_path = ensure_on_user_path(dir.to_string_lossy().as_ref())?;
        Ok(RegisterOutcome {
            shortcut: SHORTCUT.to_string(),
            shim: shim.to_string_lossy().to_string(),
            target: exe.to_string_lossy().to_string(),
            added_to_path,
        })
    } else {
        // symlink into ~/.local/bin on unix
        let home = std::env::home_dir().ok_or_else(|| {
            color_eyre::eyre::eyre!("could not find a home dir for the mlyx shim")
        })?;
        let bin = home.join(".local").join("bin");
        std::fs::create_dir_all(&bin)?;
        let link = bin.join(SHORTCUT);
        link_shortcut(&exe, &link)?;
        Ok(RegisterOutcome {
            shortcut: SHORTCUT.to_string(),
            shim: link.to_string_lossy().to_string(),
            target: exe.to_string_lossy().to_string(),
            added_to_path: false,
        })
    }
}

///Unregisters the `mlyx` shortcut.
pub fn unregister() -> color_eyre::Result<UnregisterOutcome> {
    let dir = shim_dir()?;
    if cfg!(windows) {
        let shim = dir.join(shim_file_name());
        let removed_shim = std::fs::remove_file(&shim).is_ok();
        let removed_from_path = remove_from_user_path(dir.to_string_lossy().as_ref())?;
        Ok(UnregisterOutcome {
            removed_shim,
            removed_from_path,
            shim: shim.to_string_lossy().to_string(),
        })
    } else {
        let home = std::env::home_dir().ok_or_else(|| {
            color_eyre::eyre::eyre!("could not find a home dir for the mlyx shim")
        })?;
        let link = home.join(".local").join("bin").join(SHORTCUT);
        let removed_shim = std::fs::remove_file(&link).is_ok();
        Ok(UnregisterOutcome {
            removed_shim,
            removed_from_path: false,
            shim: link.to_string_lossy().to_string(),
        })
    }
}

///Details for the success message.
pub struct RegisterOutcome {
    pub shortcut: String,
    pub shim: String,
    pub target: String,
    pub added_to_path: bool,
}

///Details for the goodbye message.
pub struct UnregisterOutcome {
    pub removed_shim: bool,
    pub removed_from_path: bool,
    pub shim: String,
}

// links the shortcut on unix, replacing any stale link
#[cfg(unix)]
fn link_shortcut(exe: &std::path::Path, link: &std::path::Path) -> std::io::Result<()> {
    let _ = std::fs::remove_file(link);
    std::os::unix::fs::symlink(exe, link)
}

// unreachable on Windows, keeps the unix-only symlink code out of the build
#[cfg(not(unix))]
fn link_shortcut(_exe: &std::path::Path, _link: &std::path::Path) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "mlyx linking is unix-only",
    ))
}

// reads the user PATH via powershell
fn ps_get_user_path() -> color_eyre::Result<String> {
    let out = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "[Environment]::GetEnvironmentVariable('Path','User')",
        ])
        .output()?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

// writes the user PATH via powershell, quoting single quotes
fn ps_set_user_path(next: &str) -> color_eyre::Result<()> {
    let escaped = next.replace('\'', "''");
    let script = format!("[Environment]::SetEnvironmentVariable('Path', '{escaped}', 'User')");
    let status = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .status()?;
    if !status.success() {
        return Err(color_eyre::eyre::eyre!(
            "powershell could not update the user PATH"
        ));
    }
    Ok(())
}

// appends `dir` to the user PATH when missing
fn ensure_on_user_path(dir: &str) -> color_eyre::Result<bool> {
    let current = ps_get_user_path()?;
    if split_entries(&current)
        .iter()
        .any(|e| e.eq_ignore_ascii_case(dir))
    {
        return Ok(false);
    }
    let next = with_entry(&current, dir);
    ps_set_user_path(&next)?;
    Ok(true)
}

// drops `dir` from the user PATH when present
fn remove_from_user_path(dir: &str) -> color_eyre::Result<bool> {
    let current = ps_get_user_path()?;
    if !split_entries(&current)
        .iter()
        .any(|e| e.eq_ignore_ascii_case(dir))
    {
        return Ok(false);
    }
    let next = without_entry(&current, dir);
    ps_set_user_path(&next)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::{shim_content, shim_file_name, with_entry, without_entry};

    #[test]
    fn adds_missing_entry_once() {
        let next = with_entry("C:\\Win;C:\\Tools", "C:\\Paw");
        assert!(next.contains("C:\\Paw"));
        let again = with_entry(&next, "c:\\paw");
        assert_eq!(
            again.matches("Paw").count() + again.matches("paw").count(),
            1
        );
    }

    #[test]
    fn removes_entry_case_insensitively() {
        let next = without_entry("C:\\Win;C:\\Paw\\;C:\\Tools", "c:\\paw");
        assert!(!next.to_lowercase().contains("paw"));
        assert!(next.contains("C:\\Tools"));
    }

    #[test]
    fn shim_points_at_exe() {
        let content = shim_content("C:\\bin\\lyx-meow.exe");
        assert!(content.contains("lyx-meow.exe"));
        assert!(!shim_file_name().is_empty());
    }
}
