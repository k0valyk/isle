<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import Icon from "../../components/Icon.svelte";
  import { locale, t } from "../../store.svelte";
  import {
    agentPhase,
    agents,
    clearAgents,
    initAgents,
    installOpenCode,
    refreshAgents,
    refreshUsage,
    testAgent,
    usage,
    type ModelUsage,
  } from "../../agents.svelte";

  let copied = $state(false);
  let error = $state("");
  let busy = $state("");
  let installMessage = $state("");
  let now = $state(Date.now());

  const events = $derived(agents.events);
  const status = $derived(agents.status);
  const phase = $derived(agentPhase(events[0]));

  function sourceClass(source: string): string {
    const key = source.toLowerCase();
    if (key.includes("claude")) return "claude";
    if (key.includes("codex")) return "codex";
    if (key.includes("opencode")) return "opencode";
    return "other";
  }

  function timeLabel(ts: number): string {
    return new Date(ts).toLocaleTimeString(locale(), {
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
      hour12: false,
    });
  }

  function compact(value: number): string {
    if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`;
    if (value >= 1_000) return `${Math.round(value / 1_000)}k`;
    return String(Math.round(value));
  }

  function usedPercent(item: ModelUsage): number {
    if (!item.contextLimit || item.contextUsed <= 0) return 0;
    return Math.max(2, Math.min(100, (item.contextUsed / item.contextLimit) * 100));
  }

  function leftLabel(item: ModelUsage): string {
    if (item.contextLeft === null) return "—";
    return `${compact(item.contextLeft)} ${t("agents.left")}`;
  }

  function since(ts: number): string {
    if (!ts) return t("agents.never");
    const seconds = Math.max(0, Math.round((now - ts) / 1000));
    if (seconds < 60) return `${seconds} ${t("agents.seconds")}`;
    const minutes = Math.round(seconds / 60);
    if (minutes < 60) return `${minutes} ${t("common.min")}`;
    return `${Math.round(minutes / 60)} ${t("agents.hours")}`;
  }

  async function copyEndpoint(): Promise<void> {
    try {
      const endpoint = status?.endpoint ?? "127.0.0.1:45871";
      await invoke("clipboard_write", { text: `http://${endpoint}/event` });
      copied = true;
      setTimeout(() => (copied = false), 1200);
    } catch (cause) {
      error = String(cause);
    }
  }

  async function runTest(): Promise<void> {
    if (busy) return;
    busy = "test";
    error = "";
    try {
      await testAgent();
    } catch (cause) {
      error = String(cause);
    } finally {
      busy = "";
    }
  }

  async function install(): Promise<void> {
    if (busy) return;
    busy = "install";
    error = "";
    try {
      installMessage = await installOpenCode();
    } catch (cause) {
      error = String(cause);
    } finally {
      busy = "";
    }
  }

  async function openFolder(): Promise<void> {
    try {
      await invoke("agents_open_opencode");
    } catch (cause) {
      error = String(cause);
    }
  }

  async function clear(): Promise<void> {
    try {
      await clearAgents();
    } catch (cause) {
      error = String(cause);
    }
  }

  onMount(() => {
    void initAgents();
    void refreshAgents();
    void refreshUsage();
    const handle = setInterval(() => (now = Date.now()), 1000);
    const usageHandle = setInterval(() => void refreshUsage(), 5000);
    return () => {
      clearInterval(handle);
      clearInterval(usageHandle);
    };
  });
</script>

