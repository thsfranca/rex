#[derive(Debug, PartialEq, Eq)]
pub enum TopLevelCommand {
    Help,
    /// Detached process entry used by auto-start.
    InternalDaemon,
    /// Interactive terminal workspace (bare `rex`).
    Tui,
    Config(Vec<String>),
    Proto(Vec<String>),
    Sidecar(Vec<String>),
    Gateway(Vec<String>),
    Omlx(Vec<String>),
}

/// Argv for auto-start when spawning a detached daemon process.
pub const INTERNAL_DAEMON_ARG: &str = "__rex_internal_daemon";

pub fn parse_top_level(mut args: impl Iterator<Item = String>) -> Result<TopLevelCommand, String> {
    match args.next().as_deref() {
        None => Ok(TopLevelCommand::Tui),
        Some("-h") | Some("--help") | Some("help") => Ok(TopLevelCommand::Help),
        Some(INTERNAL_DAEMON_ARG) => Ok(TopLevelCommand::InternalDaemon),
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
            TopLevelCommand::Tui
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
