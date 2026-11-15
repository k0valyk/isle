export const MODULES = [
  { id: "home", icon: "home", implemented: true },
  { id: "player", icon: "music", implemented: true },
  { id: "clipboard", icon: "clipboard", implemented: true },
  { id: "text", icon: "type", implemented: true },
  { id: "notes", icon: "notes", implemented: true },
  { id: "timer", icon: "timer", implemented: true },
  { id: "weather", icon: "cloud", implemented: true },
  { id: "capture", icon: "camera", implemented: true },
  { id: "chat", icon: "message", implemented: true },
  { id: "calendar", icon: "calendar", implemented: true },
  { id: "control", icon: "sliders", implemented: true },
  { id: "stats", icon: "gauge", implemented: true },
  { id: "phone", icon: "phone", implemented: true },
  { id: "agents", icon: "terminal", implemented: true },
  { id: "terminal", icon: "terminal", implemented: true },
] as const;

export type ModuleId = (typeof MODULES)[number]["id"];
