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
  island_pinned: boolean;
  island_scene: string;
  island_gif_url: string;
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
  hotkey_play_pause: string;
  hotkey_next: string;
  hotkey_prev: string;
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
  fps_metric: "cpu" | "gpu";
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
  island_pinned: false,
  island_scene: "none",
  island_gif_url: "",
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
  hotkey_play_pause: "",
  hotkey_next: "",
  hotkey_prev: "",
  autohide_fullscreen: false,
  show_seconds: false,
  show_battery: false,
  character: "eyes",
  character_speed: 1,
  theme: "classic",
  accent_color: "#3b82f6",
  font: "system",
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
  fps_metric: "cpu",
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
  // The old "Load HUD" scene painted live CPU/GPU data above every tab. It is
  // retired; performance data now lives in the System tab and the FPS widget.
  if (store.settings.island_scene === "fps") store.settings.island_scene = "none";
  if (store.settings.fps_metric !== "gpu") store.settings.fps_metric = "cpu";
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
    "--bg": "#0b0b0d",
    "--bg-elevated": "#131316",
    "--fg": "#f2f1ee",
    "--muted": "#9d9a94",
    "--surface": "rgba(255,255,255,0.045)",
    "--surface-strong": "rgba(255,255,255,0.08)",
    "--surface-raised": "rgba(255,255,255,0.11)",
    "--border": "rgba(255,255,255,0.075)",
    "--border-strong": "rgba(255,255,255,0.14)",
    "--input": "rgba(255,255,255,0.055)",
  },
  night: {
    "--bg": "#0a0d12",
    "--bg-elevated": "#11151c",
    "--fg": "#eef0f2",
    "--muted": "#959aa3",
    "--surface": "rgba(233,238,245,0.045)",
    "--surface-strong": "rgba(233,238,245,0.08)",
    "--surface-raised": "rgba(233,238,245,0.11)",
    "--border": "rgba(233,238,245,0.08)",
    "--border-strong": "rgba(233,238,245,0.15)",
    "--input": "rgba(233,238,245,0.055)",
  },
  light: {
    "--bg": "#f4f2ee",
    "--bg-elevated": "#fbfaf7",
    "--fg": "#17161a",
    "--muted": "#6d6a63",
    "--surface": "rgba(23,22,26,0.04)",
    "--surface-strong": "rgba(23,22,26,0.075)",
    "--surface-raised": "rgba(23,22,26,0.1)",
    "--border": "rgba(23,22,26,0.1)",
    "--border-strong": "rgba(23,22,26,0.18)",
    "--input": "rgba(23,22,26,0.05)",
  },
  aurora: {
    "--bg": "#061a1a",
    "--bg-elevated": "#0a2424",
    "--fg": "#e8fffb",
    "--muted": "#7dd3c7",
    "--surface": "rgba(125,211,199,0.1)",
    "--surface-strong": "rgba(125,211,199,0.18)",
    "--surface-raised": "rgba(125,211,199,0.24)",
    "--border": "rgba(125,211,199,0.16)",
    "--border-strong": "rgba(125,211,199,0.26)",
    "--input": "rgba(125,211,199,0.12)",
  },
  sunset: {
    "--bg": "#1a0b10",
    "--bg-elevated": "#2a1119",
    "--fg": "#ffe9e2",
    "--muted": "#f3a68f",
    "--surface": "rgba(243,166,143,0.1)",
    "--surface-strong": "rgba(243,166,143,0.18)",
    "--surface-raised": "rgba(243,166,143,0.24)",
    "--border": "rgba(243,166,143,0.16)",
    "--border-strong": "rgba(243,166,143,0.26)",
    "--input": "rgba(243,166,143,0.12)",
  },
  forest: {
    "--bg": "#0a140d",
    "--bg-elevated": "#102014",
    "--fg": "#e9f7ea",
    "--muted": "#93c5a3",
    "--surface": "rgba(147,197,163,0.1)",
    "--surface-strong": "rgba(147,197,163,0.18)",
    "--surface-raised": "rgba(147,197,163,0.24)",
    "--border": "rgba(147,197,163,0.16)",
    "--border-strong": "rgba(147,197,163,0.26)",
    "--input": "rgba(147,197,163,0.12)",
  },
  mono: {
    "--bg": "#111111",
    "--bg-elevated": "#1a1a1a",
    "--fg": "#f5f5f5",
    "--muted": "#a3a3a3",
    "--surface": "rgba(255,255,255,0.07)",
    "--surface-strong": "rgba(255,255,255,0.13)",
    "--surface-raised": "rgba(255,255,255,0.17)",
    "--border": "rgba(255,255,255,0.12)",
    "--border-strong": "rgba(255,255,255,0.2)",
    "--input": "rgba(255,255,255,0.09)",
  },
  rose: {
    "--bg": "#1a0b14",
    "--bg-elevated": "#26101d",
    "--fg": "#ffe8f3",
    "--muted": "#e6a3c2",
    "--surface": "rgba(230,163,194,0.1)",
    "--surface-strong": "rgba(230,163,194,0.18)",
    "--surface-raised": "rgba(230,163,194,0.24)",
    "--border": "rgba(230,163,194,0.16)",
    "--border-strong": "rgba(230,163,194,0.26)",
    "--input": "rgba(230,163,194,0.12)",
  },
};

