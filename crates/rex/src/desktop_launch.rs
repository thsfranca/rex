use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

use rex_cli::DesktopLaunch;

/// Resolve `apps/rex-desktop` (Electron shell).
fn resolve_desktop_app_dir() -> Result<PathBuf, String> {
    if let Ok(explicit) = std::env::var("REX_DESKTOP_APP") {
        let p = PathBuf::from(explicit);
        if p.is_dir() {
            return Ok(p);
        }
        return Err(format!(
            "REX_DESKTOP_APP is not a directory: {}",
            p.display()
        ));
    }

    // Dev: crates/rex → ../../apps/rex-desktop
    let manifest_relative = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("apps")
        .join("rex-desktop");
    if let Ok(canon) = manifest_relative.canonicalize() {
        if canon.is_dir() {
            return Ok(canon);
        }
    }

    // Installed / cwd-relative fallback
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    for ancestor in cwd.ancestors() {
        let candidate = ancestor.join("apps").join("rex-desktop");
        if candidate.is_dir() {
            return Ok(candidate);
        }
    }

    Err(
        "Could not find apps/rex-desktop. Set REX_DESKTOP_APP or run from the Rex repo."
            .into(),
    )
}

fn electron_bin(app_dir: &Path) -> PathBuf {
    app_dir.join("node_modules").join(".bin").join("electron")
}

/// Spawn the Electron desktop shell (loads apps/rex-web). Replaces the former Tauri host.
pub fn run_desktop(launch: DesktopLaunch) -> ExitCode {
    let app_dir = match resolve_desktop_app_dir() {
        Ok(p) => p,
        Err(msg) => {
            eprintln!("Error: {msg}");
            return ExitCode::from(1);
        }
    };

    let electron = electron_bin(&app_dir);
    if !electron.exists() {
        eprintln!(
            "Error: Electron not installed in {}. Run: cd apps/rex-desktop && npm install",
            app_dir.display()
        );
        return ExitCode::from(1);
    }

    let mut cmd = Command::new(&electron);
    cmd.arg(".").current_dir(&app_dir);
    cmd.env("REX_DESKTOP_HOST", "electron");
    if launch.debug {
        cmd.env("REX_DESKTOP_DEBUG", "1");
        cmd.arg("--").arg("--debug");
    }

    // Inherit stdio so operator sees Electron / renderer errors.
    cmd.stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    match cmd.status() {
        Ok(status) if status.success() => ExitCode::SUCCESS,
        Ok(status) => {
            let code = status.code().unwrap_or(1) as u8;
            ExitCode::from(if code == 0 { 1 } else { code })
        }
        Err(err) => {
            eprintln!("Error: failed to launch Rex desktop: {err}");
            ExitCode::from(1)
        }
    }
}
