use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tauri::State;

#[cfg(windows)]
use std::ffi::c_void;

#[cfg(windows)]
#[repr(C)]
struct DataBlob {
    size: u32,
    data: *mut u8,
}

#[cfg(windows)]
#[link(name = "crypt32")]
extern "system" {
    fn CryptProtectData(
        data_in: *const DataBlob,
        description: *const u16,
        entropy: *const DataBlob,
        reserved: *mut c_void,
        prompt: *mut c_void,
        flags: u32,
        data_out: *mut DataBlob,
    ) -> i32;
    fn CryptUnprotectData(
        data_in: *const DataBlob,
        description: *mut *mut u16,
        entropy: *const DataBlob,
        reserved: *mut c_void,
        prompt: *mut c_void,
        flags: u32,
        data_out: *mut DataBlob,
    ) -> i32;
}

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn LocalFree(memory: isize) -> isize;
}

#[cfg(windows)]
fn protect_key(key: &str) -> Option<String> {
    let mut bytes = key.as_bytes().to_vec();
    let input = DataBlob {
        size: bytes.len() as u32,
        data: bytes.as_mut_ptr(),
    };
    let mut output = DataBlob {
        size: 0,
        data: std::ptr::null_mut(),
    };
    let ok = unsafe {
        CryptProtectData(
            &input,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0,
            &mut output,
        )
    };
    if ok == 0 || output.data.is_null() {
        return None;
    }
    let encrypted = unsafe { std::slice::from_raw_parts(output.data, output.size as usize) }.to_vec();
    unsafe {
        LocalFree(output.data as isize);
    }
    Some(base64::engine::general_purpose::STANDARD.encode(encrypted))
}

#[cfg(windows)]
fn unprotect_key(encoded: &str) -> Option<String> {
    let mut bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .ok()?;
    let input = DataBlob {
        size: bytes.len() as u32,
        data: bytes.as_mut_ptr(),
    };
    let mut output = DataBlob {
        size: 0,
        data: std::ptr::null_mut(),
    };
    let ok = unsafe {
        CryptUnprotectData(
            &input,
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0,
            &mut output,
        )
    };
    if ok == 0 || output.data.is_null() {
        return None;
    }
    let plain = unsafe { std::slice::from_raw_parts(output.data, output.size as usize) }.to_vec();
    unsafe {
        LocalFree(output.data as isize);
    }
    String::from_utf8(plain).ok()
}

// macOS/Linux: the API key is stored base64-encoded inside the settings file.
// A Keychain-backed implementation can replace this without touching callers.
#[cfg(not(windows))]
fn protect_key(key: &str) -> Option<String> {
    Some(base64::engine::general_purpose::STANDARD.encode(key.as_bytes()))
}

#[cfg(not(windows))]
fn unprotect_key(encoded: &str) -> Option<String> {
    base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WidgetConfig {
    pub id: String,
    pub kind: String,
    pub size: String,
}

impl Default for WidgetConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            kind: "clock".into(),
            size: "wide".into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub language: String,
    pub autostart: bool,
    pub tray_enabled: bool,
    pub behavior: String,
    pub island_dock: String,
    pub auto_music_panel: bool,
    pub karaoke_line: bool,
    pub clipboard_watch: bool,
    pub clipboard_images: bool,
    pub clipboard_ignore_secrets: bool,
    pub hover_delay_ms: u64,
    pub close_delay_ms: u64,
    pub hotkey: String,
    pub hotkey_expand: String,
    pub hotkey_collapse: String,
    pub hotkey_settings: String,
    pub hotkey_screenshot: String,
    pub autohide_fullscreen: bool,
    pub show_seconds: bool,
    pub show_battery: bool,
    pub character: String,
    pub character_speed: f64,
    pub theme: String,
    pub accent_color: String,
    pub font: String,
    pub radius: u32,
    pub reduced_motion: bool,
    pub island_width: u32,
    pub island_height: u32,
    pub weather_city: Option<String>,
    pub weather_lat: Option<f64>,
    pub weather_lon: Option<f64>,
    pub weather_units: String,
    pub modules: HashMap<String, bool>,
    pub module_order: Vec<String>,
    pub widgets: Vec<WidgetConfig>,
    pub music_mode: String,
    pub visualizer_style: String,
    pub motion_profile: String,
    pub lyrics_style: String,
    pub chat_base_url: String,
    pub chat_api_key: String,
    pub chat_model: String,
    pub chat_provider: String,
    pub show_stats_collapsed: bool,
    pub custom_bg: String,
    pub custom_fg: String,
    pub schema_version: u32,
    pub island_pos_x: Option<i32>,
    pub island_pos_y: Option<i32>,
}

