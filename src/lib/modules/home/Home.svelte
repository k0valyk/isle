<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "../../components/Icon.svelte";
  import { store, t, updateSettings } from "../../store.svelte";
  import { ensureWeather } from "../../weather.svelte";
  import { initStats } from "../../stats.svelte";
  import { initBattery } from "../../battery.svelte";
  import { initAgents } from "../../agents.svelte";
  import WidgetCard from "../../widgets/WidgetCard.svelte";
  import {
    DEFAULT_WIDGETS,
    nextSize,
    WIDGET_META,
    widgetId,
    type WidgetConfig,
    type WidgetKind,
  } from "../../widgets/types";

  interface Props {
    go: (id: string) => void;
  }

  let { go }: Props = $props();

  let editing = $state(false);
  let dragFrom = $state<number | null>(null);
  let dropAt = $state<number | null>(null);

  const widgets = $derived(store.settings.widgets);

  const available = $derived(
    (Object.keys(WIDGET_META) as WidgetKind[]).map((kind) => ({
      kind,
      meta: WIDGET_META[kind],
    })),
  );

  function commit(next: WidgetConfig[]): void {
    updateSettings({ widgets: next });
  }

  function add(kind: WidgetKind): void {
    const meta = WIDGET_META[kind];
    commit([...widgets, { id: widgetId(), kind, size: meta.defaultSize }]);
  }

  function remove(id: string): void {
    commit(widgets.filter((widget) => widget.id !== id));
  }

  function resize(id: string): void {
    commit(
      widgets.map((widget) =>
        widget.id === id ? { ...widget, size: nextSize(widget.size) } : widget,
      ),
    );
  }

  function reset(): void {
    commit(DEFAULT_WIDGETS.map((widget) => ({ ...widget })));
  }

  function onDragStart(index: number, event: DragEvent): void {
    if (!editing) return;
    dragFrom = index;
    dropAt = index;
    event.dataTransfer?.setData("text/plain", String(index));
    if (event.dataTransfer) event.dataTransfer.effectAllowed = "move";
  }

  function onDragOver(index: number, event: DragEvent): void {
    if (!editing) return;
    event.preventDefault();
    dropAt = index;
    if (event.dataTransfer) event.dataTransfer.dropEffect = "move";
  }

  function onDrop(index: number, event: DragEvent): void {
    if (!editing) return;
    event.preventDefault();
    const raw = dragFrom ?? Number(event.dataTransfer?.getData("text/plain") ?? index);
    if (!Number.isFinite(raw) || raw === index) {
      dragFrom = null;
      dropAt = null;
      return;
    }
    const next = widgets.slice();
    const [moved] = next.splice(raw, 1);
    if (!moved) {
      dragFrom = null;
      dropAt = null;
      return;
    }
    next.splice(index, 0, moved);
    dragFrom = null;
    dropAt = null;
    commit(next);
  }

  onMount(() => {
    void ensureWeather();
    void initStats();
    void initBattery();
    void initAgents();
  });
</script>

