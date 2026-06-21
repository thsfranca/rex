use std::process::ExitCode;

use rex_config::{
    ensure_omlx_layout, is_managed_omlx, is_managed_gateway, load, omlx_dir,
};

pub fn run_omlx(mut args: impl Iterator<Item = String>) -> ExitCode {
    match args.next().as_deref() {
        Some("init") => run_init(),
        Some("doctor") => run_doctor(),
        Some("-h") | Some("--help") | None => {
            eprintln!("Usage: rex omlx <init|doctor>");
            ExitCode::from(2)
        }
        Some(other) => {
            eprintln!("Unknown rex omlx subcommand: {other}");
            ExitCode::from(2)
        }
    }
}

fn run_init() -> ExitCode {
    match ensure_omlx_layout() {
        Ok(result) => {
            if result.created_dir || result.created_env_example || result.created_readme {
                println!("oMLX layout ready under {}", omlx_dir().display());
            } else {
                println!("oMLX layout already present");
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("rex omlx init failed: {err}");
            ExitCode::from(1)
        }
    }
}

fn run_doctor() -> ExitCode {
    let loaded = match load() {
        Ok(cfg) => cfg,
        Err(err) => {
            eprintln!("rex omlx doctor failed: {err}");
            return ExitCode::from(1);
        }
    };
    let omlx = &loaded.effective.inference.omlx;
    let mut ok = true;

    if let Err(err) = loaded.effective.validate() {
        eprintln!("config invalid: {err}");
        ok = false;
    }

    if is_managed_omlx(omlx) && is_managed_gateway(&loaded.effective.inference.gateway) {
        eprintln!(
            "inference.omlx.mode and inference.gateway.mode cannot both be managed"
        );
        ok = false;
    }

    if is_managed_omlx(omlx) && !omlx_command_resolvable(&omlx.command) {
        eprintln!(
            "oMLX command not found on PATH: {} (install oMLX — see docs/DEPENDENCIES.md)",
            omlx.command
        );
        ok = false;
    }

    let model_dir = omlx.model_dir.trim();
    if is_managed_omlx(omlx) && !model_dir.is_empty() && !std::path::Path::new(model_dir).is_dir() {
        eprintln!("oMLX model_dir not found: {model_dir}");
        ok = false;
    }

    if ok {
        println!("omlx doctor OK");
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn omlx_command_resolvable(command: &str) -> bool {
    if command.contains('/') {
        return std::path::Path::new(command).is_file();
    }
    rex_config::sidecar_binary_resolvable(command)
}
