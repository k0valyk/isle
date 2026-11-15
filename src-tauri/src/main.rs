#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(windows)]
mod autostart;
#[cfg(windows)]
mod audio;
#[cfg(windows)]
mod battery;
#[cfg(windows)]
mod capture;
#[cfg(windows)]
mod cursor;
#[cfg(windows)]
mod media;
#[cfg(windows)]
mod network;
#[cfg(windows)]
mod sound;
#[cfg(windows)]
mod stats;

#[cfg(not(windows))]
#[path = "macos/autostart.rs"]
mod autostart;
#[cfg(not(windows))]
#[path = "macos/audio.rs"]
mod audio;
#[cfg(not(windows))]
#[path = "macos/battery.rs"]
mod battery;
#[cfg(not(windows))]
#[path = "macos/capture.rs"]
mod capture;
#[cfg(not(windows))]
#[path = "macos/cursor.rs"]
mod cursor;
#[cfg(not(windows))]
#[path = "macos/media.rs"]
mod media;
#[cfg(not(windows))]
#[path = "macos/network.rs"]
mod network;
#[cfg(not(windows))]
#[path = "macos/sound.rs"]
mod sound;
#[cfg(not(windows))]
#[path = "macos/stats.rs"]
mod stats;

mod agents;
mod artwork;
mod chat;
mod db;
mod downloads;
mod hotkeys;
mod lyrics;
mod phone;
mod productivity;
mod settings;
mod terminal;
mod translate;
mod weather;
mod window_util;

use std::sync::Mutex;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::ShortcutState;

use db::Database;
use settings::Settings;
use window_util::IslandLayout;

pub struct AppState {
    pub settings: Mutex<Settings>,
    pub db: Database,
    pub island_layout: Mutex<Option<IslandLayout>>,
}

fn toggle_island(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        window_util::mark_tool_window(&window);
        let _ = app.emit_to("main", "island-toggle", ());
    }
}

fn prepare_settings(window: &tauri::WebviewWindow) {
    // The settings window is created with a tiny idle page so the hidden
    // WebView2 renderer does not load the whole Svelte app until first use.
    let idle = window
        .url()
        .map(|url| url.path().contains("settings-idle"))
        .unwrap_or(false);
    if idle {
        let _ = window.eval("window.location.replace('/index.html')");
    }
}

fn show_settings(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("settings") {
        prepare_settings(&window);
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        let _ = window.eval(
            "requestAnimationFrame(()=>{document.documentElement.style.opacity='0.999';requestAnimationFrame(()=>{document.documentElement.style.opacity='1';});});",
        );
    }
}

#[tauri::command]
fn app_info() -> serde_json::Value {
    serde_json::json!({ "version": env!("CARGO_PKG_VERSION") })
}

#[tauri::command]
fn open_settings(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("settings")
        .ok_or_else(|| "settings window is missing".to_string())?;
    prepare_settings(&window);
    window.show().map_err(|error| error.to_string())?;
    let _ = window.unminimize();
    let _ = window.eval(
        "requestAnimationFrame(()=>{document.documentElement.style.opacity='0.999';requestAnimationFrame(()=>{document.documentElement.style.opacity='1';});});",
    );
    window.set_focus().map_err(|error| error.to_string())
}

