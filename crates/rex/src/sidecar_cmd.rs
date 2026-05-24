use std::process::ExitCode;

use rex_config::{ensure_global_layout, load, sidecar_binary_resolvable};

pub fn run_sidecar(mut args: impl Iterator<Item = String>) -> ExitCode {
    match args.next().as_deref() {
        Some("list") => run_list(),
        Some("init") => run_init(),
        Some("doctor") => run_doctor(),
        Some("-h") | Some("--help") | None => {
            eprintln!("Usage: rex sidecar <list|init|doctor>");
            ExitCode::from(2)
        }
        Some(other) => {
            eprintln!("Unknown rex sidecar subcommand: {other}");
            ExitCode::from(2)
        }
    }
}

fn run_list() -> ExitCode {
    match load() {
        Ok(loaded) => {
            println!("active={}", loaded.effective.sidecars.active);
            for entry in &loaded.effective.sidecars.list {
                println!(
                    "  name={} binary={} enabled={} socket={}",
                    entry.name, entry.binary, entry.enabled, entry.socket
                );
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("rex sidecar list failed: {err}");
            ExitCode::from(1)
        }
    }
}

fn run_init() -> ExitCode {
    match ensure_global_layout() {
        Ok(result) => {
            if result.created_config {
                println!("Created sidecar config in global layout");
            } else {
                println!("Sidecar config already present");
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("rex sidecar init failed: {err}");
            ExitCode::from(1)
        }
    }
}

fn run_doctor() -> ExitCode {
    let loaded = match load() {
        Ok(cfg) => cfg,
        Err(err) => {
            eprintln!("rex sidecar doctor failed: {err}");
            return ExitCode::from(1);
        }
    };
    let mut ok = true;
    if let Some(entry) = loaded.active_sidecar() {
        if !sidecar_binary_resolvable(&entry.binary) {
            eprintln!("sidecar binary not found on PATH: {}", entry.binary);
            ok = false;
        }
    } else {
        eprintln!(
            "no sidecar entry matches active={}",
            loaded.effective.sidecars.active
        );
        ok = false;
    }
    let gen = rex_config::proto_gen_path();
    if !gen.exists() {
        eprintln!(
            "proto gen dir missing: {} (run `rex proto install`)",
            gen.display()
        );
        ok = false;
    }
    if ok {
        println!("sidecar doctor OK");
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
