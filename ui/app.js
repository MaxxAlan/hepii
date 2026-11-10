'use strict';
const inv = (c, a) => window.__TAURI__.core.invoke(c, a || {});
const $ = (id) => document.getElementById(id);
const KEYS = ['contentProtection','wdaBypass','sebKioskBypass','sebDesktopSwitch','sebLogFilter','sebClipboardBypass','sebLockscreenBypass','sebLogCleaner','launchAtStartup'];

function logline(s){
  const el = $('log');
  el.textContent = (s + '\n' + el.textContent).slice(0, 8000);
}

async function refresh(){
  try{
    const st = await inv('get_engine_status');
    $('dot').classList.toggle('ok', !!st.admin);
    $('status').textContent = JSON.stringify({
      admin: st.admin, openPatches: st.openPatches,
      patchedPids: st.patchedPids, livePids: st.livePids, routes: st.routes
    }, null, 2);
    for (const k of KEYS){
      const el = $(k);
      if (el && typeof st.settings[k] === 'boolean') el.checked = st.settings[k];
    }
  }catch(e){ logline('get_engine_status: ' + e); }
}

async function save(){
  const patch = {};
  for (const k of KEYS){ const el = $(k); if (el) patch[k] = el.checked; }
  try{ await inv('apply_settings', { patch }); logline('settings saved'); }
  catch(e){ logline('apply_settings: ' + e); }
}

document.addEventListener('DOMContentLoaded', () => {
  for (const k of KEYS){ const el = $(k); if (el) el.addEventListener('change', save); }
  document.querySelectorAll('[data-cmd]').forEach((b) => {
    b.addEventListener('click', async () => {
      const cmd = b.dataset.cmd;
      try{ const r = await inv(cmd); logline(cmd + ' -> ' + (r === undefined ? 'ok' : JSON.stringify(r))); }
      catch(e){ logline(cmd + ' -> ' + e); }
      refresh();
    });
  });
  $('admin').addEventListener('click', () => inv('restart_as_admin').catch((e) => logline(String(e))));
  $('check').addEventListener('click', async () => {
    try{ logline('self-check ' + JSON.stringify(await inv('run_self_check'))); }catch(e){ logline(String(e)); }
  });
  refresh();
  setInterval(refresh, 2000);
});