#[tauri::command]
fn begin_island_drag(
    app: AppHandle,
    window: tauri::WebviewWindow,
    layout: IslandLayout,
) -> Result<(), String> {
    let start_cursor = window_util::cursor_position()
        .ok_or_else(|| "could not read the cursor position".to_string())?;
    let start_window = window_util::physical_position(&window)
        .ok_or_else(|| "could not read the island position".to_string())?;
    let state_layout = app.state::<AppState>();
    if let Ok(mut guard) = state_layout.island_layout.lock() {
        *guard = Some(layout);
    }
    let handle = app.clone();
    std::thread::spawn(move || {
        let started = std::time::Instant::now();
        let mut last = start_window;
        loop {
            std::thread::sleep(std::time::Duration::from_millis(9));
            if !window_util::left_button_down() || started.elapsed().as_secs() > 25 {
                break;
            }
            let Some(cursor) = window_util::cursor_position() else {
                continue;
            };
            let x = start_window.0 + (cursor.0 - start_cursor.0);
            let y = start_window.1 + (cursor.1 - start_cursor.1);
            if (x, y) == last {
                continue;
            }
            last = (x, y);
            if let Some(window) = handle.get_webview_window("main") {
                window_util::place_at(&window, x, y);
            }
        }

        let Some(window) = handle.get_webview_window("main") else {
            return;
        };
        let position = window_util::physical_position(&window).unwrap_or(last);
        let island_left = position.0 + layout.left;
        let island_top = position.1 + layout.top;
        let island_right = island_left + layout.width;
        let island_bottom = island_top + layout.height;
        let (work_left, work_top, work_right, work_bottom) =
            window_util::work_area_at(island_left + layout.width / 2, island_top + layout.height / 2);
        let threshold = 40;
        let distances = [
            ("left", (island_left - work_left).max(0)),
            ("right", (work_right - island_right).max(0)),
            ("top", (island_top - work_top).max(0)),
            ("bottom", (work_bottom - island_bottom).max(0)),
        ];
        let nearest = distances
            .iter()
            .min_by_key(|(_, distance)| *distance)
            .copied()
            .unwrap_or(("free", threshold + 1));
        let dock = if nearest.1 <= threshold {
            nearest.0
        } else {
            "free"
        };

        // Keep the island in place when undocking, and snap it to the edge when
        // docking. The frontend changes its flex alignment from the saved dock.
        window_util::place_for_layout(&window, layout, island_left, island_top, dock);
        let new_position = window_util::physical_position(&window).unwrap_or(position);
        if let Ok(mut guard) = handle.state::<AppState>().settings.lock() {
            guard.island_pos_x = Some(new_position.0);
            guard.island_pos_y = Some(new_position.1);
            guard.island_dock = dock.to_string();
            let _ = settings::save(&guard);
            let _ = handle.emit("settings-changed", guard.clone());
        }
        let _ = handle.emit(
            "island-drag-end",
            serde_json::json!({
                "dock": dock,
                "x": new_position.0,
                "y": new_position.1,
            }),
        );
    });
    Ok(())
}

#[tauri::command]
fn save_island_position(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "island window is missing".to_string())?;
    let position = window_util::physical_position(&window)
        .ok_or_else(|| "could not read the island position".to_string())?;
    if let Ok(mut guard) = state.settings.lock() {
        guard.island_pos_x = Some(position.0);
        guard.island_pos_y = Some(position.1);
        let _ = settings::save(&guard);
        let _ = app.emit("settings-changed", guard.clone());
    }
    Ok(())
}

#[tauri::command]
fn reset_island_position(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "island window is missing".to_string())?;
    window_util::place_primary_top_center(&window);
    let position = window_util::physical_position(&window)
        .ok_or_else(|| "could not read the island position".to_string())?;
    if let Ok(mut guard) = state.settings.lock() {
        guard.island_pos_x = Some(position.0);
        guard.island_pos_y = Some(position.1);
        guard.island_dock = "top".into();
        settings::save(&guard)?;
        let _ = app.emit("settings-changed", guard.clone());
        let _ = app.emit("island-dock", serde_json::json!({ "dock": "top" }));
    }
    Ok(())
}

#[tauri::command]
fn move_island(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    dx: i32,
    dy: i32,
) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "island window is missing".to_string())?;
    let current = window_util::physical_position(&window)
        .ok_or_else(|| "could not read the island position".to_string())?;
    let layout = state
        .island_layout
        .lock()
        .ok()
        .and_then(|layout| *layout)
        .unwrap_or(IslandLayout {
            left: 0,
            top: 0,
            width: 174,
            height: 34,
        });
    let island_left = current.0 + layout.left + dx;
    let island_top = current.1 + layout.top + dy;
    let (x, y) = window_util::clamp_to_monitors(
        &window,
        island_left - layout.left,
        island_top - layout.top,
    );
    window_util::place_at(&window, x, y);
    if let Ok(mut guard) = state.settings.lock() {
        guard.island_pos_x = Some(x);
        guard.island_pos_y = Some(y);
        guard.island_dock = "free".into();
        let _ = settings::save(&guard);
        let _ = app.emit("settings-changed", guard.clone());
        let _ = app.emit("island-dock", serde_json::json!({ "dock": "free" }));
    }
    Ok(())
}

#[derive(serde::Deserialize)]
struct RegionRect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    r: Option<f64>,
}

#[tauri::command]
fn set_island_rect(
    state: tauri::State<'_, AppState>,
    window: tauri::WebviewWindow,
    rect: RegionRect,
    layout: Option<IslandLayout>,
) {
    if let Some(layout) = layout {
        if let Ok(mut guard) = state.island_layout.lock() {
            *guard = Some(layout);
        }
    }
    window_util::apply_region(&window, rect.x, rect.y, rect.w, rect.h, rect.r.unwrap_or(24.0));
}

