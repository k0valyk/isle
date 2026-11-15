<script lang="ts">
  import Icon from "../../components/Icon.svelte";
  import { locale, t } from "../../store.svelte";
  import { system } from "../../stats.svelte";

  const stats = $derived(system.data);

  function percent(value: number | null | undefined): number {
    return Math.max(0, Math.min(100, Math.round(value ?? 0)));
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
    gap: 8px;
    min-height: 0;
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

  .grid {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 12px;
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

  :global(:root[data-reduced-motion="true"]) .bar span {
    transition: none;
  }
</style>
