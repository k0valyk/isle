import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

const image = document.getElementById("shot") as HTMLImageElement;
const stage = document.getElementById("app") as HTMLDivElement;
const toast = document.getElementById("toast") as HTMLDivElement;

let path = "";
let scale = 1;
let offsetX = 0;
let offsetY = 0;
let dragging = false;
let origin = { x: 0, y: 0 };
let startOffset = { x: 0, y: 0 };

function applyTransform(): void {
  image.style.transform = `translate(${offsetX}px, ${offsetY}px) scale(${scale})`;
}

function reset(): void {
  scale = 1;
  offsetX = 0;
  offsetY = 0;
  applyTransform();
}

async function load(): Promise<void> {
  try {
    const next = await invoke<string | null>("capture_view_path");
    if (!next) return;
    path = next;
    const data = await invoke<string>("image_data", { path });
    image.src = data;
    reset();
  } catch (error) {
    document.getElementById("hint")!.textContent = String(error);
  }
}

function flash(message: string): void {
  toast.textContent = message;
  toast.classList.add("show");
  setTimeout(() => toast.classList.remove("show"), 1600);
}

async function copy(): Promise<void> {
  if (!path) return;
  try {
    await invoke("clipboard_write_image_path", { path });
    flash("Скопійовано в буфер");
  } catch (error) {
    flash(String(error));
  }
}

function close(): void {
  void getCurrentWindow().destroy();
}

stage.addEventListener(
  "wheel",
  (event) => {
    event.preventDefault();
    const delta = event.deltaY < 0 ? 1.12 : 1 / 1.12;
    scale = Math.max(0.25, Math.min(8, scale * delta));
    applyTransform();
  },
  { passive: false },
);

stage.addEventListener("pointerdown", (event) => {
  if (event.button !== 0) return;
  dragging = true;
  origin = { x: event.clientX, y: event.clientY };
  startOffset = { x: offsetX, y: offsetY };
  stage.setPointerCapture(event.pointerId);
});

stage.addEventListener("pointermove", (event) => {
  if (!dragging) return;
  offsetX = startOffset.x + (event.clientX - origin.x);
  offsetY = startOffset.y + (event.clientY - origin.y);
  applyTransform();
});

stage.addEventListener("pointerup", () => {
  dragging = false;
});

stage.addEventListener("dblclick", reset);

document.getElementById("zoom-in")?.addEventListener("click", () => {
  scale = Math.min(8, scale * 1.2);
  applyTransform();
});
document.getElementById("zoom-out")?.addEventListener("click", () => {
  scale = Math.max(0.25, scale / 1.2);
  applyTransform();
});
document.getElementById("zoom-reset")?.addEventListener("click", reset);
document.getElementById("copy")?.addEventListener("click", () => void copy());
document.getElementById("close")?.addEventListener("click", close);

window.addEventListener("keydown", (event) => {
  if (event.key === "Escape") close();
  else if (event.key === "0") reset();
  else if (event.key === "=" || event.key === "+") {
    scale = Math.min(8, scale * 1.2);
    applyTransform();
  } else if (event.key === "-") {
    scale = Math.max(0.25, scale / 1.2);
    applyTransform();
  } else if (event.key.toLowerCase() === "c" && event.ctrlKey) {
    void copy();
  }
});

void listen("capture-view-updated", () => void load());
void load();

export {};
