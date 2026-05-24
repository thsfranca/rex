use std::process::ExitCode;

use rex_config::{
    ensure_global_layout, global_config_path, load, load_merged, proto_gen_path, rex_root,
};

pub fn run_config(mut args: impl Iterator<Item = String>) -> ExitCode {
    match args.next().as_deref() {
        Some("init") => run_init(),
        Some("show") => run_show(),
        Some("path") => run_path(),
        Some("validate") => run_validate(),
        Some("-h") | Some("--help") | None => {
            eprintln!("Usage: rex config <init|show|path|validate>");
            ExitCode::from(if args.next().is_some() { 2 } else { 0 })
        }
        Some(other) => {
            eprintln!("Unknown rex config subcommand: {other}");
            ExitCode::from(2)
        }
    }
}

fn run_init() -> ExitCode {
    match ensure_global_layout() {
        Ok(result) => {
            if result.created_config || result.created_dirs {
                println!("Created Rex layout under {}", rex_root().display());
            } else {
                println!("Rex layout already exists under {}", rex_root().display());
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("rex config init failed: {err}");
            ExitCode::from(1)
        }
    }
}

fn run_show() -> ExitCode {
    match load() {
        Ok(loaded) => match serde_json::to_string_pretty(&loaded.effective) {
            Ok(json) => {
                println!("{json}");
                ExitCode::SUCCESS
            }
            Err(err) => {
                eprintln!("rex config show failed: {err}");
                ExitCode::from(1)
            }
        },
        Err(err) => {
            eprintln!("rex config show failed: {err}");
            ExitCode::from(1)
        }
    }
}

fn run_path() -> ExitCode {
    println!("REX_ROOT={}", rex_root().display());
    println!("global_config={}", global_config_path().display());
    println!("proto_gen={}", proto_gen_path().display());
    if let Ok(loaded) = load_merged() {
        if let Some(path) = loaded.global_path {
            println!("loaded_global={}", path.display());
        }
        if let Some(path) = loaded.project_path {
            println!("loaded_project={}", path.display());
        }
    }
    ExitCode::SUCCESS
}

fn run_validate() -> ExitCode {
    match load() {
        Ok(loaded) => match loaded.effective.validate() {
            Ok(()) => {
                println!("config OK");
                ExitCode::SUCCESS
            }
            Err(err) => {
                eprintln!("config invalid: {err}");
                ExitCode::from(1)
            }
        },
        Err(err) => {
            eprintln!("rex config validate failed: {err}");
            ExitCode::from(1)
        }
    }
}