pub fn apply_dock_preserving_position(app: &AppHandle, dock: &str) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let layout = app
        .state::<AppState>()
        .island_layout
        .lock()
        .ok()
        .and_then(|layout| *layout)
        .unwrap_or(IslandLayout {
            left: 0,
            top: 0,
            width: 174,
            height: 34,
        });
    if let Some(position) = window_util::physical_position(&window) {
        let island_left = position.0 + layout.left;
        let island_top = position.1 + layout.top;
        window_util::place_for_layout(&window, layout, island_left, island_top, dock);
        if let Ok(mut guard) = app.state::<AppState>().settings.lock() {
            if let Some(position) = window_util::physical_position(&window) {
                guard.island_pos_x = Some(position.0);
                guard.island_pos_y = Some(position.1);
            }
            guard.island_dock = dock.to_string();
            let _ = settings::save(&guard);
        }
    }
}

fn tray_labels(language: &str) -> (&'static str, &'static str, &'static str) {
    match language {
        "en" => ("Settings", "Collapse island", "Quit"),
        "ru" => ("Настройки", "Свернуть остров", "Выход"),
        _ => ("Налаштування", "Згорнути острів", "Вихід"),
    }
}

fn build_tray_menu(app: &AppHandle, language: &str) -> tauri::Result<Menu<tauri::Wry>> {
    let (settings, collapse, quit) = tray_labels(language);
    let settings_item = MenuItem::with_id(app, "settings", settings, true, None::<&str>)?;
    let pause_item = MenuItem::with_id(app, "pause", collapse, true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", quit, true, None::<&str>)?;
    Menu::with_items(app, &[&settings_item, &pause_item, &quit_item])
}

fn setup_tray(app: &mut tauri::App, language: &str) -> tauri::Result<()> {
    let menu = build_tray_menu(app.handle(), language)?;

    let icon = app.default_window_icon().cloned();
    let mut builder = TrayIconBuilder::with_id("isle-tray")
        .tooltip("Isle")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "settings" => show_settings(app),
            "pause" => {
                let _ = app.emit_to("main", "island-collapse", ());
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_island(tray.app_handle());
            }
        });

    if let Some(icon) = icon {
        builder = builder.icon(icon);
    }

    builder.build(app)?;

    Ok(())
}

pub fn refresh_tray_language(app: &AppHandle, language: &str) {
    let Ok(menu) = build_tray_menu(app, language) else {
        return;
    };
    if let Some(tray) = app.tray_by_id("isle-tray") {
        let _ = tray.set_menu(Some(menu));
    }
}

pub fn apply_tray_visibility(app: &AppHandle, visible: bool) {
    if let Some(tray) = app.tray_by_id("isle-tray") {
        let _ = tray.set_visible(visible);
    }
}

fn expand_island(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        window_util::mark_tool_window(&window);
    }
    let _ = app.emit_to("main", "island-expand", ());
}

fn collapse_island(app: &AppHandle) {
    let _ = app.emit_to("main", "island-collapse", ());
}

