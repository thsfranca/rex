#[derive(Debug, PartialEq, Eq)]
pub enum TopLevelCommand {
    Help,
    /// Detached process entry used by auto-start.
    InternalDaemon,
    /// Interactive desktop workspace (bare `rex`).
    Desktop(rex_cli::DesktopLaunch),
    Config(Vec<String>),
    Proto(Vec<String>),
    Sidecar(Vec<String>),
    Gateway(Vec<String>),
    Omlx(Vec<String>),
}

/// Argv for auto-start when spawning a detached daemon process.
pub const INTERNAL_DAEMON_ARG: &str = "__rex_internal_daemon";

pub fn parse_top_level(args: impl Iterator<Item = String>) -> Result<TopLevelCommand, String> {
    let mut continue_flag = false;
    let mut last_flag = false;
    let mut debug_flag = false;
    let mut positional = Vec::new();
    for arg in args {
        match arg.as_str() {
            "--continue" => continue_flag = true,
            "--last" => last_flag = true,
            "--debug" => debug_flag = true,
            other => positional.push(other.to_string()),
        }
    }
    if continue_flag && last_flag {
        return Err("--continue and --last are mutually exclusive".to_string());
    }

    match positional.first().map(|s| s.as_str()) {
        None => {
            let session = if last_flag {
                rex_cli::DesktopSession::Last
            } else if continue_flag {
                rex_cli::DesktopSession::ContinuePicker
            } else {
                rex_cli::DesktopSession::New
            };
            Ok(TopLevelCommand::Desktop(rex_cli::DesktopLaunch {
                session,
                debug: debug_flag,
            }))
        }
        Some("-h") | Some("--help") | Some("help") => Ok(TopLevelCommand::Help),
        Some(INTERNAL_DAEMON_ARG) => Ok(TopLevelCommand::InternalDaemon),
        Some("config") => Ok(TopLevelCommand::Config(
            positional.into_iter().skip(1).collect(),
        )),
        Some("proto") => Ok(TopLevelCommand::Proto(
            positional.into_iter().skip(1).collect(),
        )),
        Some("sidecar") => Ok(TopLevelCommand::Sidecar(
            positional.into_iter().skip(1).collect(),
        )),
        Some("gateway") => Ok(TopLevelCommand::Gateway(
            positional.into_iter().skip(1).collect(),
        )),
        Some("omlx") => Ok(TopLevelCommand::Omlx(
            positional.into_iter().skip(1).collect(),
        )),
        Some(other) => Err(format!("Unknown command: {other}")),
    }
}

pub fn print_usage() {
    eprintln!(
        "\
Usage:
  rex [--continue | --last] [--debug]
  rex config <init|show|path|validate>
  rex proto <install|path|doctor>
  rex sidecar <list|init|doctor>
  rex gateway <init|doctor>
  rex omlx <init|doctor>

Open the interactive desktop workspace (default), resume a closed session, or run setup and doctor commands."
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use rex_cli::{DesktopLaunch, DesktopSession};

    #[test]
    fn bare_rex_opens_desktop() {
        assert_eq!(
            parse_top_level(std::iter::empty()).unwrap(),
            TopLevelCommand::Desktop(DesktopLaunch {
                session: DesktopSession::New,
                debug: false,
            })
        );
    }

    #[test]
    fn parses_continue_flag() {
        assert_eq!(
            parse_top_level(["--continue".to_string()].into_iter()).unwrap(),
            TopLevelCommand::Desktop(DesktopLaunch {
                session: DesktopSession::ContinuePicker,
                debug: false,
            })
        );
    }

    #[test]
    fn parses_last_flag() {
        assert_eq!(
            parse_top_level(["--last".to_string()].into_iter()).unwrap(),
            TopLevelCommand::Desktop(DesktopLaunch {
                session: DesktopSession::Last,
                debug: false,
            })
        );
    }

    #[test]
    fn parses_debug_flag() {
        assert_eq!(
            parse_top_level(["--debug".to_string()].into_iter()).unwrap(),
            TopLevelCommand::Desktop(DesktopLaunch {
                session: DesktopSession::New,
                debug: true,
            })
        );
    }

    #[test]
    fn continue_and_last_are_exclusive() {
        assert!(parse_top_level(["--continue".to_string(), "--last".to_string()].into_iter()).is_err());
    }

    #[test]
    fn parses_help_flags() {
        assert_eq!(
            parse_top_level(["--help".to_string()].into_iter()).unwrap(),
            TopLevelCommand::Help
        );
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
        assert!(parse_top_level(["tui".to_string()].into_iter()).is_err());
    }

    #[test]
    fn parses_config_subcommand() {
        assert_eq!(
            parse_top_level(["config".to_string(), "show".to_string()].into_iter()).unwrap(),
            TopLevelCommand::Config(vec!["show".to_string()])
        );
    }
}
