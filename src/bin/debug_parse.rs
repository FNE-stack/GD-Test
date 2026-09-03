//! Dev-only tool: parses a real player.gdc and prints a summary of what's
//! in it — equipped gear (including the active weapon set and relic slot),
//! bag/stash item counts — or the parse error. Not part of the shipped
//! app — a separate `src/bin/` target, so it isn't subject to main.rs's
//! `windows_subsystem = "windows"` and keeps its console. Handy whenever a
//! save fails to parse and the error alone doesn't say enough, or when
//! something this app reads looks wrong and it's faster to look at the
//! real save structure directly than guess: run with the `trace-blocks`
//! feature to also get a block-by-block trace on stderr (tag/depth/byte-
//! position of every start_block/end_block, plus a warning if any
//! String/Vec length prefix decodes to a suspiciously large number — the
//! tell-tale sign of a misaligned read):
//!
//!   cargo run --bin debug_parse --features trace-blocks -- <path to player.gdc>
//!
//! See vendor_save_parser/Cargo.toml and src/parser/parser.rs for how the
//! tracing itself is wired up.

use serde_json::Value;

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: debug_parse <path to player.gdc>");

    let raw = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let cursor = std::io::Cursor::new(raw);
    let parsed: Value = match save_parser::util::map_to_json("character", cursor) {
        Ok(json) => {
            println!("OK — parsed successfully, {} bytes of JSON", json.len());
            serde_json::from_str(&json).expect("parser's own JSON output should always parse")
        }
        Err(e) => {
            eprintln!("PARSE ERROR: {e}");
            return;
        }
    };

    println!("\n--- equipment (12 body slots; slot 11 is Relic, not weapon) ---");
    print_item_array(parsed.pointer("/inv/equipment"));

    let use_alternate = parsed
        .pointer("/inv/use_alternate")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    println!(
        "\n--- active weapon set: {} (use_alternate={use_alternate}) ---",
        if use_alternate == 0 { "weapon1" } else { "weapon2" }
    );
    let active_path = if use_alternate == 0 { "/inv/weapon1" } else { "/inv/weapon2" };
    print_item_array(parsed.pointer(active_path));

    println!("\n--- bags (inv.sacks) ---");
    if let Some(sacks) = parsed.pointer("/inv/sacks").and_then(Value::as_array) {
        for (i, sack) in sacks.iter().enumerate() {
            let n = sack.get("items").and_then(Value::as_array).map_or(0, Vec::len);
            println!("  Bag {}: {n} item(s)", i + 1);
        }
    } else {
        println!("  (inv.sacks not found)");
    }

    println!("\n--- personal stash (stash.tabs — not the shared/transfer stash) ---");
    if let Some(tabs) = parsed.pointer("/stash/tabs").and_then(Value::as_array) {
        for (i, tab) in tabs.iter().enumerate() {
            let n = tab.get("items").and_then(Value::as_array).map_or(0, Vec::len);
            println!(
                "  Stash Tab {}: {n} item(s) ({}x{})",
                i + 1,
                tab.get("width").and_then(Value::as_u64).unwrap_or(0),
                tab.get("height").and_then(Value::as_u64).unwrap_or(0)
            );
        }
    } else {
        println!("  (stash.tabs not found)");
    }
}

/// Prints one line per non-empty slot in an array of {"item": {...}|null}
/// entries (the shape shared by inv.equipment, inv.weapon1, inv.weapon2).
fn print_item_array(slots: Option<&Value>) {
    let Some(slots) = slots.and_then(Value::as_array) else {
        println!("  (not found)");
        return;
    };
    for (i, slot) in slots.iter().enumerate() {
        let Some(item) = slot.get("item").filter(|v| !v.is_null()) else {
            println!("  [{i}] (empty)");
            continue;
        };
        let base = item.get("base_name").and_then(Value::as_str).unwrap_or("");
        if base.is_empty() {
            println!("  [{i}] (empty)");
        } else {
            println!("  [{i}] {base}");
        }
    }
}