pub fn default_widgets() -> Vec<WidgetConfig> {
    [
        ("widget-clock", "clock", "wide"),
        ("widget-date", "date", "wide"),
        ("widget-media", "media", "large"),
        ("widget-system", "system", "large"),
        ("widget-actions", "quick_actions", "large"),
        ("widget-lyrics", "lyrics", "wide"),
        ("widget-timer", "timer", "small"),
        ("widget-agents", "agents", "wide"),
    ]
    .into_iter()
    .map(|(id, kind, size)| WidgetConfig {
        id: id.into(),
        kind: kind.into(),
        size: size.into(),
    })
    .collect()
}

impl Default for Settings {
    fn default() -> Self {
        let mut modules = HashMap::new();
        modules.insert("player".to_string(), true);
        modules.insert("clipboard".to_string(), true);
        modules.insert("text".to_string(), true);
        modules.insert("notes".to_string(), true);
        modules.insert("timer".to_string(), true);
        modules.insert("weather".to_string(), true);
        modules.insert("capture".to_string(), true);
        modules.insert("calendar".to_string(), true);
        modules.insert("chat".to_string(), true);
        modules.insert("control".to_string(), true);
        modules.insert("stats".to_string(), true);
        modules.insert("agents".to_string(), true);
        modules.insert("phone".to_string(), true);
        modules.insert("terminal".to_string(), true);

        Self {
            language: "uk".into(),
            autostart: false,
            tray_enabled: true,
            behavior: "hover".into(),
            island_dock: "top".into(),
            auto_music_panel: true,
            karaoke_line: true,
            clipboard_watch: true,
            clipboard_images: true,
            clipboard_ignore_secrets: true,
            hover_delay_ms: 120,
            close_delay_ms: 400,
            hotkey: "Alt+Space".into(),
            hotkey_expand: String::new(),
            hotkey_collapse: String::new(),
            hotkey_settings: String::new(),
            hotkey_screenshot: String::new(),
            autohide_fullscreen: false,
            show_seconds: false,
            show_battery: false,
            character: "eyes".into(),
            character_speed: 1.0,
            theme: "classic".into(),
            accent_color: "#3b82f6".into(),
            font: "system".into(),
            radius: 24,
            reduced_motion: false,
            island_width: 176,
            island_height: 34,
            weather_city: Some("Київ".into()),
            weather_lat: Some(50.4501),
            weather_lon: Some(30.5234),
            weather_units: "c".into(),
            modules,
            module_order: vec![
                "home".into(),
                "player".into(),
                "clipboard".into(),
                "text".into(),
                "notes".into(),
                "timer".into(),
                "weather".into(),
                "capture".into(),
                "calendar".into(),
                "chat".into(),
                "control".into(),
                "stats".into(),
                "agents".into(),
                "phone".into(),
            ],
            widgets: default_widgets(),
            music_mode: "standard".into(),
            visualizer_style: "auto".into(),
            motion_profile: "soft".into(),
            lyrics_style: "slide".into(),
            chat_base_url: "http://127.0.0.1:11434/v1".into(),
            chat_api_key: String::new(),
            chat_model: "llama3.2".into(),
            chat_provider: "ollama".into(),
            show_stats_collapsed: false,
            custom_bg: "#0b1220".into(),
            custom_fg: "#f1f5f9".into(),
            schema_version: 1,
            island_pos_x: None,
            island_pos_y: None,
        }
    }
}

