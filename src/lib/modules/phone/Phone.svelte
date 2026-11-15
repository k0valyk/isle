<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import Icon from "../../components/Icon.svelte";
  import { locale, t } from "../../store.svelte";

  interface PhoneNotification {
    id: number;
    app: string;
    title: string;
    text: string;
    ts: number;
  }

  interface PhoneFile {
    name: string;
    path: string;
    size: number;
    ts: number;
  }

  interface PhoneInfo {
    ip: string;
    port: number;
    code: string;
    paired: boolean;
    device: string | null;
    notifications: PhoneNotification[];
    files: PhoneFile[];
  }

  let info = $state<PhoneInfo | null>(null);
  let tab = $state<"notifications" | "files">("notifications");
  let copied = $state("");
  let error = $state("");
  let qrImage = $state("");
  let qrText = "";

  const address = $derived(info ? `${info.ip}:${info.port}` : "…");

  function timeLabel(ts: number): string {
    return new Date(ts).toLocaleTimeString(locale(), {
      hour: "2-digit",
      minute: "2-digit",
      hour12: false,
    });
  }

  function sizeLabel(size: number): string {
    if (size < 1024) return `${size} B`;
    if (size < 1024 * 1024) return `${(size / 1024).toFixed(0)} KB`;
    return `${(size / 1024 / 1024).toFixed(1)} MB`;
  }

  async function load(): Promise<void> {
    try {
      info = await invoke<PhoneInfo>("phone_info");
      if (info && !info.paired) {
        const url = `http://${info.ip}:${info.port}/`;
        if (qrText !== url) {
          qrText = url;
          qrImage = await invoke<string>("qr_generate", { text: url, size: 220 }).catch(() => "");
        }
      } else {
        qrImage = "";
        qrText = "";
      }
    } catch (cause) {
      error = String(cause);
    }
  }

  async function copy(value: string): Promise<void> {
    try {
      await invoke("clipboard_write", { text: value });
      copied = value;
      setTimeout(() => (copied = ""), 1200);
    } catch (cause) {
      error = String(cause);
    }
  }

  async function unpair(): Promise<void> {
    try {
      await invoke("phone_unpair");
      await load();
    } catch (cause) {
      error = String(cause);
    }
  }

  async function newCode(): Promise<void> {
    try {
      await invoke("phone_refresh_code");
      await load();
    } catch (cause) {
      error = String(cause);
    }
  }

  async function openFile(path: string): Promise<void> {
    try {
      await invoke("open_path", { path });
    } catch (cause) {
      error = String(cause);
    }
  }

  async function openWeb(): Promise<void> {
    if (!info) return;
    try {
      await invoke("open_path", { path: `http://${info.ip}:${info.port}/` });
    } catch (cause) {
      error = String(cause);
    }
  }

  onMount(() => {
    void load();
    const offNotification = listen("phone-notification", () => void load());
    const offFile = listen("phone-file", () => void load());
    const offPair = listen("phone-paired", () => void load());
    const timer = setInterval(() => void load(), 15000);
    return () => {
      void offNotification.then((unlisten) => unlisten());
      void offFile.then((unlisten) => unlisten());
      void offPair.then((unlisten) => unlisten());
      clearInterval(timer);
    };
  });
</script>

