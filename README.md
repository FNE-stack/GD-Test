# GD Gear Compare

A personal, local tool for comparing Grim Dawn items against what you
currently have equipped — weighted by your own build priorities (fire
damage, resistances, survivability, whatever matters to your character),
not just a generic grade.

Ships as a single `.exe`. Double-click it, it opens the compare UI in your
browser at `http://127.0.0.1:8934`. Nothing to install on the machine you
actually play on.

## What it does

- Reads your character's currently-equipped gear straight from your Grim
  Dawn save file (`player.gdc`), so comparisons use your real current
  resistances/stats as the baseline — not guesswork.
- Lets you set adjustable, per-character stat priorities (0–4 stars each),
  same idea as [grim_gleaner](https://github.com/kultcher/grim_gleaner)'s
  weighting system.
- Compares a candidate item against whatever's in that slot right now:
  weighted priority score (letter grade), plus resistance-cap and
  survivability impact — so a shinier fire-damage ring that pushes you over
  your resist cap (wasted) or under 0% (dangerous) gets flagged, not just
  graded higher.
- Resistances are always treated as top priority by default (default to
  4 stars, and the verdict downweights a swap that drops an uncapped
  resistance even if the raw damage score looks better) — capping resist is
  close to a hard requirement in Grim Dawn before raw damage matters much.

## Not built yet

- **Clipboard/live item capture.** The dream workflow — drop an item
  in-game, Ctrl+C its tooltip, hit a button here and get an instant
  comparison — needs a parser for Grim Dawn's actual tooltip text (e.g.
  "+18% Fire Damage", "24% Chance for X Bleeding Damage") back into the
  internal stat IDs. That's real, separate work (GD's tooltip phrasing
  isn't 1:1 documented anywhere we can just import) and needs a real copied
  tooltip to build against. For now, candidate items are entered by DBR
  record path in the "Candidate Item" panel.
- **grim_gleaner profile import.** Pulling in an existing grim_gleaner
  priority profile so you don't have to re-set weights here. Needs a real
  profile file to build the importer against.

## Requirements to run the built .exe

None. Just the `.exe` and the `data/` folder next to it (already included
in this repo / any release build).

## Requirements to build it yourself

- Rust, **nightly** toolchain (pinned via `rust-toolchain.toml`) — the
  vendored save-parser uses a nightly-only array API.
- On Windows, the GNU host toolchain (`*-gnu`), not MSVC, to avoid needing
  Visual Studio Build Tools:
  ```
  rustup toolchain install nightly-2025-06-01-x86_64-pc-windows-gnu
  ```
  You'll also need a MinGW-w64 toolchain on PATH for the linker (`gcc`,
  `dlltool`) — e.g. via winget: `winget install BrechtSanders.WinLibs.POSIX.UCRT`.

Then:
```
cargo build --release
```
The binary lands in `target/release/gd-gear-compare.exe`. Copy `data/` and
an empty `profiles/` folder next to it before running (or run it once from
the repo root — it looks for both relative to the exe's own location).

## Using it

1. Run the `.exe`. It opens your browser to the compare UI.
2. Pick your character from the dropdown (auto-detected from
   `Documents\My Games\Grim Dawn\save\main`). If it's not found, you can
   still compare items manually.
3. Set your stat priorities (★0–4) for this character — saved automatically
   per character under `profiles/`.
4. Pick a gear slot to compare against, enter the candidate item's base
   name/prefix/suffix (as DBR record paths, e.g.
   `records/items/gearhead/d004_head.dbr`), and hit Compare.
5. Read the verdict: priority-score comparison plus a resistance/cap table
   showing exactly what the swap does to your totals.

## Credits / data sources

This project is a personal fan tool built on top of two open-source,
MIT-licensed projects — full credit to their authors:

- **[grim_gleaner](https://github.com/kultcher/grim_gleaner)** by its
  contributors — the vendored item/affix catalog under `data/catalog/`
  (extracted Grim Dawn game data, current as of game version 1.3.0.7) comes
  from this project.
- **[grim-save-parser](https://github.com/nbak/grim-save-parser)** by
  Kirill Bulatov — the save-file (`.gdc`) parsing logic under
  `vendor_save_parser/` is ported from this project's `parser` crate.

Grim Dawn and its game data are property of Crate Entertainment; this tool
only reads local save files you already own and is not affiliated with
Crate Entertainment. Personal use only, not published/distributed as a
product.
