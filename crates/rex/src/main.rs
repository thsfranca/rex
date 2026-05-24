mod command;

use std::process::ExitCode;

use command::{print_usage, TopLevelCommand};

#[tokio::main]
async fn main() -> ExitCode {
    run(std::env::args().skip(1)).await
}

pub async fn run(args: impl Iterator<Item = String>) -> ExitCode {
    match command::parse_top_level(args) {
        Ok(TopLevelCommand::Help) => {
            print_usage();
            ExitCode::SUCCESS
        }
        Ok(TopLevelCommand::Daemon) => match rex_daemon::run_daemon().await {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("Error: {err}");
                ExitCode::from(1)
            }
        },
        Ok(TopLevelCommand::Cli(rest)) => rex_cli::run_cli(rest.into_iter()).await,
        Ok(TopLevelCommand::ConfigStub) => command::print_r015_stub("config"),
        Ok(TopLevelCommand::ProtoStub(rest)) => command::run_proto_stub(rest.into_iter()),
        Ok(TopLevelCommand::SidecarStub) => command::print_r015_stub("sidecar"),
        Err(message) => {
            eprintln!("{message}");
            print_usage();
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn help_exits_zero() {
        let code = run(["--help"].into_iter().map(str::to_string)).await;
        assert_eq!(code, ExitCode::SUCCESS);
    }

    #[tokio::test]
    async fn unknown_command_exits_two() {
        let code = run(["nope"].into_iter().map(str::to_string)).await;
        assert_eq!(code, ExitCode::from(2));
    }

    #[tokio::test]
    async fn config_stub_exits_two() {
        let code = run(["config", "init"].into_iter().map(str::to_string)).await;
        assert_eq!(code, ExitCode::from(2));
    }

    #[tokio::test]
    async fn sidecar_stub_exits_two() {
        let code = run(["sidecar", "list"].into_iter().map(str::to_string)).await;
        assert_eq!(code, ExitCode::from(2));
    }

    #[tokio::test]
    async fn proto_doctor_succeeds_when_protoc_present() {
        if std::process::Command::new("protoc")
            .arg("--version")
            .output()
            .is_err()
        {
            return;
        }
        let code = run(["proto", "doctor"].into_iter().map(str::to_string)).await;
        assert_eq!(code, ExitCode::SUCCESS);
    }
}
