// GD Gear Compare — front end. Talks to the local Rust server's /api/* routes.

const SLOT_LABELS = [
  "Head", "Chest", "Legs", "Hands", "Feet", "Shoulders",
  "Belt", "Amulet", "Ring 1", "Ring 2", "Medal", "Weapon/Off-hand",
];

// Priority tabs/packages/stat labels are NOT hand-curated here — they're
// fetched from /api/priority-taxonomy, a direct port of grim_gleaner's own
// stats/registry.py (see data/priority_taxonomy.json), so this matches
// grim_gleaner's own categories 1:1 rather than an ad-hoc grouping.
let TAXONOMY = { tabs: [] }; // filled by loadTaxonomy()

// Resistances (the "Resistances" package inside grim_gleaner's "Defenses"
// tab) default to max priority (4 stars) — capping resist is close to a
// hard requirement in Grim Dawn before raw damage matters, so we don't
// make the user remember to set these. Populated once the taxonomy loads.
let RESISTANCE_STATS = [];

async function loadTaxonomy() {
  const data = await api("/api/priority-taxonomy");
  TAXONOMY = data;
  const defensesTab = TAXONOMY.tabs.find((t) => t.tab_id === "defenses");
  const resistPkg = defensesTab?.packages.find((p) => p.package_id === "defense_resistances");
  RESISTANCE_STATS = resistPkg ? resistPkg.stats.map((s) => s.stat_id) : [];
  renderWeights(); // nothing renders correctly until the taxonomy arrives
}

// A stat_id's label, if the taxonomy defines one (grim_gleaner's curated
// text); otherwise fall back to a mechanically prettified property_id.
function statLabel(id) {
  for (const tab of TAXONOMY.tabs) {
    for (const pkg of tab.packages) {
      const found = pkg.stats.find((s) => s.stat_id === id);
      if (found) return found.label;
    }
  }
  // skill_bonus/granted_item_skill stat_ids carry the actual skill's name
  // after the colon (see extract_stats in stats.rs) — already correctly
  // capitalized straight from the game's own data, so show it as-is
  // rather than running it through prettyStat, which would mangle
  // "skill_bonus:Blitz" into "Skill Bonus:blitz".
  const skillMatch = id.match(/^(skill_bonus|granted_item_skill):(.+)$/);
  if (skillMatch) {
    return skillMatch[1] === "granted_item_skill"
      ? `${skillMatch[2]} (granted skill)`
      : skillMatch[2];
  }
  return prettyStat(id);
}

let state = {
  character: null,
  equipped: [], // resolved items for the current character
  baselineTotals: {},
  selectedSlot: 0,
  weights: {}, // stat -> 0..4
  candidateItem: null,
};

async function api(path, opts) {
  const res = await fetch(path, opts);
  if (!res.ok) {
    // Every error response from this server is JSON { error: "..." } — surface
    // that actual message instead of just the status code, since the status
    // alone ("500") gives no way to tell a save-parser failure apart from a
    // missing file or a malformed request.
    let detail = "";
    try {
      const body = await res.json();
      if (body && body.error) detail = `: ${body.error}`;
    } catch {
      // response body wasn't JSON (or was empty) — fall back to just the status
    }
    throw new Error(`${path} -> ${res.status}${detail}`);
  }
  return res.json();
}

function el(tag, attrs = {}, children = []) {
  const node = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (k === "text") node.textContent = v;
    else if (k === "html") node.innerHTML = v;
    else node.setAttribute(k, v);
  }
  for (const c of children) node.appendChild(c);
  return node;
}

