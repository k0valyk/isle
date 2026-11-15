<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import Icon from "../components/Icon.svelte";
  import Clock from "../components/Clock.svelte";
  import Character from "../components/Character.svelte";
  import MiniSpectrum from "../components/MiniSpectrum.svelte";
  import AnimatedNumber from "../components/AnimatedNumber.svelte";
  import { locale, store, t } from "../store.svelte";
  import { clock, initClock } from "../clock.svelte";
  import { ensureWeather, temperature, weather, weatherIcon, weatherKey } from "../weather.svelte";
  import {
    activeLyricIndex,
    audio,
    isPlaying,
    media,
    mediaControl,
  } from "../media.svelte";
  import { history, initStats, system } from "../stats.svelte";
  import { initBattery, battery } from "../battery.svelte";
  import { addFiveMinutes, resetTimer, timer, timerLabel, toggleTimer } from "../timer.svelte";
  import { agentPhase, agents, initAgents } from "../agents.svelte";
  import { WIDGET_META, type WidgetConfig } from "./types";

  interface Props {
    widget: WidgetConfig;
    editing: boolean;
    go: (id: string) => void;
    onremove: (id: string) => void;
    onresize: (id: string) => void;
  }

  let { widget, editing, go, onremove, onresize }: Props = $props();

  const meta = $derived(WIDGET_META[widget.kind] ?? WIDGET_META.clock);
  const stats = $derived(system.data);
  const current = $derived(weather.data?.current ?? null);
  const city = $derived(store.settings.weather_city || t("weather.city"));
  const lyricIndex = $derived(activeLyricIndex(media.position));
  const lyricLine = $derived(lyricIndex >= 0 ? media.lyrics[lyricIndex]?.text ?? "" : media.lyrics[0]?.text ?? "");
  const lyricNext = $derived(
    lyricIndex >= 0 ? media.lyrics[lyricIndex + 1]?.text ?? "" : media.lyrics[1]?.text ?? "",
  );

  let upcoming = $state<{ title: string; start: number; allDay: boolean } | null>(null);

  type CalendarRecord = {
    id: number;
    title: string;
    start: number;
    end: number;
    all_day: boolean;
    notes: string;
  };

  async function loadUpcoming(): Promise<void> {
    try {
      const from = Date.now() - 60_000;
      const to = Date.now() + 7 * 24 * 60 * 60 * 1000;
      const events = await invoke<CalendarRecord[]>("events_list", { from, to });
      const next = events
        .filter((event) => event.end >= Date.now())
        .sort((a, b) => a.start - b.start)[0];
      upcoming = next
        ? { title: next.title, start: next.start, allDay: next.all_day }
        : null;
    } catch {
      upcoming = null;
    }
  }

  onMount(() => {
    initClock();
    initStats();
    initBattery();
    initAgents();
    void ensureWeather();
    if (widget.kind === "calendar") {
      void loadUpcoming();
      const handle = setInterval(loadUpcoming, 60_000);
      return () => clearInterval(handle);
    }
    return;
  });

  function percent(value: number | null | undefined): number {
    return Math.max(0, Math.min(100, Math.round(value ?? 0)));
  }

  function metricValue(kind: string): number | null {
    if (!stats) return null;
    if (kind === "cpu") return stats.cpu;
    if (kind === "gpu") return stats.gpu;
    if (kind === "ram") return stats.ram;
    return null;
  }

  function metricHistory(kind: string): number[] {
    if (kind === "cpu") return history.cpu;
    if (kind === "gpu") return history.gpu;
    if (kind === "ram") return history.ram;
    return [];
  }

  function metricLabel(kind: string): string {
    if (kind === "cpu") return "CPU";
    if (kind === "gpu") return "GPU";
    return "RAM";
  }

  function sparkline(values: number[]): string {
    if (values.length < 2) return "0,26 100,26";
    const max = Math.max(10, ...values);
    return values
      .map((value, index) => {
        const x = (index / (values.length - 1)) * 100;
        const y = 26 - (Math.min(max, value) / max) * 22;
        return `${x.toFixed(1)},${y.toFixed(1)}`;
      })
      .join(" ");
  }

  function bytesPerSecond(value: number): string {
    if (!Number.isFinite(value) || value <= 0) return "0";
    if (value >= 1024 * 1024) return `${(value / 1024 / 1024).toFixed(1)} MB/s`;
    if (value >= 1024) return `${(value / 1024).toFixed(0)} KB/s`;
    return `${value.toFixed(0)} B/s`;
  }

  function eventTime(ts: number): string {
    return new Date(ts).toLocaleString(locale(), {
      day: "numeric",
      month: "short",
      hour: "2-digit",
      minute: "2-digit",
      hour12: false,
    });
  }

  const dateLabel = $derived(
    clock.now.toLocaleDateString(locale(), {
      weekday: "long",
      day: "numeric",
      month: "long",
    }),
  );
  const weekday = $derived(
    clock.now.toLocaleDateString(locale(), { weekday: "long" }).toLocaleUpperCase(),
  );
  const dayNumber = $derived(clock.now.getDate());
  const monthLabel = $derived(
    clock.now.toLocaleDateString(locale(), { month: "long" }).toLocaleUpperCase(),
  );

  const progress = $derived(
    media.current.duration > 0
      ? Math.min(100, (media.position / media.current.duration) * 100)
      : 0,
  );

  const latestAgent = $derived(agents.events[0]);
  const phase = $derived(agentPhase(latestAgent));
  const agentStatus = $derived(agents.status);

  let artBroken = $state(false);
  $effect(() => {
    void media.current.artwork;
    artBroken = false;
  });

  const actions = [
    { id: "clipboard", icon: "clipboard", key: "home.action_clipboard" },
    { id: "text", icon: "type", key: "home.action_text" },
    { id: "timer", icon: "timer", key: "home.action_timer" },
    { id: "weather", icon: "cloud", key: "home.action_weather" },
    { id: "stats", icon: "gauge", key: "home.action_stats" },
    { id: "chat", icon: "message", key: "home.action_chat" },
  ];

  const bars = $derived(
    audio.bands.length
      ? audio.bands.map((value) => Math.max(6, Math.round(value * 100)))
      : Array.from({ length: 12 }, () => 8),
  );
