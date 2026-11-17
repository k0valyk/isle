import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

type Tool = "crop" | "pen" | "eraser" | "rect" | "arrow" | "text";
type Point = { x: number; y: number };
type Shape =
  | { kind: "rect"; x1: number; y1: number; x2: number; y2: number; color: string; width: number }
  | { kind: "arrow"; x1: number; y1: number; x2: number; y2: number; color: string; width: number }
  | { kind: "text"; x: number; y: number; text: string; color: string; width: number }
  | { kind: "brush"; points: Point[]; color: string; width: number; erase: boolean };

interface CaptureResult {
  path: string;
  name: string;
  width: number;
  height: number;
  preview: string;
}

const image = document.getElementById("shot") as HTMLImageElement;
const stage = document.getElementById("app") as HTMLDivElement;
const pad = document.getElementById("pad") as HTMLCanvasElement;
const bar = document.getElementById("bar") as HTMLDivElement;
const editorBar = document.getElementById("editor-bar") as HTMLDivElement;
const hint = document.getElementById("hint") as HTMLDivElement;
const toast = document.getElementById("toast") as HTMLDivElement;
const textInput = document.getElementById("text-input") as HTMLInputElement;
const colorInput = document.getElementById("color") as HTMLInputElement;
const widthInput = document.getElementById("width") as HTMLInputElement;

let path = "";
let scale = 1;
let offsetX = 0;
let offsetY = 0;
let dragging = false;
let origin = { x: 0, y: 0 };
let startOffset = { x: 0, y: 0 };

let editing = false;
let tool: Tool = "crop";
let base: HTMLCanvasElement | null = null;
let annotations: Shape[] = [];
let drawing = false;
let startPoint: Point = { x: 0, y: 0 };
let currentPoint: Point = { x: 0, y: 0 };
let currentStroke: Shape | null = null;
let pendingEdit = new URLSearchParams(window.location.search).has("edit");

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
    await image.decode().catch(() => {});
    reset();
    if (pendingEdit) {
      pendingEdit = false;
      await startEditing();
    }
  } catch (error) {
    hint.textContent = String(error);
  }
}

function flash(message: string): void {
  toast.textContent = message;
  toast.classList.add("show");
  setTimeout(() => toast.classList.remove("show"), 1800);
}

async function copy(): Promise<void> {
  try {
    if (editing) {
      await invoke("clipboard_write_image", { dataUrl: pad.toDataURL("image/png") });
    } else {
      if (!path) return;
      await invoke("clipboard_write_image_path", { path });
    }
    flash("Скопійовано в буфер");
  } catch (error) {
    flash(String(error));
  }
}

function close(): void {
  if (editing) {
    void stopEditing();
    return;
  }
  void getCurrentWindow().destroy();
}

function editorPoint(event: PointerEvent): Point {
  const rect = pad.getBoundingClientRect();
  return {
    x: ((event.clientX - rect.left) / rect.width) * pad.width,
    y: ((event.clientY - rect.top) / rect.height) * pad.height,
  };
}

function drawArrow(
  context: CanvasRenderingContext2D,
  x1: number,
  y1: number,
  x2: number,
  y2: number,
  width: number,
): void {
  const angle = Math.atan2(y2 - y1, x2 - x1);
  const head = Math.max(width * 3.2, 14);
  context.beginPath();
  context.moveTo(x1, y1);
  context.lineTo(x2, y2);
  context.lineTo(
    x2 - head * Math.cos(angle - Math.PI / 7),
    y2 - head * Math.sin(angle - Math.PI / 7),
  );
  context.moveTo(x2, y2);
  context.lineTo(
    x2 - head * Math.cos(angle + Math.PI / 7),
    y2 - head * Math.sin(angle + Math.PI / 7),
  );
  context.stroke();
}

function paint(shape: Shape, context: CanvasRenderingContext2D, defaultWidth: number): void {
  const width = Math.max(1, shape.width || defaultWidth);
  if (shape.kind === "brush") {
    if (shape.points.length === 0) return;
    context.save();
    context.strokeStyle = shape.color;
    context.lineWidth = width;
    context.lineCap = "round";
    context.lineJoin = "round";
    if (shape.erase) context.globalCompositeOperation = "destination-out";
    context.beginPath();
    context.moveTo(shape.points[0].x, shape.points[0].y);
    for (const point of shape.points.slice(1)) context.lineTo(point.x, point.y);
    if (shape.points.length === 1) {
      context.lineTo(shape.points[0].x + 0.1, shape.points[0].y + 0.1);
    }
    context.stroke();
    context.restore();
    if (shape.erase && base) {
      context.save();
      context.globalCompositeOperation = "destination-over";
      context.drawImage(base, 0, 0);
      context.restore();
    }
    return;
  }

  context.strokeStyle = shape.color;
  context.fillStyle = shape.color;
  context.lineWidth = width;
  context.lineCap = "round";
  context.lineJoin = "round";
  if (shape.kind === "rect") {
    context.strokeRect(shape.x1, shape.y1, shape.x2 - shape.x1, shape.y2 - shape.y1);
  } else if (shape.kind === "arrow") {
    drawArrow(context, shape.x1, shape.y1, shape.x2, shape.y2, width);
  } else {
    const size = Math.max(14, Math.round(width * 4));
    context.font = `600 ${size}px "Segoe UI", sans-serif`;
    context.shadowColor = "rgba(0,0,0,0.6)";
    context.shadowBlur = 6;
    context.fillText(shape.text, shape.x, shape.y);
    context.shadowBlur = 0;
  }
}

