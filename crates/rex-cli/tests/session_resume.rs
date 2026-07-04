//! Integration tests for closed-session resume helpers.

use std::fs;
use std::path::Path;

use rex_cli::lock_util::{lock_holder_alive, try_acquire_lock};
use rex_cli::session_resume::{
    is_session_available, list_closed_sessions, record_closed_session,
    resolve_last_available_session_id,
};

fn write_session_log(workspace: &Path, id: &str) {
    let dir = workspace.join(".rex/sessions");
    fs::create_dir_all(&dir).expect("sessions dir");
    fs::write(dir.join(format!("{id}.jsonl")), "{\"sequence\":1}\n").expect("log");
}

#[test]
fn locked_session_is_not_available() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ws = dir.path();
    let id = "hs-lock-test";
    write_session_log(ws, id);
    record_closed_session(ws, id).expect("record");
    let lock_path = ws
        .join(".rex/sessions/.locks")
        .join(format!("{id}.lock"));
    let lock = try_acquire_lock(&lock_path).expect("acquire");
    assert!(lock_holder_alive(&lock_path));
    assert!(!is_session_available(ws, id));
    drop(lock);
    assert!(is_session_available(ws, id));
}

#[test]
fn resolve_last_skips_locked_session() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ws = dir.path();
    write_session_log(ws, "hs-old");
    write_session_log(ws, "hs-new");
    record_closed_session(ws, "hs-old").expect("record old");
    record_closed_session(ws, "hs-new").expect("record new");
    let lock_path = ws
        .join(".rex/sessions/.locks")
        .join("hs-new.lock");
    let _lock = try_acquire_lock(&lock_path).expect("lock new");
    let picked = resolve_last_available_session_id(ws).expect("resolve");
    assert_eq!(picked, "hs-old");
}

#[test]
fn list_closed_excludes_locked() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ws = dir.path();
    write_session_log(ws, "hs-a");
    record_closed_session(ws, "hs-a").expect("record");
    let lock_path = ws.join(".rex/sessions/.locks/hs-a.lock");
    let _lock = try_acquire_lock(&lock_path).expect("lock");
    let items = list_closed_sessions(ws).expect("list");
    assert!(items.is_empty());
}
