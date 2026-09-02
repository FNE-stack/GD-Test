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
  if (!res.ok) throw new Error(`${path} -> ${res.status}`);
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

// ---------- Character + slot selection ----------

async function loadCharacters() {
  const data = await api("/api/characters");
  const select = document.getElementById("character-select");
  select.innerHTML = "";
  if (!data.save_dir_found) {
    document.getElementById("save-dir-warning").hidden = false;
  }
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

async function loadEquipped() {
  if (!state.character) return;
  const data = await api(`/api/equipped/${encodeURIComponent(state.character)}`);
  state.equipped = data.items || [];
  state.baselineTotals = data.totals || {};
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

// ---------- Candidate item ----------

document.getElementById("resolve-candidate-btn").addEventListener("click", async () => {
  const base_name = document.getElementById("candidate-base").value.trim();
  const prefix_name = document.getElementById("candidate-prefix").value.trim();
  const suffix_name = document.getElementById("candidate-suffix").value.trim();
  if (!base_name) return alert("Enter a base item name/path first.");

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
  await loadEquipped();
  await loadProfile();
});

// ---------- init ----------
renderWeights();
loadTaxonomy().catch((err) => console.error("priority taxonomy load failed", err));
loadCharacters().catch((err) => {
  console.error(err);
  document.getElementById("save-dir-warning").hidden = false;
});
