#[derive(Debug, PartialEq, Eq)]
pub enum CliCommand {
    Status {
        no_daemon_autostart: bool,
    },
    Tui {
        no_daemon_autostart: bool,
    },
}

pub fn parse_command(mut args: impl Iterator<Item = String>) -> Result<CliCommand, String> {
    match args.next().as_deref() {
        Some("status") => {
            let no_daemon_autostart = parse_status_trailing(&mut args)?;
            Ok(CliCommand::Status {
                no_daemon_autostart,
            })
        }
        Some("tui") => {
            let no_daemon_autostart = parse_status_trailing(&mut args)?;
            Ok(CliCommand::Tui {
                no_daemon_autostart,
            })
        }
        Some("complete") => Err(
            "`rex complete` was removed. Open the interactive workspace with `rex` or `rex tui`."
                .to_string(),
        ),
        Some(other) => Err(format!("Unknown command: {other}")),
        None => Err("Missing command.".to_string()),
    }
}

fn parse_status_trailing(args: &mut impl Iterator<Item = String>) -> Result<bool, String> {
    let mut no_daemon_autostart = false;
    for arg in args.by_ref() {
        match arg.as_str() {
            "--no-daemon-autostart" => no_daemon_autostart = true,
            other => return Err(format!("Unknown argument: {other}")),
        }
    }
    Ok(no_daemon_autostart)
}

pub fn print_usage() {
    eprintln!("Usage:");
    eprintln!("  rex-cli status [ --no-daemon-autostart ]");
    eprintln!("  rex-cli tui [ --no-daemon-autostart ]");
}

#[cfg(test)]
mod tests {
    use super::{parse_command, CliCommand};

    #[test]
    fn parses_status() {
        let cmd = parse_command(vec!["status".to_string()].into_iter()).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Status {
                no_daemon_autostart: false
            }
        );
    }

    #[test]
    fn parses_tui_with_no_autostart() {
        let cmd = parse_command(
            vec!["tui".to_string(), "--no-daemon-autostart".to_string()].into_iter(),
        )
        .unwrap();
        assert_eq!(
            cmd,
            CliCommand::Tui {
                no_daemon_autostart: true
            }
        );
    }

    #[test]
    fn rejects_complete_with_guidance() {
        let err = parse_command(vec!["complete".to_string(), "hi".to_string()].into_iter())
            .unwrap_err();
        assert!(err.contains("removed"));
        assert!(err.contains("rex tui") || err.contains("`rex`"));
    }
}
