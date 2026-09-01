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

mod catalog;
mod resolve;
mod save_parser;
mod server;
mod stats;

use std::path::PathBuf;

const PORT: u16 = 8934;

fn main() {
    let catalog_dir = exe_relative_dir("data/catalog");
    let profiles_dir = exe_relative_dir("profiles");

    let catalog = match catalog::Catalog::load(&catalog_dir) {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "Failed to load item catalog from {}: {e}\n\
                 Make sure the data/catalog folder is next to the .exe.",
                catalog_dir.display()
            );
            wait_for_enter();
            std::process::exit(1);
        }
    };

    let save_dir = save_parser::default_save_dir();
    match &save_dir {
        Some(dir) => println!("Found Grim Dawn save folder: {}", dir.display()),
        None => println!(
            "Could not auto-detect your Grim Dawn save folder \
             (expected under Documents\\My Games\\Grim Dawn\\save\\main). \
             You can still use manual item comparison in the UI."
        ),
    }

    let state = server::AppState {
        catalog,
        save_dir,
        profiles_dir,
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
    std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
        .spawn()
        .map(|_| ())
}

fn wait_for_enter() {
    use std::io::Read;
    println!("Press Enter to exit...");
    let _ = std::io::stdin().read(&mut [0u8]);
}
