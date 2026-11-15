<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import Icon from "../../components/Icon.svelte";
  import { store, t } from "../../store.svelte";

  interface ChatSession {
    id: number;
    title: string;
    created: number;
    updated: number;
    messages: number;
  }

  interface ChatMessage {
    id: number;
    session_id: number;
    role: string;
    content: string;
    created: number;
  }

  let sessions = $state<ChatSession[]>([]);
  let activeId = $state<number | null>(null);
  let messages = $state<ChatMessage[]>([]);
  let draft = $state("");
  let busy = $state(false);
  let error = $state("");
  let copiedId = $state<number | null>(null);
  let listEl: HTMLDivElement | undefined = $state();

  const providerLabel = $derived(
    `${store.settings.chat_provider || "custom"} · ${store.settings.chat_model || "—"}`,
  );

  async function copyMessage(message: ChatMessage): Promise<void> {
    try {
      await invoke("clipboard_write", { text: message.content });
      copiedId = message.id;
      setTimeout(() => (copiedId = null), 1200);
    } catch (cause) {
      error = String(cause);
    }
  }

  const active = $derived(sessions.find((session) => session.id === activeId) ?? null);

  async function scrollDown(): Promise<void> {
    await new Promise((resolve) => setTimeout(resolve, 30));
    if (listEl) listEl.scrollTop = listEl.scrollHeight;
  }

  async function loadSessions(): Promise<void> {
    sessions = await invoke<ChatSession[]>("chat_sessions");
    if (!sessions.length) {
      activeId = await invoke<number>("chat_session_create", { title: null });
      sessions = await invoke<ChatSession[]>("chat_sessions");
    } else if (!activeId || !sessions.some((session) => session.id === activeId)) {
      activeId = sessions[0].id;
    }
    if (activeId) await loadMessages(activeId);
  }

  async function loadMessages(id: number): Promise<void> {
    messages = await invoke<ChatMessage[]>("chat_messages", { sessionId: id });
    await scrollDown();
  }

  async function selectSession(id: number): Promise<void> {
    if (busy || id === activeId) return;
    activeId = id;
    error = "";
    await loadMessages(id);
  }

  async function newSession(): Promise<void> {
    if (busy) return;
    error = "";
    activeId = await invoke<number>("chat_session_create", { title: null });
    await loadSessions();
  }

  async function removeSession(): Promise<void> {
    if (!activeId || busy) return;
    await invoke("chat_session_delete", { id: activeId });
    activeId = null;
    messages = [];
    await loadSessions();
  }

  async function send(): Promise<void> {
    const text = draft.trim();
    if (!text || busy || !activeId) return;
    busy = true;
    error = "";
    draft = "";
    const sessionId = activeId;
    messages = [
      ...messages,
      {
        id: -Date.now(),
        session_id: sessionId,
        role: "user",
        content: text,
        created: Date.now(),
      },
    ];
    await scrollDown();
    try {
      const reply = await invoke<{ content: string; title: string; model: string }>("chat_send", {
        sessionId,
        message: text,
      });
      messages = [
        ...messages,
        {
          id: -Date.now() - 1,
          session_id: sessionId,
          role: "assistant",
          content: reply.content,
          created: Date.now(),
        },
      ];
      sessions = await invoke<ChatSession[]>("chat_sessions");
      await scrollDown();
    } catch (cause) {
      error = String(cause);
    } finally {
      busy = false;
    }
  }

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === "Enter" && !event.shiftKey && !event.isComposing) {
      event.preventDefault();
      void send();
    }
  }

  onMount(() => {
    void loadSessions().catch((cause) => (error = String(cause)));
  });
</script>

