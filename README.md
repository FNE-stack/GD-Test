# GD Gear Compare

A personal, local tool for comparing Grim Dawn items against what you
currently have equipped — weighted by your own build priorities (fire
damage, resistances, survivability, whatever matters to your character),
not just a generic grade.

Ships as a single `.exe`. Double-click it, it opens the compare UI in your
browser at `http://127.0.0.1:8934`. Nothing to install on the machine you
actually play on. Closing that browser tab shuts the server down on its
own (near-instantly on a normal close; within about a minute either way,
e.g. if the browser itself crashes) — no need to remember to end the
process by hand, and no stale instance left behind blocking a later
launch from using the same port.

## What it does

- Reads your character's currently-equipped gear straight from your Grim
  Dawn save file (`player.gdc`), so comparisons use your real current
  resistances/stats as the baseline — not guesswork. Since Grim Dawn only
  writes that file on specific triggers (autosave, leaving an area,
  opening the menu, quitting — never instantly on pickup/move/sell), the
  header always shows **"Save data as of ..."** with exactly when that
  file was last written, so a stale read (an item you moved that still
  shows up, gear you don't have anymore) is obvious rather than confusing.
- Item names include their full prefix/suffix ("Unyielding Imperial
  Necklace of Mending", not just "Imperial Necklace") and, when the
  catalog has one, the item's level ("(ilvl 65)") — Grim Dawn reuses one
  name across an item's power tiers (the community's "Empowered" drops
  are the same name, a much higher item level), so this is the one way to
  tell which tier you're actually looking at.
- Granted/bonus skills resolve to their real names — "Blitz (+2)" instead
  of an internal `skill_bonus:skill_level` key — and a skill an item
  merely *grants* (a proc, not a rank bonus) is labeled "(granted skill)"
  so it's not confused with the two behaving the same way.
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
- Priority categories/labels are ported directly from grim_gleaner's own
  stat registry (`data/priority_taxonomy.json`), so the tabs match
  grim_gleaner 1:1 rather than an ad-hoc grouping.
- Auto-detects your save folder at the default location; if it's not found
  (redirected Documents, save on another drive, etc), set it manually in
  **⚙ Settings** — validated and persisted to `%APPDATA%\GD Gear
  Compare\settings.json`, no restart needed. That's a stable, per-user
  location independent of which folder any given release's `.exe` happens
  to be unzipped into, so this (and your saved priority profiles, and the
  "items in your bags" new-item tracking) survive updating to a newer
  build — no re-entering it every version.
- **Import a grim_gleaner profile.** Click "Import grim_gleaner profile…"
  above the priority tabs and pick a profile JSON exported from
  grim_gleaner's own UI — its `weights` map ports over directly (same
  stat_id vocabulary, same vendored catalog), and a `resistance_cap_weights`
  override (when the source profile had Resistance Cap Mode on) is applied
  on top. grim_gleaner concepts this app doesn't score yet — per-skill
  weights and mastery selection — aren't silently dropped; the import
  summary reports exactly how many of each were skipped. The result
  replaces this character's current priorities and is saved immediately,
  same as setting stars by hand.
- **Items in your bags…** Click it in the Candidate Item panel to get a
  clickable list of every equippable item currently in your personal
  inventory bags — not just recently-found ones, so anything you've been
  carrying around is just as easy to compare as today's loot. Items new
  since your last click are flagged `NEW` and sorted to the top, but
  everything else stays listed right below them. No typing DBR paths for
  stuff that's already sitting in your bag. Not instant (it only sees
  what Grim Dawn has actually written to the save file — leaving an area,
  opening the menu, autosave, etc, not the moment you pick something up),
  and it only looks at your personal inventory bags, not the shared
  stash.

## Not built yet

- **Live item capture while playing.** The actual "hover an item, get an
  instant comparison, no clicking anything here" workflow needs either a
  parser for Grim Dawn's copy-to-clipboard item text (only available via
  linking an item into multiplayer chat — not usable in solo play, and
  the exact stat-line phrasing isn't documented anywhere to build a
  parser against without a real sample) or a live memory hook into the
  running game (the same category of technique overlay/trainer tools use
  — DLL injection, real crash risk, antivirus flags it, breaks on every
  game patch). "Items in your bags…" above is the safe middle ground:
  same end result once the save file catches up, none of that risk.
- **Per-skill priority and mastery scoring.** grim_gleaner profiles carry
  `skill_weights` and `masteries` for scoring skill-modifier items and
  mastery bonuses; importing a profile reports how many of each it found
  but doesn't act on them yet — this app currently only scores the flat
  stat `weights` map.

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
The binary lands in `target/release/gd-gear-compare.exe`. Copy `data/`
next to it before running (or run it once from the repo root — it looks
for that one relative to the exe's own location). Settings and profiles
live under `%APPDATA%\GD Gear Compare` instead, not next to the exe — see
"What it does" above.

## Using it

1. Run the `.exe`. It opens your browser to the compare UI.
2. Pick your character from the dropdown (auto-detected from
   `Documents\My Games\Grim Dawn\save\main`). If it's not found, you can
   still compare items manually.
3. Set your stat priorities (★0–4) for this character — saved automatically
   per character under `%APPDATA%\GD Gear Compare\profiles`.
4. Pick a gear slot to compare against, enter the candidate item's base
   name/prefix/suffix (as DBR record paths, e.g.
   `records/items/gearhead/d004_head.dbr`), and hit Compare.
5. Read the verdict: priority-score comparison plus a resistance/cap table
   showing exactly what the swap does to your totals.

## Troubleshooting a save that won't load

The vendored save-parser was written against one snapshot of Grim Dawn's
`.gdc` format; the game has moved on since (per-item UIDs, grown loot-filter
arrays, and other small drifts have already turned up in real saves). If
loading equipped gear fails with a `save parse failed: ...` error, that's
almost always this — some structure has one more field than the parser
expects, at some point in the file it hasn't hit before.

`src/bin/debug_parse.rs` is a small dev-only tool for chasing this down
locally instead of guessing blind against a release build:

```
cargo run --bin debug_parse --features trace-blocks -- "path\to\_YourCharacter\player.gdc"
```

It parses the file standalone (with a real console, unlike the shipped
app) and, with `trace-blocks` enabled, prints every block it enters/exits
with its tag, nesting depth, and byte position, plus a warning if any
string or list length decodes to a suspiciously large number — the
tell-tale sign of a misaligned read. The tag in a `block end position
(tag=N, ...)` error matches a `start_block*(N, ...)` call in exactly one
`vendor_save_parser/src/parser/model/*.rs` file, which is where to look.

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
