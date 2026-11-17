use std::collections::HashMap;
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::settings::Settings;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyEntry {
    pub action: String,
    pub keys: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyFailure {
    pub action: String,
    pub keys: String,
    pub error: String,
}

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyStatus {
    pub registered: Vec<HotkeyEntry>,
    pub failed: Vec<HotkeyFailure>,
    pub actual_toggle: Option<String>,
}

#[derive(Default)]
pub struct HotkeyState {
    actions: Mutex<HashMap<u32, String>>,
    status: Mutex<HotkeyStatus>,
}

impl HotkeyState {
    pub fn action(&self, id: u32) -> Option<String> {
        self.actions.lock().ok()?.get(&id).cloned()
    }

    fn set_report(&self, registered: Vec<HotkeyEntry>, failed: Vec<HotkeyFailure>, actual: Option<String>) {
        if let Ok(mut status) = self.status.lock() {
            status.registered = registered;
            status.failed = failed;
            status.actual_toggle = actual;
        }
    }
}

#[tauri::command]
pub fn hotkeys_status(state: tauri::State<'_, HotkeyState>) -> HotkeyStatus {
    state.status.lock().map(|status| status.clone()).unwrap_or_default()
}

fn candidates(settings: &Settings) -> Vec<(&'static str, String)> {
    vec![
        ("toggle", settings.hotkey.trim().to_string()),
        ("expand", settings.hotkey_expand.trim().to_string()),
        ("collapse", settings.hotkey_collapse.trim().to_string()),
        ("settings", settings.hotkey_settings.trim().to_string()),
        ("screenshot", settings.hotkey_screenshot.trim().to_string()),
        ("play_pause", settings.hotkey_play_pause.trim().to_string()),
        ("next", settings.hotkey_next.trim().to_string()),
        ("prev", settings.hotkey_prev.trim().to_string()),
    ]
}

pub fn apply(app: &AppHandle, settings: &Settings) -> HotkeyStatus {
    let state = app.state::<HotkeyState>();
    let _ = app.global_shortcut().unregister_all();
    if let Ok(mut actions) = state.actions.lock() {
        actions.clear();
    }

    let mut registered = Vec::new();
    let mut failed = Vec::new();
    let mut ids: HashMap<u32, String> = HashMap::new();
    let mut actual_toggle = None;

    for (action, keys) in candidates(settings) {
        if keys.is_empty() {
            continue;
        }
        let shortcut = match keys.parse::<Shortcut>() {
            Ok(shortcut) => shortcut,
            Err(error) => {
                failed.push(HotkeyFailure {
                    action: action.into(),
                    keys,
                    error: error.to_string(),
                });
                continue;
            }
        };
        if let Some(existing) = ids.get(&shortcut.id()) {
            failed.push(HotkeyFailure {
                action: action.into(),
                keys,
                error: format!("duplicate of {existing}"),
            });
            continue;
        }
        match app.global_shortcut().register(shortcut) {
            Ok(()) => {
                ids.insert(shortcut.id(), action.into());
                registered.push(HotkeyEntry {
                    action: action.into(),
                    keys: keys.clone(),
                });
                if action == "toggle" {
                    actual_toggle = Some(keys);
                }
            }
            Err(error) => {
                // Keep the island reachable even when the user's first choice
                // for the toggle shortcut is already taken by another app.
                if action == "toggle" {
                    let mut fallback = None;
                    for candidate in ["Alt+Shift+Space", "Ctrl+Alt+Space"] {
                        if candidate == keys {
                            continue;
                        }
                        let Ok(shortcut) = candidate.parse::<Shortcut>() else {
                            continue;
                        };
                        if ids.contains_key(&shortcut.id()) {
                            continue;
                        }
                        if app.global_shortcut().register(shortcut).is_ok() {
                            ids.insert(shortcut.id(), action.into());
                            registered.push(HotkeyEntry {
                                action: action.into(),
                                keys: candidate.into(),
                            });
                            fallback = Some(candidate.to_string());
                            break;
                        }
                    }
                    if let Some(candidate) = fallback {
                        actual_toggle = Some(candidate);
                        failed.push(HotkeyFailure {
                            action: action.into(),
                            keys,
                            error: format!("{error}; fallback registered"),
                        });
                        continue;
                    }
                }
                failed.push(HotkeyFailure {
                    action: action.into(),
                    keys,
                    error: error.to_string(),
                });
            }
        }
    }

    if let Ok(mut actions) = state.actions.lock() {
        actions.extend(ids);
    }
    state.set_report(registered.clone(), failed.clone(), actual_toggle.clone());
    let status = HotkeyStatus {
        registered,
        failed,
        actual_toggle,
    };
    let _ = tauri::Emitter::emit(app, "hotkeys-status", status.clone());
    status
}