<div class="agents">
  <div class="status">
    <span class="live" class:online={status?.listening === true} class:offline={status !== null && status.listening === false}>
      <span class="dot"></span>
      {#if status?.listening}
        {t("agents.listening")}
      {:else}
        {t("agents.offline")}
      {/if}
    </span>
    <span class="phase" class:working={phase === "working"} class:done={phase === "done"} class:error={phase === "error"} class:attention={phase === "attention"}>
      {t(`agents.phase_${phase}`)}
    </span>
    <span class="spacer"></span>
    <span class="stat" title={t("agents.requests")}>
      {status?.requests ?? 0}
    </span>
    <span class="stat" title={t("agents.last")}>{since(status?.lastAt ?? 0)}</span>
  </div>

  <div class="head">
    <button class="endpoint" onclick={copyEndpoint} title={t("agents.copy_endpoint")}>
      <Icon name={copied ? "check" : "link"} size={10} />
      {copied ? t("common.copied") : `http://${status?.endpoint ?? "127.0.0.1:45871"}/event`}
    </button>
    <span class="spacer"></span>
    <button class="mini" onclick={runTest} disabled={busy !== ""} title={t("agents.test")} aria-label={t("agents.test")}>
      <Icon name="send" size={12} />
    </button>
    <button class="mini" onclick={() => void refreshAgents()} title={t("common.refresh")} aria-label={t("common.refresh")}>
      <Icon name="refresh" size={12} />
    </button>
    <button class="mini" onclick={clear} disabled={!events.length} title={t("common.clear")} aria-label={t("common.clear")}>
      <Icon name="trash" size={12} />
    </button>
  </div>

  {#if usage.models.length}
    <div class="usage">
      <div class="usage-head">
        <span class="usage-title"><Icon name="activity" size={11} />{t("agents.usage")}</span>
        <span class="usage-hint">{t("agents.usage_live")}</span>
      </div>
      <div class="usage-list">
        {#each usage.models.slice(0, 3) as item (item.provider + item.model)}
          <div class="usage-item">
            <div class="usage-line">
              <span class="usage-model" title={`${item.provider} / ${item.model}`}>{item.label}</span>
              <span class="usage-cost">${item.cost.toFixed(3)}</span>
            </div>
            <div class="usage-bar" title={`${compact(item.contextUsed)} / ${item.contextLimit ? compact(item.contextLimit) : "—"}`}>
              <span style:width={`${usedPercent(item)}%`}></span>
            </div>
            <div class="usage-meta">
              <span>{compact(item.inputTokens + item.outputTokens + item.reasoningTokens)} {t("agents.tokens")}</span>
              <span>{leftLabel(item)}</span>
            </div>
          </div>
        {/each}
      </div>
    </div>
  {/if}

  <div class="integration">
    <span class="integration-icon" class:installed={status?.opencodePlugin === true}>
      <Icon name="terminal" size={12} />
    </span>
    <span class="integration-copy">
      <span class="strong">
        {status?.opencodePlugin ? t("agents.opencode_installed") : t("agents.opencode_missing")}
      </span>
      <span class="sub ellipsis">
        {installMessage || status?.opencodePath || t("agents.opencode_hint")}
      </span>
    </span>
    <button class="btn small" onclick={install} disabled={busy === "install"}>
      <Icon name="download" size={11} />
      {status?.opencodePlugin ? t("agents.reinstall") : t("agents.install")}
    </button>
    <button class="btn small ghost" onclick={openFolder} title={t("agents.open_folder")} aria-label={t("agents.open_folder")}>
      <Icon name="folder" size={11} />
    </button>
  </div>

  <div class="list">
    {#each events as event (event.id)}
      <div class="event" class:working={agentPhase(event) === "working"} class:done={agentPhase(event) === "done"} class:error={agentPhase(event) === "error"} class:attention={agentPhase(event) === "attention"}>
        <span class="source" class:claude={sourceClass(event.source) === "claude"} class:codex={sourceClass(event.source) === "codex"} class:opencode={sourceClass(event.source) === "opencode"}>
          {event.source}
        </span>
        <div class="body">
          <div class="line">
            <span class="event-kind">{event.event}</span>
            <span class="time">{timeLabel(event.ts)}</span>
          </div>
          <span class="title" title={event.title}>{event.title}</span>
          {#if event.detail}
            <span class="detail" title={event.detail}>{event.detail}</span>
          {/if}
        </div>
      </div>
    {:else}
      <div class="empty">
        <Icon name="terminal" size={24} />
        <span class="empty-title">{t("agents.empty_title")}</span>
        <span class="empty-hint">{t("agents.empty_hint")}</span>
        <code>isle-hook claude stop "Task finished"</code>
        <span class="hint-small">{t("agents.hint")}</span>
      </div>
    {/each}
  </div>

  {#if error}<div class="error" role="alert">{error}</div>{/if}
</div>

<style>
  .agents {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-height: 0;
  }

  .status,
  .head,
  .integration {
    display: flex;
    align-items: center;
    gap: 5px;
    flex: none;
    min-width: 0;
  }

  .live {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 9px;
    color: var(--muted);
    white-space: nowrap;
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #ef4444;
    box-shadow: 0 0 5px #ef4444;
  }

  .live.online .dot {
    background: #22c55e;
    box-shadow: 0 0 6px #22c55e;
  }

  .phase {
    font-size: 8px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    border-radius: 999px;
    padding: 1px 6px;
    border: 1px solid var(--border);
    color: var(--muted);
  }

  .phase.working {
    color: #f5b301;
    border-color: color-mix(in srgb, #f5b301 45%, var(--border));
  }

  .phase.done {
    color: #22c55e;
    border-color: color-mix(in srgb, #22c55e 45%, var(--border));
  }

  .phase.error {
    color: var(--danger);
    border-color: color-mix(in srgb, var(--danger) 45%, var(--border));
  }

  .phase.attention {
    color: var(--accent);
    border-color: color-mix(in srgb, var(--accent) 45%, var(--border));
  }

  .stat {
    font-size: 8px;
    color: var(--muted);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 1px 5px;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .endpoint {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--muted);
    border-radius: 5px;
    font-size: 8.5px;
    padding: 1px 6px;
    max-width: 230px;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .endpoint:hover {
    color: var(--fg);
  }

  .usage {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 4px 6px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: color-mix(in srgb, var(--surface) 62%, transparent);
    animation: usage-in 0.35s cubic-bezier(0.22, 1, 0.36, 1);
  }

  .usage-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
  }

  .usage-title {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 8px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--accent);
  }

  .usage-hint {
    font-size: 7.5px;
    color: var(--muted);
  }

  .usage-list {
    display: flex;
    gap: 5px;
    overflow-x: auto;
    scrollbar-width: none;
  }

  .usage-list::-webkit-scrollbar {
    display: none;
  }

  .usage-item {
    flex: 1 1 0;
    min-width: 112px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 3px 5px;
    border-radius: 5px;
    background: var(--surface);
  }

  .usage-line,
  .usage-meta {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 5px;
    min-width: 0;
  }

  .usage-model {
    font-size: 8.5px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .usage-cost {
    font-size: 7.5px;
    color: #22c55e;
    font-variant-numeric: tabular-nums;
    flex: none;
  }

  .usage-bar {
    height: 3px;
    border-radius: 999px;
    background: var(--surface-strong);
    overflow: hidden;
  }

  .usage-bar span {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: linear-gradient(90deg, var(--live-accent), #93c5fd);
    transition: width 0.5s cubic-bezier(0.22, 1, 0.36, 1);
  }

  .usage-meta {
    font-size: 7px;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  @keyframes usage-in {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .integration {
    padding: 4px 6px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface);
  }

  .integration-icon {
    width: 22px;
    height: 22px;
    border-radius: 6px;
    display: grid;
    place-items: center;
    background: var(--surface-strong);
    color: var(--muted);
    flex: none;
  }

  .integration-icon.installed {
    color: #22c55e;
  }

  .integration-copy {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0;
  }

  .strong {
    font-size: 9.5px;
    font-weight: 600;
  }

  .sub {
    font-size: 8px;
    color: var(--muted);
  }

  .ellipsis {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .btn.small {
    padding: 2px 6px;
    min-height: 20px;
    font-size: 8.5px;
    gap: 3px;
    flex: none;
  }

  .btn.ghost {
    border-color: transparent;
    background: transparent;
  }

  .spacer {
    flex: 1;
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

  .mini:disabled {
    opacity: 0.4;
  }

  .list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 3px;
    scrollbar-width: thin;
  }

  .event {
    flex: none;
    display: flex;
    gap: 7px;
    border: 1px solid var(--border);
    background: var(--surface);
    border-radius: 8px;
    padding: 5px 7px;
    border-left: 2px solid var(--border);
  }

  .event.working {
    border-left-color: #f5b301;
  }

  .event.done {
    border-left-color: #22c55e;
  }

  .event.error {
    border-left-color: var(--danger);
  }

  .event.attention {
    border-left-color: var(--accent);
  }

  .source {
    align-self: flex-start;
    font-size: 8px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    border-radius: 4px;
    padding: 1px 5px;
    background: var(--surface-strong);
    color: var(--muted);
    max-width: 70px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .source.claude {
    color: #d97757;
  }

  .source.codex {
    color: #22c55e;
  }

  .source.opencode {
    color: #38bdf8;
  }

  .body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .line {
    display: flex;
    align-items: baseline;
    gap: 6px;
  }

  .event-kind {
    font-size: 8.5px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--accent);
  }

  .time {
    font-size: 8px;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
    margin-left: auto;
  }

  .title {
    font-size: 10.5px;
    font-weight: 550;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .detail {
    font-size: 9px;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .empty {
    margin: auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 5px;
    text-align: center;
    color: var(--muted);
    padding: 10px;
  }

  .empty-title {
    font-size: 11.5px;
    color: var(--fg);
  }

  .empty-hint {
    font-size: 9.5px;
    line-height: 1.45;
    max-width: 380px;
  }

  code {
    font-size: 9px;
    background: var(--surface-strong);
    border: 1px solid var(--border);
    border-radius: 5px;
    padding: 3px 7px;
    color: var(--fg);
    max-width: 90%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .hint-small {
    font-size: 8.5px;
  }

  .error {
    font-size: 9px;
    color: var(--danger);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
