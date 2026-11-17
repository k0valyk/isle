<script lang="ts">
  import { onMount, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import Icon from "../../components/Icon.svelte";
  import { t } from "../../store.svelte";

  interface CaptureResult {
    path: string;
    name: string;
    width: number;
    height: number;
    preview: string;
  }

  interface CaptureFile {
    path: string;
    name: string;
    modified: number;
  }

  interface CaptureMonitor {
    index: number;
    name: string;
    x: number;
    y: number;
    width: number;
    height: number;
    primary: boolean;
  }

  let mode = $state<"screen" | "qr">("screen");
  let last = $state<CaptureResult | null>(null);
  let selectedPath = $state<string | null>(null);
  let preview = $state<string>("");
  let files = $state<CaptureFile[]>([]);
  let monitors = $state<CaptureMonitor[]>([]);
  let selectedMonitor = $state<number | null>(null);
  let busy = $state(false);
  let ocrBusy = $state(false);
  let ocrText = $state("");
  let error = $state("");
  let copied = $state(false);
  let imageCopied = $state(false);

  let qrText = $state("");
  let qrImage = $state("");
  let qrBusy = $state(false);
  let qrResults = $state<string[]>([]);
  let qrScanned = $state(false);

  type Tool = "crop" | "brush" | "eraser" | "rect" | "arrow" | "text";
  type Point = { x: number; y: number };
  type Shape =
    | { kind: "rect"; x1: number; y1: number; x2: number; y2: number; color: string; width: number }
    | { kind: "arrow"; x1: number; y1: number; x2: number; y2: number; color: string; width: number }
    | { kind: "text"; x: number; y: number; text: string; color: string; width: number }
    | { kind: "brush"; points: Point[]; color: string; width: number; erase: boolean };

  let editing = $state(false);
  let tool = $state<Tool>("crop");
  let editorText = $state("");
  let editorColor = $state("#ef4444");
  let editorWidth = $state(6);
  let editorCanvas: HTMLCanvasElement | undefined = $state();
  let baseCanvas: HTMLCanvasElement | undefined;
  let annotations: Shape[] = [];
  let drawing = false;
  let startPoint = { x: 0, y: 0 };
  let currentPoint = { x: 0, y: 0 };
  let currentStroke: Shape | null = null;

  async function refreshFiles(): Promise<void> {
    try {
      files = await invoke<CaptureFile[]>("capture_list");
    } catch (cause) {
      error = String(cause);
    }
  }

  async function refreshMonitors(): Promise<void> {
    try {
      monitors = await invoke<CaptureMonitor[]>("capture_monitors");
      if (selectedMonitor !== null && !monitors.some((monitor) => monitor.index === selectedMonitor)) {
        selectedMonitor = null;
      }
    } catch {
      monitors = [];
    }
  }

  async function take(): Promise<void> {
    if (busy) return;
    busy = true;
    error = "";
    try {
      last = await invoke<CaptureResult>("capture_screenshot", {
        monitor: selectedMonitor,
      });
      selectedPath = last.path;
      preview = last.preview;
      await refreshFiles();
    } catch (cause) {
      error = String(cause);
    } finally {
      busy = false;
    }
  }

  async function runOcr(): Promise<void> {
    const target = selectedPath ?? last?.path ?? files[0]?.path;
    if (!target || ocrBusy) return;
    ocrBusy = true;
    error = "";
    try {
      ocrText = await invoke<string>("ocr_image", { path: target });
    } catch (cause) {
      error = String(cause);
    } finally {
      ocrBusy = false;
    }
  }

  async function scanQr(): Promise<void> {
    const target = selectedPath ?? last?.path ?? files[0]?.path;
    if (!target) return;
    qrScanned = true;
    error = "";
    try {
      qrResults = await invoke<string[]>("qr_scan", { path: target });
    } catch (cause) {
      error = String(cause);
    }
  }

  async function generate(): Promise<void> {
    if (!qrText.trim() || qrBusy) return;
    qrBusy = true;
    error = "";
    try {
      qrImage = await invoke<string>("qr_generate", { text: qrText, size: 300 });
    } catch (cause) {
      error = String(cause);
    } finally {
      qrBusy = false;
    }
  }

  async function openFile(path: string): Promise<void> {
    selectedPath = path;
    last = null;
    ocrText = "";
    try {
      preview = await invoke<string>("image_preview", { path });
    } catch (cause) {
      error = String(cause);
    }
  }

  async function openFolder(): Promise<void> {
    try {
      await invoke("open_capture_folder");
    } catch (cause) {
      error = String(cause);
    }
  }

  async function openFullscreen(edit = false): Promise<void> {
    const target = selectedPath ?? last?.path ?? files[0]?.path;
    if (!target) return;
    try {
      await invoke("open_capture_view", { path: target, edit });
    } catch (cause) {
      error = String(cause);
    }
  }

  async function copyImage(): Promise<void> {
    if (!editorCanvas) return;
    try {
      await invoke("clipboard_write_image", {
        dataUrl: editorCanvas.toDataURL("image/png"),
      });
      imageCopied = true;
      setTimeout(() => (imageCopied = false), 1400);
    } catch (cause) {
      error = String(cause);
    }
  }

  async function fullscreenEdit(): Promise<void> {
    if (!editorCanvas) return;
    await saveEdit();
    await openFullscreen(true);
  }

  async function copy(text: string): Promise<void> {
    if (!text) return;
    try {
      await invoke("clipboard_write", { text });
      copied = true;
      setTimeout(() => (copied = false), 1200);
    } catch (cause) {
      error = String(cause);
    }
  }

  async function openEditor(): Promise<void> {
    const target = selectedPath ?? last?.path ?? files[0]?.path;
    if (!target) return;
    error = "";
    try {
      const data = await invoke<string>("image_data", { path: target });
      const image = new Image();
      await new Promise<void>((resolve, reject) => {
        image.onload = () => resolve();
        image.onerror = () => reject(new Error("Could not load the screenshot"));
        image.src = data;
      });
      const base = document.createElement("canvas");
      base.width = image.naturalWidth;
      base.height = image.naturalHeight;
      base.getContext("2d")?.drawImage(image, 0, 0);
      baseCanvas = base;
      annotations = [];
      currentStroke = null;
      editing = true;
      tool = "crop";
      editorColor = "#ef4444";
      editorWidth = 6;
      await tick();
      if (editorCanvas) {
        editorCanvas.width = base.width;
        editorCanvas.height = base.height;
      }
      redraw();
    } catch (cause) {
      error = String(cause);
    }
  }

  function editorPoint(event: PointerEvent): { x: number; y: number } {
    const canvas = editorCanvas;
    if (!canvas) return { x: 0, y: 0 };
    const rect = canvas.getBoundingClientRect();
    return {
      x: ((event.clientX - rect.left) / rect.width) * canvas.width,
      y: ((event.clientY - rect.top) / rect.height) * canvas.height,
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

  function paint(shape: Shape, defaultWidth: number): void {
    const canvas = editorCanvas;
    const context = canvas?.getContext("2d");
    if (!context) return;
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
      if (shape.erase && baseCanvas) {
        // Restore the original pixels inside the erased stroke, leaving
        // annotations outside it untouched.
        context.save();
        context.globalCompositeOperation = "destination-over";
        context.drawImage(baseCanvas, 0, 0);
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
      context.strokeRect(
        shape.x1,
        shape.y1,
        shape.x2 - shape.x1,
        shape.y2 - shape.y1,
      );
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
    const canvas = editorCanvas;
    const base = baseCanvas;
    const context = canvas?.getContext("2d");
    if (!canvas || !base || !context) return;
    context.clearRect(0, 0, canvas.width, canvas.height);
    context.drawImage(base, 0, 0);
    const width = Math.max(3, Math.round(canvas.width / 600));
    for (const shape of annotations) paint(shape, width);
    if (preview) paint(preview, width);
  }

  function onEditorDown(event: PointerEvent): void {
    if (event.button !== 0) return;
    const point = editorPoint(event);
    if (tool === "text") {
      const text = editorText.trim();
      if (!text) {
        error = t("capture.text_empty");
        return;
      }
      annotations.push({
        kind: "text",
        x: point.x,
        y: point.y,
        text,
        color: editorColor,
        width: editorWidth,
      });
      redraw();
      return;
    }
    drawing = true;
    startPoint = point;
    currentPoint = point;
    if (tool === "brush" || tool === "eraser") {
      currentStroke = {
        kind: "brush",
        points: [point],
        color: editorColor,
        width: editorWidth,
        erase: tool === "eraser",
      };
    } else {
      currentStroke = null;
    }
    try {
      (event.currentTarget as HTMLCanvasElement).setPointerCapture(event.pointerId);
    } catch {
      /* synthetic pointers and some WebView2 cases do not support capture */
    }
  }

  function onEditorMove(event: PointerEvent): void {
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
      color: editorColor,
      width: editorWidth,
    });
  }

  function onEditorUp(): void {
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
      const canvas = editorCanvas;
      const base = baseCanvas;
      if (!canvas || !base || x2 - x1 < 6 || y2 - y1 < 6) {
        redraw();
        return;
      }
      const flat = document.createElement("canvas");
      flat.width = canvas.width;
      flat.height = canvas.height;
      flat.getContext("2d")?.drawImage(canvas, 0, 0);

      const cropped = document.createElement("canvas");
      cropped.width = Math.round(x2 - x1);
      cropped.height = Math.round(y2 - y1);
      cropped
        .getContext("2d")
        ?.drawImage(flat, x1, y1, x2 - x1, y2 - y1, 0, 0, cropped.width, cropped.height);
      baseCanvas = cropped;
      annotations = [];
      canvas.width = cropped.width;
      canvas.height = cropped.height;
      redraw();
      return;
    }

    annotations.push({
      kind: tool === "arrow" ? "arrow" : "rect",
      x1: startPoint.x,
      y1: startPoint.y,
      x2: currentPoint.x,
      y2: currentPoint.y,
      color: editorColor,
      width: editorWidth,
    });
    redraw();
  }

  function undo(): void {
    annotations.pop();
    redraw();
  }

  async function saveEdit(): Promise<void> {
    if (!editorCanvas) return;
    try {
      const result = await invoke<CaptureResult>("save_capture_data", {
        dataUrl: editorCanvas.toDataURL("image/png"),
        name: null,
      });
      last = result;
      selectedPath = result.path;
      preview = result.preview;
      editing = false;
      await refreshFiles();
    } catch (cause) {
      error = String(cause);
    }
  }

  onMount(() => {
    void refreshFiles();
    void refreshMonitors();
  });
</script>

<div class="capture">
  <div class="modes" role="tablist">
    <button class:active={mode === "screen"} role="tab" onclick={() => (mode = "screen")}>
      <Icon name="camera" size={13} />
      {t("capture.screen")}
    </button>
    <button class:active={mode === "qr"} role="tab" onclick={() => { editing = false; mode = "qr"; }}>
      <Icon name="qr" size={13} />
      {t("capture.qr")}
    </button>
    <span class="spacer"></span>
    <span class="count">{t("capture.count", { count: files.length })}</span>
  </div>

  {#if mode === "screen"}
    <div class="monitors">
      <span class="monitor-label">{t("capture.monitor")}</span>
      <button
        class="monitor-chip"
        class:active={selectedMonitor === null}
        onclick={() => (selectedMonitor = null)}
        title={t("capture.monitor_all")}
      >
        {t("capture.monitor_all")}
      </button>
      {#each monitors as monitor (monitor.index)}
        <button
          class="monitor-chip"
          class:active={selectedMonitor === monitor.index}
          onclick={() => (selectedMonitor = monitor.index)}
          title={`${monitor.width}×${monitor.height}`}
        >
          {monitor.index + 1}{#if monitor.primary}<span class="primary">★</span>{/if}
        </button>
      {/each}
      {#if monitors.length > 1}
        <span class="monitor-hint">{t("capture.monitor_count", { count: monitors.length })}</span>
      {/if}
      <span class="spacer"></span>
      <button
        class="btn small"
        onclick={() => void openFullscreen()}
        disabled={!preview}
        title={t("capture.fullscreen")}
      >
        <Icon name="expand" size={12} />{t("capture.fullscreen")}
      </button>
    </div>
    {#if editing}
      <div class="editor">
        <div class="editor-bar">
          <button class="etool" class:active={tool === "crop"} onclick={() => (tool = "crop")} title={t("capture.crop")} aria-label={t("capture.crop")}>
            <Icon name="crop" size={12} />
          </button>
          <button class="etool" class:active={tool === "brush"} onclick={() => (tool = "brush")} title={t("capture.brush")} aria-label={t("capture.brush")}>
            <Icon name="brush" size={12} />
          </button>
          <button class="etool" class:active={tool === "eraser"} onclick={() => (tool = "eraser")} title={t("capture.eraser")} aria-label={t("capture.eraser")}>
            <Icon name="eraser" size={12} />
          </button>
          <button class="etool" class:active={tool === "rect"} onclick={() => (tool = "rect")} title={t("capture.rect")} aria-label={t("capture.rect")}>
            <Icon name="square" size={12} />
          </button>
          <button class="etool" class:active={tool === "arrow"} onclick={() => (tool = "arrow")} title={t("capture.arrow")} aria-label={t("capture.arrow")}>
            <Icon name="arrow" size={12} />
          </button>
          <button class="etool" class:active={tool === "text"} onclick={() => (tool = "text")} title={t("capture.text")} aria-label={t("capture.text")}>
            <Icon name="type" size={12} />
          </button>
          {#if tool === "text"}
            <input class="etext" bind:value={editorText} placeholder={t("capture.text_placeholder")} aria-label={t("capture.text")} />
          {/if}
          {#if tool !== "crop" && tool !== "text"}
            <input
              class="ecolor"
              type="color"
              bind:value={editorColor}
              title={t("capture.color")}
              aria-label={t("capture.color")}
            />
            <input
              class="ewidth"
              type="range"
              min="1"
              max="24"
              bind:value={editorWidth}
              title={t("capture.thickness")}
              aria-label={t("capture.thickness")}
            />
          {/if}
          <span class="spacer"></span>
          <button class="etool" onclick={undo} title={t("capture.undo")} aria-label={t("capture.undo")}>
            <Icon name="undo" size={12} />
          </button>
          <button class="etool" onclick={copyImage} title={t("capture.copy_image")} aria-label={t("capture.copy_image")}>
            <Icon name={imageCopied ? "check" : "copy"} size={12} />
          </button>
          <button class="etool" onclick={fullscreenEdit} title={t("capture.fullscreen")} aria-label={t("capture.fullscreen")}>
            <Icon name="expand" size={12} />
          </button>
          <button class="etool" onclick={() => (editing = false)} title={t("common.cancel")} aria-label={t("common.cancel")}>
            <Icon name="x" size={12} />
          </button>
          <button class="esave" onclick={saveEdit} title={t("capture.save")}>
            <Icon name="check" size={12} />{t("capture.save")}
          </button>
        </div>
        <div class="editor-stage" data-no-drag>
          <canvas
            bind:this={editorCanvas}
            data-no-drag
            onpointerdown={onEditorDown}
            onpointermove={onEditorMove}
            onpointerup={onEditorUp}
            onpointercancel={onEditorUp}
            aria-label={t("capture.editor")}
          ></canvas>
        </div>
      </div>
    {:else}
      <div class="screen-grid">
        <div class="left">
          <button class="shot" class:empty={!preview} onclick={take} disabled={busy} aria-label={t("capture.take")}>
            {#if preview}
              <img src={preview} alt={t("capture.preview")} />
            {:else}
              <span class="shot-empty">
                <Icon name="camera" size={22} />
                <span>{busy ? t("capture.capturing") : t("capture.take")}</span>
              </span>
            {/if}
            {#if busy}<span class="shooting">{t("capture.capturing")}</span>{/if}
          </button>
          <div class="file-row">
            {#each files.slice(0, 4) as file (file.path)}
              <button class="file-chip" title={file.name} onclick={() => openFile(file.path)}>
                {file.name.replace("Isle-", "").replace(".png", "")}
              </button>
            {/each}
            {#if !files.length}<span class="muted">{t("capture.empty")}</span>{/if}
          </div>
        </div>

        <div class="right">
          <div class="actions">
            <button class="btn primary" onclick={take} disabled={busy}>
              <Icon name="camera" size={13} />{t("capture.take")}
            </button>
            <button class="btn" onclick={openEditor} disabled={!last && !files.length}>
              <Icon name="crop" size={13} />{t("capture.edit")}
            </button>
            <button class="btn" onclick={() => void openFullscreen()} disabled={!preview}>
              <Icon name="expand" size={13} />{t("capture.fullscreen")}
            </button>
            <button class="btn" onclick={runOcr} disabled={ocrBusy || (!last && !files.length)}>
              <Icon name="scan" size={13} />{ocrBusy ? t("capture.ocr_running") : t("capture.ocr")}
            </button>
            <button class="btn icon-only" onclick={scanQr} disabled={!last && !files.length} aria-label={t("capture.qr_scan")}>
              <Icon name="qr" size={13} />
            </button>
            <button class="btn icon-only" onclick={openFolder} aria-label={t("capture.open_folder")}>
              <Icon name="folder" size={13} />
            </button>
          </div>

          <div class="ocr">
            <div class="ocr-head">
              <span>{t("capture.ocr_result")}</span>
              {#if ocrText}
                <button class="mini" onclick={() => copy(ocrText)}>
                  <Icon name={copied ? "check" : "copy"} size={11} />
                  {copied ? t("common.copied") : t("common.copy")}
                </button>
              {/if}
            </div>
            {#if ocrText}
              <textarea class="ocr-text" readonly value={ocrText}></textarea>
            {:else}
              <span class="muted pad">{t("capture.ocr_hint")}</span>
            {/if}
          </div>
        </div>
      </div>

      {#if qrScanned}
        <div class="scan-row">
          <Icon name="qr" size={12} />
          <span class="scan-label">{t("capture.qr_result")}:</span>
          {#if qrResults.length}
            {#each qrResults as result}
              <button class="code" onclick={() => copy(result)}>{result}</button>
            {/each}
          {:else}
            <span class="muted">{t("capture.qr_none")}</span>
          {/if}
        </div>
      {/if}
    {/if}
  {:else}
    <div class="qr-grid">
      <div class="qr-input">
        <textarea
          bind:value={qrText}
          placeholder={t("capture.qr_placeholder")}
          aria-label={t("capture.qr")}
        ></textarea>
        <div class="actions">
          <button class="btn primary" onclick={generate} disabled={qrBusy || !qrText.trim()}>
            <Icon name="qr" size={13} />{t("capture.qr_generate")}
          </button>
          <button class="btn" onclick={scanQr} disabled={!last && !files.length}>
            <Icon name="scan" size={13} />{t("capture.qr_scan")}
          </button>
          <button class="btn icon-only" onclick={openFolder} aria-label={t("capture.open_folder")}>
            <Icon name="folder" size={13} />
          </button>
        </div>
        {#if qrResults.length}
          <div class="scan-list">
            {#each qrResults as result}
              <button class="code" onclick={() => copy(result)}>{result}</button>
            {/each}
          </div>
        {:else if qrScanned}
          <span class="muted">{t("capture.qr_none")}</span>
        {/if}
      </div>
      <div class="qr-preview">
        {#if qrImage}
          <img src={qrImage} alt={t("capture.qr")} />
        {:else}
          <Icon name="qr" size={30} />
        {/if}
      </div>
    </div>
  {/if}

  {#if error}
    <div class="error" role="alert">{error}</div>
  {/if}
</div>

<style>
  .capture {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-height: 0;
  }

  .modes {
    display: flex;
    align-items: center;
    gap: 3px;
    flex: none;
  }

  .modes button {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    border: 1px solid transparent;
    background: transparent;
    color: var(--muted);
    border-radius: 7px;
    padding: 4px 9px;
    font-size: 11px;
  }

  .modes button.active {
    background: var(--surface-strong);
    color: var(--fg);
    border-color: var(--border);
  }

  .spacer {
    flex: 1;
  }

  .monitors {
    flex: none;
    display: flex;
    align-items: center;
    gap: 4px;
    min-height: 24px;
  }

  .monitor-label {
    font-size: 9px;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--muted);
    margin-right: 2px;
  }

  .monitor-chip {
    min-width: 24px;
    height: 22px;
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--muted);
    border-radius: 6px;
    padding: 0 6px;
    font-size: 10px;
    font-variant-numeric: tabular-nums;
  }

  .monitor-chip:hover,
  .monitor-chip.active {
    background: var(--surface-strong);
    color: var(--fg);
    border-color: color-mix(in srgb, var(--accent) 45%, var(--border));
  }

  .monitor-chip.active {
    color: var(--accent);
  }

  .monitor-chip .primary {
    margin-left: 2px;
    font-size: 8px;
  }

  .monitor-hint {
    font-size: 8.5px;
    color: var(--muted);
  }

  .btn.small {
    min-height: 23px;
    font-size: 10px;
    padding: 3px 7px;
  }

  .ecolor {
    width: 24px;
    height: 22px;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: transparent;
    cursor: pointer;
  }

  .ewidth {
    width: 54px;
    accent-color: var(--accent);
  }

  .count {
    font-size: 9.5px;
    color: var(--muted);
  }

  .screen-grid {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 1.05fr 1fr;
    gap: 8px;
  }

  .left,
  .right {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
    min-height: 0;
  }

  .shot {
    position: relative;
    flex: 1;
    min-height: 0;
    border-radius: 10px;
    border: 1px solid var(--border);
    background: var(--surface);
    padding: 0;
    overflow: hidden;
    color: var(--muted);
  }

  .shot img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .shot-empty {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    font-size: 10.5px;
  }

  .shooting {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    background: rgba(0, 0, 0, 0.5);
    font-size: 11px;
    color: #fff;
  }

  .file-row {
    display: flex;
    gap: 4px;
    min-height: 18px;
    overflow: hidden;
    align-items: center;
  }

  .file-chip {
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--muted);
    border-radius: 6px;
    font-size: 9px;
    padding: 2px 6px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 78px;
  }

  .file-chip:hover {
    color: var(--fg);
    background: var(--surface-strong);
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-wrap: wrap;
  }

  .ocr {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border);
    border-radius: 9px;
    background: var(--surface);
    padding: 5px 7px;
    gap: 3px;
  }

  .ocr-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 9px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--muted);
  }

  .mini {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    border: none;
    background: transparent;
    color: var(--muted);
    font-size: 9px;
    padding: 1px 3px;
  }

  .ocr-text {
    flex: 1;
    min-height: 0;
    border: none;
    background: transparent;
    color: var(--fg);
    font-size: 10.5px;
    line-height: 1.4;
    resize: none;
    padding: 0;
    font-family: inherit;
  }

  .pad {
    padding: 10px 2px;
  }

  .muted {
    color: var(--muted);
    font-size: 9.5px;
  }

  .scan-row {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 10px;
    flex: none;
    overflow: hidden;
  }

  .scan-label {
    color: var(--muted);
    white-space: nowrap;
  }

  .code {
    border: 1px solid var(--border);
    background: var(--surface-strong);
    color: var(--fg);
    border-radius: 5px;
    font-size: 9.5px;
    padding: 2px 6px;
    max-width: 220px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .qr-grid {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 1fr 136px;
    gap: 8px;
  }

  .qr-input {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-height: 0;
  }

  .qr-input textarea {
    flex: 1;
    min-height: 0;
    border-radius: 9px;
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--fg);
    padding: 8px;
    font-family: inherit;
    font-size: 11px;
    resize: none;
  }

  .qr-preview {
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--surface);
    display: grid;
    place-items: center;
    color: var(--muted);
    overflow: hidden;
    padding: 6px;
  }

  .qr-preview img {
    width: 100%;
    height: 100%;
    object-fit: contain;
    image-rendering: pixelated;
    border-radius: 3px;
    background: #fff;
  }

  .scan-list {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
    max-height: 46px;
    overflow-y: auto;
  }

  .error {
    font-size: 10px;
    color: var(--danger);
    flex: none;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .editor {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .editor-bar {
    display: flex;
    align-items: center;
    gap: 3px;
    flex: none;
  }

  .etool {
    width: 22px;
    height: 22px;
    display: grid;
    place-items: center;
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--muted);
    border-radius: 5px;
    padding: 0;
    flex: none;
  }

  .etool.active {
    background: var(--accent);
    border-color: transparent;
    color: #fff;
  }

  .etool:hover {
    color: var(--fg);
  }

  .etext {
    min-width: 60px;
    flex: 1;
    height: 22px;
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--fg);
    border-radius: 5px;
    padding: 1px 6px;
    font-size: 9.5px;
    font-family: inherit;
  }

  .esave {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    height: 22px;
    border: none;
    border-radius: 5px;
    background: var(--accent);
    color: #fff;
    font-size: 9.5px;
    padding: 0 7px;
    flex: none;
  }

  .editor-stage {
    flex: 1;
    min-height: 0;
    display: grid;
    place-items: center;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: #050506;
    overflow: hidden;
    padding: 4px;
  }

  .editor-stage canvas {
    max-width: 100%;
    max-height: 100%;
    width: auto;
    height: auto;
    display: block;
    border-radius: 4px;
    cursor: crosshair;
  }
</style>
