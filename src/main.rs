//! GD Gear Compare — a personal, local tool for comparing Grim Dawn items
//! against your currently-equipped gear, weighted by your own build
//! priorities (fire damage, resistances, survivability, etc).
//!
//! Ships as a single .exe: double-click it, it starts a local web server
//! and opens the compare UI in your browser. No installs required.
//!
//! Built on top of two MIT-licensed open-source projects:
//!   - grim_gleaner (https://github.com/kultcher/grim_gleaner) — item/affix
//!     catalog data, vendored under data/catalog.
//!   - grim-save-parser (https://github.com/nbak/grim-save-parser) —
//!     Grim Dawn save-file (.gdc) parsing, vendored under vendor_save_parser.
//!
//! `windows_subsystem = "windows"` below means no console window pops up
//! when double-clicking the exe — normal for a local-server-plus-browser
//! app. The tradeoff: stdout/stderr go nowhere by default (no terminal to
//! print to), so the one real startup-failure case (bad/missing data/
//! folder) is reported via a native message box instead of eprintln!,
//! otherwise it would fail completely silently.
#![windows_subsystem = "windows"]

mod catalog;
mod import;
mod resolve;
mod save_parser;
mod server;
mod settings;
mod stats;

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;

const PORT: u16 = 8934;

fn main() {
    let catalog_dir = exe_relative_dir("data/catalog");
    let profiles_dir = exe_relative_dir("profiles");
    let settings_path = exe_relative_dir("settings.json");

    let catalog = match catalog::Catalog::load(&catalog_dir) {
        Ok(c) => c,
        Err(e) => {
            fatal_error(&format!(
                "Failed to load item catalog from {}:\n\n{e}\n\n\
                 Make sure the data/catalog folder is next to the .exe.",
                catalog_dir.display()
            ));
            std::process::exit(1);
        }
    };

    let settings = settings::Settings::load(&settings_path);
    let save_dir = settings
        .save_dir_override
        .clone()
        .or_else(save_parser::default_save_dir);
    match &save_dir {
        Some(dir) if settings.save_dir_override.is_some() => {
            println!("Using configured Grim Dawn save folder: {}", dir.display())
        }
        Some(dir) => println!("Found Grim Dawn save folder: {}", dir.display()),
        None => println!(
            "Could not auto-detect your Grim Dawn save folder \
             (expected under Documents\\My Games\\Grim Dawn\\save\\main). \
             Set one in the app's Settings, or use manual item comparison."
        ),
    }

    let state = server::AppState {
        catalog,
        save_dir: Mutex::new(save_dir),
        profiles_dir,
        settings_path,
        // Seeded to "now" rather than something already-stale, so the
        // auto-shutdown watchdog (server.rs) can't fire before the browser
        // even finishes opening and sending its first heartbeat.
        last_heartbeat: Mutex::new(Instant::now()),
    };

    let url = format!("http://127.0.0.1:{PORT}");
    let _ = open_browser(&url);

    server::run(state, PORT);
}

fn exe_relative_dir(rel: &str) -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
        .join(rel)
}

fn open_browser(url: &str) -> std::io::Result<()> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map(|_| ())
}

/// Shows a native message box — the only way to surface a startup error to
/// the user once the app has no console (see the windows_subsystem note at
/// the top of this file).
fn fatal_error(message: &str) {
    use std::iter::once;
    use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};

    let wide_message: Vec<u16> = message.encode_utf16().chain(once(0)).collect();
    let wide_title: Vec<u16> = "GD Gear Compare".encode_utf16().chain(once(0)).collect();
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            wide_message.as_ptr(),
            wide_title.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}
