// GD Gear Compare — front end. Talks to the local Rust server's /api/* routes.

const SLOT_LABELS = [
  "Head", "Chest", "Legs", "Hands", "Feet", "Shoulders",
  "Belt", "Amulet", "Ring 1", "Ring 2", "Medal", "Weapon/Off-hand",
];

const DEFAULT_STATS = [
  "fire_resistance", "cold_resistance", "lightning_resistance",
  "aether_resistance", "chaos_resistance", "vitality_resistance",
  "poison_acid_resistance", "physical_resistance", "pierce_resistance",
  "total_health", "armor", "defensive_ability", "offensive_ability",
];

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

function renderWeights() {
  const list = document.getElementById("weights-list");
  list.innerHTML = "";
  const stats = Object.keys(state.weights).length ? Object.keys(state.weights) : DEFAULT_STATS;
  for (const stat of stats) {
    if (!(stat in state.weights)) state.weights[stat] = 0;
    list.appendChild(weightRow(stat));
  }
}

function weightRow(stat) {
  const row = el("div", { class: "weight-row" });
  row.appendChild(el("span", { class: "stat-name", text: prettyStat(stat) }));
  const stars = el("div", { class: "stars" });
  for (let i = 1; i <= 4; i++) {
    const star = el("span", { class: "star" + (state.weights[stat] >= i ? " filled" : ""), text: "★" });
    star.addEventListener("click", () => {
      state.weights[stat] = state.weights[stat] === i ? i - 1 : i;
      renderWeights();
      saveProfile();
    });
    stars.appendChild(star);
  }
  row.appendChild(stars);
  return row;
}

function prettyStat(id) {
  return id.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
}

document.getElementById("add-stat-btn").addEventListener("click", () => {
  const name = prompt("Stat property_id (e.g. fire_resistance, offensive_ability):");
  if (name) {
    state.weights[name.trim()] = 1;
    renderWeights();
    saveProfile();
  }
});

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
        el("span", { text: prettyStat(stat) }),
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

  let cls = "keep-a";
  let headline = `Keep "${itemA ? itemA.display_name : "current item"}"`;

  if (dangerous) {
    cls = "danger";
    headline = "Caution — this swap leaves a resistance at or below 0%";
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
loadCharacters().catch((err) => {
  console.error(err);
  document.getElementById("save-dir-warning").hidden = false;
});
