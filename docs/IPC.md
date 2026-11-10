# IPC - command contract (legacy app)

Source: command names present in both frontend JS and `hepi.exe` strings; call snippets in
`docs/raw/ipc-calls.tsv`. Params column lists only literal keys observed in the invoke payload;
no type is inferred. Return type is `UNKNOWN` unless the frontend reveals a schema (GAP-02).

Totals: 86 name candidates, 85 confirmed as real `invoke` commands (1 row per command, 85 rows).
`network_error` was rejected: no `invoke` call site, it only matched `ENTITLEMENT_NETWORK_ERROR`.

| # | Command | Params (observed) | Returns | Description (name-derived) |
|---|---|---|---|---|
| 1 | `apply_app_identity` | request | UNKNOWN | apply app identity |
| 2 | `apply_lifecycle_settings` | settings, keepRunningInTray | UNKNOWN | apply lifecycle settings |
| 3 | `capture_persistent_region` | x, y, width, height | UNKNOWN | capture persistent region |
| 4 | `check_admin_status` | processNames | UNKNOWN | check admin status |
| 5 | `clear_entitlement_receipt` | no args | UNKNOWN | clear entitlement receipt |
| 6 | `close_ai_response_window` | no args | UNKNOWN | close ai response window |
| 7 | `close_region_selector` | no args | UNKNOWN | close region selector |
| 8 | `diagnostics_delete_report` | reportId | UNKNOWN | diagnostics delete report |
| 9 | `diagnostics_list_pending_reports` | no args | UNKNOWN | diagnostics list pending reports |
| 10 | `diagnostics_mark_clean_exit` | sessionId | UNKNOWN | diagnostics mark clean exit |
| 11 | `diagnostics_mark_report_uploaded` | reportId | UNKNOWN | diagnostics mark report uploaded |
| 12 | `diagnostics_open_log_directory` | no args | UNKNOWN | diagnostics open log directory |
| 13 | `diagnostics_save_js_crash_report` | report | UNKNOWN | diagnostics save js crash report |
| 14 | `get_active_tab` | no args | UNKNOWN | get active tab |
| 15 | `get_click_through_snapshot` | no args | UNKNOWN | get click through snapshot |
| 16 | `get_clipboard_bypass_monitor_status` | no args | UNKNOWN | get clipboard bypass monitor status |
| 17 | `get_detected_programs` | no args | UNKNOWN | get detected programs |
| 18 | `get_entitlement_status` | no args | UNKNOWN | get entitlement status |
| 19 | `get_help_panel_visible` | no args | UNKNOWN | get help panel visible |
| 20 | `get_kiosk_monitor_status` | no args | UNKNOWN | get kiosk monitor status |
| 21 | `get_lockscreen_bypass_monitor_status` | no args | UNKNOWN | get lockscreen bypass monitor status |
| 22 | `get_log_filter_monitor_status` | no args | UNKNOWN | get log filter monitor status |
| 23 | `get_patch_enforcement_monitor_status` | no args | UNKNOWN | get patch enforcement monitor status |
| 24 | `get_program_detection_state` | no args | UNKNOWN | get program detection state |
| 25 | `get_quiz_answer_overlay_state` | no args | UNKNOWN | get quiz answer overlay state |
| 26 | `get_quiz_answer_overlay_window_position` | no args | UNKNOWN | get quiz answer overlay window position |
| 27 | `get_rust_shortcuts` | no args | UNKNOWN | get rust shortcuts |
| 28 | `get_seb_log_cleaner_status` | destination | UNKNOWN | get seb log cleaner status |
| 29 | `get_text_grabber_settings` | no args | UNKNOWN | get text grabber settings |
| 30 | `get_trial_device_proof` | nonce | UNKNOWN | get trial device proof |
| 31 | `get_wda_monitor_status` | no args | UNKNOWN | get wda monitor status |
| 32 | `hide_from_task_switcher` | hide | UNKNOWN | hide from task switcher |
| 33 | `hide_persistent_overlay` | no args | UNKNOWN | hide persistent overlay |
| 34 | `hide_quiz_answer_overlay` | clearAnswer | UNKNOWN | hide quiz answer overlay |
| 35 | `hide_to_tray` | sessionId | UNKNOWN | hide to tray |
| 36 | `open_ai_response_window` | no args | UNKNOWN | open ai response window |
| 37 | `read_files_base64` | paths | UNKNOWN | read files base64 |
| 38 | `reset_media_permission` | origin | UNKNOWN | reset media permission |
| 39 | `reset_quiz_answer_overlay_position` | no args | UNKNOWN | reset quiz answer overlay position |
| 40 | `scan_and_clean_seb_logs` | destination | UNKNOWN | scan and clean seb logs |
| 41 | `select_question_number` | number | UNKNOWN | select question number |
| 42 | `set_active_tab` | tab | UNKNOWN | set active tab |
| 43 | `set_content_protection` | enabled | UNKNOWN | set content protection |
| 44 | `set_ghost_input_mode` | active | UNKNOWN | set ghost input mode |
| 45 | `set_kiosk_allow_switch_desktop` | enabled | UNKNOWN | set kiosk allow switch desktop |
| 46 | `set_program_detection_state` | active, program | UNKNOWN | set program detection state |
| 47 | `set_sidekick_active` | active | UNKNOWN | set sidekick active |
| 48 | `set_topmost_keeper_enabled` | enabled | UNKNOWN | set topmost keeper enabled |
| 49 | `set_user_click_through` | enabled | UNKNOWN | set user click through |
| 50 | `set_window_virtually_hidden` | hidden | UNKNOWN | set window virtually hidden |
| 51 | `show_from_tray` | no args | UNKNOWN | show from tray |
| 52 | `show_persistent_overlay` | x, y, width, height | UNKNOWN | show persistent overlay |
| 53 | `show_quiz_answer_overlay` | answer, customPosition, opacity, fontSize, durationSeconds, resetTimer | UNKNOWN | show quiz answer overlay |
| 54 | `show_region_selector` | hideVisuals | UNKNOWN | show region selector |
| 55 | `start_clean_uninstall` | no args | UNKNOWN | start clean uninstall |
| 56 | `start_clipboard_bypass_monitor` | no args | UNKNOWN | start clipboard bypass monitor |
| 57 | `start_key_hold_capture` | hideVisuals | UNKNOWN | start key hold capture |
| 58 | `start_kiosk_monitor` | no args | UNKNOWN | start kiosk monitor |
| 59 | `start_lockscreen_bypass_monitor` | no args | UNKNOWN | start lockscreen bypass monitor |
| 60 | `start_log_filter_monitor` | no args | UNKNOWN | start log filter monitor |
| 61 | `start_patch_enforcement_monitor` | no args | UNKNOWN | start patch enforcement monitor |
| 62 | `start_quiz_answer_overlay_reposition` | sampleAnswer, customPosition, opacity, fontSize | UNKNOWN | start quiz answer overlay reposition |
| 63 | `start_seb_log_cleaner_monitor` | no args | UNKNOWN | start seb log cleaner monitor |
| 64 | `start_system_audio_capture` | no args | UNKNOWN | start system audio capture |
| 65 | `start_text_grabber` | no args | UNKNOWN | start text grabber |
| 66 | `start_typing_text` | text | UNKNOWN | start typing text |
| 67 | `start_wda_monitor` | processNames | UNKNOWN | start wda monitor |
| 68 | `stop_clipboard_bypass_monitor` | no args | UNKNOWN | stop clipboard bypass monitor |
| 69 | `stop_kiosk_monitor` | no args | UNKNOWN | stop kiosk monitor |
| 70 | `stop_lockscreen_bypass_monitor` | no args | UNKNOWN | stop lockscreen bypass monitor |
| 71 | `stop_log_filter_monitor` | no args | UNKNOWN | stop log filter monitor |
| 72 | `stop_patch_enforcement_monitor` | no args | UNKNOWN | stop patch enforcement monitor |
| 73 | `stop_seb_log_cleaner_monitor` | no args | UNKNOWN | stop seb log cleaner monitor |
| 74 | `stop_text_grabber` | settings | UNKNOWN | stop text grabber |
| 75 | `stop_wda_monitor` | processName | UNKNOWN | stop wda monitor |
| 76 | `switch_to_default_desktop` | no args | UNKNOWN | switch to default desktop |
| 77 | `switch_to_seb_desktop` | enabled | UNKNOWN | switch to seb desktop |
| 78 | `sync_ai_response_window_position` | no args | UNKNOWN | sync ai response window position |
| 79 | `sync_help_overlay` | no args | UNKNOWN | sync help overlay |
| 80 | `toggle_click_through` | enabled | UNKNOWN | toggle click through |
| 81 | `toggle_ghost_mode_with_shortcuts` | no args | UNKNOWN | toggle ghost mode with shortcuts |
| 82 | `toggle_help_panel` | no args | UNKNOWN | toggle help panel |
| 83 | `update_global_shortcuts` | oldAccelerators, newAccelerators | UNKNOWN | update global shortcuts |
| 84 | `update_text_grabber_settings` | settings | UNKNOWN | update text grabber settings |
| 85 | `write_attachment_preview_temp` | filename, bytes | UNKNOWN | write attachment preview temp |
