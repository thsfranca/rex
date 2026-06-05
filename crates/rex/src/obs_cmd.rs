use std::fs;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::Duration;

use rex_config::{
    load, observability_enabled, resolve_store_path, ui_enabled, validate_read_api_listen,
};
use rex_obs_api::{serve, ReadApiState};
use rex_obs_store::{instrument_catalog, ObsStore};

const TEMPLATE_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../templates/obs");

pub fn run_obs(mut args: impl Iterator<Item = String>) -> ExitCode {
    match args.next().as_deref() {
        Some("serve") => run_serve(),
        Some("up") => run_up(),
        Some("down") => run_down(),
        Some("doctor") => run_doctor(),
        Some("catalog") => run_catalog(),
        Some("-h") | Some("--help") | None => {
            eprintln!(
                "Usage: rex obs <serve|up|down|doctor|catalog>\n\
                 serve  — loopback read API\n\
                 up     — read API + Grafana (PATH or $REX_ROOT/obs/vendor/grafana)\n\
                 down   — stop supervised obs processes\n\
                 doctor — health checks\n\
                 catalog — list OTel instruments"
            );
            ExitCode::from(2)
        }
        Some(other) => {
            eprintln!("Unknown rex obs subcommand: {other}");
            ExitCode::from(2)
        }
    }
}