</script>

<div class="card" class:editing>
  {#if editing}
    <div class="editbar" data-no-drag>
      <span class="drag-handle" aria-hidden="true"><Icon name="grip" size={12} /></span>
      <span class="edit-label">{t(meta.label)}</span>
      <button
        class="edit-btn"
        onclick={(event) => {
          event.stopPropagation();
          onresize(widget.id);
        }}
        title={t("widget.resize")}
        aria-label={t("widget.resize")}
      >
        <Icon name="layout" size={12} />
      </button>
      <button
        class="edit-btn danger"
        onclick={(event) => {
          event.stopPropagation();
          onremove(widget.id);
        }}
        title={t("widget.remove")}
        aria-label={t("widget.remove")}
      >
        <Icon name="x" size={12} />
      </button>
    </div>
  {/if}

  <div class="content" class:with-edit={editing}>
    {#if widget.kind === "clock"}
      <div class="clock-widget">
        <Clock />
        <span class="sub">{dateLabel}</span>
      </div>
    {:else if widget.kind === "date"}
      <div class="date-widget">
        <span class="weekday">{weekday}</span>
        <span class="day">{dayNumber}</span>
        <span class="month">{monthLabel}</span>
      </div>
    {:else if widget.kind === "weather"}
      <div class="weather-widget">
        {#if current}
          <span class="weather-icon"><Icon name={weatherIcon(current.code, current.is_day)} size={26} /></span>
          <span class="big">{temperature(current.temperature)}</span>
          <span class="weather-copy">
            <span class="strong">{t(weatherKey(current.code))}</span>
            <span class="sub">{city}</span>
          </span>
        {:else}
          <span class="weather-icon"><Icon name="cloud" size={26} /></span>
          <span class="weather-copy">
            <span class="strong">{t("weather.no_data")}</span>
            <span class="sub">{weather.loading ? t("common.loading") : city}</span>
          </span>
        {/if}
      </div>
    {:else if widget.kind === "media"}
      <button class="media-widget" onclick={() => go("player")} aria-label={t("nav.player")}>
        <span class="cover">
          {#if media.current.artwork && !artBroken}
            <img src={media.current.artwork} alt="" onerror={() => (artBroken = true)} />
          {:else}
            <Icon name="music" size={22} />
          {/if}
          {#if isPlaying()}
            <span class="cover-live"><span></span><span></span><span></span></span>
          {/if}
        </span>
        <span class="media-meta">
          <span class="strong ellipsis">
            {media.current.active ? media.current.title || t("player.untitled") : t("home.music_none")}
          </span>
          <span class="sub ellipsis">
            {media.current.active
              ? media.current.artist || t("player.unknown_artist")
              : t("home.music_hint")}
          </span>
          {#if media.current.active}
            <span class="progress"><span style:width={`${progress}%`}></span></span>
          {/if}
        </span>
        {#if media.current.active}
          <span class="media-controls" data-no-drag>
            <span
              class="mini-control"
              role="button"
              tabindex="0"
              aria-label={t("player.prev")}
              onclick={(event) => {
                event.stopPropagation();
                void mediaControl("previous");
              }}
              onkeydown={(event) => {
                if (event.key === "Enter") void mediaControl("previous");
              }}
            >
              <Icon name="skip-back" size={14} />
            </span>
            <span
              class="mini-control play"
              role="button"
              tabindex="0"
              aria-label={isPlaying() ? t("player.pause") : t("player.play")}
              onclick={(event) => {
                event.stopPropagation();
                void mediaControl("play_pause");
              }}
              onkeydown={(event) => {
                if (event.key === "Enter") void mediaControl("play_pause");
              }}
            >
              <Icon name={isPlaying() ? "pause" : "play"} size={14} />
            </span>
            <span
              class="mini-control"
              role="button"
              tabindex="0"
              aria-label={t("player.next")}
              onclick={(event) => {
                event.stopPropagation();
                void mediaControl("next");
              }}
              onkeydown={(event) => {
                if (event.key === "Enter") void mediaControl("next");
              }}
            >
              <Icon name="skip-forward" size={14} />
            </span>
          </span>
        {/if}
      </button>
    {:else if widget.kind === "lyrics"}
      <button class="lyrics-widget" onclick={() => go("player")} aria-label={t("nav.player")}>
        {#if media.lyricsState === "ready" && media.lyrics.length}
          {#key lyricLine}
            <span class="lyric-active">{lyricLine || "♪"}</span>
          {/key}
          <span class="lyric-next">{lyricNext || "♪"}</span>
        {:else if media.lyricsPlain}
          <span class="lyric-plain">{media.lyricsPlain.split(/\r?\n/).slice(0, 3).join(" · ")}</span>
        {:else}
          <span class="sub">{t("player.lyrics_hint")}</span>
        {/if}
      </button>
    {:else if widget.kind === "system"}
      <div class="system-widget">
        {#each ["cpu", "gpu", "ram"] as key (key)}
          <div class="metric-row">
            <span class="metric-name">{metricLabel(key)}</span>
            <span class="metric-bar">
              <span style:width={`${percent(metricValue(key))}%`}></span>
            </span>
            <span class="metric-value">
              <AnimatedNumber value={stats ? percent(metricValue(key)) : null} suffix="%" />
            </span>
          </div>
        {/each}
        <div class="system-foot">
          <span><Icon name="network" size={10} /> {bytesPerSecond(stats?.netDown ?? 0)}</span>
          <span><Icon name="disk" size={10} /> {stats ? `${Math.round(stats.disk)}%` : "—"}</span>
        </div>
      </div>
    {:else if widget.kind === "cpu" || widget.kind === "gpu" || widget.kind === "ram"}
      <div class="metric-widget">
        <div class="metric-head">
          <span class="metric-name">{metricLabel(widget.kind)}</span>
          <span class="big metric-big">
            <AnimatedNumber value={stats ? percent(metricValue(widget.kind)) : null} />
            <span class="unit">%</span>
          </span>
        </div>
        <svg class="spark" viewBox="0 0 100 28" preserveAspectRatio="none" aria-hidden="true">
          <polyline points={sparkline(metricHistory(widget.kind))} />
        </svg>
        <span class="sub">
          {widget.kind === "ram" && stats
            ? `${stats.ramUsedGb.toFixed(1)} / ${stats.ramTotalGb.toFixed(0)} GB`
            : widget.kind === "gpu" && stats?.gpu === null
              ? t("widget.no_gpu")
              : t("widget.live")}
        </span>
      </div>
    {:else if widget.kind === "temp"}
      <div class="temp-widget">
        <div class="temp-row">
          <span class="metric-name">CPU</span>
          <span class="temp-value">{stats?.cpuTemp != null ? `${Math.round(stats.cpuTemp)}°` : "—"}</span>
        </div>
        <div class="temp-row">
          <span class="metric-name">GPU</span>
          <span class="temp-value">{stats?.gpuTemp != null ? `${Math.round(stats.gpuTemp)}°` : "—"}</span>
        </div>
        {#if stats?.cpuTemp == null && stats?.gpuTemp == null}
          <span class="sub temp-hint">{t("widget.temp_unavailable")}</span>
        {/if}
      </div>
    {:else if widget.kind === "fps"}
      <div class="metric-widget">
        <div class="metric-head">
          <span class="metric-name">FPS</span>
          <span class="big metric-big">—</span>
        </div>
        <span class="sub">
          {stats?.refreshHz ? `${t("widget.screen_hz", { hz: stats.refreshHz })}` : "—"}
        </span>
        <span class="sub temp-hint">{t("widget.fps_hint")}</span>
      </div>
    {:else if widget.kind === "disk"}
      <div class="metric-widget">
        <div class="metric-head">
          <span class="metric-name">C:</span>
          <span class="big metric-big">{stats ? Math.round(stats.disk) : "—"}<span class="unit">%</span></span>
        </div>
        <span class="metric-bar wide">
          <span style:width={`${percent(stats?.disk)}%`}></span>
        </span>
        <span class="sub">
          {stats ? `${stats.diskUsedGb.toFixed(0)} / ${stats.diskTotalGb.toFixed(0)} GB` : "—"}
        </span>
      </div>
    {:else if widget.kind === "network"}
      <div class="network-widget">
        <div class="net-row">
          <span class="net-arrow down">↓</span>
          <span class="strong">{bytesPerSecond(stats?.netDown ?? 0)}</span>
          <span class="sub">{t("widget.download")}</span>
        </div>
        <div class="net-row">
          <span class="net-arrow up">↑</span>
          <span class="strong">{bytesPerSecond(stats?.netUp ?? 0)}</span>
          <span class="sub">{t("widget.upload")}</span>
        </div>
      </div>
    {:else if widget.kind === "battery"}
      <div class="metric-widget">
        <div class="metric-head">
          <span class="metric-name"><Icon name="battery" size={14} /> {t("widget.battery")}</span>
          <span class="big metric-big">
            {battery.data?.percent != null ? battery.data.percent : "—"}<span class="unit">%</span>
          </span>
        </div>
        <span class="metric-bar wide">
          <span style:width={`${percent(battery.data?.percent ?? 0)}%`}></span>
        </span>
        <span class="sub">
          {battery.data?.charging
            ? t("widget.charging")
            : battery.data
              ? t("widget.on_battery")
              : t("widget.no_battery")}
        </span>
      </div>
    {:else if widget.kind === "timer"}
      <div class="timer-widget">
        <span class="timer-value">{timerLabel()}</span>
        <div class="timer-actions">
          <button class="mini-control play" onclick={toggleTimer} aria-label={t("timer.start")}>
            <Icon name={timer.running ? "pause" : "play"} size={13} />
          </button>
          <button class="mini-control" onclick={resetTimer} aria-label={t("common.reset")}>
            <Icon name="refresh" size={13} />
          </button>
          <button class="mini-control" onclick={addFiveMinutes} aria-label={t("timer.plus5")}>
            +5
          </button>
        </div>
      </div>
    {:else if widget.kind === "quick_actions"}
      <div class="actions-widget">
        {#each actions as action (action.id)}
          <button class="quick" onclick={() => go(action.id)}>
            <Icon name={action.icon} size={16} />
            <span>{t(action.key)}</span>
          </button>
        {/each}
      </div>
    {:else if widget.kind === "character"}
      <div class="character-widget">
        <Character />
        <MiniSpectrum variant={store.settings.visualizer_style} />
      </div>
    {:else if widget.kind === "spectrum"}
      <div class="spectrum-widget" class:live={isPlaying()}>
        {#each bars as height, index (index)}
          <span style:height={`${height}%`}></span>
        {/each}
      </div>
    {:else if widget.kind === "agents"}
      <button class="agents-widget" onclick={() => go("agents")} aria-label={t("nav.agents")}>
        <span class="agent-dot" class:working={phase === "working"} class:done={phase === "done"} class:error={phase === "error"} class:attention={phase === "attention"}></span>
        <span class="agent-copy">
          <span class="strong ellipsis">
            {latestAgent ? latestAgent.title : t("widget.agents_idle")}
          </span>
          <span class="sub ellipsis">
            {latestAgent
              ? `${latestAgent.source} · ${latestAgent.event}`
              : agentStatus?.listening
                ? t("widget.agents_listening")
                : t("widget.agents_offline")}
          </span>
        </span>
        <span class="agent-count">{agentStatus?.requests ?? 0}</span>
      </button>
    {:else if widget.kind === "calendar"}
      <button class="calendar-widget" onclick={() => go("calendar")} aria-label={t("nav.calendar")}>
        <span class="calendar-icon"><Icon name="calendar" size={18} /></span>
        <span class="calendar-copy">
          <span class="strong ellipsis">{upcoming ? upcoming.title : t("widget.no_events")}</span>
          <span class="sub">{upcoming ? eventTime(upcoming.start) : t("widget.calendar_hint")}</span>
        </span>
      </button>
    {/if}
  </div>
</div>

<style>
  .card {
    height: 100%;
    min-height: 0;
    border-radius: 12px;
    border: 1px solid var(--border);
    background: linear-gradient(150deg, var(--surface), color-mix(in srgb, var(--surface) 40%, transparent));
    overflow: hidden;
    position: relative;
    transition: border-color 0.18s ease, transform 0.18s ease, background 0.18s ease;
  }

  .card:hover {
    border-color: color-mix(in srgb, var(--accent) 30%, var(--border));
  }

  .card.editing {
    border-style: dashed;
    border-color: color-mix(in srgb, var(--accent) 55%, var(--border));
    cursor: grab;
  }

  .card.editing:active {
    cursor: grabbing;
  }

  .content {
    height: 100%;
    min-height: 0;
    padding: 8px 10px;
    display: flex;
    flex-direction: column;
  }

  .content.with-edit {
    padding-top: 26px;
    pointer-events: none;
  }

  .editbar {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 25px;
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 0 5px;
    background: color-mix(in srgb, var(--bg) 70%, transparent);
    border-bottom: 1px solid var(--border);
    z-index: 3;
  }

  .drag-handle {
    color: var(--muted);
    display: grid;
    place-items: center;
    cursor: grab;
  }

  .edit-label {
    flex: 1;
    font-size: 8.5px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .edit-btn {
    width: 20px;
    height: 18px;
    display: grid;
    place-items: center;
    border: none;
    border-radius: 5px;
    background: transparent;
    color: var(--muted);
    padding: 0;
  }

  .edit-btn:hover {
    background: var(--surface-strong);
    color: var(--fg);
  }

  .edit-btn.danger:hover {
    color: var(--danger);
  }

  .big {
    font-size: 24px;
    font-weight: 650;
    line-height: 1;
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.5px;
  }

  .unit {
    font-size: 11px;
    color: var(--muted);
    margin-left: 2px;
  }

  .sub {
    font-size: 8.5px;
    line-height: 1.25;
    color: var(--muted);
  }

  .strong {
    font-size: 11px;
    font-weight: 600;
    color: var(--fg);
  }

  .ellipsis {
    max-width: 100%;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .clock-widget {
    height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 2px;
  }

  .clock-widget :global(.clock) {
    font-size: 26px;
    font-weight: 650;
  }

  .date-widget {
    height: 100%;
    display: grid;
    grid-template-columns: auto 1fr;
    grid-template-rows: auto auto;
    align-items: center;
    column-gap: 8px;
    padding: 2px 2px;
  }

  .weekday {
    grid-column: 1;
    grid-row: 1;
    font-size: 8.5px;
    letter-spacing: 0.08em;
    color: var(--accent);
    font-weight: 650;
  }

  .day {
    grid-column: 1;
    grid-row: 2;
    font-size: 30px;
    line-height: 0.95;
    font-weight: 650;
    font-variant-numeric: tabular-nums;
  }

  .month {
    grid-column: 2;
    grid-row: 2;
    font-size: 9px;
    letter-spacing: 0.08em;
    color: var(--muted);
  }

  .weather-widget {
    height: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .weather-icon {
    color: var(--accent);
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border-radius: 10px;
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    flex: none;
  }

  .weather-copy {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }

  .media-widget {
    height: 100%;
    width: 100%;
    display: flex;
    align-items: center;
    gap: 9px;
    border: none;
    background: transparent;
    color: var(--fg);
    padding: 0;
    text-align: left;
    min-width: 0;
  }

  .cover {
    width: 46px;
    height: 46px;
    border-radius: 10px;
    background: linear-gradient(140deg, rgba(255, 255, 255, 0.16), rgba(255, 255, 255, 0.03));
    display: grid;
    place-items: center;
    overflow: hidden;
    flex: none;
    position: relative;
  }

  .cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .cover-live {
    position: absolute;
    left: 4px;
    right: 4px;
    bottom: 3px;
    height: 12px;
    display: flex;
    align-items: flex-end;
    gap: 1.5px;
    padding: 2px 3px;
    border-radius: 4px;
    background: rgba(0, 0, 0, 0.45);
  }

  .cover-live span {
    flex: 1;
    min-height: 2px;
    border-radius: 1px;
    background: var(--accent);
    animation: cover-bounce 0.9s ease-in-out infinite;
  }

  .cover-live span:nth-child(2) {
    animation-delay: 0.18s;
  }

  .cover-live span:nth-child(3) {
    animation-delay: 0.36s;
  }

  @keyframes cover-bounce {
    0%, 100% { height: 2px; }
    50% { height: 8px; }
  }

  .media-meta {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .progress {
    height: 2.5px;
    border-radius: 999px;
    background: var(--surface-strong);
    overflow: hidden;
    margin-top: 3px;
  }

  .progress span {
    display: block;
    height: 100%;
    background: var(--accent);
    transition: width 0.4s linear;
  }

  .media-controls {
    display: flex;
    gap: 1px;
    flex: none;
  }

  .mini-control {
    min-width: 22px;
    height: 22px;
    display: grid;
    place-items: center;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--muted);
    padding: 0 4px;
    font-size: 9px;
    cursor: pointer;
  }

  .mini-control:hover {
    background: var(--surface-strong);
    color: var(--fg);
  }

  .mini-control.play {
    background: var(--accent);
    color: #fff;
  }

  .lyrics-widget {
    height: 100%;
    width: 100%;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 3px;
    border: none;
    background: transparent;
    color: var(--fg);
    padding: 0;
    text-align: left;
    overflow: hidden;
  }

  .lyric-active {
    font-size: 12.5px;
    font-weight: 650;
    line-height: 1.25;
    color: var(--fg);
    animation: lyric-pop 0.3s ease;
    text-shadow: 0 0 16px color-mix(in srgb, var(--accent) 55%, transparent);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .lyric-next {
    font-size: 9.5px;
    line-height: 1.25;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .lyric-plain {
    font-size: 9.5px;
    line-height: 1.35;
    color: var(--muted);
  }

  @keyframes lyric-pop {
    from { opacity: 0; transform: translateY(3px); }
    to { opacity: 1; transform: translateY(0); }
  }

  .system-widget {
    height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 5px;
  }

  .metric-row {
    display: grid;
    grid-template-columns: 28px 1fr 32px;
    align-items: center;
    gap: 6px;
  }

  .metric-name {
    font-size: 8.5px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--muted);
    white-space: nowrap;
    display: inline-flex;
    align-items: center;
    gap: 3px;
  }

  .metric-bar {
    height: 4px;
    border-radius: 999px;
    background: var(--surface-strong);
    overflow: hidden;
  }

  .metric-bar.wide {
    width: 100%;
  }

  .metric-bar span {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: linear-gradient(90deg, var(--accent), color-mix(in srgb, var(--accent) 55%, #22c55e));
    transition: width 0.45s cubic-bezier(0.34, 1.2, 0.64, 1);
  }

  .metric-value {
    font-size: 9px;
    text-align: right;
    color: var(--fg);
    font-variant-numeric: tabular-nums;
  }

  .system-foot {
    display: flex;
    justify-content: space-between;
    font-size: 8px;
    color: var(--muted);
  }

  .system-foot span {
    display: inline-flex;
    align-items: center;
    gap: 3px;
  }

  .metric-widget {
    height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 3px;
  }

  .metric-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 6px;
  }

  .metric-big {
    font-size: 22px;
  }

  .spark {
    width: 100%;
    height: 20px;
    overflow: visible;
  }

  .spark polyline {
    fill: none;
    stroke: var(--accent);
    stroke-width: 1.6;
    vector-effect: non-scaling-stroke;
    stroke-linecap: round;
    stroke-linejoin: round;
    opacity: 0.85;
  }

  .temp-widget {
    height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 3px;
  }

  .temp-row {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 8px;
  }

  .temp-value {
    font-size: 17px;
    font-weight: 650;
    font-variant-numeric: tabular-nums;
    color: var(--fg);
  }

  .temp-hint {
    font-size: 7.5px;
    line-height: 1.2;
    color: var(--muted);
  }

  .network-widget {
    height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 5px;
  }

  .net-row {
    display: flex;
    align-items: baseline;
    gap: 6px;
  }

  .net-arrow {
    font-size: 12px;
    font-weight: 700;
  }

  .net-arrow.down {
    color: #22c55e;
  }

  .net-arrow.up {
    color: var(--accent);
  }

  .timer-widget {
    height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    gap: 3px;
  }

  .timer-value {
    font-size: 20px;
    font-weight: 650;
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.4px;
  }

  .timer-actions {
    display: flex;
    gap: 2px;
  }

  .actions-widget {
    height: 100%;
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    grid-template-rows: repeat(2, minmax(0, 1fr));
    gap: 4px;
  }

  .quick {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: transparent;
    color: var(--fg);
    font-size: 8px;
    padding: 2px;
    transition: background 0.15s ease, transform 0.12s ease, border-color 0.15s ease;
  }

  .quick:hover {
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    border-color: color-mix(in srgb, var(--accent) 38%, var(--border));
    transform: translateY(-1px);
  }

  .quick:active {
    transform: scale(0.96);
  }

  .character-widget {
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
  }

  .spectrum-widget {
    height: 100%;
    display: flex;
    align-items: flex-end;
    gap: 2px;
    padding: 4px 0;
  }

  .spectrum-widget span {
    flex: 1;
    min-height: 3px;
    border-radius: 3px;
    background: linear-gradient(to top, var(--accent), color-mix(in srgb, var(--accent) 30%, #93c5fd));
    transition: height 90ms linear;
    opacity: 0.75;
  }

  .spectrum-widget.live span {
    opacity: 0.95;
  }

  .agents-widget {
    height: 100%;
    width: 100%;
    display: flex;
    align-items: center;
    gap: 7px;
    border: none;
    background: transparent;
    color: var(--fg);
    text-align: left;
    padding: 0;
    min-width: 0;
  }

  .agent-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--muted);
    flex: none;
  }

  .agent-dot.working {
    background: #f5b301;
    box-shadow: 0 0 8px #f5b301;
    animation: agent-pulse 1.2s ease-in-out infinite;
  }

  .agent-dot.done {
    background: #22c55e;
    box-shadow: 0 0 8px #22c55e;
  }

  .agent-dot.error {
    background: var(--danger);
    box-shadow: 0 0 8px var(--danger);
  }

  .agent-dot.attention {
    background: var(--accent);
    box-shadow: 0 0 8px var(--accent);
  }

  @keyframes agent-pulse {
    0%, 100% { opacity: 0.45; }
    50% { opacity: 1; }
  }

  .agent-copy {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .agent-count {
    font-size: 9px;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }

  .calendar-widget {
    height: 100%;
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    border: none;
    background: transparent;
    color: var(--fg);
    padding: 0;
    text-align: left;
    min-width: 0;
  }

  .calendar-icon {
    width: 32px;
    height: 32px;
    border-radius: 9px;
    display: grid;
    place-items: center;
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    color: var(--accent);
    flex: none;
  }

  .calendar-copy {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  :global(:root[data-reduced-motion="true"]) .cover-live span,
  :global(:root[data-reduced-motion="true"]) .agent-dot.working {
    animation: none;
  }

  :global(:root[data-reduced-motion="true"]) .lyric-active {
    animation: none;
  }
</style>
