#[derive(Debug, PartialEq, Eq)]
pub enum CliCommand {
    Status,
    Complete {
        prompt: String,
        model: String,
        mode: String,
        approval_id: String,
        trace_id: String,
        active_file_path: String,
        language_id: String,
        selection_text: String,
        format: CompleteOutputFormat,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompleteOutputFormat {
    Text,
    Ndjson,
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
            let (
                format,
                model,
                mode,
                approval_id,
                trace_id,
                active_file_path,
                language_id,
                selection_text,
            ) = parse_complete_trailing(&mut args)?;
            Ok(CliCommand::Complete {
                prompt,
                model,
                mode,
                approval_id,
                trace_id,
                active_file_path,
                language_id,
                selection_text,
                format,
            })
        }
        Some(other) => Err(format!("Unknown command: {other}")),
        None => Err("Missing command.".to_string()),
    }
}

type CompleteTrailingArgs = (
    CompleteOutputFormat,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
);

fn parse_complete_trailing(
    args: &mut impl Iterator<Item = String>,
) -> Result<CompleteTrailingArgs, String> {
    let mut format = CompleteOutputFormat::Text;
    let mut model = String::new();
    let mut mode = String::new();
    let mut approval_id = String::new();
    let mut trace_id = String::new();
    let mut active_file_path = String::new();
    let mut language_id = String::new();
    let mut selection_text = String::new();
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
                model = args
                    .next()
                    .ok_or_else(|| "Missing value for `--model`.".to_string())?;
            }
            "--mode" => {
                mode = args
                    .next()
                    .ok_or_else(|| "Missing value for `--mode`.".to_string())?;
            }
            "--approval-id" => {
                approval_id = args
                    .next()
                    .ok_or_else(|| "Missing value for `--approval-id`.".to_string())?;
            }
            "--trace-id" => {
                trace_id = args
                    .next()
                    .ok_or_else(|| "Missing value for `--trace-id`.".to_string())?;
            }
            "--active-file" => {
                active_file_path = args
                    .next()
                    .ok_or_else(|| "Missing value for `--active-file`.".to_string())?;
            }
            "--language-id" => {
                language_id = args
                    .next()
                    .ok_or_else(|| "Missing value for `--language-id`.".to_string())?;
            }
            "--selection-text" => {
                selection_text = args
                    .next()
                    .ok_or_else(|| "Missing value for `--selection-text`.".to_string())?;
            }
            other => return Err(format!("Unknown argument for `complete`: {other}")),
        }
    }
    Ok((
        format,
        model,
        mode,
        approval_id,
        trace_id,
        active_file_path,
        language_id,
        selection_text,
    ))
}

pub fn print_usage() {
    eprintln!("Usage:");
    eprintln!("  rex-cli status");
    eprintln!("  rex-cli complete \"<prompt>\"");
    eprintln!(
        "  rex-cli complete \"<prompt>\" [ --format <text|ndjson> ] [ --model <id> ] [ --mode <ask|plan|agent> ] [ --approval-id <id> ] [ --trace-id <id> ] [ --active-file <path> ] [ --language-id <id> ] [ --selection-text <text> ]"
    );
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
                trace_id: String::new(),
                active_file_path: String::new(),
                language_id: String::new(),
                selection_text: String::new(),
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
    fn parses_complete_command_with_ndjson_format() {
        let cmd = parse_command(
            vec![
                "complete".to_string(),
                "hello".to_string(),
                "--format".to_string(),
                "ndjson".to_string(),
            ]
            .into_iter(),
        )
        .expect("complete ndjson should parse");
        assert_eq!(
            cmd,
            CliCommand::Complete {
                prompt: "hello".to_string(),
                model: String::new(),
                mode: String::new(),
                approval_id: String::new(),
                trace_id: String::new(),
                active_file_path: String::new(),
                language_id: String::new(),
                selection_text: String::new(),
                format: CompleteOutputFormat::Ndjson,
            }
        );
    }

    #[test]
    fn rejects_complete_command_with_unknown_flag() {
        let err = parse_command(
            vec![
                "complete".to_string(),
                "hello".to_string(),
                "--unknown".to_string(),
            ]
            .into_iter(),
        )
        .expect_err("unknown flag should fail");
        assert_eq!(
            err.to_string(),
            "Unknown argument for `complete`: --unknown"
        );
    }

    #[test]
    fn parses_model_and_mode_with_ndjson_in_different_order() {
        let cmd = parse_command(
            vec![
                "complete".to_string(),
                "hi".to_string(),
                "--model".to_string(),
                "m1".to_string(),
                "--format".to_string(),
                "ndjson".to_string(),
                "--mode".to_string(),
                "ask".to_string(),
            ]
            .into_iter(),
        )
        .expect("parse");
        assert_eq!(
            cmd,
            CliCommand::Complete {
                prompt: "hi".to_string(),
                model: "m1".to_string(),
                mode: "ask".to_string(),
                approval_id: String::new(),
                trace_id: String::new(),
                active_file_path: String::new(),
                language_id: String::new(),
                selection_text: String::new(),
                format: CompleteOutputFormat::Ndjson,
            }
        );
    }

    #[test]
    fn parses_client_hint_flags() {
        let cmd = parse_command(
            vec![
                "complete".to_string(),
                "hi".to_string(),
                "--active-file".to_string(),
                "/proj/a.ts".to_string(),
                "--language-id".to_string(),
                "typescript".to_string(),
                "--selection-text".to_string(),
                "fn main".to_string(),
            ]
            .into_iter(),
        )
        .expect("parse hints");
        assert_eq!(
            cmd,
            CliCommand::Complete {
                prompt: "hi".to_string(),
                model: String::new(),
                mode: String::new(),
                approval_id: String::new(),
                trace_id: String::new(),
                active_file_path: "/proj/a.ts".to_string(),
                language_id: "typescript".to_string(),
                selection_text: "fn main".to_string(),
                format: CompleteOutputFormat::Text,
            }
        );
    }
}
