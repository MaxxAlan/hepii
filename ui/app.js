/* Hepi frontend — gọi IPC Tauri 2, không bundler.
 * 3 panel: 1 Quiz (Gemini + ChatGPT web) · 2 Private (chống SEB) · 3 Status.
 */
const invoke = window.__TAURI__?.core?.invoke;
const win = window.__TAURI__?.window?.getCurrentWindow?.();

/* Panel 2 — mỗi toggle gắn 1 khoá trong Settings (camelCase). */
const TOGGLES = [
  ["sebKioskBypass", "Chặn SEB (kiosk bypass)"],
  ["sebPatchEnforcement", "Né giám sát (patch enforcement)"],
  ["sebLogFilter", "Chặn gửi log SEB"],
  ["sebLogCleaner", "Tự dọn log SEB"],
  ["sebClipboardBypass", "Bypass clipboard"],
  ["sebLockscreenBypass", "Bypass khoá màn hình"],
  ["sebDesktopSwitch", "Tự đổi desktop của SEB"],
  ["contentProtection", "Chặn quay/chụp màn hình"],
  ["wdaBypass", "Bypass WDA tiến trình khác"],
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
  if (!el) return;
  el.textContent =
    typeof value === "string" ? value : JSON.stringify(value, null, 2);
}

function badge(id, ok, okText, badText) {
  const el = $(id);
  if (!el) return;
  el.textContent = ok ? okText : badText;
  el.classList.toggle("ok", !!ok);
  el.classList.toggle("warn", !ok);
}

/* ---------- Panel 2: toggles ---------- */