<div class="chat">
  <div class="topbar">
    <select
      value={activeId ?? ""}
      onchange={(event) => selectSession(Number((event.currentTarget as HTMLSelectElement).value))}
      aria-label={t("chat.sessions")}
    >
      {#each sessions as session (session.id)}
        <option value={session.id}>{session.title || t("chat.new")}</option>
      {/each}
    </select>
    <button class="mini" onclick={newSession} aria-label={t("chat.new")} title={t("chat.new")}>
      <Icon name="plus" size={12} />
    </button>
    <button class="mini" onclick={removeSession} aria-label={t("chat.delete")} title={t("chat.delete")}>
      <Icon name="trash" size={12} />
    </button>
    <span class="spacer"></span>
    <span class="model" title={providerLabel}>{providerLabel}</span>
  </div>

  <div class="messages" bind:this={listEl}>
    {#if messages.length}
      {#each messages as message (message.id)}
        <div class="message" class:user={message.role === "user"}>
          <span class="text">{message.content}</span>
          {#if message.role !== "user"}
            <button
              class="copy-msg"
              onclick={() => copyMessage(message)}
              title={copiedId === message.id ? t("common.copied") : t("common.copy")}
              aria-label={t("common.copy")}
            >
              <Icon name={copiedId === message.id ? "check" : "copy"} size={10} />
            </button>
          {/if}
        </div>
      {/each}
    {:else}
      <div class="empty">
        <Icon name="sparkles" size={22} />
        <span class="empty-title">{t("chat.empty_title")}</span>
        <span class="empty-hint">{t("chat.empty_hint")}</span>
      </div>
    {/if}
    {#if busy}
      <div class="message thinking"><span class="dots">•••</span></div>
    {/if}
  </div>

  {#if error}
    <div class="error" role="alert" title={error}>{error}</div>
    <span class="error-hint">{t("chat.error_hint")}</span>
  {/if}

  <form class="composer" onsubmit={(event) => { event.preventDefault(); void send(); }}>
    <textarea
      bind:value={draft}
      onkeydown={onKeydown}
      rows="1"
      placeholder={t("chat.placeholder")}
      aria-label={t("chat.placeholder")}
    ></textarea>
    <button class="send" type="submit" disabled={busy || !draft.trim()} aria-label={t("chat.send")}>
      <Icon name="send" size={14} />
    </button>
  </form>
</div>

<style>
  .chat {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-height: 0;
  }

  .topbar {
    display: flex;
    align-items: center;
    gap: 3px;
    flex: none;
  }

  .topbar select {
    flex: 1;
    min-width: 0;
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--fg);
    border-radius: 6px;
    padding: 2px 6px;
    font-size: 10px;
    font-family: inherit;
    height: 20px;
  }

  .mini {
    width: 22px;
    height: 20px;
    display: grid;
    place-items: center;
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--muted);
    border-radius: 5px;
    padding: 0;
  }

  .mini:hover {
    color: var(--fg);
  }

  .spacer {
    flex: 1;
  }

  .model {
    font-size: 8.5px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--muted);
  }

  .messages {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 5px;
    padding: 2px;
    scrollbar-width: thin;
  }

  .message {
    max-width: 88%;
    align-self: flex-start;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 9px 9px 9px 3px;
    padding: 5px 8px;
    transition: background 0.18s ease, border-color 0.18s ease;
  }

  .message:hover {
    border-color: color-mix(in srgb, var(--accent) 35%, var(--border));
  }

  .copy-msg {
    float: right;
    margin-left: 6px;
    border: none;
    background: transparent;
    color: var(--muted);
    padding: 0;
    opacity: 0;
    transition: opacity 0.15s ease;
  }

  .message:hover .copy-msg {
    opacity: 0.8;
  }

  .copy-msg:hover {
    color: var(--fg);
  }

  .message.user {
    align-self: flex-end;
    background: var(--accent);
    border-color: transparent;
    color: #fff;
    border-radius: 9px 9px 3px 9px;
  }

  .message .text {
    font-size: 10.5px;
    line-height: 1.4;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .thinking {
    padding: 3px 10px;
  }

  .dots {
    letter-spacing: 2px;
    color: var(--muted);
    font-size: 11px;
    animation: pulse 1.2s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 0.35; }
    50% { opacity: 1; }
  }

  .empty {
    margin: auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 5px;
    color: var(--muted);
    text-align: center;
    padding: 12px;
  }

  .empty-title {
    font-size: 11.5px;
    color: var(--fg);
  }

  .empty-hint {
    font-size: 9.5px;
    max-width: 330px;
    line-height: 1.45;
  }

  .composer {
    display: flex;
    align-items: flex-end;
    gap: 4px;
    flex: none;
  }

  .composer textarea {
    flex: 1;
    min-height: 26px;
    max-height: 62px;
    resize: none;
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--fg);
    border-radius: 7px;
    padding: 5px 7px;
    font-family: inherit;
    font-size: 10.5px;
    line-height: 1.35;
  }

  .send {
    width: 28px;
    height: 28px;
    border: none;
    border-radius: 7px;
    background: var(--accent);
    color: #fff;
    display: grid;
    place-items: center;
    flex: none;
  }

  .send:disabled {
    opacity: 0.4;
  }

  .error {
    font-size: 9px;
    color: var(--danger);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: none;
  }

  .error-hint {
    font-size: 8.5px;
    color: var(--muted);
    flex: none;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  :global(:root[data-reduced-motion="true"]) .dots {
    animation: none;
  }
</style>