// Grim Dawn only writes player.gdc on specific triggers (autosave, leaving
// an area, opening the menu, quitting), never instantly on pickup/move/sell
// — so anything read from it can be showing a save from a while ago,
// including items you no longer actually have. Surfacing exactly when that
// save was written (rather than implying "live") is the whole fix for that
// confusion. Called after every /api/equipped and /api/bag-items response,
// both of which return the same save_file_mtime for whichever character
// was just read.
function showSaveFreshness(unixSecs) {
  const freshnessEl = document.getElementById("save-freshness");
  if (!unixSecs) {
    freshnessEl.textContent = "";
    return;
  }
  const when = new Date(unixSecs * 1000);
  const ageMin = Math.round((Date.now() - when.getTime()) / 60000);
  const ageText =
    ageMin < 1 ? "just now" : ageMin === 1 ? "1 min ago" : `${ageMin} min ago`;
  const timeText = when.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
  freshnessEl.textContent = `Save data as of ${timeText} (${ageText})`;
}

// ---------- Character + slot selection ----------

async function loadCharacters() {
  const data = await api("/api/characters");
  const select = document.getElementById("character-select");
  select.innerHTML = "";
  document.getElementById("save-dir-warning").hidden = data.save_dir_found;
  if (data.characters.length === 0) {
    select.appendChild(el("option", { text: "(none found)" }));
    return;
  }
  for (const name of data.characters) {
    select.appendChild(el("option", { value: name, text: name.replace(/^_/, "") }));
  }
  state.character = data.characters[0];
  await loadEquipped();
  await loadProfile();
}

// ---------- Settings (save folder override) ----------

function toggleSettingsPanel(show) {
  document.getElementById("settings-panel").hidden = !show;
}

document.getElementById("settings-btn").addEventListener("click", async () => {
  const panel = document.getElementById("settings-panel");
  const opening = panel.hidden;
  toggleSettingsPanel(opening);
  if (opening) {
    try {
      const data = await api("/api/settings");
      document.getElementById("save-dir-input").value = data.save_dir || "";
    } catch {
      // ignore — leave the field as-is
    }
  }
});

document.getElementById("open-settings-btn").addEventListener("click", () => {
  toggleSettingsPanel(true);
  document.getElementById("save-dir-input").focus();
});

document.getElementById("save-dir-apply-btn").addEventListener("click", async () => {
  const status = document.getElementById("save-dir-status");
  const path = document.getElementById("save-dir-input").value.trim();
  if (!path) {
    status.textContent = "Enter a folder path first.";
    return;
  }
  status.textContent = "Checking…";
  try {
    const res = await fetch("/api/settings/save-dir", {
      method: "POST",
      body: JSON.stringify({ path }),
    });
    const data = await res.json();
    if (!res.ok) {
      status.textContent = "❌ " + (data.error || "Could not use that folder.");
      return;
    }
    status.textContent = "✓ Save folder set. Reloading characters…";
    document.getElementById("save-dir-warning").hidden = true;
    await loadCharacters();
  } catch (err) {
    status.textContent = "❌ " + err.message;
  }
});

async function loadEquipped() {
  if (!state.character) return;
  const warning = document.getElementById("equipped-warning");
  warning.hidden = true;
  try {
    const data = await api(`/api/equipped/${encodeURIComponent(state.character)}`);
    state.equipped = data.items || [];
    state.baselineTotals = data.totals || {};
    showSaveFreshness(data.save_file_mtime);
  } catch (err) {
    // Previously unhandled here — a failure (e.g. the save parser choking on
    // this character's player.gdc) surfaced only as a console rejection,
    // while the slot list still rendered every slot as "(empty)", which
    // looks identical to genuinely having nothing equipped. Show the real
    // reason instead of silently pretending the character has no gear.
    console.error(err);
    state.equipped = [];
    state.baselineTotals = {};
    warning.hidden = false;
    warning.textContent = "Could not read this character's equipped gear — " + err.message;
  }
  renderSlotSelect();
  renderItemA();
}

function renderSlotSelect() {
  const select = document.getElementById("slot-select");
  select.innerHTML = "";
  for (let i = 0; i < 12; i++) {
    const has = state.equipped.find((it) => it.slot_index === i);
    const label = SLOT_LABELS[i] || `Slot ${i}`;
    select.appendChild(
      el("option", { value: i, text: has ? `${label} — ${has.display_name}` : `${label} (empty)` })
    );
  }
  select.value = state.selectedSlot;
}

