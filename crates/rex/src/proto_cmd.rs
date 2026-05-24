use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use rex_config::{ensure_global_layout, load, proto_gen_path, proto_src_path};

pub fn run_proto(mut args: impl Iterator<Item = String>) -> ExitCode {
    match args.next().as_deref() {
        Some("doctor") => run_doctor(),
        Some("install") => run_install(),
        Some("path") => run_path(),
        Some("-h") | Some("--help") | None => {
            eprintln!("Usage: rex proto <install|path|doctor>");
            ExitCode::from(2)
        }
        Some(other) => {
            eprintln!("Unknown rex proto subcommand: {other}");
            ExitCode::from(2)
        }
    }
}

fn run_doctor() -> ExitCode {
    match std::process::Command::new("protoc")
        .arg("--version")
        .output()
    {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            println!("protoc OK: {}", version.trim());
            ExitCode::SUCCESS
        }
        Ok(output) => {
            eprintln!(
                "protoc failed (exit {}): {}",
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            );
            ExitCode::from(1)
        }
        Err(err) => {
            eprintln!("protoc not found on PATH: {err}");
            eprintln!("Install prerequisites from docs/DEPENDENCIES.md");
            ExitCode::from(1)
        }
    }
}

fn run_path() -> ExitCode {
    let _ = load();
    println!("{}", proto_gen_path().display());
    ExitCode::SUCCESS
}

fn run_install() -> ExitCode {
    if let Err(err) = ensure_global_layout() {
        eprintln!("rex proto install failed: {err}");
        return ExitCode::from(1);
    }
    if let Err(err) = load() {
        eprintln!("rex proto install failed: {err}");
        return ExitCode::from(1);
    }

    let repo_proto = find_repo_proto_root();
    let Some(repo_proto) = repo_proto else {
        eprintln!(
            "rex proto install: could not locate repo proto/rex tree; run from a Rex checkout or set REX_PROTO_SRC"
        );
        return ExitCode::from(1);
    };

    let dest_src = proto_src_path();
    if let Err(err) = copy_proto_tree(&repo_proto, &dest_src) {
        eprintln!("rex proto install failed: {err}");
        return ExitCode::from(1);
    }

    let gen_root = proto_gen_path();
    if let Err(err) = fs::create_dir_all(&gen_root) {
        eprintln!("rex proto install failed: {err}");
        return ExitCode::from(1);
    }

    let python_out = format!("--python_out={}", gen_root.display());
    let grpc_out = format!("--grpc_python_out={}", gen_root.display());
    let proto_path = format!("--proto_path={}", dest_src.display());

    let protos = collect_proto_files(&dest_src);
    if protos.is_empty() {
        eprintln!(
            "rex proto install: no .proto files under {}",
            dest_src.display()
        );
        return ExitCode::from(1);
    }

    let mut cmd = std::process::Command::new("python3");
    cmd.arg("-m").arg("grpc_tools.protoc");
    cmd.arg(&proto_path).arg(&python_out).arg(&grpc_out);
    for proto in &protos {
        cmd.arg(proto);
    }

    match cmd.output() {
        Ok(output) if output.status.success() => {
            println!("Installed Python gRPC stubs under {}", gen_root.display());
            ExitCode::SUCCESS
        }
        Ok(output) => {
            eprintln!(
                "grpc_tools.protoc failed (exit {}): {}",
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            );
            eprintln!("Install with: pip install grpcio-tools");
            ExitCode::from(1)
        }
        Err(err) => {
            eprintln!("python3 grpc_tools.protoc failed: {err}");
            eprintln!("Install with: pip install grpcio-tools");
            ExitCode::from(1)
        }
    }
}

fn find_repo_proto_root() -> Option<PathBuf> {
    if let Ok(raw) = std::env::var("REX_PROTO_SRC") {
        let path = PathBuf::from(raw.trim());
        if path.join("rex").is_dir() {
            return Some(path);
        }
    }
    let mut dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    for _ in 0..8 {
        let candidate = dir.join("proto");
        if candidate.join("rex").is_dir() {
            return Some(candidate);
        }
        let candidate = dir.join("../../proto");
        if candidate.join("rex").is_dir() {
            return candidate.canonicalize().ok();
        }
        if !dir.pop() {
            break;
        }
    }
    std::env::current_dir()
        .ok()
        .map(|cwd| cwd.join("proto"))
        .filter(|p| p.join("rex").is_dir())
}

fn copy_proto_tree(src: &Path, dest: &Path) -> Result<(), String> {
    if dest.exists() {
        fs::remove_dir_all(dest).map_err(|e| e.to_string())?;
    }
    copy_dir_recursive(src, dest)
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<(), String> {
    fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let file_type = entry.file_type().map_err(|e| e.to_string())?;
        let target = dest.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_recursive(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn collect_proto_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_proto_files_inner(root, &mut files);
    files.sort();
    files
}

fn collect_proto_files_inner(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(read) = fs::read_dir(dir) else {
        return;
    };
    for entry in read.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_proto_files_inner(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "proto") {
            out.push(path);
        }
    }
}
