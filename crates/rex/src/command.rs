#[derive(Debug, PartialEq, Eq)]
pub enum TopLevelCommand {
    Help,
    /// Detached auto-start / internal only — not a public operator command.
    InternalDaemon,
    Cli(Vec<String>),
    Config(Vec<String>),
    Proto(Vec<String>),
    Sidecar(Vec<String>),
    Gateway(Vec<String>),
    Omlx(Vec<String>),
}

/// Argv used by TUI auto-start to spawn a detached daemon process.
pub const INTERNAL_DAEMON_ARG: &str = "__rex_internal_daemon";

/// Argv for integration tests that need ensure+status without a public `status` command.
pub const INTERNAL_STATUS_ARG: &str = "__rex_internal_status";

pub fn parse_top_level(mut args: impl Iterator<Item = String>) -> Result<TopLevelCommand, String> {
    match args.next().as_deref() {
        // Bare `rex` opens the TUI (primary product entry).
        None => Ok(TopLevelCommand::Cli(vec!["tui".to_string()])),
        Some("-h") | Some("--help") | Some("help") => Ok(TopLevelCommand::Help),
        Some(INTERNAL_DAEMON_ARG) => Ok(TopLevelCommand::InternalDaemon),
        Some(INTERNAL_STATUS_ARG) => {
            let mut rest = vec!["status".to_string()];
            rest.extend(args);
            Ok(TopLevelCommand::Cli(rest))
        }
        Some("daemon") => Err(
            "`rex daemon` was removed. Open the interactive workspace with `rex` or `rex tui` (daemon auto-starts)."
                .to_string(),
        ),
        Some("status") => Err(
            "`rex status` was removed. Health and phase appear in the TUI (`rex` / `rex tui`)."
                .to_string(),
        ),
        Some("complete") => Err(
            "`rex complete` was removed. Open the interactive workspace with `rex` or `rex tui`."
                .to_string(),
        ),
        Some("tui") => {
            let mut rest = vec!["tui".to_string()];
            rest.extend(args);
            Ok(TopLevelCommand::Cli(rest))
        }
        Some("config") => Ok(TopLevelCommand::Config(args.collect())),
        Some("proto") => Ok(TopLevelCommand::Proto(args.collect())),
        Some("sidecar") => Ok(TopLevelCommand::Sidecar(args.collect())),
        Some("gateway") => Ok(TopLevelCommand::Gateway(args.collect())),
        Some("omlx") => Ok(TopLevelCommand::Omlx(args.collect())),
        Some(other) => Err(format!("Unknown command: {other}")),
    }
}

pub fn print_usage() {
    eprintln!(
        "\
Usage:
  rex
  rex tui [ --no-daemon-autostart ]
  rex config <init|show|path|validate>
  rex proto <install|path|doctor>
  rex sidecar <list|init|doctor>
  rex gateway <init|doctor>
  rex omlx <init|doctor>

Open the interactive terminal workspace (default), or run setup and doctor commands."
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_rex_opens_tui() {
        assert_eq!(
            parse_top_level(std::iter::empty()).unwrap(),
            TopLevelCommand::Cli(vec!["tui".to_string()])
        );
    }

    #[test]
    fn parses_help_flags() {
        assert_eq!(
            parse_top_level(["--help".to_string()].into_iter()).unwrap(),
            TopLevelCommand::Help
        );
    }

    #[test]
    fn rejects_public_daemon_and_status() {
        assert!(parse_top_level(["daemon".to_string()].into_iter())
            .unwrap_err()
            .contains("removed"));
        assert!(parse_top_level(["status".to_string()].into_iter())
            .unwrap_err()
            .contains("removed"));
    }

    #[test]
    fn internal_daemon_arg_is_accepted() {
        assert_eq!(
            parse_top_level([INTERNAL_DAEMON_ARG.to_string()].into_iter()).unwrap(),
            TopLevelCommand::InternalDaemon
        );
    }

    #[test]
    fn rejects_unknown_top_level() {
        assert!(parse_top_level(["wat".to_string()].into_iter()).is_err());
    }

    #[test]
    fn parses_config_subcommand() {
        assert_eq!(
            parse_top_level(["config".to_string(), "show".to_string()].into_iter()).unwrap(),
            TopLevelCommand::Config(vec!["show".to_string()])
        );
    }
}