function renderItemA() {
  const card = document.getElementById("item-a-card");
  const item = state.equipped.find((it) => it.slot_index === Number(state.selectedSlot));
  card.innerHTML = "";
  if (!item) {
    card.appendChild(el("p", { class: "empty", text: "Nothing equipped in this slot." }));
    return;
  }
  card.appendChild(el("p", { class: "item-name", text: item.display_name }));
  if (item.unresolved) {
    card.appendChild(el("p", { class: "warning", text: "Item not found in catalog data — stats unavailable." }));
  }
  renderStatLines(card, item.stats);
}

// ---------- Priority weights ----------

function ensureResistanceDefaults() {
  // Resistances default to 4 stars (max priority) unless the user has
  // already touched them — capping resist is close to a hard requirement
  // in Grim Dawn, so we don't make the user remember to set this.
  for (const stat of RESISTANCE_STATS) {
    if (!(stat in state.weights)) state.weights[stat] = 4;
  }
}

// Tabs come straight from the taxonomy (Damage, Defenses, Core, Advanced,
// Pets — grim_gleaner's own tab set), defaulting to Defenses since that's
// where Resistances lives and resistances are the always-relevant default.
let activeTabId = "defenses";

function renderWeights() {
  ensureResistanceDefaults();
  renderTabBar();
  renderTabBody();
}

function renderTabBar() {
  const bar = document.getElementById("weights-tabs");
  bar.innerHTML = "";
  for (const tab of TAXONOMY.tabs) {
    const btn = el("button", {
      type: "button",
      class: "tab-btn" + (tab.tab_id === activeTabId ? " active" : ""),
      text: tab.label,
    });
    btn.addEventListener("click", () => {
      activeTabId = tab.tab_id;
      renderTabBar();
      renderTabBody();
    });
    bar.appendChild(btn);
  }
}

function renderTabBody() {
  const list = document.getElementById("weights-list");
  list.innerHTML = "";

  const tab = TAXONOMY.tabs.find((t) => t.tab_id === activeTabId);
  if (!tab) {
    list.appendChild(el("p", { class: "hint", text: "Loading stat categories…" }));
    return;
  }

  for (const pkg of tab.packages) {
    list.appendChild(el("h3", { class: "weights-subhead", text: pkg.label }));
    for (const statDef of pkg.stats) {
      list.appendChild(weightRow(statDef.stat_id, statDef.label));
    }
  }
}

function weightRow(stat, label) {
  const isResist = RESISTANCE_STATS.includes(stat);
  const row = el("div", { class: "weight-row" });
  row.appendChild(el("span", { class: "stat-name", text: label || statLabel(stat) }));
  const stars = el("div", { class: "stars" });
  const current = state.weights[stat] || 0;
  for (let i = 1; i <= 4; i++) {
    const star = el("span", { class: "star" + (current >= i ? " filled" : ""), text: "★" });
    star.addEventListener("click", () => {
      state.weights[stat] = state.weights[stat] === i ? i - 1 : i;
      renderTabBody();
      saveProfile();
    });
    stars.appendChild(star);
  }
  row.appendChild(stars);
  if (!isResist && current > 0) {
    const remove = el("span", { class: "remove-stat", text: "✕", title: "Reset to 0" });
    remove.addEventListener("click", () => {
      delete state.weights[stat];
      renderTabBody();
      saveProfile();
    });
    row.appendChild(remove);
  }
  return row;
}

function prettyStat(id) {
  return id.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
}

async function loadProfile() {
  if (!state.character) return;
  try {
    const data = await api(`/api/profile/${encodeURIComponent(state.character)}`);
    state.weights = data.weights && Object.keys(data.weights).length ? data.weights : {};
  } catch {
    state.weights = {};
  }
  renderWeights();
}

async function saveProfile() {
  if (!state.character) return;
  await fetch(`/api/profile/${encodeURIComponent(state.character)}`, {
    method: "POST",
    body: JSON.stringify({ weights: state.weights }),
  });
}