<div class="phone">
  <div class="head">
    <span class="state" class:paired={info?.paired}>
      <Icon name="phone" size={13} />
      {info?.paired ? info.device || t("phone.connected") : t("phone.not_paired")}
    </span>
    <span class="spacer"></span>
    <button class="address" onclick={() => copy(`${address}`)} title={t("phone.copy_address")}>
      {copied === address ? t("common.copied") : address}
    </button>
    {#if info?.paired}
      <button class="mini" onclick={unpair} aria-label={t("phone.unpair")} title={t("phone.unpair")}>
        <Icon name="x" size={12} />
      </button>
    {/if}
  </div>

  {#if info?.paired}
    <div class="tabs">
      <button class:active={tab === "notifications"} onclick={() => (tab = "notifications")}>
        <Icon name="bell" size={11} />{t("phone.notifications")}
        {#if info.notifications.length}<span class="badge">{info.notifications.length}</span>{/if}
      </button>
      <button class:active={tab === "files"} onclick={() => (tab = "files")}>
        <Icon name="folder" size={11} />{t("phone.files")}
        {#if info.files.length}<span class="badge">{info.files.length}</span>{/if}
      </button>
      <span class="spacer"></span>
      <span class="hint">{t("phone.connected_hint")}</span>
    </div>

    <div class="list">
      {#if tab === "notifications"}
        {#each info.notifications as notification (notification.id)}
          <div class="notification">
            <div class="notif-head">
              <span class="app">{notification.app || t("phone.app")}</span>
              <span class="time">{timeLabel(notification.ts)}</span>
            </div>
            {#if notification.title}<span class="notif-title">{notification.title}</span>{/if}
            {#if notification.text}<span class="notif-text">{notification.text}</span>{/if}
          </div>
        {:else}
          <div class="empty">
            <Icon name="bell" size={20} />
            <span>{t("phone.no_notifications")}</span>
            <span class="empty-hint">{t("phone.notifications_hint")}</span>
          </div>
        {/each}
      {:else}
        {#each info.files as file (file.path)}
          <button class="file" onclick={() => openFile(file.path)} title={file.path}>
            <span class="file-icon"><Icon name="download" size={13} /></span>
            <span class="file-name">{file.name}</span>
            <span class="file-meta">{sizeLabel(file.size)} · {timeLabel(file.ts)}</span>
            <Icon name="external" size={12} />
          </button>
        {:else}
          <div class="empty">
            <Icon name="folder" size={20} />
            <span>{t("phone.no_files")}</span>
            <span class="empty-hint">{t("phone.files_hint")}</span>
          </div>
        {/each}
      {/if}
    </div>
  {:else}
    <div class="pair">
      <div class="code-box">
        <span class="code-label">{t("phone.code")}</span>
        <button class="code" onclick={() => copy(info?.code ?? "")} title={t("phone.copy_code")}>
          {info?.code ?? "------"}
        </button>
        <button class="refresh-code" onclick={newCode}>
          <Icon name="refresh" size={11} />{t("phone.new_code")}
        </button>
      </div>

      <div class="qr-box" title={t("phone.scan_hint")}>
        {#if qrImage}
          <img src={qrImage} alt={t("phone.scan_hint")} />
        {:else}
          <Icon name="qr" size={24} />
        {/if}
      </div>

      <div class="steps">
        <span class="steps-title">{t("phone.how_to")}</span>
        <ol>
          <li>{t("phone.step_iphone", { address })}</li>
          <li>{t("phone.step_app")}</li>
          <li>{t("phone.step_code", { code: info?.code ?? "------" })}</li>
        </ol>
        <div class="web-row">
          <span class="hint">{t("phone.lan_hint")}</span>
          <button class="web-btn" onclick={openWeb}>
            <Icon name="external" size={11} />{t("phone.open_web")}
          </button>
        </div>
      </div>
    </div>
  {/if}

  {#if error}<div class="error" role="alert">{error}</div>{/if}
</div>

<style>
  .phone {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-height: 0;
  }

  .head {
    display: flex;
    align-items: center;
    gap: 5px;
    flex: none;
  }

  .state {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 10.5px;
    color: var(--muted);
  }

  .state.paired {
    color: var(--fg);
  }

  .spacer {
    flex: 1;
  }

  .address {
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--muted);
    border-radius: 5px;
    font-size: 8.5px;
    padding: 1px 6px;
    max-width: 170px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
    color: var(--danger);
  }

  .tabs {
    display: flex;
    align-items: center;
    gap: 3px;
    flex: none;
    font-size: 9.5px;
  }

  .tabs button {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    border: 1px solid transparent;
    background: transparent;
    color: var(--muted);
    border-radius: 6px;
    padding: 2px 7px;
    font-size: 9.5px;
  }

  .tabs button.active {
    background: var(--surface-strong);
    color: var(--fg);
    border-color: var(--border);
  }

  .badge {
    background: var(--accent);
    color: #fff;
    border-radius: 999px;
    font-size: 8px;
    padding: 0 4px;
  }

  .hint {
    font-size: 8.5px;
    color: var(--muted);
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

  .notification {
    display: flex;
    flex-direction: column;
    gap: 1px;
    border: 1px solid var(--border);
    background: var(--surface);
    border-radius: 8px;
    padding: 5px 7px;
  }

  .notif-head {
    display: flex;
    align-items: baseline;
    gap: 6px;
  }

  .app {
    font-size: 8.5px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--accent);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 260px;
  }

  .time {
    margin-left: auto;
    font-size: 8px;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }

  .notif-title {
    font-size: 10.5px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .notif-text {
    font-size: 9.5px;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .file {
    display: flex;
    align-items: center;
    gap: 6px;
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--fg);
    border-radius: 7px;
    padding: 4px 7px;
    font-size: 10px;
    text-align: left;
  }

  .file:hover {
    background: var(--surface-strong);
  }

  .file-icon {
    width: 22px;
    height: 22px;
    border-radius: 5px;
    background: var(--surface-strong);
    display: grid;
    place-items: center;
    color: var(--muted);
    flex: none;
  }

  .file-name {
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .file-meta {
    font-size: 8px;
    color: var(--muted);
    white-space: nowrap;
  }

  .pair {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 138px 102px 1fr;
    gap: 8px;
    align-items: center;
  }

  .qr-box {
    width: 102px;
    height: 102px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: #fff;
    display: grid;
    place-items: center;
    color: #111;
    overflow: hidden;
    padding: 6px;
  }

  .qr-box img {
    width: 100%;
    height: 100%;
    object-fit: contain;
    image-rendering: pixelated;
  }

  .code-box {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    border: 1px solid var(--border);
    background: var(--surface);
    border-radius: 10px;
    padding: 10px 8px;
  }

  .code-label {
    font-size: 8px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--muted);
  }

  .code {
    border: none;
    background: transparent;
    color: var(--fg);
    font-size: 27px;
    font-weight: 700;
    letter-spacing: 5px;
    font-variant-numeric: tabular-nums;
    padding: 0;
  }

  .refresh-code {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    border: none;
    background: transparent;
    color: var(--muted);
    font-size: 8.5px;
    padding: 0;
  }

  .refresh-code:hover {
    color: var(--fg);
  }

  .steps {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }

  .steps-title {
    font-size: 9px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--muted);
  }

  .steps ol {
    margin: 0;
    padding-left: 16px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 9.5px;
    line-height: 1.4;
    color: var(--fg);
  }

  .web-row {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
    margin-top: 2px;
  }

  .web-btn {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--fg);
    border-radius: 5px;
    font-size: 8.5px;
    padding: 2px 6px;
  }

  .web-btn:hover {
    background: var(--surface-strong);
  }

  .empty {
    margin: auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    color: var(--muted);
    font-size: 10.5px;
    text-align: center;
    padding: 12px;
  }

  .empty-hint {
    font-size: 9px;
    max-width: 340px;
    line-height: 1.4;
  }

  .error {
    font-size: 9px;
    color: var(--danger);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: none;
  }
</style>
