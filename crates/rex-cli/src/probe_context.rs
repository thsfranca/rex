//! Detect the isolated tuiwright probe fixture from process context (no extra env vars).
//!
//! Probe runs set `REX_ROOT` to `fixtures/tui_probe/rex_root` and cwd to
//! `fixtures/tui_probe/workspace` (see `tuiwright.toml.example` launch wrapper).

const PROBE_WORKSPACE_SUFFIX: &str = "fixtures/tui_probe/workspace";
const PROBE_REX_ROOT_MARKER: &str = "tui_probe/rex_root";

/// True when `rex` runs inside the tuiwright probe fixture (stepped clock, stable session id).
pub fn is_tui_probe_fixture() -> bool {
    cwd_is_probe_workspace() || rex_root_is_probe_fixture()
}

fn cwd_is_probe_workspace() -> bool {
    std::env::current_dir()
        .ok()
        .and_then(|p| p.canonicalize().ok())
        .is_some_and(|p| path_contains(&p, PROBE_WORKSPACE_SUFFIX))
}

fn rex_root_is_probe_fixture() -> bool {
    std::env::var("REX_ROOT")
        .ok()
        .map(|p| std::path::Path::new(&p).to_string_lossy().contains(PROBE_REX_ROOT_MARKER))
        .unwrap_or(false)
}

fn path_contains(path: &std::path::Path, needle: &str) -> bool {
    path.to_string_lossy().replace('\\', "/").contains(needle)
}

#[cfg(test)]
mod tests {
    #[test]
    fn rex_root_marker_detects_probe_fixture() {
        assert!(std::path::Path::new("/tmp/fixtures/tui_probe/rex_root")
            .to_string_lossy()
            .contains("tui_probe/rex_root"));
    }
}
