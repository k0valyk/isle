<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import Icon from "../../components/Icon.svelte";
  import { t, locale, store, updateSettings } from "../../store.svelte";
  import {
    captureClipboard,
    clearClipboard,
    copyClipboardItem,
    deleteClipboard,
    listClipboard,
    pinClipboard,
    previewClipboardItem,
    searchClipboard,
    writeClipboard,
    type ClipboardItem,
  } from "../../ipc";

  let items = $state<ClipboardItem[]>([]);
  let query = $state("");
  let loading = $state(false);
  let error = $state<string | null>(null);
  let busyId = $state<number | null>(null);
  let previews = $state<Record<number, string>>({});
  let searchTimer: ReturnType<typeof setTimeout> | undefined;
  let refreshTimer: ReturnType<typeof setTimeout> | undefined;

  async function refresh(): Promise<void> {
    loading = true;
    error = null;
    try {
      items = query.trim() ? await searchClipboard(query.trim()) : await listClipboard(300);
      const next: Record<number, string> = {};
      for (const item of items) {
        if (item.kind !== "image") continue;
        if (previews[item.id]) {
          next[item.id] = previews[item.id];
          continue;
        }
        try {
          const data = await previewClipboardItem(item.id);
          if (data) next[item.id] = data;
        } catch {
          /* the image file may have been removed */
        }
      }
      previews = next;
    } catch (err) {
      error = String(err);
    } finally {
      loading = false;
    }
  }

  function scheduleRefresh(): void {
    if (refreshTimer) clearTimeout(refreshTimer);
    refreshTimer = setTimeout(() => void refresh(), 250);
  }

  function onQuery(): void {
    if (searchTimer) clearTimeout(searchTimer);
    searchTimer = setTimeout(() => void refresh(), 180);
  }

  async function capture(): Promise<void> {
    try {
      await captureClipboard();
      await refresh();
    } catch (err) {
      error = String(err);
    }
  }

  async function copy(item: ClipboardItem): Promise<void> {
    busyId = item.id;
    try {
      if (item.kind === "image") await copyClipboardItem(item.id);
      else await writeClipboard(item.content);
    } catch (err) {
      error = String(err);
    } finally {
      busyId = null;
    }
  }

  async function togglePin(item: ClipboardItem): Promise<void> {
    busyId = item.id;
    try {
      await pinClipboard(item.id, !item.pinned);
      await refresh();
    } catch (err) {
      error = String(err);
    } finally {
      busyId = null;
    }
  }

  async function remove(item: ClipboardItem): Promise<void> {
    busyId = item.id;
    try {
      await deleteClipboard(item.id);
      await refresh();
    } catch (err) {
      error = String(err);
    } finally {
      busyId = null;
    }
  }

  async function clearAll(): Promise<void> {
    if (!window.confirm(t("clipboard.confirm_clear"))) return;
    try {
      await clearClipboard(true);
      await refresh();
    } catch (err) {
      error = String(err);
    }
  }

  function toggleWatch(): void {
    updateSettings({ clipboard_watch: !store.settings.clipboard_watch });
  }

  function when(ts: number): string {
    const delta = Date.now() - ts;
    const minutes = Math.floor(delta / 60000);
    if (minutes < 1) return t("weather.now").toLocaleLowerCase();
    if (minutes < 60) return `${minutes} ${t("common.min")}`;
    const date = new Date(ts);
    return date.toLocaleTimeString(locale(), { hour: "2-digit", minute: "2-digit", hour12: false });
  }

  const pinned = $derived(items.filter((item) => item.pinned));
  const recent = $derived(items.filter((item) => !item.pinned));

  onMount(() => {
    void refresh();
    const off = listen("clipboard-changed", scheduleRefresh);
    return () => {
      if (searchTimer) clearTimeout(searchTimer);
      if (refreshTimer) clearTimeout(refreshTimer);
      void off.then((unlisten) => unlisten());
    };
  });
</script>

