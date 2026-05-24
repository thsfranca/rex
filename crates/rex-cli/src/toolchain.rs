use std::path::{Path, PathBuf};

use rex_config::{init_user_config, load_merged, user_config_path, validate, write_json};

pub fn run_config(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("init") => {
            let path = init_user_config().map_err(|err| err.to_string())?;
            println!("Wrote {}", path.display());
            Ok(())
        }
        Some("path") => {
            println!("{}", user_config_path().display());
            Ok(())
        }
        Some("show") => {
            let config = load_merged().map_err(|err| err.to_string())?;
            let json = serde_json::to_string_pretty(&config).map_err(|err| err.to_string())?;
            println!("{json}");
            Ok(())
        }
        Some("validate") => {
            let config = load_merged().map_err(|err| err.to_string())?;
            validate(&config).map_err(|err| err.to_string())?;
            if let Some(entry) = config.active_sidecar_entry() {
                if !entry.enabled {
                    eprintln!(
                        "warning: active sidecar '{}' is disabled in config",
                        entry.name
                    );
                }
            }
            println!("config ok");
            Ok(())
        }
        Some(other) => Err(format!("Unknown config subcommand: {other}")),
        None => Err("Missing config subcommand. Use: init | show | path | validate".to_string()),
    }
}

pub fn assets_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/proto")
}

pub fn install_proto_assets() -> Result<PathBuf, String> {
    let home = rex_config::rex_home();
    let gen_root = home.join("proto/gen");
    let python_dest = gen_root.join("python");
    let src_dest = home.join("proto/src");
    std::fs::create_dir_all(&python_dest).map_err(|err| err.to_string())?;
    std::fs::create_dir_all(&src_dest).map_err(|err| err.to_string())?;
    let assets = assets_root();
    copy_tree(&assets.join("gen/python"), &python_dest)?;
    copy_tree(&assets.join("src"), &src_dest)?;
    let _ = init_user_config().map_err(|err| err.to_string())?;
    let mut config = load_merged().map_err(|err| err.to_string())?;
    config.proto.gen_root = "~/.rex/proto/gen".to_string();
    write_json(&user_config_path(), &config).map_err(|err| err.to_string())?;
    Ok(python_dest)
}

pub fn run_proto(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("install") => {
            let dest = install_proto_assets()?;
            println!(
                "Installed proto stubs under {}; updated proto.gen_root in config",
                dest.display()
            );
            Ok(())
        }
        Some("path") => {
            let lang = args.get(1).map(String::as_str).unwrap_or("");
            let config = load_merged().map_err(|err| err.to_string())?;
            if lang == "python" || lang.is_empty() {
                println!("{}", config.proto_python_path().display());
            } else {
                println!(
                    "{}",
                    rex_config::expand_tilde(&config.proto.gen_root)
                        .join(lang)
                        .display()
                );
            }
            Ok(())
        }
        Some("doctor") => {
            let config = load_merged().map_err(|err| err.to_string())?;
            let python = config.proto_python_path();
            let required = [
                python.join("rex/v1/rex_pb2.py"),
                python.join("rex/sidecar/v1/sidecar_pb2.py"),
            ];
            for path in required {
                if !path.is_file() {
                    return Err(format!("missing proto stub: {}", path.display()));
                }
            }
            println!("proto stubs ok");
            Ok(())
        }
        Some(other) => Err(format!("Unknown proto subcommand: {other}")),
        None => Err("Missing proto subcommand. Use: install | path | doctor".to_string()),
    }
}

pub fn run_sidecar(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("list") => {
            let config = load_merged().map_err(|err| err.to_string())?;
            for entry in &config.sidecars.list {
                let marker = if entry.name == config.sidecars.active {
                    "*"
                } else {
                    " "
                };
                println!(
                    "{marker} {} binary={} socket={} enabled={}",
                    entry.name, entry.binary, entry.socket, entry.enabled
                );
            }
            Ok(())
        }
        Some("doctor") => {
            run_proto(&["doctor".to_string()])?;
            let config = load_merged().map_err(|err| err.to_string())?;
            validate(&config).map_err(|err| err.to_string())?;
            println!("sidecar config ok");
            Ok(())
        }
        Some("init") => {
            let dir = args
                .get(1)
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("my-rex-sidecar"));
            scaffold_sidecar(&dir)?;
            println!("Scaffolded sidecar at {}", dir.display());
            Ok(())
        }
        Some(other) => Err(format!("Unknown sidecar subcommand: {other}")),
        None => Err("Missing sidecar subcommand. Use: list | init | doctor".to_string()),
    }
}

fn scaffold_sidecar(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir.join("my_sidecar")).map_err(|err| err.to_string())?;
    std::fs::write(
        dir.join("pyproject.toml"),
        r#"[project]
name = "my-rex-sidecar"
version = "0.1.0"
requires-python = ">=3.11"
dependencies = ["grpcio"]

[project.scripts]
my-rex-sidecar = "my_sidecar.__main__:main"
"#,
    )
    .map_err(|err| err.to_string())?;
    std::fs::write(
        dir.join("my_sidecar/__main__.py"),
        r#"import sys
from pathlib import Path

def bootstrap_proto_path() -> None:
    from rex_agent.config import load_merged_config
    cfg = load_merged_config()
    gen = Path(cfg["proto"]["gen_root"]).expanduser().resolve() / "python"
    if str(gen) not in sys.path:
        sys.path.insert(0, str(gen))

def main() -> None:
  bootstrap_proto_path()
  print("sidecar scaffold — implement SidecarService server")

if __name__ == "__main__":
    main()
"#,
    )
    .map_err(|err| err.to_string())?;
    Ok(())
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
    if !from.exists() {
        return Err(format!("missing embedded assets at {}", from.display()));
    }
    for entry in walkdir(from)? {
        let rel = entry.strip_prefix(from).map_err(|err| err.to_string())?;
        let dest = to.join(rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&dest).map_err(|err| err.to_string())?;
        } else {
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
            }
            std::fs::copy(&entry, &dest).map_err(|err| err.to_string())?;
        }
    }
    Ok(())
}

fn walkdir(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    walkdir_inner(root, &mut out)?;
    Ok(out)
}

fn walkdir_inner(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in std::fs::read_dir(dir).map_err(|err| err.to_string())? {
        let entry = entry.map_err(|err| err.to_string())?;
        let path = entry.path();
        out.push(path.clone());
        if path.is_dir() {
            walkdir_inner(&path, out)?;
        }
    }
    Ok(())
}

pub fn ensure_proto_installed() -> Result<(), String> {
    let config = load_merged().unwrap_or_default();
    let python = config.proto_python_path();
    if python.join("rex/v1/rex_pb2.py").is_file() {
        return Ok(());
    }
    install_proto_assets()?;
    Ok(())
}