export interface MotionProfile {
  duration: number;
  ease: string;
  overshoot: number;
}

export const MOTION_PROFILES: Record<string, MotionProfile> = {
  // Apple-like ease-out: quick departure, long soft landing, no bounce.
  soft: { duration: 380, ease: "cubic-bezier(0.32, 0.72, 0, 1)", overshoot: 1.02 },
  snappy: { duration: 240, ease: "cubic-bezier(0.28, 0.84, 0.36, 1)", overshoot: 1.05 },
  cinematic: { duration: 520, ease: "cubic-bezier(0.22, 0.61, 0.16, 1)", overshoot: 1.01 },
  calm: { duration: 460, ease: "cubic-bezier(0.4, 0, 0.2, 1)", overshoot: 1 },
};

export function motionProfile(): MotionProfile {
  return MOTION_PROFILES[store.settings.motion_profile] ?? MOTION_PROFILES.soft;
}

export function motionDuration(): number {
  return motionProfile().duration;
}

function isLightColor(hex: string): boolean {
  const value = hex.replace("#", "");
  if (value.length !== 6) return false;
  const r = parseInt(value.slice(0, 2), 16) / 255;
  const g = parseInt(value.slice(2, 4), 16) / 255;
  const b = parseInt(value.slice(4, 6), 16) / 255;
  return 0.2126 * r + 0.7152 * g + 0.0722 * b > 0.55;
}

function customTheme(bg: string, fg: string): Record<string, string> {
  const light = isLightColor(bg);
  return {
    "--bg": bg,
    "--bg-elevated": `color-mix(in srgb, ${bg} 88%, ${fg} 12%)`,
    "--fg": fg,
    "--muted": `color-mix(in srgb, ${fg} 62%, ${bg})`,
    "--surface": `color-mix(in srgb, ${fg} 8%, transparent)`,
    "--surface-strong": `color-mix(in srgb, ${fg} 15%, transparent)`,
    "--surface-raised": `color-mix(in srgb, ${fg} 21%, transparent)`,
    "--border": `color-mix(in srgb, ${fg} 14%, transparent)`,
    "--border-strong": `color-mix(in srgb, ${fg} 24%, transparent)`,
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
  root.style.setProperty("--focus-ring", `color-mix(in srgb, ${store.settings.accent_color} 70%, transparent)`);
  // Custom themes bypass the light/dark blocks, so derive the inverse button
  // tokens from the background luminance.
  if (themeKey === "custom") {
    const light = isLightColor(store.settings.custom_bg);
    root.style.setProperty("--btn-primary-bg", light ? "#1b1a1e" : "#f2f1ee");
    root.style.setProperty("--btn-primary-fg", light ? "#f7f6f2" : "#121214");
  } else {
    root.style.removeProperty("--btn-primary-bg");
    root.style.removeProperty("--btn-primary-fg");
  }
  root.style.setProperty("--radius", `${store.settings.radius}px`);
  const font =
    !store.settings.font || store.settings.font === "system"
      ? '"Segoe UI Variable Text", "Segoe UI Variable Display", "Segoe UI", system-ui, sans-serif'
      : `"${store.settings.font}", "${store.settings.font} Variable", "Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif`;
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