fn run_serve() -> ExitCode {
    let loaded = match load() {
        Ok(cfg) => cfg,
        Err(err) => {
            eprintln!("rex obs serve failed: {err}");
            return ExitCode::from(1);
        }
    };
    if !observability_enabled(&loaded.effective.observability) {
        eprintln!("observability.enabled is not true; enable it in config first");
        return ExitCode::from(1);
    }
    if let Err(err) = validate_read_api_listen(&loaded.effective.observability.read_api.listen) {
        eprintln!("{err}");
        return ExitCode::from(1);
    }
    let store_path = resolve_store_path(&loaded.rex_root, &loaded.effective.observability.store);
    let store = match ObsStore::open(&store_path) {
        Ok(store) => store,
        Err(err) => {
            eprintln!("failed to open observability store: {err}");
            return ExitCode::from(1);
        }
    };
    let listen = loaded.effective.observability.read_api.listen.clone();
    let service_name = loaded.effective.observability.service_name.clone();
    println!("rex obs read API listening on http://{listen}");
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    if let Err(err) = rt.block_on(serve(&listen, ReadApiState::new(service_name, store))) {
        eprintln!("rex obs serve failed: {err}");
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn run_up() -> ExitCode {
    if let Err(err) = materialize_obs_layout() {
        eprintln!("rex obs up failed: {err}");
        return ExitCode::from(1);
    }
    let pid_dir = obs_run_dir();
    if let Err(err) = fs::create_dir_all(&pid_dir) {
        eprintln!("rex obs up failed: {err}");
        return ExitCode::from(1);
    }

    let serve_pid_path = pid_dir.join("read-api.pid");
    if !serve_pid_path.is_file() {
        let binary = std::env::current_exe().expect("current exe");
        let child = Command::new(binary)
            .args(["obs", "serve"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        match child {
            Ok(child) => {
                let _ = fs::write(&serve_pid_path, child.id().to_string());
            }
            Err(err) => {
                eprintln!("failed to start read API: {err}");
                return ExitCode::from(1);
            }
        }
        std::thread::sleep(Duration::from_millis(300));
    }

    if let Some(grafana) = resolve_grafana_binary() {
        let loaded = load().ok();
        let port = loaded
            .as_ref()
            .map(|l| l.effective.observability.ui.grafana.port)
            .unwrap_or(3000);
        let provisioning = obs_grafana_home().join("provisioning");
        let child = Command::new(&grafana)
            .arg("server")
            .env("GF_PATHS_PROVISIONING", &provisioning)
            .env("GF_SERVER_HTTP_PORT", port.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        match child {
            Ok(child) => {
                let _ = fs::write(pid_dir.join("grafana.pid"), child.id().to_string());
                println!("Grafana starting on http://127.0.0.1:{port}");
            }
            Err(err) => eprintln!("grafana start failed (read API still running): {err}"),
        }
    } else {
        eprintln!(
            "Grafana binary not found; install Grafana or place it under $REX_ROOT/obs/vendor/grafana/bin/"
        );
    }

    println!(
        "rex obs up: read API running; provisioning under {}",
        obs_grafana_home().display()
    );
    ExitCode::SUCCESS
}

fn run_down() -> ExitCode {
    let pid_dir = obs_run_dir();
    let mut stopped = 0u32;
    for name in ["read-api.pid", "grafana.pid"] {
        let path = pid_dir.join(name);
        if let Ok(raw) = fs::read_to_string(&path) {
            if let Ok(pid) = raw.trim().parse::<u32>() {
                let _ = Command::new("kill").arg(pid.to_string()).status();
                stopped += 1;
            }
            let _ = fs::remove_file(path);
        }
    }
    println!("rex obs down: stopped {stopped} process(es)");
    ExitCode::SUCCESS
}

fn run_doctor() -> ExitCode {
    let loaded = match load() {
        Ok(cfg) => cfg,
        Err(err) => {
            eprintln!("config load failed: {err}");
            return ExitCode::from(1);
        }
    };
    let mut ok = true;
    if !observability_enabled(&loaded.effective.observability) {
        eprintln!("observability.enabled is false");
        ok = false;
    }
    let listen = &loaded.effective.observability.read_api.listen;
    if validate_read_api_listen(listen).is_err() {
        eprintln!("invalid read_api.listen: {listen}");
        ok = false;
    } else if !tcp_open(listen) {
        eprintln!("read API not reachable at {listen} (run `rex obs serve` or `rex obs up`)");
        ok = false;
    }
    let store_path = resolve_store_path(&loaded.rex_root, &loaded.effective.observability.store);
    match ObsStore::open(&store_path) {
        Ok(store) => match store.stream_count() {
            Ok(count) => println!("store streams: {count}"),
            Err(err) => {
                eprintln!("store read failed: {err}");
                ok = false;
            }
        },
        Err(err) => {
            eprintln!("store open failed: {err}");
            ok = false;
        }
    }
    if ui_enabled(&loaded.effective.observability) {
        let port = loaded.effective.observability.ui.grafana.port;
        if !tcp_open(&format!("127.0.0.1:{port}")) {
            eprintln!("Grafana not reachable on port {port}");
            ok = false;
        }
    }
    if ok {
        println!("rex obs doctor OK");
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn run_catalog() -> ExitCode {
    for entry in instrument_catalog() {
        println!("{} ({}) — {}", entry.name, entry.kind, entry.description);
    }
    ExitCode::SUCCESS
}

fn materialize_obs_layout() -> std::io::Result<()> {
    let grafana_home = obs_grafana_home();
    copy_dir_all(Path::new(TEMPLATE_ROOT).join("grafana"), &grafana_home)?;
    Ok(())
}

fn obs_grafana_home() -> PathBuf {
    rex_config::rex_root().join("obs/grafana")
}

fn obs_run_dir() -> PathBuf {
    rex_config::rex_root().join("obs/run")
}

fn resolve_grafana_binary() -> Option<PathBuf> {
    let vendor = rex_config::rex_root().join("obs/vendor/grafana/bin/grafana");
    if vendor.is_file() {
        return Some(vendor);
    }
    which_grafana("grafana")
}

fn which_grafana(cmd: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(cmd);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn tcp_open(addr: &str) -> bool {
    TcpStream::connect_timeout(
        &addr
            .parse()
            .unwrap_or_else(|_| "127.0.0.1:3000".parse().unwrap()),
        Duration::from_millis(500),
    )
    .is_ok()
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src.as_ref())? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest = dst.as_ref().join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(entry.path(), dest)?;
        } else {
            fs::copy(entry.path(), dest)?;
        }
    }
    Ok(())
}