// ---------- grim_gleaner profile import ----------
// Converts an exported grim_gleaner build-profile JSON file into this app's
// weights format server-side (see src/import.rs) and persists it as the
// current character's profile. grim_gleaner-only concepts this app doesn't
// use yet (skill_weights, masteries) are reported back in the summary
// rather than silently vanishing.

document.getElementById("import-gg-btn").addEventListener("click", () => {
  document.getElementById("import-gg-file").click();
});

document.getElementById("import-gg-file").addEventListener("change", async (e) => {
  const file = e.target.files[0];
  const status = document.getElementById("import-gg-status");
  if (!file) return;
  if (!state.character) {
    status.className = "status-error";
    status.textContent = "Pick a character first.";
    e.target.value = "";
    return;
  }
  status.className = "hint";
  status.textContent = "Importing…";
  try {
    const text = await file.text();
    const res = await fetch(
      `/api/profile/${encodeURIComponent(state.character)}/import-grim-gleaner`,
      { method: "POST", body: text }
    );
    const data = await res.json();
    if (!res.ok) {
      status.className = "status-error";
      status.textContent = "❌ " + (data.error || "Import failed.");
      return;
    }
    state.weights = data.weights || {};
    renderWeights();

    const s = data.summary || {};
    const notes = [];
    if (s.resistance_overrides_applied) {
      notes.push(`${s.resistance_overrides_applied} resistance-cap override(s) applied`);
    }
    if (s.skipped_skill_weight_count) {
      notes.push(`${s.skipped_skill_weight_count} skill weight(s) not imported (not supported yet)`);
    }
    if (s.skipped_mastery_count) {
      const word = s.skipped_mastery_count === 1 ? "mastery" : "masteries";
      notes.push(`${s.skipped_mastery_count} ${word} not imported (not supported yet)`);
    }
    if (s.invalid_weight_count) {
      notes.push(`${s.invalid_weight_count} invalid weight value(s) skipped`);
    }

    status.className = "status-ok";
    status.textContent =
      `✓ Imported ${s.imported_stat_count ?? Object.keys(state.weights).length} stat weight(s)` +
      ` from "${s.profile_name || file.name}"` +
      (notes.length ? " — " + notes.join("; ") : "");
  } catch (err) {
    status.className = "status-error";
    status.textContent = "❌ " + err.message;
  } finally {
    e.target.value = "";
  }
});

// ---------- Items in your bags ----------
// Lists every equippable item currently in the character's stash/backpack
// — not just recently-picked-up ones, so anything you've been carrying
// around is just as pickable as today's loot. Items new since the last
// time this ran (tracked server-side per character) are flagged and
// sorted to the top so a fresh haul is still easy to spot; everything
// else is still right there below it, not hidden. Not instant — it only
// sees what Grim Dawn has actually written to the save file (autosave,
// leaving an area, opening the menu, etc), not the moment an item is
// picked up. Clicking a result fills in the manual fields below and
// resolves it as the candidate, same as typing the paths in by hand.

document.getElementById("browse-bag-items-btn").addEventListener("click", async () => {
  const status = document.getElementById("bag-items-status");
  const list = document.getElementById("bag-items-list");
  if (!state.character) {
    status.className = "hint";
    status.textContent = "Pick a character first.";
    return;
  }
  status.className = "hint";
  status.textContent = "Reading your bags…";
  list.hidden = true;
  list.innerHTML = "";
  try {
    const data = await api(`/api/bag-items/${encodeURIComponent(state.character)}`);
    showSaveFreshness(data.save_file_mtime);
    if (!data.items || data.items.length === 0) {
      status.className = "hint";
      status.textContent = "Nothing equippable found in your bags.";
      return;
    }
    const newCount = data.items.filter((item) => item.is_new).length;
    status.className = "status-ok";
    status.textContent =
      `${data.items.length} item(s)` +
      (newCount > 0 ? ` — ${newCount} new since last check` : "") +
      " — click one to compare it:";
    list.hidden = false;
    for (const item of data.items) {
      const label = el("span", { text: item.display_name || item.base_name });
      const rowChildren = [label];
      if (item.is_new) {
        rowChildren.push(el("span", { class: "new-badge", text: "NEW" }));
      }
      rowChildren.push(el("button", { type: "button", class: "use-btn", text: "Use as candidate" }));
      const row = el("div", { class: "new-item-row" }, rowChildren);
      row.addEventListener("click", () => {
        document.getElementById("candidate-base").value = item.base_name || "";
        document.getElementById("candidate-prefix").value = item.prefix_name || "";
        document.getElementById("candidate-suffix").value = item.suffix_name || "";
        state.candidateItem = {
          display_name: item.display_name,
          stats: item.stats || {},
          unresolved: item.unresolved,
        };
        renderItemB();
      });
      list.appendChild(row);
    }
  } catch (err) {
    console.error(err);
    status.className = "status-error";
    status.textContent = "❌ " + err.message;
  }
});