function renderToggles() {
  const box = $("toggles");
  if (!box) return;
  box.innerHTML = "";
  for (const [key, label] of TOGGLES) {
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

async function save() {
  if (!invoke || !settings) return;
  try {
    await invoke("apply_settings", { patch: settings });
    show("private-out", "Đã lưu cấu hình.");
  } catch (err) {
    show("private-out", String(err));
  }
}

/* ---------- Panel 1: Quiz ---------- */

function chatAdd(role, content) {
  const box = $("quiz-chat");
  if (!box) return;
  const empty = box.querySelector(".empty");
  if (empty) empty.remove();
  const row = document.createElement("div");
  row.className = "msg " + role;
  const who = document.createElement("span");
  who.className = "who";
  who.textContent = role === "user" ? "Bạn" : "Gemini";
  const body = document.createElement("div");
  body.className = "body";
  body.textContent = content;
  row.append(who, body);
  box.append(row);
  box.scrollTop = box.scrollHeight;
}

async function ask() {
  const prompt = ($("quiz-prompt")?.value || "").trim();
  if (!prompt) {
    show("private-out", "Cần nhập câu hỏi.");
    return;
  }
  const system = ($("quiz-system")?.value || "").trim();
  const btn = $("btn-ask");
  if (btn) btn.disabled = true;
  chatAdd("user", prompt);
  try {
    let answer;
    if (shotB64) {
      // Có ảnh chụp ⇒ Gemini vision (đọc được đề/đáp án trên màn hình).
      answer = await invoke("quiz_ask_image", {
        prompt,
        imageB64: shotB64,
        system: system || null,
      });
    } else {
      answer = await invoke("quiz_ask", { prompt, system: system || null });
    }
    chatAdd("ai", answer);
  } catch (err) {
    chatAdd("ai", "Lỗi: " + String(err));
  } finally {
    if (btn) btn.disabled = false;
    if ($("quiz-prompt")) $("quiz-prompt").value = "";
    refreshStatus();
  }
}

/* ---------- Panel 1: chụp màn hình ---------- */

let shotB64 = null;

/** Bật overlay chọn vùng. Chụp xong trả về chuỗi base64 PNG. */
function pickRegion() {
  return new Promise((resolve) => {
    const picker = $("picker");
    const rect = $("picker-rect");
    if (!picker || !rect) return resolve(null);
    picker.hidden = false;
    let start = null;

    const onMove = (e) => {
      if (!start) return;
      const x = Math.min(start.x, e.clientX);
      const y = Math.min(start.y, e.clientY);
      const w = Math.abs(e.clientX - start.x);
      const h = Math.abs(e.clientY - start.y);
      rect.style.left = x + "px";
      rect.style.top = y + "px";
      rect.style.width = w + "px";
      rect.style.height = h + "px";
    };
    const onUp = async (e) => {
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);
      document.removeEventListener("keydown", onKey);
      picker.hidden = true;
      rect.style.width = "0px";
      rect.style.height = "0px";
      const x = Math.min(start.x, e.clientX);
      const y = Math.min(start.y, e.clientY);
      const w = Math.abs(e.clientX - start.x);
      const h = Math.abs(e.clientY - start.y);
      if (w < 8 || h < 8) return resolve(null);
      try {
        const b64 = await invoke("capture_preview", { x, y, w, h });
        resolve({ b64, x, y, w, h });
      } catch (err) {
        resolve(null);
      }
    };
    const onKey = (e) => {
      if (e.key === "Escape") {
        document.removeEventListener("mousemove", onMove);
        document.removeEventListener("mouseup", onUp);
        document.removeEventListener("keydown", onKey);
        picker.hidden = true;
        rect.style.width = "0px";
        rect.style.height = "0px";
        resolve(null);
      }
    };
    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", onUp);
    document.addEventListener("keydown", onKey);
    document.addEventListener(
      "mousedown",
      (e) => {
        start = { x: e.clientX, y: e.clientY };
      },
      { once: true }
    );
  });
}

async function captureScreen() {
  if (!invoke) return;
  const picked = await pickRegion();
  if (!picked) return;
  shotB64 = picked.b64;
  const img = $("shot-img");
  if (img) img.src = "data:image/png;base64," + picked.b64;
  const box = $("shot-box");
  if (box) box.hidden = false;
  const meta = $("shot-meta");
  if (meta) {
    meta.textContent = `Đã chụp ${picked.w}×${picked.h} tại (${picked.x}, ${picked.y}) — dùng ảnh này khi hỏi Gemini`;
  }
  // Ảnh chụp màn hình ở trên desktop mặc định; cần chụp vùng ở desktop SEB.
  chatAdd("user", "[đã đính kèm ảnh chụp màn hình]");
}

/* ---------- Panel 3: Status ---------- */

async function refreshStatus() {
  if (!invoke) {
    badge("gemini-badge", false, "", "IPC chưa sẵn sàng");
    return;
  }

  // AI
  try {
    const ai = await invoke("quiz_ai_status");
    badge("gemini-badge", ai.geminiKey, "đã có khoá", "thiếu GEMINI_API_KEY");
    text("st-gemini", ai.geminiKey ? "sẵn sàng" : "thiếu khoá");
    text("st-model", ai.geminiModel || "—");
  } catch {
    badge("gemini-badge", false, "", "lỗi");
  }

  // ChatGPT login (cookie phiên trong webview)
  try {
    const cg = await invoke("quiz_chatgpt_login");
    badge("chatgpt-badge", cg.loggedIn, "đã đăng nhập", "chưa đăng nhập");
    text("st-chatgpt", cg.loggedIn ? "đã đăng nhập" : "chưa đăng nhập");
  } catch {
    badge("chatgpt-badge", false, "", "lỗi");
    text("st-chatgpt", "lỗi");
  }

  // SEB
  try {
    const s = await invoke("seb_status");
    const p = s.processes || {};
    const one = (k) =>
      p[k] ? `${p[k].name} (pid ${p[k].pid})` : "không chạy";
    text("st-seb-client", one("client"));
    text("st-seb-runtime", one("runtime"));
    text("st-seb-browser", one("browser"));
    text("st-patched", (s.patchedPids || []).join(", ") || "—");
    text("st-open", String(s.openPatches ?? 0));
    text("st-guard", s.windowGuard ? "bật" : "tắt");
    text("st-logdir", s.logDir || "—");
    text("st-admin", s.admin ? "có" : "không");
    const pill = $("admin-pill");
    if (pill) {
      pill.textContent = s.admin ? "Administrator" : "Chưa nâng quyền";
      pill.classList.toggle("ok", !!s.admin);
      pill.classList.toggle("warn", !s.admin);
    }
    show("status-out", s);
  } catch (err) {
    show("status-out", String(err));
  }

  // Engine
  try {
    const e = await invoke("get_engine_status");
    text("st-live", (e.livePids || []).join(", ") || "—");
    text("st-routes", String(e.routes ?? 0));
    if (!settings) settings = e.settings;
    renderToggles();
  } catch (err) {
    show("status-out", String(err));
  }
}

/* ---------- Generic action runner ---------- */

async function run(cmd, outId = "private-out", args = {}) {
  if (!invoke) return;
  try {
    const out = await invoke(cmd, args);
    show(outId, out ?? "ok");
    refreshStatus();
  } catch (err) {
    show(outId, String(err));
  }
}

/* ---------- Wire up ---------- */

$("btn-ask")?.addEventListener("click", ask);
$("btn-capture")?.addEventListener("click", captureScreen);
$("btn-clear-quiz")?.addEventListener("click", () => {
  const box = $("quiz-chat");
  if (!box) return;
  box.innerHTML = '<p class="muted empty">Chưa có câu hỏi nào.</p>';
  shotB64 = null;
  const sb = $("shot-box");
  if (sb) sb.hidden = true;
});
$("btn-open-chatgpt")?.addEventListener("click", () =>
  run("quiz_open_chatgpt").then(refreshStatus)
);
$("btn-close-chatgpt")?.addEventListener("click", () =>
  run("quiz_close_chatgpt").then(refreshStatus)
);
$("btn-refresh-chatgpt")?.addEventListener("click", refreshStatus);

$("btn-apply-now")?.addEventListener("click", () => run("seb_apply_now"));
$("btn-clean-logs")?.addEventListener("click", () => run("scan_and_clean_seb_logs"));
$("btn-seb-desktop")?.addEventListener("click", () =>
  run("switch_to_seb_desktop")
);
$("btn-default-desktop")?.addEventListener("click", () =>
  run("switch_to_default_desktop")
);
$("btn-emergency")?.addEventListener("click", () =>
  run("trigger_emergency_exit")
);

$("btn-refresh-status")?.addEventListener("click", refreshStatus);
$("btn-min")?.addEventListener("click", () => win?.minimize());
$("btn-close")?.addEventListener("click", () => win?.close());

/* ---------- Panel 3: updater ---------- */

async function checkUpdate() {
  if (!invoke) return;
  try {
    const u = await invoke("check_update");
    text("st-update", u.available ? `có — v${u.version}` : "không");
    show("status-out", u);
  } catch (err) {
    text("st-update", "lỗi: " + String(err));
  }
}

$("btn-check-update")?.addEventListener("click", checkUpdate);
$("btn-install-update")?.addEventListener("click", async () => {
  if (!invoke) return;
  try {
    await invoke("install_update");
    show("status-out", "Đang cài bản cập nhật…");
  } catch (err) {
    show("status-out", String(err));
  }
});

async function boot() {
  if (!invoke) {
    badge("gemini-badge", false, "", "chạy trong Tauri");
    return;
  }
  try {
    settings = await invoke("get_settings");
  } catch {
    settings = {};
  }
  renderToggles();
  refreshStatus();
  setInterval(refreshStatus, 4000);
  text("st-ver", await appVersion());
}

/** Phiên bản app hiện tại (tauri app API nếu có, fallback theo build). */
async function appVersion() {
  try {
    const v = await window.__TAURI__?.app?.getVersion?.();
    if (v) return "v" + v;
  } catch {
    /* ignore */
  }
  return "v0.0.1";
}

boot();
