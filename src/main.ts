import "./app.css";
import "animate.css";
import { mount } from "svelte";
import { getCurrentWindow } from "@tauri-apps/api/window";
import IslandApp from "./IslandApp.svelte";
import SettingsApp from "./SettingsApp.svelte";
import { initSettings } from "./lib/store.svelte";

const target = document.getElementById("app");
if (!target) throw new Error("Missing #app root element");

const label = getCurrentWindow().label;
if (label === "settings") {
  document.documentElement.classList.add("settings-window");
  document.body.classList.add("settings-window");
}

void initSettings().then(() => {
  if (label === "settings") {
    mount(SettingsApp, { target });
    // Force one full repaint after the theme variables are applied; this avoids
    // a stale white frame on WebView2 with software compositing.
    requestAnimationFrame(() => {
      document.documentElement.style.opacity = "0.999";
      requestAnimationFrame(() => {
        document.documentElement.style.opacity = "1";
      });
    });
  } else {
    mount(IslandApp, { target });
  }
});
