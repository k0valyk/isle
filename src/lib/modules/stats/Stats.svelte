<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import Icon from "../../components/Icon.svelte";
  import { locale, t } from "../../store.svelte";
  import { history, system } from "../../stats.svelte";

  const stats = $derived(system.data);
  const spark = $derived(history.cpu.slice(-36));
  let fpsActionError = $state("");

  async function restartAsAdmin(): Promise<void> {
    fpsActionError = "";
    try {
      await invoke("restart_as_admin");
    } catch (error) {
      fpsActionError = String(error);
    }
  }

  function percent(value: number | null | undefined): number {
    return Math.max(0, Math.min(100, Math.round(value ?? 0)));
  }

  function sparkline(values: number[]): string {
    if (values.length < 2) return "";
    const step = 100 / (values.length - 1);
    return values
      .map((value, index) => `${(index * step).toFixed(2)},${(100 - value).toFixed(2)}`)
      .join(" ");
  }

  function hardware(value: string | undefined): string {
    return value && value.trim() ? value : t("stats.unavailable");
  }

  function fpsHint(value: typeof stats): string {
    if (value?.fps != null) return t("stats.fps_live");
    if (value?.fpsError === "admin") return t("stats.fps_admin");
    if (value?.fpsError === "missing") return t("stats.fps_missing");
    if (value?.fpsError === "capture") return t("stats.fps_capture");
    return t("stats.fps_hint");
  }

  function bytes(value: number): string {
    if (!Number.isFinite(value) || value <= 0) return "0";
    if (value >= 1024 * 1024) return `${(value / 1024 / 1024).toFixed(1)} MB/s`;
    if (value >= 1024) return `${(value / 1024).toFixed(0)} KB/s`;
    return `${value.toFixed(0)} B/s`;
  }
</script>

