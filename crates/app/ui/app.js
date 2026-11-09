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

/// One row: a Claude copy with its sign-in badge, a note, a warning and buttons.
function copyItem({ name, color, meta, copy, warning, open, extra = [] }) {
  const li = document.createElement("li");
  li.className = "card profile";

  const dot = document.createElement("span");
  dot.className = "dot";
  dot.style.background = color;

  const info = document.createElement("div");
  const title = document.createElement("div");
  title.className = "name";
  title.textContent = name;
  const [label, cls] = SIGN_IN[copy.sign_in];
  const badge = document.createElement("span");
  badge.className = `badge ${cls}`;
  badge.textContent = label;
  title.append(" ", badge);
  const sub = document.createElement("div");
  sub.className = "meta";
  sub.textContent = meta;
  info.append(title, sub);
  if (copy.same_account_as.length) {
    const warn = document.createElement("div");
    warn.className = "warn";
    warn.textContent = `Same account as ${copy.same_account_as.join(", ")}. ${warning}`;
    info.append(warn);
  }

  const openButton = document.createElement("button");
  openButton.className = "primary";
  openButton.textContent = "Open";
  openButton.addEventListener("click", async () => {
    await open();
    say(`Opening ${name}. If it's already open, its window comes to the front.`);
  });

  const actions = document.createElement("div");
  actions.className = "actions";
  actions.append(openButton, ...extra);
  li.append(dot, info, actions);
  return li;
}

// The stock copy's name in `same_account_as` (set by ops::Overview::names).
const MAIN = "main Claude";

/// Main Claude is not one of the accounts: shown apart, only while it has data, with a way to import it.
function showMain(main) {
  $("main-section").hidden = main.sign_in === "never-opened";
  const [label, cls] = SIGN_IN[main.sign_in];
  $("main-badge").textContent = label;
  $("main-badge").className = `badge ${cls}`;
  const signedIn = main.sign_in === "signed-in";
  const imported = main.same_account_as.length > 0;
  $("main-text").textContent = !signedIn
    ? "Claude Desktop outside Claude House. Not needed: every account can be a profile."
    : imported
      ? "Claude Desktop outside Claude House. Keep it closed and use the account here instead."
      : "Claude Desktop outside Claude House. Import it to keep its sign-in and Code sessions as an account here.";
  $("import-form").hidden = !signedIn || imported;
  const warn = $("main-warn");
  warn.hidden = !imported;
  warn.textContent = imported
    ? `Same account as ${main.same_account_as.join(", ")}. Running both can sign one of them out; quit Main Claude and turn off its "start with Windows" setting.`
    : "";
}

function profileItem(p) {
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

  return copyItem({
    name: p.name,
    color: p.color,
    meta: `${p.id} · Code tab ${p.isolate_code ? "separate" : "shared"}`,
    // Sharing with Main Claude is expected after an import; the Main Claude section covers it.
    copy: { ...p, same_account_as: p.same_account_as.filter((n) => n !== MAIN) },
    warning: "Sign this one out and in with a different account.",
    open: () => call("launch_profile", { id: p.id }),
    extra: [remove],
  });
}

let shown = "";

// Redraws only when something changed, so a half-confirmed Remove click survives the periodic refresh.
async function showProfiles() {
  const o = await call("overview");
  const key = JSON.stringify(o);
  if (key === shown) return;
  const countChanged = !shown || JSON.parse(shown).profiles.length !== o.profiles.length;
  shown = key;
  $("profiles").replaceChildren(...o.profiles.map(profileItem));
  showMain(o.main);
  $("empty").hidden = o.profiles.length > 0;
  if (countChanged) $("add-color").value = PALETTE[o.profiles.length % PALETTE.length];
}

$("main-open").addEventListener("click", async () => {
  await call("launch_main");
  say("Opening Main Claude.");
});

$("import-form").addEventListener("submit", async (e) => {
  e.preventDefault();
  const button = e.submitter;
  button.disabled = true;
  const name = $("import-name").value;
  say("Copying Main Claude's data. This can take a minute…");
  try {
    const done = await call("import_main", { id: slug(name) || "main", name, color: PALETTE[0] });
    say(`Imported as ${done}. Open it from the list; its Code sessions came along.`);
    shown = "";
    await showProfiles();
  } catch {
    // message already shown
  } finally {
    button.disabled = false;
  }
});

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
    const name = await call("add_profile", {
      id: $("add-id").value.trim(),
      name: $("add-name").value,
      color: $("add-color").value,
      isolateCode: $("add-isolate").checked,
    });
    e.target.reset();
    idEdited = false;
    say(`Added ${name}. Click Open, then sign in with that account.`);
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
