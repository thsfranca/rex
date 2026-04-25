#[derive(Debug, PartialEq, Eq)]
pub enum CliCommand {
    Status,
    Complete { prompt: String },
}

pub fn parse_command(mut args: impl Iterator<Item = String>) -> Result<CliCommand, String> {
    match args.next().as_deref() {
        Some("status") => Ok(CliCommand::Status),
        Some("complete") => {
            let prompt = args
                .next()
                .ok_or_else(|| "Missing prompt for `complete` command.".to_string())?;
            if prompt.trim().is_empty() {
                return Err("Prompt cannot be empty.".to_string());
            }
            Ok(CliCommand::Complete { prompt })
        }
        Some(other) => Err(format!("Unknown command: {other}")),
        None => Err("Missing command.".to_string()),
    }
}

pub fn print_usage() {
    eprintln!("Usage:");
    eprintln!("  rex-cli status");
    eprintln!("  rex-cli complete \"<prompt>\"");
}

#[cfg(test)]
mod tests {
    use super::{parse_command, CliCommand};

    #[test]
    fn parses_status_command() {
        let cmd =
            parse_command(vec!["status".to_string()].into_iter()).expect("status should parse");
        assert_eq!(cmd, CliCommand::Status);
    }

    #[test]
    fn parses_complete_command_with_prompt() {
        let cmd = parse_command(vec!["complete".to_string(), "hello".to_string()].into_iter())
            .expect("complete should parse");
        assert_eq!(
            cmd,
            CliCommand::Complete {
                prompt: "hello".to_string(),
            }
        );
    }

    #[test]
    fn rejects_complete_without_prompt() {
        let err = parse_command(vec!["complete".to_string()].into_iter())
            .expect_err("missing prompt should fail");
        assert_eq!(err.to_string(), "Missing prompt for `complete` command.");
    }
}
