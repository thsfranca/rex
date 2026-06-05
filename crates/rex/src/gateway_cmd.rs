use std::process::ExitCode;

use rex_config::{ensure_gateway_layout, is_managed_gateway, load, resolve_gateway_config_path};

pub fn run_gateway(mut args: impl Iterator<Item = String>) -> ExitCode {
    match args.next().as_deref() {
        Some("init") => run_init(),
        Some("doctor") => run_doctor(),
        Some("-h") | Some("--help") | None => {
            eprintln!("Usage: rex gateway <init|doctor>");
            ExitCode::from(2)
        }
        Some(other) => {
            eprintln!("Unknown rex gateway subcommand: {other}");
            ExitCode::from(2)
        }
    }
}

fn run_init() -> ExitCode {
    match ensure_gateway_layout() {
        Ok(result) => {
            if result.created_dir || result.created_config || result.created_env_example {
                println!(
                    "Gateway layout ready under {}",
                    rex_config::gateway_dir().display()
                );
            } else {
                println!("Gateway layout already present");
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("rex gateway init failed: {err}");
            ExitCode::from(1)
        }
    }
}

fn run_doctor() -> ExitCode {
    let loaded = match load() {
        Ok(cfg) => cfg,
        Err(err) => {
            eprintln!("rex gateway doctor failed: {err}");
            return ExitCode::from(1);
        }
    };
    let gateway = &loaded.effective.inference.gateway;
    let mut ok = true;

    if let Err(err) = loaded.effective.validate() {
        eprintln!("config invalid: {err}");
        ok = false;
    }

    let config_path = resolve_gateway_config_path(gateway, &loaded.rex_root);
    if is_managed_gateway(gateway) && !config_path.is_file() {
        eprintln!(
            "gateway config missing: {} (run `rex gateway init`)",
            config_path.display()
        );
        ok = false;
    }

    if is_managed_gateway(gateway) && !gateway_command_resolvable(&gateway.command) {
        eprintln!(
            "gateway command not found on PATH: {} (install litellm proxy)",
            gateway.command
        );
        ok = false;
    }

    if ok {
        println!("gateway doctor OK");
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn gateway_command_resolvable(command: &str) -> bool {
    if command.contains('/') {
        return std::path::Path::new(command).is_file();
    }
    rex_config::sidecar_binary_resolvable(command)
}