function redraw(preview?: Shape): void {
  const context = pad.getContext("2d");
  if (!context || !base) return;
  context.clearRect(0, 0, pad.width, pad.height);
  context.drawImage(base, 0, 0);
  const width = Math.max(3, Math.round(pad.width / 600));
  for (const shape of annotations) paint(shape, context, width);
  if (preview) paint(preview, context, width);
}

function setTool(next: Tool): void {
  tool = next;
  for (const button of editorBar.querySelectorAll<HTMLButtonElement>("[data-tool]")) {
    button.classList.toggle("active", button.dataset.tool === next);
  }
  textInput.classList.toggle("show", next === "text");
  if (next === "text") textInput.focus();
}

function onPadDown(event: PointerEvent): void {
  if (event.button !== 0) return;
  const point = editorPoint(event);
  if (tool === "text") {
    const text = textInput.value.trim();
    if (!text) {
      flash("Спочатку введи текст");
      textInput.focus();
      return;
    }
    annotations.push({
      kind: "text",
      x: point.x,
      y: point.y,
      text,
      color: colorInput.value,
      width: Number(widthInput.value),
    });
    redraw();
    return;
  }
  drawing = true;
  startPoint = point;
  currentPoint = point;
  if (tool === "pen" || tool === "eraser") {
    currentStroke = {
      kind: "brush",
      points: [point],
      color: colorInput.value,
      width: Number(widthInput.value),
      erase: tool === "eraser",
    };
  } else {
    currentStroke = null;
  }
  try {
    pad.setPointerCapture(event.pointerId);
  } catch {
    /* pointer capture is optional */
  }
}

function onPadMove(event: PointerEvent): void {
  if (!drawing) return;
  currentPoint = editorPoint(event);
  if (currentStroke && currentStroke.kind === "brush") {
    currentStroke.points.push(currentPoint);
    redraw(currentStroke);
    return;
  }
  redraw({
    kind: tool === "crop" ? "rect" : tool === "arrow" ? "arrow" : "rect",
    x1: startPoint.x,
    y1: startPoint.y,
    x2: currentPoint.x,
    y2: currentPoint.y,
    color: colorInput.value,
    width: Number(widthInput.value),
  });
}

function onPadUp(): void {
  if (!drawing) return;
  drawing = false;
  if (currentStroke) {
    annotations.push(currentStroke);
    currentStroke = null;
    redraw();
    return;
  }
  const x1 = Math.min(startPoint.x, currentPoint.x);
  const y1 = Math.min(startPoint.y, currentPoint.y);
  const x2 = Math.max(startPoint.x, currentPoint.x);
  const y2 = Math.max(startPoint.y, currentPoint.y);

  if (tool === "crop") {
    if (!base || x2 - x1 < 6 || y2 - y1 < 6) {
      redraw();
      return;
    }
    const flat = document.createElement("canvas");
    flat.width = pad.width;
    flat.height = pad.height;
    flat.getContext("2d")?.drawImage(pad, 0, 0);
    const cropped = document.createElement("canvas");
    cropped.width = Math.round(x2 - x1);
    cropped.height = Math.round(y2 - y1);
    cropped
      .getContext("2d")
      ?.drawImage(flat, x1, y1, x2 - x1, y2 - y1, 0, 0, cropped.width, cropped.height);
    base = cropped;
    annotations = [];
    pad.width = cropped.width;
    pad.height = cropped.height;
    redraw();
    return;
  }

  annotations.push({
    kind: tool === "arrow" ? "arrow" : "rect",
    x1: startPoint.x,
    y1: startPoint.y,
    x2: currentPoint.x,
    y2: currentPoint.y,
    color: colorInput.value,
    width: Number(widthInput.value),
  });
  redraw();
}

function undo(): void {
  annotations.pop();
  redraw();
}

