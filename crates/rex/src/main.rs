mod command;
mod config_cmd;
mod gateway_cmd;
mod proto_cmd;
mod sidecar_cmd;

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
        Ok(TopLevelCommand::Config(rest)) => config_cmd::run_config(rest.into_iter()),
        Ok(TopLevelCommand::Proto(rest)) => proto_cmd::run_proto(rest.into_iter()),
        Ok(TopLevelCommand::Sidecar(rest)) => sidecar_cmd::run_sidecar(rest.into_iter()),
        Ok(TopLevelCommand::Gateway(rest)) => gateway_cmd::run_gateway(rest.into_iter()),
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
    async fn config_init_exits_zero_when_layout_created() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let prev = std::env::var("REX_ROOT").ok();
        std::env::set_var("REX_ROOT", tmp.path());
        let code = run(["config", "init"].into_iter().map(str::to_string)).await;
        match prev {
            Some(v) => std::env::set_var("REX_ROOT", v),
            None => std::env::remove_var("REX_ROOT"),
        }
        assert_eq!(code, ExitCode::SUCCESS);
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