// ---------- Candidate item ----------

document.getElementById("resolve-candidate-btn").addEventListener("click", async () => {
  const base_name = document.getElementById("candidate-base").value.trim();
  const prefix_name = document.getElementById("candidate-prefix").value.trim();
  const suffix_name = document.getElementById("candidate-suffix").value.trim();
  if (!base_name) return alert("Enter a base item name/path first.");

  try {
    const item = await api("/api/resolve-item", {
      method: "POST",
      body: JSON.stringify({
        slot_index: 0,
        base_name, prefix_name, suffix_name,
        modifier_name: "", relic_bonus: "", component_name: "", augment_name: "",
      }),
    });
    state.candidateItem = item;
    renderItemB();
  } catch (err) {
    console.error(err);
    alert("Could not resolve that item — " + err.message);
  }
});

function renderItemB() {
  const card = document.getElementById("item-b-card");
  card.innerHTML = "";
  const item = state.candidateItem;
  if (!item) {
    card.appendChild(el("p", { class: "empty", text: "No candidate item yet." }));
    return;
  }
  card.appendChild(el("p", { class: "item-name", text: item.display_name }));
  if (item.unresolved) {
    card.appendChild(el("p", { class: "warning", text: "Item not found in catalog data — check the spelling/path." }));
  }
  renderStatLines(card, item.stats);
}

function renderStatLines(card, stats) {
  const entries = Object.entries(stats).sort((a, b) => Math.abs(b[1]) - Math.abs(a[1]));
  for (const [stat, value] of entries) {
    card.appendChild(
      el("div", { class: "stat-line" }, [
        el("span", { text: statLabel(stat) }),
        el("span", { text: (value > 0 ? "+" : "") + value.toFixed(1) }),
      ])
    );
  }
}

// ---------- Compare ----------

document.getElementById("slot-select").addEventListener("change", (e) => {
  state.selectedSlot = Number(e.target.value);
  renderItemA();
});

document.getElementById("compare-btn").addEventListener("click", async () => {
  const itemA = state.equipped.find((it) => it.slot_index === Number(state.selectedSlot));
  const itemB = state.candidateItem;
  if (!itemB) return alert("Resolve a candidate item first.");

  try {
    const result = await api("/api/compare", {
      method: "POST",
      body: JSON.stringify({
        weights: state.weights,
        baseline_totals: state.baselineTotals,
        item_a_stats: (itemA && itemA.stats) || {},
        item_b_stats: itemB.stats || {},
      }),
    });

    renderVerdict(result, itemA, itemB);
    renderResistTable(result.resist_impact);
  } catch (err) {
    console.error(err);
    alert("Compare failed — " + err.message);
  }
});

