#[derive(Debug, PartialEq, Eq)]
pub enum TopLevelCommand {
    Help,
    Daemon,
    Cli(Vec<String>),
    Config(Vec<String>),
    Proto(Vec<String>),
    Sidecar(Vec<String>),
    Gateway(Vec<String>),
}

pub fn parse_top_level(mut args: impl Iterator<Item = String>) -> Result<TopLevelCommand, String> {
    match args.next().as_deref() {
        None | Some("-h") | Some("--help") | Some("help") => Ok(TopLevelCommand::Help),
        Some("daemon") => Ok(TopLevelCommand::Daemon),
        Some("status") => {
            let mut rest = vec!["status".to_string()];
            rest.extend(args);
            Ok(TopLevelCommand::Cli(rest))
        }
        Some("complete") => {
            let mut rest = vec!["complete".to_string()];
            rest.extend(args);
            Ok(TopLevelCommand::Cli(rest))
        }
        Some("config") => Ok(TopLevelCommand::Config(args.collect())),
        Some("proto") => Ok(TopLevelCommand::Proto(args.collect())),
        Some("sidecar") => Ok(TopLevelCommand::Sidecar(args.collect())),
        Some("gateway") => Ok(TopLevelCommand::Gateway(args.collect())),
        Some(other) => Err(format!("Unknown command: {other}")),
    }
}

pub fn print_usage() {
    eprintln!(
        "\
Usage:
  rex daemon
  rex status
  rex complete \"<prompt>\" [--format text|ndjson] [--model <id>] [--mode ask|plan|agent] [--approval-id <id>] [--trace-id <id>]
  rex config <init|show|path|validate>
  rex proto <install|path|doctor>
  rex sidecar <list|init|doctor>
  rex gateway <init|doctor>

Run the local daemon, query status, or stream a completion via the daemon UDS API."
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_daemon() {
        assert_eq!(
            parse_top_level(["daemon".to_string()].into_iter()).unwrap(),
            TopLevelCommand::Daemon
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
