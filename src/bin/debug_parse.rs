//! Dev-only tool: parses a real player.gdc and prints the result or error,
//! plus the equipped-gear slots (the one thing this app actually reads).
//! Not part of the shipped app — a separate `src/bin/` target, so it isn't
//! subject to main.rs's `windows_subsystem = "windows"` and keeps its
//! console. Handy whenever a save fails to parse and the error alone
//! doesn't say enough: run with the `trace-blocks` feature to also get a
//! block-by-block trace on stderr (tag/depth/byte-position of every
//! start_block/end_block, plus a warning if any String/Vec length prefix
//! decodes to a suspiciously large number — the tell-tale sign of a
//! misaligned read):
//!
//!   cargo run --bin debug_parse --features trace-blocks -- <path to player.gdc>
//!
//! See vendor_save_parser/Cargo.toml and src/parser/parser.rs for how the
//! tracing itself is wired up.

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: debug_parse <path to player.gdc>");

    let raw = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let cursor = std::io::Cursor::new(raw);
    match save_parser::util::map_to_json("character", cursor) {
        Ok(json) => {
            println!("OK — parsed successfully, {} bytes of JSON", json.len());
            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
            let equipment = &parsed["inv"]["equipment"];
            println!(
                "\n--- inv.equipment ({} slots) ---",
                equipment.as_array().map(|a| a.len()).unwrap_or(0)
            );
            println!("{}", serde_json::to_string_pretty(equipment).unwrap());
            println!("\n--- inv.use_alternate / alternate1 / alternate2 / flag ---");
            println!("use_alternate={:?}", parsed["inv"]["use_alternate"]);
            println!("alternate1={:?}", parsed["inv"]["alternate1"]);
            println!("alternate2={:?}", parsed["inv"]["alternate2"]);
            println!("flag={:?}", parsed["inv"]["flag"]);
            println!("\n--- inv.weapon1 ---");
            println!("{}", serde_json::to_string_pretty(&parsed["inv"]["weapon1"]).unwrap());
            println!("\n--- inv.weapon2 ---");
            println!("{}", serde_json::to_string_pretty(&parsed["inv"]["weapon2"]).unwrap());
        }
        Err(e) => eprintln!("PARSE ERROR: {e}"),
    }
}