function renderVerdict(result, itemA, itemB) {
  const box = document.getElementById("verdict");
  box.hidden = false;

  const dangerous = result.resist_impact.some((r) => r.dangerous);
  const scoreDelta = result.item_b.score - result.item_a.score;
  const overcapCount = result.resist_impact.filter((r) => r.over_cap).length;

  // Resistances are always top priority: any resist that drops and ends
  // *under* cap after the swap counts against the item regardless of how
  // good its damage score looks. Overcap resist loss (you were wasting the
  // excess anyway) doesn't count against it.
  const uncappedResistLoss = result.resist_impact
    .filter((r) => r.delta < 0 && !r.was_over_cap_before)
    .reduce((sum, r) => sum - r.delta, 0);

  let cls = "keep-a";
  let headline = `Keep "${itemA ? itemA.display_name : "current item"}"`;

  if (dangerous) {
    cls = "danger";
    headline = "Caution — this swap leaves a resistance at or below 0%";
  } else if (uncappedResistLoss >= 10) {
    cls = "keep-a";
    headline = `Keep "${itemA ? itemA.display_name : "current item"}" — resistance drop outweighs the damage gain`;
  } else if (scoreDelta > 5) {
    cls = "equip-b";
    headline = `Equip "${itemB.display_name}"`;
  } else if (scoreDelta < -5) {
    cls = "keep-a";
    headline = `Keep "${itemA ? itemA.display_name : "current item"}"`;
  } else {
    cls = "keep-a";
    headline = "Roughly a wash — close call, check resistances below";
  }

  box.className = "verdict " + cls;
  box.innerHTML = "";
  box.appendChild(el("h3", { text: headline }));
  box.appendChild(
    el("p", {
      text: `Priority score: ${result.item_a.grade} (${result.item_a.score.toFixed(0)}) vs ${result.item_b.grade} (${result.item_b.score.toFixed(0)})` +
        (overcapCount ? ` · ${overcapCount} resistance(s) pushed over cap` : ""),
    })
  );
}

function renderResistTable(rows) {
  const wrap = document.getElementById("resist-table-wrap");
  const tbody = document.querySelector("#resist-table tbody");
  tbody.innerHTML = "";
  if (!rows.length) {
    wrap.hidden = true;
    return;
  }
  wrap.hidden = false;
  for (const r of rows) {
    const flag = r.dangerous ? "Dangerous" : r.over_cap ? "Over cap (wasted)" : "OK";
    const flagClass = r.dangerous ? "flag-dangerous" : r.over_cap ? "flag-overcap" : "flag-ok";
    tbody.appendChild(
      el("tr", {}, [
        el("td", { text: prettyStat(r.stat) }),
        el("td", { text: r.current_total.toFixed(0) + "%" }),
        el("td", { text: r.after_total.toFixed(0) + "%" }),
        el("td", { text: (r.delta > 0 ? "+" : "") + r.delta.toFixed(0) }),
        el("td", { text: flag, class: flagClass }),
      ])
    );
  }
}

document.getElementById("character-select").addEventListener("change", async (e) => {
  state.character = e.target.value;
  state.selectedSlot = 0;
  document.getElementById("bag-items-status").textContent = "";
  document.getElementById("bag-items-list").hidden = true;
  document.getElementById("bag-items-list").innerHTML = "";
  await loadEquipped();
  await loadProfile();
});

// ---------- Auto-shutdown ----------
// This server backs one local browser tab, not a long-running service —
// left running after the tab closes, it just sits there until someone
// remembers to kill it in Task Manager (and holds the port in the
// meantime, so a freshly-launched build silently fails to start). Two
// mechanisms, matching the two ways "the page is gone" actually happens:
// - pagehide fires reliably when the tab is closed/navigated away, so
//   sendBeacon here (fire-and-forget, survives the page tearing down,
//   unlike a plain fetch) shuts the server down near-instantly.
// - The heartbeat is a fallback for anything that skips pagehide (a
//   browser crash, the browser process being force-killed) — see
//   HEARTBEAT_TIMEOUT in server.rs for why its timeout is generous.
window.addEventListener("pagehide", () => {
  navigator.sendBeacon("/api/shutdown");
});
setInterval(() => {
  fetch("/api/heartbeat", { method: "POST" }).catch(() => {});
}, 5000);

// ---------- init ----------
renderWeights();
loadTaxonomy().catch((err) => console.error("priority taxonomy load failed", err));
loadCharacters().catch((err) => {
  console.error(err);
  document.getElementById("save-dir-warning").hidden = false;
});
