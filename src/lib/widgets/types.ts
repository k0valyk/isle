export type WidgetSize = "small" | "wide" | "tall" | "large" | "full" | "hero";

export type WidgetKind =
  | "clock"
  | "date"
  | "weather"
  | "media"
  | "lyrics"
  | "system"
  | "cpu"
  | "gpu"
  | "ram"
  | "temp"
  | "fps"
  | "disk"
  | "network"
  | "battery"
  | "timer"
  | "quick_actions"
  | "character"
  | "spectrum"
  | "agents"
  | "calendar";

export interface WidgetConfig {
  id: string;
  kind: WidgetKind;
  size: WidgetSize;
}

export interface WidgetMeta {
  label: string;
  icon: string;
  description: string;
  defaultSize: WidgetSize;
}

export const WIDGET_META: Record<WidgetKind, WidgetMeta> = {
  clock: {
    label: "widget.clock",
    icon: "timer",
    description: "widget.clock_desc",
    defaultSize: "wide",
  },
  date: {
    label: "widget.date",
    icon: "calendar",
    description: "widget.date_desc",
    defaultSize: "wide",
  },
  weather: {
    label: "widget.weather",
    icon: "cloud",
    description: "widget.weather_desc",
    defaultSize: "wide",
  },
  media: {
    label: "widget.media",
    icon: "music",
    description: "widget.media_desc",
    defaultSize: "large",
  },
  lyrics: {
    label: "widget.lyrics",
    icon: "message",
    description: "widget.lyrics_desc",
    defaultSize: "wide",
  },
  system: {
    label: "widget.system",
    icon: "gauge",
    description: "widget.system_desc",
    defaultSize: "large",
  },
  cpu: {
    label: "widget.cpu",
    icon: "cpu",
    description: "widget.cpu_desc",
    defaultSize: "small",
  },
  gpu: {
    label: "widget.gpu",
    icon: "monitor",
    description: "widget.gpu_desc",
    defaultSize: "small",
  },
  ram: {
    label: "widget.ram",
    icon: "memory",
    description: "widget.ram_desc",
    defaultSize: "small",
  },
  temp: {
    label: "widget.temp",
    icon: "thermometer",
    description: "widget.temp_desc",
    defaultSize: "small",
  },
  fps: {
    label: "widget.fps",
    icon: "activity",
    description: "widget.fps_desc",
    defaultSize: "small",
  },
  disk: {
    label: "widget.disk",
    icon: "disk",
    description: "widget.disk_desc",
    defaultSize: "small",
  },
  network: {
    label: "widget.network",
    icon: "network",
    description: "widget.network_desc",
    defaultSize: "wide",
  },
  battery: {
    label: "widget.battery",
    icon: "battery",
    description: "widget.battery_desc",
    defaultSize: "small",
  },
  timer: {
    label: "widget.timer",
    icon: "timer",
    description: "widget.timer_desc",
    defaultSize: "small",
  },
  quick_actions: {
    label: "widget.quick_actions",
    icon: "grid",
    description: "widget.quick_actions_desc",
    defaultSize: "large",
  },
  character: {
    label: "widget.character",
    icon: "eye",
    description: "widget.character_desc",
    defaultSize: "small",
  },
  spectrum: {
    label: "widget.spectrum",
    icon: "music",
    description: "widget.spectrum_desc",
    defaultSize: "wide",
  },
  agents: {
    label: "widget.agents",
    icon: "bot",
    description: "widget.agents_desc",
    defaultSize: "wide",
  },
  calendar: {
    label: "widget.calendar",
    icon: "calendar",
    description: "widget.calendar_desc",
    defaultSize: "wide",
  },
};

export const DEFAULT_WIDGETS: WidgetConfig[] = [
  { id: "widget-clock", kind: "clock", size: "wide" },
  { id: "widget-date", kind: "date", size: "wide" },
  { id: "widget-media", kind: "media", size: "large" },
  { id: "widget-system", kind: "system", size: "large" },
  { id: "widget-actions", kind: "quick_actions", size: "large" },
  { id: "widget-lyrics", kind: "lyrics", size: "wide" },
  { id: "widget-timer", kind: "timer", size: "small" },
  { id: "widget-agents", kind: "agents", size: "wide" },
];

export const SIZE_CYCLE: WidgetSize[] = ["small", "wide", "tall", "large", "full", "hero"];

export function nextSize(size: WidgetSize): WidgetSize {
  const index = SIZE_CYCLE.indexOf(size);
  return SIZE_CYCLE[(index + 1) % SIZE_CYCLE.length];
}

export function widgetId(): string {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return `w-${crypto.randomUUID()}`;
  }
  return `w-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}
