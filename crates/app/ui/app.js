// Claude House window. All rules run in Rust; this file only shows state and sends clicks.
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const $ = (id) => document.getElementById(id);
const PALETTE = ["#d97757", "#3366cc", "#2f9e6e", "#9b59b6", "#c0392b", "#d4a017"];

function say(text, isError = false) {
  const m = $("message");
  m.textContent = text;
  m.classList.toggle("error", isError);
}

async function call(cmd, args) {
  try {
    return await invoke(cmd, args);
  } catch (e) {
    say(String(e), true);
    throw e;
  }
}

// ---- Login routing ----

const OWNER_TEXT = {
  "claude-house": ["Google logins go to the profile that started them.", "status-ok"],
  "claude-desktop": ["Off. Google logins go to your normal Claude window.", ""],
  other: ["Another app handles claude:// links.", "status-warn"],
};

async function showRouting(view) {
  const v = view ?? (await call("routing_status"));
  const toggle = $("routing-toggle");
  toggle.checked = v.enabled;
  toggle.disabled = false;
  const [text, cls] = v.error
    ? [`Stopped: ${v.error}`, "status-warn"]
    : v.enabled && v.owner !== "claude-house"
      ? ["Taking claude:// links back from Claude…", "status-warn"]
      : OWNER_TEXT[v.owner];
  const p = $("routing-text");
  p.textContent = text;
  p.className = cls || "muted";
}

$("routing-toggle").addEventListener("change", async (e) => {
  const toggle = e.target;
  toggle.disabled = true;
  try {
    await showRouting(await call("set_routing", { enabled: toggle.checked }));
  } catch {
    await showRouting();
  }
});

// ---- Profiles ----

const SIGN_IN = {
  "signed-in": ["Signed in", "ok"],
  "signed-out": ["Not signed in", "todo"],
  "never-opened": ["Not opened yet", "todo"],
  unreadable: ["Status unknown", "todo"],
};

function profileItem(p) {
  const li = document.createElement("li");
  li.className = "card profile";

  const dot = document.createElement("span");
  dot.className = "dot";
  dot.style.background = p.color;

  const info = document.createElement("div");
  const name = document.createElement("div");
  name.className = "name";
  name.textContent = p.name;
  const [label, cls] = SIGN_IN[p.sign_in];
  const badge = document.createElement("span");
  badge.className = `badge ${cls}`;
  badge.textContent = label;
  name.append(" ", badge);
  const meta = document.createElement("div");
  meta.className = "meta";
  meta.textContent = `${p.id} · Code tab ${p.isolate_code ? "separate" : "shared"}`;
  info.append(name, meta);
  if (p.same_account_as.length) {
    const warn = document.createElement("div");
    warn.className = "warn";
    warn.textContent = `Same account as ${p.same_account_as.join(", ")}. Sign this one out and in with a different account.`;
    info.append(warn);
  }

  const open = document.createElement("button");
  open.className = "primary";
  open.textContent = "Open";
  open.addEventListener("click", async () => {
    await call("launch_profile", { id: p.id });
    say(`Opening ${p.name}. If it's already open, its window comes to the front.`);
  });

  const remove = document.createElement("button");
  remove.className = "danger";
  remove.textContent = "Remove";
  let armed;
  remove.addEventListener("click", async () => {
    if (!armed) {
      remove.textContent = "Click again to remove";
      remove.classList.add("confirm");
      armed = setTimeout(() => {
        armed = undefined;
        remove.textContent = "Remove";
        remove.classList.remove("confirm");
      }, 3000);
      return;
    }
    clearTimeout(armed);
    const kept = await call("remove_profile", { id: p.id });
    say(`Removed ${p.name}. Its sign-in data is kept in ${kept}`);
    await showProfiles();
  });

  const actions = document.createElement("div");
  actions.className = "actions";
  actions.append(open, remove);
  li.append(dot, info, actions);
  return li;
}

let shown = "";

// Redraws only when something changed, so a half-confirmed Remove click survives the periodic refresh.
async function showProfiles() {
  const list = await call("list_profiles");
  const key = JSON.stringify(list);
  if (key === shown) return;
  const countChanged = !shown || JSON.parse(shown).length !== list.length;
  shown = key;
  $("profiles").replaceChildren(...list.map(profileItem));
  $("empty").hidden = list.length > 0;
  if (countChanged) $("add-color").value = PALETTE[list.length % PALETTE.length];
}

// Sign-in happens in Claude's own windows, so look again now and then.
setInterval(() => { if (!document.hidden) showProfiles().catch(() => {}); }, 5000);
window.addEventListener("focus", () => showProfiles().catch(() => {}));

// ---- Add form ----

let idEdited = false;
const slug = (s) => s.normalize("NFKD").replace(/[̀-ͯ]/g, "").toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "").slice(0, 32).replace(/-+$/, "");

$("add-name").addEventListener("input", () => {
  if (!idEdited) $("add-id").value = slug($("add-name").value);
});
$("add-id").addEventListener("input", () => { idEdited = $("add-id").value !== ""; });

$("add-form").addEventListener("submit", async (e) => {
  e.preventDefault();
  const button = e.submitter;
  button.disabled = true;
  try {
    const p = await call("add_profile", {
      id: $("add-id").value.trim(),
      name: $("add-name").value,
      color: $("add-color").value,
      isolateCode: $("add-isolate").checked,
    });
    e.target.reset();
    idEdited = false;
    say(`Added ${p.name}. Click Open, then sign in with that account.`);
    await showProfiles();
  } catch {
    // message already shown
  } finally {
    button.disabled = false;
  }
});

listen("routing-changed", () => showRouting());
listen("error", (e) => say(String(e.payload), true));

showProfiles();
showRouting();