pub fn data_dir() -> PathBuf {
    let base = directories::BaseDirs::new()
        .map(|dirs| dirs.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("Isle");
    let _ = fs::create_dir_all(&dir);
    dir
}

fn settings_path() -> PathBuf {
    data_dir().join("settings.json")
}

pub fn load() -> Settings {
    let path = settings_path();
    let mut settings = match fs::read_to_string(&path) {
        Ok(raw) => {
            let mut value: serde_json::Value = serde_json::from_str(&raw).unwrap_or_else(|error| {
                eprintln!("Isle: could not parse settings ({error}), using defaults");
                json!({})
            });
            if let Some(encoded) = value
                .get("chat_api_key_enc")
                .and_then(|value| value.as_str())
                .map(|value| value.to_string())
            {
                if let Some(key) = unprotect_key(&encoded) {
                    value["chat_api_key"] = json!(key);
                }
                if let Some(object) = value.as_object_mut() {
                    object.remove("chat_api_key_enc");
                }
            }
            serde_json::from_value(value).unwrap_or_else(|error| {
                eprintln!("Isle: could not parse settings ({error}), using defaults");
                Settings::default()
            })
        }
        Err(_) => Settings::default(),
    };

    // Older builds shipped these modules disabled (or did not know about them).
    // Enable them once for existing installations; afterwards the user's choice
    // is persisted like any other setting.
    if settings.schema_version < 2 {
        for id in [
            "capture",
            "calendar",
            "chat",
            "control",
            "agents",
            "phone",
            "terminal",
        ] {
            settings.modules.insert(id.to_string(), true);
            if !settings.module_order.iter().any(|item| item == id) {
                settings.module_order.push(id.to_string());
            }
        }
        settings.schema_version = 2;
        if let Err(error) = save(&settings) {
            eprintln!("Isle: could not persist settings migration: {error}");
        }
    }

    if settings.schema_version < 3 {
        if settings.widgets.is_empty() {
            settings.widgets = default_widgets();
        }
        if settings.island_dock.trim().is_empty() {
            settings.island_dock = "top".into();
        }
        settings.schema_version = 3;
        if let Err(error) = save(&settings) {
            eprintln!("Isle: could not persist widget migration: {error}");
        }
    }

    if settings.widgets.is_empty() {
        settings.widgets = default_widgets();
    }
    let valid_kinds = [
        "clock", "date", "weather", "media", "lyrics", "system", "cpu", "gpu", "ram", "temp",
        "fps", "disk", "network", "battery", "timer", "quick_actions", "character", "spectrum",
        "agents", "calendar",
    ];
    let valid_sizes = ["small", "wide", "tall", "large", "full", "hero"];
    settings.widgets.retain(|widget| {
        valid_kinds.contains(&widget.kind.as_str()) && !widget.id.trim().is_empty()
    });
    for widget in &mut settings.widgets {
        if !valid_sizes.contains(&widget.size.as_str()) {
            widget.size = "wide".into();
        }
    }
    if settings.widgets.is_empty() {
        settings.widgets = default_widgets();
    }
    if !matches!(
        settings.island_dock.as_str(),
        "top" | "left" | "right" | "bottom" | "free"
    ) {
        settings.island_dock = "top".into();
    }
    if !matches!(settings.behavior.as_str(), "hover" | "click" | "double" | "always") {
        settings.behavior = "hover".into();
    }
    if !matches!(
        settings.visualizer_style.as_str(),
        "auto" | "bars" | "wave" | "pulse" | "orbit" | "sparks" | "aurora" | "meteors" | "glow"
            | "ribbon"
    ) {
        settings.visualizer_style = "auto".into();
    }
    if !matches!(
        settings.motion_profile.as_str(),
        "soft" | "snappy" | "cinematic" | "calm"
    ) {
        settings.motion_profile = "soft".into();
    }
    if !matches!(
        settings.lyrics_style.as_str(),
        "slide" | "pop" | "blur" | "karaoke"
    ) {
        settings.lyrics_style = "slide".into();
    }
    if !matches!(
        settings.theme.as_str(),
        "classic"
            | "night"
            | "light"
            | "aurora"
            | "sunset"
            | "forest"
            | "mono"
            | "rose"
            | "system"
            | "custom"
    ) {
        settings.theme = "classic".into();
    }

    // Store a plaintext key from an older build encrypted as soon as possible.
    if !settings.chat_api_key.is_empty() {
        let _ = save(&settings);
    }

    settings
}

pub fn save(settings: &Settings) -> Result<(), String> {
    let mut value = serde_json::to_value(settings).map_err(|error| error.to_string())?;
    if settings.chat_api_key.trim().is_empty() {
        if let Some(object) = value.as_object_mut() {
            object.remove("chat_api_key");
            object.remove("chat_api_key_enc");
        }
    } else {
        let encrypted = protect_key(&settings.chat_api_key)
            .ok_or_else(|| "Could not encrypt the AI API key".to_string())?;
        if let Some(object) = value.as_object_mut() {
            object.remove("chat_api_key");
            object.insert("chat_api_key_enc".into(), json!(encrypted));
        }
    }
    let text = serde_json::to_string_pretty(&value).map_err(|error| error.to_string())?;
    fs::write(settings_path(), text).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_settings(state: State<'_, crate::AppState>) -> Settings {
    state
        .settings
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_default()
}

#[tauri::command]
pub fn save_settings(
    app: tauri::AppHandle,
    state: State<'_, crate::AppState>,
    settings: Settings,
) -> Result<(), String> {
    let previous_dock = state
        .settings
        .lock()
        .map(|guard| guard.island_dock.clone())
        .unwrap_or_default();
    if let Ok(mut guard) = state.settings.lock() {
        *guard = settings.clone();
    }
    if let Err(error) = crate::autostart::set_enabled(settings.autostart) {
        eprintln!("Isle: autostart update failed: {error}");
    }
    crate::refresh_tray_language(&app, &settings.language);
    crate::apply_tray_visibility(&app, settings.tray_enabled);
    crate::hotkeys::apply(&app, &settings);
    let result = save(&settings);
    if result.is_ok() {
        let mut final_settings = settings.clone();
        if previous_dock != settings.island_dock {
            crate::apply_dock_preserving_position(&app, &settings.island_dock);
            final_settings = state
                .settings
                .lock()
                .map(|guard| guard.clone())
                .unwrap_or(settings);
            let _ = save(&final_settings);
            let _ = tauri::Emitter::emit(
                &app,
                "island-dock",
                serde_json::json!({ "dock": final_settings.island_dock }),
            );
        }
        let _ = tauri::Emitter::emit(&app, "settings-changed", final_settings);
    }
    result
}
