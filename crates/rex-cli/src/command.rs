#[derive(Debug, PartialEq, Eq)]
pub enum CliCommand {
    Daemon,
    Status,
    Complete {
        prompt: String,
        model: String,
        mode: String,
        approval_id: String,
        format: CompleteOutputFormat,
    },
    Config(Vec<String>),
    Proto(Vec<String>),
    Sidecar(Vec<String>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompleteOutputFormat {
    Text,
    Ndjson,
}

pub fn parse_command(mut args: impl Iterator<Item = String>) -> Result<CliCommand, String> {
    match args.next().as_deref() {
        Some("daemon") => Ok(CliCommand::Daemon),
        Some("status") => Ok(CliCommand::Status),
        Some("complete") => {
            let prompt = args
                .next()
                .ok_or_else(|| "Missing prompt for `complete` command.".to_string())?;
            if prompt.trim().is_empty() {
                return Err("Prompt cannot be empty.".to_string());
            }
            let (format, model, mode, approval_id) = parse_complete_trailing(&mut args)?;
            Ok(CliCommand::Complete {
                prompt,
                model,
                mode,
                approval_id,
                format,
            })
        }
        Some("config") => Ok(CliCommand::Config(args.collect())),
        Some("proto") => Ok(CliCommand::Proto(args.collect())),
        Some("sidecar") => Ok(CliCommand::Sidecar(args.collect())),
        Some(other) => Err(format!("Unknown command: {other}")),
        None => Err("Missing command.".to_string()),
    }
}

/// Parses optional `complete` flags: `--format`, `--model`, `--model <id>`, `--mode <ask|plan|agent>` (any order).
fn parse_complete_trailing(
    args: &mut impl Iterator<Item = String>,
) -> Result<(CompleteOutputFormat, String, String, String), String> {
    let mut format = CompleteOutputFormat::Text;
    let mut model = String::new();
    let mut mode = String::new();
    let mut approval_id = String::new();
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--format" => {
                let value = args
                    .next()
                    .ok_or_else(|| "Missing value for `--format`.".to_string())?;
                format = match value.as_str() {
                    "text" => CompleteOutputFormat::Text,
                    "ndjson" => CompleteOutputFormat::Ndjson,
                    _ => {
                        return Err(format!(
                            "Unsupported format: {value}. Use `text` or `ndjson`."
                        ));
                    }
                };
            }
            "--model" => {
                let value = args
                    .next()
                    .ok_or_else(|| "Missing value for `--model`.".to_string())?;
                model = value;
            }
            "--mode" => {
                let value = args
                    .next()
                    .ok_or_else(|| "Missing value for `--mode`.".to_string())?;
                mode = value;
            }
            "--approval-id" => {
                let value = args
                    .next()
                    .ok_or_else(|| "Missing value for `--approval-id`.".to_string())?;
                approval_id = value;
            }
            other => {
                return Err(format!("Unknown argument for `complete`: {other}"));
            }
        }
    }
    Ok((format, model, mode, approval_id))
}

pub fn print_usage() {
    eprintln!("Usage:");
    eprintln!("  rex daemon");
    eprintln!("  rex status");
    eprintln!("  rex complete \"<prompt>\"");
    eprintln!("  rex complete \"<prompt>\" [ --format <text|ndjson> ] [ --model <id> ] [ --mode <ask|plan|agent> ] [ --approval-id <id> ]");
    eprintln!("  rex config {{ init | show | path | validate }}");
    eprintln!("  rex proto {{ install | path [python] | doctor }}");
    eprintln!("  rex sidecar {{ list | init [dir] | doctor }}");
}

#[cfg(test)]
mod tests {
    use super::{parse_command, CliCommand, CompleteOutputFormat};

    #[test]
    fn parses_status_command() {
        let cmd =
            parse_command(vec!["status".to_string()].into_iter()).expect("status should parse");
        assert_eq!(cmd, CliCommand::Status);
    }

    #[test]
    fn parses_daemon_command() {
        let cmd =
            parse_command(vec!["daemon".to_string()].into_iter()).expect("daemon should parse");
        assert_eq!(cmd, CliCommand::Daemon);
    }

    #[test]
    fn parses_complete_command_with_prompt() {
        let cmd = parse_command(vec!["complete".to_string(), "hello".to_string()].into_iter())
            .expect("complete should parse");
        assert_eq!(
            cmd,
            CliCommand::Complete {
                prompt: "hello".to_string(),
                model: String::new(),
                mode: String::new(),
                approval_id: String::new(),
                format: CompleteOutputFormat::Text,
            }
        );
    }

    #[test]
    fn rejects_complete_without_prompt() {
        let err = parse_command(vec!["complete".to_string()].into_iter())
            .expect_err("missing prompt should fail");
        assert_eq!(err.to_string(), "Missing prompt for `complete` command.");
    }

    #[test]
    fn parses_config_subcommand() {
        let cmd = parse_command(vec!["config".to_string(), "show".to_string()].into_iter())
            .expect("config show");
        assert_eq!(cmd, CliCommand::Config(vec!["show".to_string()]));
    }
}