<div class="stats">
  <div class="head">
    <span class="live"><span class="dot"></span>{t("stats.live")}</span>
    {#if stats}
      <span class="time">{t("stats.updated", { time: new Date(stats.checkedAt).toLocaleTimeString(locale(), { hour: "2-digit", minute: "2-digit" }) })}</span>
    {/if}
  </div>

  <div class="spark" aria-hidden="true">
    <svg viewBox="0 0 100 100" preserveAspectRatio="none">
      <polyline points={sparkline(spark)} />
    </svg>
  </div>

  <div class="grid">
    <div class="stat">
      <div class="label"><Icon name="cpu" size={13} /><span>CPU</span><strong>{percent(stats?.cpu)}%</strong></div>
      <div class="bar"><span style:width={`${percent(stats?.cpu)}%`}></span></div>
    </div>

    <div class="stat">
      <div class="label"><Icon name="monitor" size={13} /><span>GPU</span><strong>{stats?.gpu === null || stats?.gpu === undefined ? t("stats.unavailable") : `${percent(stats.gpu)}%`}</strong></div>
      <div class="bar"><span style:width={`${percent(stats?.gpu)}%`}></span></div>
    </div>

    <div class="stat">
      <div class="label"><Icon name="memory" size={13} /><span>RAM</span><strong>{percent(stats?.ram)}%</strong></div>
      <div class="bar"><span style:width={`${percent(stats?.ram)}%`}></span></div>
      {#if stats}
        <span class="hint">{t("stats.memory", { used: stats.ramUsedGb.toFixed(1), total: stats.ramTotalGb.toFixed(0) })}</span>
      {/if}
    </div>
  </div>

  <div class="fps" class:live={stats?.fps != null}>
    <span class="fps-number">{stats?.fps != null ? Math.round(stats.fps) : "—"}</span>
    <div class="fps-copy">
      <span class="fps-label">
        FPS
        {#if stats?.fps != null && stats.fpsProcess}
          <span class="fps-game">· {stats.fpsProcess}</span>
        {/if}
      </span>
      <span class="fps-hint">{fpsHint(stats)}{fpsActionError ? ` · ${fpsActionError}` : ""}</span>
      {#if stats?.fpsError === "admin"}
        <button class="fps-admin" onclick={() => void restartAsAdmin()}>
          {t("stats.elevate")}
        </button>
      {/if}
    </div>
  </div>

  <div class="hardware">
    <span class="hw-title">{t("stats.hardware")}</span>
    <div class="hw-row">
      <Icon name="cpu" size={11} />
      <span class="hw-key">{t("stats.cpu_name")}</span>
      <span class="hw-value" title={hardware(stats?.cpuName)}>{hardware(stats?.cpuName)}</span>
    </div>
    <div class="hw-row">
      <Icon name="monitor" size={11} />
      <span class="hw-key">{t("stats.gpu_name")}</span>
      <span class="hw-value" title={hardware(stats?.gpuName)}>{hardware(stats?.gpuName)}</span>
    </div>
    <div class="hw-row">
      <Icon name="memory" size={11} />
      <span class="hw-key">{t("stats.ram_modules")}</span>
      <span class="hw-value" title={hardware(stats?.ramModules)}>{hardware(stats?.ramModules)}</span>
    </div>
    <div class="hw-row">
      <Icon name="grid" size={11} />
      <span class="hw-key">{t("stats.motherboard")}</span>
      <span class="hw-value" title={hardware(stats?.motherboard)}>{hardware(stats?.motherboard)}</span>
    </div>
    <div class="hw-row">
      <Icon name="globe" size={11} />
      <span class="hw-key">{t("stats.os")}</span>
      <span class="hw-value" title={hardware(stats?.osName)}>{hardware(stats?.osName)}</span>
    </div>
  </div>

  <div class="extra">
    <span class="chip" title={t("widget.disk_desc")}>
      <Icon name="disk" size={11} />
      C: {stats ? `${Math.round(stats.disk)}%` : "—"}
    </span>
    <span class="chip" title={t("widget.network_desc")}>
      <Icon name="network" size={11} />
      ↓{bytes(stats?.netDown ?? 0)} ↑{bytes(stats?.netUp ?? 0)}
    </span>
    <span class="chip" title={t("widget.temp_desc")}>
      <Icon name="thermometer" size={11} />
      {stats?.cpuTemp != null ? `${Math.round(stats.cpuTemp)}°` : "—"}
      {#if stats?.gpuTemp != null}/ {Math.round(stats.gpuTemp)}°{/if}
    </span>
    <span class="chip" title={t("widget.fps_desc")}>
      <Icon name="activity" size={11} />
      {stats?.refreshHz ? `${stats.refreshHz} Hz` : "—"}
    </span>
  </div>
</div>

<style>
  .stats {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 7px;
    min-height: 0;
    overflow-y: auto;
    scrollbar-width: none;
  }

  .stats::-webkit-scrollbar {
    display: none;
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex: none;
    font-size: 9px;
    color: var(--muted);
  }

  .live {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #22c55e;
    box-shadow: 0 0 6px #22c55e;
  }

  .time {
    font-variant-numeric: tabular-nums;
  }

  .spark {
    flex: none;
    height: 18px;
    margin-top: -2px;
    opacity: 0.75;
  }

  .spark svg {
    width: 100%;
    height: 100%;
  }

  .spark polyline {
    fill: none;
    stroke: var(--accent);
    stroke-width: 1.6;
    vector-effect: non-scaling-stroke;
    stroke-linecap: round;
    stroke-linejoin: round;
    filter: drop-shadow(0 0 3px color-mix(in srgb, var(--accent) 60%, transparent));
    transition: all 0.45s ease;
  }

  .grid {
    flex: none;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 9px;
  }

  .stat {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .label {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
    font-size: 11px;
  }

  .label strong {
    margin-left: auto;
    color: var(--fg);
    font-size: 14px;
    font-variant-numeric: tabular-nums;
  }

  .bar {
    height: 6px;
    border-radius: 999px;
    background: var(--surface-strong);
    overflow: hidden;
  }

  .bar span {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: linear-gradient(to right, var(--accent), #93c5fd);
    transition: width 0.45s ease;
  }

  .hint {
    font-size: 9px;
    color: var(--muted);
  }

  .fps {
    flex: none;
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 5px 9px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--surface);
    transition: border-color 0.25s ease, background 0.25s ease;
  }

  .fps.live {
    border-color: color-mix(in srgb, #22c55e 35%, var(--border));
    background: color-mix(in srgb, #22c55e 7%, var(--surface));
  }

  .fps-number {
    min-width: 42px;
    font-size: 22px;
    font-weight: 750;
    line-height: 1;
    letter-spacing: -0.5px;
    font-variant-numeric: tabular-nums;
    color: var(--muted);
  }

  .fps.live .fps-number {
    color: #22c55e;
    text-shadow: 0 0 14px color-mix(in srgb, #22c55e 45%, transparent);
  }

  .fps-copy {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .fps-label {
    font-size: 9px;
    font-weight: 650;
    letter-spacing: 0.08em;
    color: var(--fg);
  }

  .fps-game {
    font-weight: 500;
    letter-spacing: 0;
    color: var(--muted);
  }

  .fps-hint {
    font-size: 8px;
    line-height: 1.3;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .fps-admin {
    align-self: flex-start;
    margin-top: 1px;
    border: 1px solid color-mix(in srgb, var(--accent) 35%, var(--border));
    border-radius: 6px;
    background: color-mix(in srgb, var(--accent) 14%, transparent);
    color: var(--fg);
    font-size: 7.5px;
    padding: 1px 6px;
    min-height: 17px;
  }

  .hardware {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 7px 8px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--surface);
    animation: hw-in 0.45s cubic-bezier(0.22, 1, 0.36, 1);
  }

  .hw-title {
    font-size: 7.5px;
    letter-spacing: 0.6px;
    text-transform: uppercase;
    color: var(--muted);
    margin-bottom: 1px;
  }

  .hw-row {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 9px;
    min-width: 0;
  }

  .hw-row :global(svg) {
    color: var(--accent);
    flex: none;
    opacity: 0.85;
  }

  .hw-key {
    color: var(--muted);
    flex: none;
    width: 42px;
  }

  .hw-value {
    color: var(--fg);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }

  @keyframes hw-in {
    from {
      opacity: 0;
      transform: translateY(5px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .extra {
    flex: none;
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    font-size: 8.5px;
    color: var(--muted);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 2px 6px;
    background: var(--surface);
    font-variant-numeric: tabular-nums;
  }

  :global(:root[data-reduced-motion="true"]) .bar span,
  :global(:root[data-reduced-motion="true"]) .spark polyline {
    transition: none;
  }

  :global(:root[data-reduced-motion="true"]) .hardware {
    animation: none;
  }
</style>
