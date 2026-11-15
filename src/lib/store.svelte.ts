import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { resolveLocale, translate, type Locale } from "./i18n";
import { DEFAULT_WIDGETS, type WidgetConfig } from "./widgets/types";

export interface Settings {
  language: string;
  autostart: boolean;
  tray_enabled: boolean;
  behavior: string;
  island_dock: string;
  auto_music_panel: boolean;
  karaoke_line: boolean;
  clipboard_watch: boolean;
  clipboard_images: boolean;
  clipboard_ignore_secrets: boolean;
  hover_delay_ms: number;
  close_delay_ms: number;
  hotkey: string;
  hotkey_expand: string;
  hotkey_collapse: string;
  hotkey_settings: string;
  hotkey_screenshot: string;
  autohide_fullscreen: boolean;
  show_seconds: boolean;
  show_battery: boolean;
  character: string;
  character_speed: number;
  theme: string;
  accent_color: string;
  font: string;
  radius: number;
  reduced_motion: boolean;
  island_width: number;
  island_height: number;
  weather_city: string | null;
  weather_lat: number | null;
  weather_lon: number | null;
  weather_units: string;
  modules: Record<string, boolean>;
  module_order: string[];
  widgets: WidgetConfig[];
  music_mode: string;
  visualizer_style: string;
  motion_profile: string;
  lyrics_style: string;
  chat_base_url: string;
  chat_api_key: string;
  chat_model: string;
  chat_provider: string;
  show_stats_collapsed: boolean;
  custom_bg: string;
  custom_fg: string;
  schema_version: number;
  island_pos_x: number | null;
  island_pos_y: number | null;
}

export const DEFAULT_MODULES: Record<string, boolean> = {
  player: true,
  clipboard: true,
  text: true,
  notes: true,
  timer: true,
  weather: true,
  capture: true,
  chat: true,
  calendar: true,
  control: true,
  stats: true,
  phone: true,
  agents: true,
  terminal: true,
};

export const DEFAULT_SETTINGS: Settings = {
  language: "uk",
  autostart: false,
  tray_enabled: true,
  behavior: "hover",
  island_dock: "top",
  auto_music_panel: true,
  karaoke_line: true,
  clipboard_watch: true,
  clipboard_images: true,
  clipboard_ignore_secrets: true,
  hover_delay_ms: 120,
  close_delay_ms: 400,
  hotkey: "Alt+Space",
  hotkey_expand: "",
  hotkey_collapse: "",
  hotkey_settings: "",
  hotkey_screenshot: "",
  autohide_fullscreen: false,
  show_seconds: false,
  show_battery: false,
  character: "eyes",
  character_speed: 1,
  theme: "classic",
  accent_color: "#3b82f6",
  font: "Inter",
  radius: 24,
  reduced_motion: false,
  island_width: 176,
  island_height: 34,
  weather_city: "Київ",
  weather_lat: 50.4501,
  weather_lon: 30.5234,
  weather_units: "c",
  modules: { ...DEFAULT_MODULES },
  module_order: [
    "home",
    "player",
    "clipboard",
    "text",
    "notes",
    "timer",
    "weather",
    "capture",
    "calendar",
    "chat",
    "control",
    "stats",
    "agents",
    "phone",
    "terminal",
  ],
  widgets: DEFAULT_WIDGETS.map((widget) => ({ ...widget })),
  music_mode: "standard",
  visualizer_style: "auto",
  motion_profile: "soft",
  lyrics_style: "slide",
  chat_base_url: "http://127.0.0.1:11434/v1",
  chat_api_key: "",
  chat_model: "llama3.2",
  chat_provider: "ollama",
  show_stats_collapsed: false,
  custom_bg: "#0b1220",
  custom_fg: "#f1f5f9",
  schema_version: 2,
  island_pos_x: null,
  island_pos_y: null,
};

interface Store {
  settings: Settings;
  loaded: boolean;
  saveError: string | null;
  version: string;
}

export const store = $state<Store>({
  settings: { ...DEFAULT_SETTINGS, modules: { ...DEFAULT_MODULES } },
  loaded: false,
  saveError: null,
  version: "0.1.0",
});

let saveTimer: ReturnType<typeof setTimeout> | undefined;
let watchingSettings = false;

function adoptSettings(raw: Partial<Settings>): void {
  store.settings = {
    ...DEFAULT_SETTINGS,
    ...raw,
    modules: { ...DEFAULT_MODULES, ...(raw.modules ?? {}) },
    module_order:
      raw.module_order && raw.module_order.length
        ? raw.module_order
        : DEFAULT_SETTINGS.module_order,
    widgets:
      raw.widgets && raw.widgets.length
        ? raw.widgets.map((widget) => ({ ...widget }))
        : DEFAULT_WIDGETS.map((widget) => ({ ...widget })),
  };
  applyTheme();
}

