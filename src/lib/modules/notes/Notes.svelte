<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import Icon from "../../components/Icon.svelte";
  import { t, locale } from "../../store.svelte";

  interface Note {
    id: number;
    title: string;
    content: string;
    updated: number;
  }

  let notes = $state<Note[]>([]);
  let activeId = $state<number | null>(null);
  let title = $state("");
  let content = $state("");
  let error = $state<string | null>(null);
  let savedAt = $state(0);
  let saveTimer: ReturnType<typeof setTimeout> | undefined;

  const active = $derived(notes.find((note) => note.id === activeId) ?? null);

  async function load(): Promise<void> {
    try {
      notes = await invoke<Note[]>("notes_list");
      if (activeId === null && notes.length) {
        select(notes[0]);
      }
    } catch (err) {
      error = String(err);
    }
  }

  function select(note: Note): void {
    activeId = note.id;
    title = note.title;
    content = note.content;
  }

  async function create(): Promise<void> {
    try {
      const id = await invoke<number>("notes_save", {
        note: { id: null, title: "", content: "" },
      });
      await load();
      const created = notes.find((note) => note.id === id);
      if (created) select(created);
    } catch (err) {
      error = String(err);
    }
  }

  function scheduleSave(): void {
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => void persist(), 500);
  }

  async function persist(): Promise<void> {
    if (activeId === null) return;
    try {
      await invoke<number>("notes_save", {
        note: { id: activeId, title, content },
      });
      savedAt = Date.now();
      const found = notes.find((note) => note.id === activeId);
      if (found) {
        found.title = title;
        found.content = content;
        found.updated = savedAt;
      }
    } catch (err) {
      error = String(err);
    }
  }

  async function remove(): Promise<void> {
    if (activeId === null) return;
    try {
      await invoke("notes_delete", { id: activeId });
      activeId = null;
      title = "";
      content = "";
      await load();
    } catch (err) {
      error = String(err);
    }
  }

  function when(ts: number): string {
    const date = new Date(ts);
    return date.toLocaleDateString(locale(), { day: "2-digit", month: "2-digit" });
  }

  onMount(() => {
    void load();
    return () => {
      if (saveTimer) clearTimeout(saveTimer);
    };
  });
</script>

<div class="notes">
  <aside class="list">
    <button class="btn new" onclick={create}>
      <Icon name="plus" size={14} />
      <span>{t("notes.new")}</span>
    </button>
    <div class="items">
      {#each notes as note (note.id)}
        <button
          class="item"
          class:active={note.id === activeId}
          onclick={() => select(note)}
        >
          <span class="item-title">{note.title || t("notes.untitled")}</span>
          <span class="item-meta">{when(note.updated)}</span>
        </button>
      {/each}
    </div>
  </aside>

  <section class="editor">
    {#if activeId !== null}
      <div class="editor-head">
        <input
          class="title"
          bind:value={title}
          placeholder={t("notes.untitled")}
          oninput={scheduleSave}
          aria-label={t("notes.untitled")}
        />
        <span class="saved">
          {savedAt ? t("notes.autosaved") : ""}
        </span>
        <button class="btn icon-only" onclick={remove} aria-label={t("notes.delete")} title={t("notes.delete")}>
          <Icon name="trash" size={14} />
        </button>
      </div>
      <textarea
        bind:value={content}
        placeholder={t("notes.placeholder")}
        oninput={scheduleSave}
        aria-label={t("notes.placeholder")}
      ></textarea>
    {:else}
      <div class="placeholder">
        <Icon name="notes" size={26} />
        <span>{t("notes.empty")}</span>
        <button class="btn primary" onclick={create}>
          <Icon name="plus" size={14} />
          {t("notes.new")}
        </button>
      </div>
    {/if}
    {#if error}
      <div class="error" role="alert">{error}</div>
    {/if}
  </section>
</div>

<style>
  .notes {
    height: 100%;
    display: grid;
    grid-template-columns: 172px 1fr;
    gap: 10px;
    min-height: 0;
  }

  .list {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-height: 0;
  }

  .new {
    justify-content: center;
    flex: none;
  }

  .items {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-height: 0;
  }

  .item {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 1px;
    border: 1px solid transparent;
    background: transparent;
    color: var(--fg);
    border-radius: 9px;
    padding: 6px 8px;
    text-align: left;
    transition: background 0.14s ease;
  }

  .item:hover {
    background: var(--surface);
  }

  .item.active {
    background: var(--surface-strong);
    border-color: var(--border);
  }

  .item-title {
    font-size: 11.5px;
    font-weight: 550;
    max-width: 100%;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .item-meta {
    font-size: 9.5px;
    color: var(--muted);
  }

  .editor {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-height: 0;
  }

  .editor-head {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: none;
  }

  .title {
    flex: 1;
    font-size: 13px;
    font-weight: 600;
    background: transparent;
    border: none;
    padding: 4px 2px;
  }

  .saved {
    font-size: 9.5px;
    color: var(--muted);
    white-space: nowrap;
  }

  textarea {
    flex: 1;
    min-height: 0;
    resize: none;
    font-size: 12px;
    line-height: 1.55;
    padding: 10px;
    border-radius: 11px;
    background: var(--input);
    border: 1px solid var(--border);
  }

  .placeholder {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: var(--muted);
    font-size: 11.5px;
  }

  .error {
    font-size: 11px;
    color: var(--danger);
  }
</style>
