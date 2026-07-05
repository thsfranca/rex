#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rex_cli::{DesktopLaunch, DesktopSession};

fn main() {
    let mut debug = false;
    for arg in std::env::args().skip(1) {
        if arg == "--debug" {
            debug = true;
        }
    }
    rex_desktop_lib::run(DesktopLaunch {
        session: DesktopSession::New,
        debug,
    });
}