<div class="clipboard">
  <div class="toolbar">
    <label class="search">
      <Icon name="search" size={14} />
      <input
        type="search"
        placeholder={t("clipboard.search_placeholder")}
        bind:value={query}
        oninput={onQuery}
        aria-label={t("clipboard.search_placeholder")}
      />
    </label>
    <button class="btn" onclick={capture} title={t("clipboard.capture")}>
      <Icon name="plus" size={14} />
      <span>{t("clipboard.capture")}</span>
    </button>
    <button
      class="btn icon-only"
      class:active={!store.settings.clipboard_watch}
      onclick={toggleWatch}
      title={store.settings.clipboard_watch ? t("clipboard.pause") : t("clipboard.resume")}
      aria-label={store.settings.clipboard_watch ? t("clipboard.pause") : t("clipboard.resume")}
    >
      <Icon name={store.settings.clipboard_watch ? "pause" : "play"} size={14} />
    </button>
    <button class="btn icon-only" onclick={clearAll} title={t("common.clear")} aria-label={t("common.clear")}>
      <Icon name="trash" size={14} />
    </button>
    <button class="btn icon-only" onclick={() => void refresh()} title={t("common.refresh")} aria-label={t("common.refresh")}>
      <Icon name="refresh" size={14} />
    </button>
  </div>

  {#if error}
    <div class="error" role="alert">{error}</div>
  {/if}

  <div class="list" aria-busy={loading}>
    {#if pinned.length}
      <div class="group-label">{t("clipboard.pinned")}</div>
      {#each pinned as item (item.id)}
        <div class="item pinned">
          <button class="content" onclick={() => void copy(item)} title={t("clipboard.copied")}>
            {#if item.kind === "image" && previews[item.id]}
              <img class="thumb" src={previews[item.id]} alt={item.preview ?? ""} />
            {:else}
              <span class="preview">{item.preview || item.content}</span>
            {/if}
            <span class="meta">
              <span>{when(item.ts)}</span>
              {#if item.kind === "image"}<span>· {t("clipboard.image")}</span>{/if}
              {#if item.source_app}<span>· {item.source_app}</span>{/if}
            </span>
          </button>
          <div class="actions">
            <button
              class="btn icon-only"
              onclick={() => void togglePin(item)}
              disabled={busyId === item.id}
              title={t("clipboard.unpin")}
              aria-label={t("clipboard.unpin")}
            >
              <Icon name="pin" size={14} />
            </button>
            <button
              class="btn icon-only"
              onclick={() => void remove(item)}
              disabled={busyId === item.id}
              title={t("common.delete")}
              aria-label={t("common.delete")}
            >
              <Icon name="trash" size={14} />
            </button>
          </div>
        </div>
      {/each}
    {/if}

    {#if recent.length}
      {#if pinned.length}<div class="group-label">{t("clipboard.recent")}</div>{/if}
      {#each recent as item (item.id)}
        <div class="item">
          <button class="content" onclick={() => void copy(item)} title={t("clipboard.copied")}>
            {#if item.kind === "image" && previews[item.id]}
              <img class="thumb" src={previews[item.id]} alt={item.preview ?? ""} />
            {:else}
              <span class="preview">{item.preview || item.content}</span>
            {/if}
            <span class="meta">
              <span>{when(item.ts)}</span>
              {#if item.kind === "image"}<span>· {t("clipboard.image")}</span>{/if}
              {#if item.source_app}<span>· {item.source_app}</span>{/if}
            </span>
          </button>
          <div class="actions">
            <button
              class="btn icon-only"
              onclick={() => void copy(item)}
              disabled={busyId === item.id}
              title={t("common.copy")}
              aria-label={t("common.copy")}
            >
              <Icon name="copy" size={14} />
            </button>
            <button
              class="btn icon-only"
              onclick={() => void togglePin(item)}
              disabled={busyId === item.id}
              title={t("clipboard.pin")}
              aria-label={t("clipboard.pin")}
            >
              <Icon name="pin" size={14} />
            </button>
            <button
              class="btn icon-only"
              onclick={() => void remove(item)}
              disabled={busyId === item.id}
              title={t("common.delete")}
              aria-label={t("common.delete")}
            >
              <Icon name="trash" size={14} />
            </button>
          </div>
        </div>
      {/each}
    {/if}

    {#if !items.length && !loading}
      <div class="empty">
        <Icon name="clipboard" size={24} />
        <span class="empty-title">{t("clipboard.empty")}</span>
        <span class="empty-hint">{t("clipboard.empty_hint")}</span>
      </div>
    {/if}
  </div>
</div>

<style>
  .clipboard {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-height: 0;
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: none;
  }

  .search {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 8px;
    background: var(--input);
    border: 1px solid var(--border);
    border-radius: 9px;
    color: var(--muted);
  }

  .search input {
    flex: 1;
    border: none;
    background: transparent;
    padding: 6px 0;
    font-size: 12px;
    outline: none;
  }

  .list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 5px;
    padding-right: 2px;
  }

  .group-label {
    font-size: 9.5px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--muted);
    padding: 4px 2px 0;
  }

  .item {
    flex: none;
    display: flex;
    align-items: stretch;
    gap: 4px;
    border: 1px solid var(--border);
    background: var(--surface);
    border-radius: 10px;
    overflow: hidden;
  }

  .item.pinned {
    border-color: color-mix(in srgb, var(--accent) 45%, var(--border));
  }

  .content {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
    align-items: flex-start;
    border: none;
    background: transparent;
    color: var(--fg);
    padding: 7px 9px;
    text-align: left;
  }

  .content:hover {
    background: var(--surface-strong);
  }

  .preview {
    max-width: 100%;
    font-size: 11.5px;
    line-height: 1.35;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .thumb {
    width: 100%;
    max-height: 54px;
    object-fit: cover;
    border-radius: 7px;
    border: 1px solid var(--border);
    background: var(--surface-strong);
  }

  .btn.active {
    color: var(--accent);
    border-color: color-mix(in srgb, var(--accent) 45%, var(--border));
    background: color-mix(in srgb, var(--accent) 12%, transparent);
  }

  .meta {
    font-size: 9.5px;
    color: var(--muted);
    display: flex;
    gap: 4px;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 1px;
    padding: 0 4px;
    opacity: 0;
    transition: opacity 0.12s ease;
  }

  .item:hover .actions,
  .item:focus-within .actions {
    opacity: 1;
  }

  .actions .btn {
    padding: 4px;
    min-width: 24px;
    min-height: 24px;
    border-color: transparent;
    background: transparent;
  }

  .empty {
    margin: auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    color: var(--muted);
    padding: 24px;
    text-align: center;
  }

  .empty-title {
    font-size: 12px;
    font-weight: 600;
    color: var(--fg);
  }

  .empty-hint {
    font-size: 11px;
    max-width: 260px;
    line-height: 1.4;
  }

  .error {
    font-size: 11px;
    color: var(--danger);
    padding: 0 2px;
  }
</style>