<div class="home" data-no-drag>
  <div class="home-head">
    <span class="panel-title">
      <Icon name="grid" size={12} />
      {t("home.widgets")}
    </span>
    <span class="count">{widgets.length}</span>
    <span class="spacer"></span>
    {#if editing}
      <button class="head-btn" onclick={reset} title={t("widget.reset")} aria-label={t("widget.reset")}>
        <Icon name="refresh" size={12} />
      </button>
    {/if}
    <button
      class="head-btn"
      class:active={editing}
      onclick={() => (editing = !editing)}
      title={editing ? t("widget.done") : t("widget.edit")}
      aria-label={editing ? t("widget.done") : t("widget.edit")}
    >
      <Icon name={editing ? "check" : "pencil"} size={12} />
    </button>
  </div>

  <div class="grid" class:editing role="list">
    {#each widgets as widget, index (widget.id)}
      <div
        class="slot size-{widget.size}"
        class:dragging={dragFrom === index}
        class:drop-target={dropAt === index && dragFrom !== index}
        draggable={editing}
        role="listitem"
        aria-label={t(WIDGET_META[widget.kind].label)}
        ondragstart={(event) => onDragStart(index, event)}
        ondragover={(event) => onDragOver(index, event)}
        ondrop={(event) => onDrop(index, event)}
        ondragend={() => {
          dragFrom = null;
          dropAt = null;
        }}
      >
        <WidgetCard
          {widget}
          {editing}
          {go}
          onremove={remove}
          onresize={resize}
        />
      </div>
    {/each}

    {#if !widgets.length}
      <div class="empty">
        <Icon name="grid" size={22} />
        <span>{t("widget.empty")}</span>
      </div>
    {/if}
  </div>

  {#if editing}
    <div class="palette" aria-label={t("widget.palette")}>
      {#each available as item (item.kind)}
        <button class="palette-item" onclick={() => add(item.kind)} title={t(item.meta.description)}>
          <Icon name={item.meta.icon} size={12} />
          <span>{t(item.meta.label)}</span>
          <Icon name="plus" size={10} />
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .home {
    height: 100%;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .home-head {
    flex: none;
    display: flex;
    align-items: center;
    gap: 5px;
    height: 20px;
    padding: 0 2px;
  }

  .panel-title {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    color: var(--muted);
  }

  .panel-title :global(svg) {
    color: var(--accent);
  }

  .count {
    min-width: 15px;
    height: 14px;
    display: grid;
    place-items: center;
    border-radius: 999px;
    background: var(--surface-strong);
    color: var(--muted);
    font-size: 8px;
    font-variant-numeric: tabular-nums;
  }

  .spacer {
    flex: 1;
  }

  .head-btn {
    width: 22px;
    height: 20px;
    display: grid;
    place-items: center;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    color: var(--muted);
    padding: 0;
  }

  .head-btn:hover,
  .head-btn.active {
    color: var(--fg);
    border-color: color-mix(in srgb, var(--accent) 45%, var(--border));
    background: color-mix(in srgb, var(--accent) 12%, var(--surface));
  }

  .grid {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    grid-auto-rows: 56px;
    gap: 5px;
    align-content: start;
    scrollbar-width: thin;
    padding-right: 2px;
  }

  .grid::-webkit-scrollbar {
    width: 4px;
  }

  .slot {
    min-width: 0;
    min-height: 0;
    animation: widget-in 0.28s cubic-bezier(0.34, 1.25, 0.64, 1);
    transition: transform 0.16s ease, opacity 0.16s ease;
  }

  .slot.dragging {
    opacity: 0.4;
    transform: scale(0.96);
  }

  .slot.drop-target {
    outline: 1px dashed color-mix(in srgb, var(--accent) 65%, transparent);
    outline-offset: 1px;
    border-radius: 12px;
  }

  .size-small {
    grid-column: span 1;
    grid-row: span 1;
  }

  .size-wide {
    grid-column: span 2;
    grid-row: span 1;
  }

  .size-tall {
    grid-column: span 1;
    grid-row: span 2;
  }

  .size-large {
    grid-column: span 2;
    grid-row: span 2;
  }

  .size-full {
    grid-column: span 4;
    grid-row: span 1;
  }

  .size-hero {
    grid-column: span 4;
    grid-row: span 2;
  }

  .empty {
    grid-column: 1 / -1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 5px;
    color: var(--muted);
    font-size: 10px;
    min-height: 80px;
  }

  .palette {
    flex: none;
    display: flex;
    gap: 4px;
    overflow-x: auto;
    padding: 1px 1px 2px;
    scrollbar-width: none;
    mask-image: linear-gradient(to right, #000 94%, transparent);
  }

  .palette::-webkit-scrollbar {
    display: none;
  }

  .palette-item {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 3px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    color: var(--muted);
    font-size: 8px;
    padding: 3px 5px;
    transition: color 0.14s ease, background 0.14s ease, border-color 0.14s ease;
  }

  .palette-item:hover {
    color: var(--fg);
    background: color-mix(in srgb, var(--accent) 12%, var(--surface));
    border-color: color-mix(in srgb, var(--accent) 40%, var(--border));
  }

  @keyframes widget-in {
    from {
      opacity: 0;
      transform: translateY(4px) scale(0.97);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  :global(:root[data-reduced-motion="true"]) .slot {
    animation: none;
  }
</style>