async function watchSettings(): Promise<void> {
  if (watchingSettings) return;
  watchingSettings = true;
  await listen<Settings>("settings-changed", (event) => {
    adoptSettings(event.payload);
  });
}

export function locale(): Locale {
  return resolveLocale(store.settings.language);
}

export function t(
  key: string,
  params: Record<string, string | number> = {},
): string {
  return translate(locale(), key, params);
}

export async function initSettings(): Promise<void> {
  if (store.loaded) return;
  let raw: Partial<Settings> | null = null;
  // The island's WebView can start loading before Tauri's setup() has managed
  // AppState. Retry briefly instead of silently falling back to defaults,
  // otherwise the first frame would use the wrong theme, dock and widgets.
  for (let attempt = 0; attempt < 12; attempt += 1) {
    try {
      raw = await invoke<Partial<Settings>>("get_settings");
      break;
    } catch (error) {
      if (attempt === 11) {
        console.warn("Could not load settings", error);
      } else {
        await new Promise((resolve) => setTimeout(resolve, 120));
      }
    }
  }
  if (raw) {
    adoptSettings(raw);
    try {
      const info = await invoke<{ version: string }>("app_info");
      store.version = info.version;
    } catch {
      /* version is optional */
    }
  }
  await watchSettings();
  applyTheme();
  store.loaded = true;
}

export function updateSettings(patch: Partial<Settings>): void {
  Object.assign(store.settings, patch);
  applyTheme();
  scheduleSave();
}

export function updateModule(id: string, enabled: boolean): void {
  updateSettings({ modules: { ...store.settings.modules, [id]: enabled } });
}

function scheduleSave(): void {
  if (saveTimer) clearTimeout(saveTimer);
  saveTimer = setTimeout(() => {
    void persist();
  }, 250);
}

async function persist(): Promise<void> {
  try {
    await invoke("save_settings", { settings: store.settings });
    store.saveError = null;
  } catch (error) {
    store.saveError = String(error);
    console.error("Could not save settings", error);
  }
}

const THEMES: Record<string, Record<string, string>> = {
  classic: {
    "--bg": "#000000",
    "--bg-elevated": "#0c0c0e",
    "--fg": "#ffffff",
    "--muted": "#a1a1aa",
    "--surface": "rgba(255,255,255,0.06)",
    "--surface-strong": "rgba(255,255,255,0.1)",
    "--border": "rgba(255,255,255,0.08)",
    "--input": "rgba(255,255,255,0.07)",
  },
  night: {
    "--bg": "#0b1220",
    "--bg-elevated": "#111a2c",
    "--fg": "#f1f5f9",
    "--muted": "#94a3b8",
    "--surface": "rgba(148,163,184,0.12)",
    "--surface-strong": "rgba(148,163,184,0.2)",
    "--border": "rgba(148,163,184,0.16)",
    "--input": "rgba(148,163,184,0.12)",
  },
  light: {
    "--bg": "#f7f7fa",
    "--bg-elevated": "#ffffff",
    "--fg": "#14161a",
    "--muted": "#5f6774",
    "--surface": "rgba(15,23,42,0.05)",
    "--surface-strong": "rgba(15,23,42,0.09)",
    "--border": "rgba(15,23,42,0.1)",
    "--input": "rgba(15,23,42,0.06)",
  },
  aurora: {
    "--bg": "#061a1a",
    "--bg-elevated": "#0a2424",
    "--fg": "#e8fffb",
    "--muted": "#7dd3c7",
    "--surface": "rgba(125,211,199,0.1)",
    "--surface-strong": "rgba(125,211,199,0.18)",
    "--border": "rgba(125,211,199,0.16)",
    "--input": "rgba(125,211,199,0.12)",
  },
  sunset: {
    "--bg": "#1a0b10",
    "--bg-elevated": "#2a1119",
    "--fg": "#ffe9e2",
    "--muted": "#f3a68f",
    "--surface": "rgba(243,166,143,0.1)",
    "--surface-strong": "rgba(243,166,143,0.18)",
    "--border": "rgba(243,166,143,0.16)",
    "--input": "rgba(243,166,143,0.12)",
  },
  forest: {
    "--bg": "#0a140d",
    "--bg-elevated": "#102014",
    "--fg": "#e9f7ea",
    "--muted": "#93c5a3",
    "--surface": "rgba(147,197,163,0.1)",
    "--surface-strong": "rgba(147,197,163,0.18)",
    "--border": "rgba(147,197,163,0.16)",
    "--input": "rgba(147,197,163,0.12)",
  },
  mono: {
    "--bg": "#111111",
    "--bg-elevated": "#1a1a1a",
    "--fg": "#f5f5f5",
    "--muted": "#a3a3a3",
    "--surface": "rgba(255,255,255,0.07)",
    "--surface-strong": "rgba(255,255,255,0.13)",
    "--border": "rgba(255,255,255,0.12)",
    "--input": "rgba(255,255,255,0.09)",
  },
  rose: {
    "--bg": "#1a0b14",
    "--bg-elevated": "#26101d",
    "--fg": "#ffe8f3",
    "--muted": "#e6a3c2",
    "--surface": "rgba(230,163,194,0.1)",
    "--surface-strong": "rgba(230,163,194,0.18)",
    "--border": "rgba(230,163,194,0.16)",
    "--input": "rgba(230,163,194,0.12)",
  },
};

