use std::process::ExitCode;

#[derive(Debug, PartialEq, Eq)]
pub enum TopLevelCommand {
    Help,
    Daemon,
    Cli(Vec<String>),
    ConfigStub,
    ProtoStub(Vec<String>),
    SidecarStub,
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
        Some("config") => Ok(TopLevelCommand::ConfigStub),
        Some("proto") => Ok(TopLevelCommand::ProtoStub(args.collect())),
        Some("sidecar") => Ok(TopLevelCommand::SidecarStub),
        Some(other) => Err(format!("Unknown command: {other}")),
    }
}

pub fn print_usage() {
    eprintln!(
        "\
Usage:
  rex daemon
  rex status
  rex complete \"<prompt>\" [--format text|ndjson] [--model <id>] [--mode ask|plan|agent] [--approval-id <id>]
  rex config <init|show|path|validate>   (planned — R015)
  rex proto <install|path|doctor>        (doctor available; install/path — R015)
  rex sidecar <list|init|doctor>         (planned — R015)

Run the local daemon, query status, or stream a completion via the daemon UDS API."
    );
}

pub fn print_r015_stub(group: &str) -> ExitCode {
    eprintln!(
        "rex {group} is not implemented yet (JSON configuration — R015). \
         Use environment variables until then; see docs/CONFIGURATION.md."
    );
    match group {
        "config" => eprintln!("Planned: rex config init|show|path|validate"),
        "sidecar" => eprintln!("Planned: rex sidecar list|init|doctor"),
        _ => {}
    }
    ExitCode::from(2)
}

pub fn run_proto_stub(mut args: impl Iterator<Item = String>) -> ExitCode {
    match args.next().as_deref() {
        Some("doctor") => match std::process::Command::new("protoc")
            .arg("--version")
            .output()
        {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout);
                println!("protoc OK: {}", version.trim());
                ExitCode::SUCCESS
            }
            Ok(output) => {
                eprintln!(
                    "protoc failed (exit {}): {}",
                    output.status,
                    String::from_utf8_lossy(&output.stderr).trim()
                );
                ExitCode::from(1)
            }
            Err(err) => {
                eprintln!("protoc not found on PATH: {err}");
                eprintln!("Install prerequisites from docs/DEPENDENCIES.md");
                ExitCode::from(1)
            }
        },
        Some("install") | Some("path") | Some("-h") | Some("--help") | None => {
            eprintln!(
                "rex proto install and rex proto path require JSON configuration (R015). \
                 Use `rex proto doctor` to verify protoc."
            );
            ExitCode::from(2)
        }
        Some(other) => {
            eprintln!("Unknown rex proto subcommand: {other}");
            ExitCode::from(2)
        }
    }
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
    fn proto_install_stub_exits_two() {
        assert_eq!(
            run_proto_stub(["install".to_string()].into_iter()),
            ExitCode::from(2)
        );
    }
}