async function startEditing(): Promise<void> {
  if (!image.complete || image.naturalWidth === 0) return;
  reset();
  base = document.createElement("canvas");
  base.width = image.naturalWidth;
  base.height = image.naturalHeight;
  base.getContext("2d")?.drawImage(image, 0, 0);
  pad.width = base.width;
  pad.height = base.height;
  annotations = [];
  currentStroke = null;
  editing = true;
  stage.classList.add("editing");
  image.style.display = "none";
  pad.classList.add("show");
  bar.style.display = "none";
  hint.style.display = "none";
  editorBar.classList.add("show");
  setTool("crop");
  redraw();
}

async function stopEditing(): Promise<void> {
  editing = false;
  stage.classList.remove("editing");
  pad.classList.remove("show");
  image.style.display = "";
  bar.style.display = "";
  hint.style.display = "";
  editorBar.classList.remove("show");
  textInput.classList.remove("show");
  await load();
}

async function saveEditing(): Promise<void> {
  if (!editing) return;
  try {
    const result = await invoke<CaptureResult>("save_capture_data", {
      dataUrl: pad.toDataURL("image/png"),
      name: null,
    });
    path = result.path;
    image.src = result.preview;
    flash("Збережено");
    editing = false;
    stage.classList.remove("editing");
    pad.classList.remove("show");
    image.style.display = "";
    bar.style.display = "";
    hint.style.display = "";
    editorBar.classList.remove("show");
    textInput.classList.remove("show");
    reset();
  } catch (error) {
    flash(String(error));
  }
}

stage.addEventListener(
  "wheel",
  (event) => {
    if (editing) return;
    event.preventDefault();
    const delta = event.deltaY < 0 ? 1.12 : 1 / 1.12;
    scale = Math.max(0.25, Math.min(8, scale * delta));
    applyTransform();
  },
  { passive: false },
);

stage.addEventListener("pointerdown", (event) => {
  if (editing || event.button !== 0) return;
  dragging = true;
  origin = { x: event.clientX, y: event.clientY };
  startOffset = { x: offsetX, y: offsetY };
  stage.setPointerCapture(event.pointerId);
});

stage.addEventListener("pointermove", (event) => {
  if (editing || !dragging) return;
  offsetX = startOffset.x + (event.clientX - origin.x);
  offsetY = startOffset.y + (event.clientY - origin.y);
  applyTransform();
});

stage.addEventListener("pointerup", () => {
  dragging = false;
});

stage.addEventListener("dblclick", () => {
  if (!editing) reset();
});

pad.addEventListener("pointerdown", onPadDown);
pad.addEventListener("pointermove", onPadMove);
pad.addEventListener("pointerup", onPadUp);
pad.addEventListener("pointercancel", onPadUp);

document.getElementById("zoom-in")?.addEventListener("click", () => {
  scale = Math.min(8, scale * 1.2);
  applyTransform();
});
document.getElementById("zoom-out")?.addEventListener("click", () => {
  scale = Math.max(0.25, scale / 1.2);
  applyTransform();
});
document.getElementById("zoom-reset")?.addEventListener("click", reset);
document.getElementById("edit")?.addEventListener("click", () => void startEditing());
document.getElementById("copy")?.addEventListener("click", () => void copy());
document.getElementById("close")?.addEventListener("click", close);
document.getElementById("undo")?.addEventListener("click", undo);
document.getElementById("save")?.addEventListener("click", () => void saveEditing());
document.getElementById("cancel")?.addEventListener("click", () => void stopEditing());

for (const button of editorBar.querySelectorAll<HTMLButtonElement>("[data-tool]")) {
  button.addEventListener("click", () => setTool(button.dataset.tool as Tool));
}

window.addEventListener("keydown", (event) => {
  if (event.key === "Escape") close();
  else if (event.ctrlKey && event.key.toLowerCase() === "s") {
    event.preventDefault();
    void saveEditing();
  } else if (event.ctrlKey && event.key.toLowerCase() === "z") {
    event.preventDefault();
    undo();
  } else if (event.key.toLowerCase() === "e" && !editing) {
    void startEditing();
  } else if (!editing && event.key === "0") reset();
  else if (!editing && (event.key === "=" || event.key === "+")) {
    scale = Math.min(8, scale * 1.2);
    applyTransform();
  } else if (!editing && event.key === "-") {
    scale = Math.max(0.25, scale / 1.2);
    applyTransform();
  } else if (!editing && event.key.toLowerCase() === "c" && event.ctrlKey) {
    void copy();
  }
});

void listen("capture-view-updated", () => {
  if (!editing) void load();
});
void listen("capture-view-edit", () => {
  void startEditing();
});
void load();

export {};