export interface MotionProfile {
  duration: number;
  ease: string;
  overshoot: number;
}

export const MOTION_PROFILES: Record<string, MotionProfile> = {
  soft: { duration: 320, ease: "cubic-bezier(0.34, 1.2, 0.64, 1)", overshoot: 1.02 },
  snappy: { duration: 210, ease: "cubic-bezier(0.22, 1.4, 0.36, 1)", overshoot: 1.05 },
  cinematic: { duration: 480, ease: "cubic-bezier(0.16, 1, 0.3, 1)", overshoot: 1.06 },
  calm: { duration: 420, ease: "cubic-bezier(0.4, 0, 0.2, 1)", overshoot: 1 },
};

export function motionProfile(): MotionProfile {
  return MOTION_PROFILES[store.settings.motion_profile] ?? MOTION_PROFILES.soft;
}

export function motionDuration(): number {
  return motionProfile().duration;
}

function customTheme(bg: string, fg: string): Record<string, string> {
  return {
    "--bg": bg,
    "--bg-elevated": `color-mix(in srgb, ${bg} 88%, ${fg} 12%)`,
    "--fg": fg,
    "--muted": `color-mix(in srgb, ${fg} 62%, ${bg})`,
    "--surface": `color-mix(in srgb, ${fg} 8%, transparent)`,
    "--surface-strong": `color-mix(in srgb, ${fg} 15%, transparent)`,
    "--border": `color-mix(in srgb, ${fg} 14%, transparent)`,
    "--input": `color-mix(in srgb, ${fg} 10%, transparent)`,
  };
}

const systemThemeQuery =
  typeof window !== "undefined" && typeof window.matchMedia === "function"
    ? window.matchMedia("(prefers-color-scheme: dark)")
    : null;
let systemThemeListening = false;

function resolveThemeKey(): string {
  if (store.settings.theme !== "system") return store.settings.theme;
  return systemThemeQuery?.matches ? "night" : "light";
}

export function applyTheme(): void {
  const root = document.documentElement;
  const themeKey = resolveThemeKey();
  const theme =
    themeKey === "custom"
      ? customTheme(store.settings.custom_bg, store.settings.custom_fg)
      : (THEMES[themeKey] ?? THEMES.classic);
  for (const [key, value] of Object.entries(theme)) {
    root.style.setProperty(key, value);
  }
  root.style.setProperty("--accent", store.settings.accent_color);
  root.style.setProperty("--radius", `${store.settings.radius}px`);
  const font =
    !store.settings.font || store.settings.font === "system"
      ? '"Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif'
      : `"${store.settings.font}", "Segoe UI", system-ui, sans-serif`;
  root.style.setProperty("--font", font);
  root.dataset.theme = themeKey;
  root.dataset.themeMode = store.settings.theme;
  root.dataset.reducedMotion = store.settings.reduced_motion ? "true" : "false";
  root.dataset.motion = store.settings.motion_profile;
  const profile = motionProfile();
  root.style.setProperty("--motion-duration", `${profile.duration}ms`);
  root.style.setProperty("--motion-ease", profile.ease);
  root.style.setProperty("--motion-fast", `${Math.round(profile.duration * 0.55)}ms`);
  root.lang = locale();

  // Keep the native WebView2 surface in sync with the theme. The island is
  // always transparent (so resizing never reveals a white rectangle); the
  // settings window uses the theme background to avoid a white first frame.
  try {
    const window = getCurrentWindow();
    if (window.label === "settings") {
      void window.setBackgroundColor(theme["--bg"] ?? "#0b1220").catch(() => {});
    } else {
      void window.setBackgroundColor([0, 0, 0, 0]).catch(() => {});
    }
  } catch {
    /* window API is unavailable during tests */
  }

  if (!systemThemeListening && systemThemeQuery) {
    systemThemeListening = true;
    systemThemeQuery.addEventListener("change", () => {
      if (store.settings.theme === "system") applyTheme();
    });
  }
}

export function formatNumber(value: number, digits = 0): string {
  return value.toLocaleString(locale(), {
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
  });
}

export function formatTimeOfDay(timestamp: string): string {
  const date = new Date(timestamp);
  if (Number.isNaN(date.getTime())) return "--:--";
  return date.toLocaleTimeString(locale(), {
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  });
}

export function formatDayName(timestamp: string, index: number): string {
  if (index === 0) return t("common.today");
  if (index === 1) return t("weather.tomorrow");
  const date = new Date(timestamp);
  return date.toLocaleDateString(locale(), { weekday: "short" });
}
