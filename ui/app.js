/* Hepi frontend — gọi IPC Tauri 2, không bundler. */
const invoke = window.__TAURI__?.core?.invoke;
const win = window.__TAURI__?.window?.getCurrentWindow?.();

const FLAGS = [
  ["contentProtection", "Bảo vệ nội dung"],
  ["wdaBypass", "WDA"],
  ["sebKioskBypass", "Kiosk"],
  ["sebDesktopSwitch", "Desktop switch"],
  ["sebLogFilter", "Log filter"],
  ["sebPatchEnforcement", "Patch enforcement"],
  ["sebClipboardBypass", "Clipboard"],
  ["sebLockscreenBypass", "Lockscreen"],
  ["sebLogCleaner", "Log cleaner"],
  ["launchAtStartup", "Chạy cùng Windows"],
  ["keepRunningInTray", "Giữ trong tray"],
];

let settings = null;

function $(id) {
  return document.getElementById(id);
}

function text(id, value) {
  const el = $(id);
  if (el) el.textContent = value;
}

function show(id, value) {
  const el = $(id);
  if (el) el.textContent = typeof value === "string" ? value : JSON.stringify(value, null, 2);
}

function renderToggles() {
  const box = $("toggles");
  box.innerHTML = "";
  for (const [key, label] of FLAGS) {
    const row = document.createElement("label");
    row.className = "toggle";
    const name = document.createElement("span");
    name.textContent = label;
    const input = document.createElement("input");
    input.type = "checkbox";
    input.checked = !!settings?.[key];
    input.addEventListener("change", () => {
      settings[key] = input.checked;
      save();
    });
    row.append(name, input);
    box.append(row);
  }
}

function renderPrograms(names) {
  const box = $("programs");
  box.innerHTML = "";
  const selected = new Set(settings?.selectedPrograms ?? []);
  for (const name of names) {
    const chip = document.createElement("span");
    chip.className = "chip" + (selected.has(name) ? " on" : "");
    chip.textContent = name;
    chip.addEventListener("click", () => {
      if (selected.has(name)) selected.delete(name);
      else selected.add(name);
      settings.selectedPrograms = [...selected];
      renderPrograms(names);
      save();
    });
    box.append(chip);
  }
}

async function save() {
  if (!invoke || !settings) return;
  try {
    await invoke("apply_settings", { patch: settings });
  } catch (err) {
    show("action-out", String(err));
  }
}

async function refresh() {
  if (!invoke) {
    text("admin-pill", "IPC chưa sẵn sàng");
    return;
  }
  try {
    const st = await invoke("get_engine_status");
    settings = settings ?? st.settings;
    text("st-admin", st.admin ? "có" : "không");
    text("st-pids", st.patchedPids?.join(", ") || "—");
    text("st-open", String(st.openPatches ?? 0));
    text("st-routes", String(st.routes ?? 0));
    const pill = $("admin-pill");
    pill.textContent = st.admin ? "Administrator" : "Chưa nâng quyền";
    pill.classList.toggle("ok", !!st.admin);
    pill.classList.toggle("warn", !st.admin);
    renderToggles();
  } catch (err) {
    text("admin-pill", "lỗi IPC");
    show("action-out", String(err));
  }
}

async function scan() {
  if (!invoke) return;
  try {
    const names = await invoke("get_detected_programs");
    renderPrograms(names);
  } catch (err) {
    show("action-out", String(err));
  }
}

$("btn-refresh")?.addEventListener("click", refresh);
$("btn-save")?.addEventListener("click", save);
$("btn-scan")?.addEventListener("click", scan);
$("btn-min")?.addEventListener("click", () => win?.minimize());
$("btn-close")?.addEventListener("click", () => win?.close());
$("btn-selfcheck")?.addEventListener("click", async () => {
  try {
    show("selfcheck", await invoke("run_self_check"));
  } catch (err) {
    show("selfcheck", String(err));
  }
});

for (const btn of document.querySelectorAll("[data-cmd]")) {
  btn.addEventListener("click", async () => {
    try {
      const out = await invoke(btn.dataset.cmd);
      show("action-out", out ?? "ok");
      refresh();
    } catch (err) {
      show("action-out", String(err));
    }
  });
}

async function boot() {
  if (!invoke) {
    text("admin-pill", "Chạy trong Tauri");
    return;
  }
  try {
    settings = await invoke("get_settings");
  } catch {
    settings = {};
  }
  renderToggles();
  refresh();
  scan();
}

boot();