fn spawn_fullscreen_watcher(app: AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(2500));

        let enabled = app
            .state::<AppState>()
            .settings
            .lock()
            .map(|guard| guard.autohide_fullscreen)
            .unwrap_or(false);

        if !enabled {
            continue;
        }

        if let Some(window) = app.get_webview_window("main") {
            if window_util::fullscreen_active() {
                let _ = window.hide();
            } else {
                let _ = window.show();
                window_util::mark_tool_window(&window);
            }
        }
    });
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            toggle_island(app);
        }))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        let action = app
                            .state::<hotkeys::HotkeyState>()
                            .action(shortcut.id());
                        match action.as_deref() {
                            Some("toggle") => toggle_island(app),
                            Some("expand") => expand_island(app),
                            Some("collapse") => collapse_island(app),
                            Some("settings") => show_settings(app),
                            Some("screenshot") => {
                                let handle = app.clone();
                                tauri::async_runtime::spawn(async move {
                                    if let Ok(result) = capture::capture_screenshot(handle.clone(), None).await {
                                        let _ = handle.emit("screenshot-taken", result);
                                    }
                                });
                            }
                            _ => {}
                        }
                    }
                })
                .build(),
        )
        .setup(|app| {
            let loaded = settings::load();
            let database = Database::open().map_err(|error| {
                std::io::Error::other(error)
            })?;

            app.manage(AppState {
                settings: Mutex::new(loaded.clone()),
                db: database,
                island_layout: Mutex::new(None),
            });

            if let Err(error) = autostart::set_enabled(loaded.autostart) {
                eprintln!("Isle: could not sync autostart: {error}");
            }

            if let Some(window) = app.get_webview_window("main") {
                if let (Some(x), Some(y)) = (loaded.island_pos_x, loaded.island_pos_y) {
                    window_util::place_at(&window, x, y);
                } else {
                    window_util::place_top_center(&window);
                }
                if loaded.island_dock != "free" {
                    window_util::place_docked(&window, &loaded.island_dock);
                }
                let character = loaded.character != "none" && !loaded.character.is_empty();
                window_util::collapsed_region(
                    &window,
                    character,
                    loaded.show_battery,
                    &loaded.island_dock,
                );
                // Keep the WebView2 surface fully transparent. Without this the
                // default white surface can peek through while the island's
                // region grows/shrinks during the open/close animation.
                let _ = window
                    .set_background_color(Some(tauri::window::Color(0, 0, 0, 0)));
                let _ = window.show();
                window_util::mark_tool_window(&window);

                // Some Windows versions re-add WS_EX_APPWINDOW when WebView2 first
                // paints, so reinforce the tool-window style just after startup.
                let handle = window.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(900));
                    window_util::mark_tool_window(&handle);
                });
            }

            if let Some(window) = app.get_webview_window("settings") {
                let handle = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = handle.hide();
                    }
                });
            }

            media::init(app.handle());
            audio::init(app.handle().clone());
            cursor::init(app.handle().clone());
            stats::init(app.handle().clone());
            app.manage(agents::AgentsState::default());
            app.manage(downloads::DownloadState::default());
            app.manage(terminal::TerminalState::default());
            app.manage(sound::TimerState::default());
            app.manage(capture::CaptureViewState::default());
            agents::spawn_server(app.handle().clone());
            db::spawn_watcher(app.handle().clone());
            phone::init(app.handle());
            productivity::spawn_reminder_watcher(app.handle().clone());
            app.manage(hotkeys::HotkeyState::default());
            setup_tray(app, &loaded.language)?;
            apply_tray_visibility(app.handle(), loaded.tray_enabled);
            hotkeys::apply(app.handle(), &loaded);
            spawn_fullscreen_watcher(app.handle().clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_info,
            open_settings,
            hotkeys::hotkeys_status,
            reset_island_position,
            move_island,
            save_island_position,
            begin_island_drag,
            set_island_rect,
            settings::get_settings,
            settings::save_settings,
            weather::fetch_weather,
            weather::geocode_city,
            media::media_control,
            media::media_current,
            lyrics::fetch_lyrics,
            artwork::fetch_cover,
            translate::translate_text,
            audio::audio_spectrum,
            stats::system_stats,
            capture::capture_screenshot,
            capture::capture_monitors,
            capture::capture_list,
            capture::image_preview,
            capture::image_data,
            capture::save_capture_data,
            capture::ocr_image,
            capture::qr_generate,
            capture::qr_scan,
            capture::open_path,
            capture::open_capture_folder,
            capture::capture_view_path,
            capture::open_capture_view,
            capture::clipboard_write_image,
            capture::clipboard_write_image_path,
            productivity::reminders_list,
            productivity::reminders_save,
            productivity::reminders_toggle,
            productivity::reminders_delete,
            productivity::events_list,
            productivity::events_save,
            productivity::events_delete,
            productivity::calendar_import_ics,
            productivity::calendar_export_ics,
            chat::chat_sessions,
            chat::chat_session_create,
            chat::chat_session_delete,
            chat::chat_messages,
            chat::chat_send,
            chat::chat_probe,
            network::network_status,
            downloads::download_start,
            downloads::downloads_list,
            downloads::download_cancel,
            downloads::download_clear,
            downloads::downloads_open_folder,
            agents::agents_events,
            agents::agents_clear,
            agents::agents_endpoint,
            agents::agents_status,
            agents::agents_test,
            agents::agents_install_opencode,
            agents::agents_open_opencode,
            phone::phone_info,
            phone::phone_unpair,
            phone::phone_refresh_code,
            terminal::terminal_open,
            terminal::terminal_write,
            terminal::terminal_resize,
            terminal::terminal_close,
            sound::play_timer_sound,
            sound::stop_timer_sound,
            sound::timer_schedule,
            sound::timer_cancel,
            battery::battery_status,
            db::clipboard_list,
            db::clipboard_search,
            db::clipboard_pin,
            db::clipboard_delete,
            db::clipboard_capture,
            db::clipboard_write,
            db::clipboard_preview_data,
            db::clipboard_copy_item,
            db::clipboard_clear,
            db::notes_list,
            db::notes_save,
            db::notes_delete
        ])
        .run(tauri::generate_context!())
        .expect("error while running Isle");
}
