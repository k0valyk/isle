<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import Icon from "../../components/Icon.svelte";
  import { locale, t } from "../../store.svelte";

  interface Adapter {
    name: string;
    description: string;
    status: string;
    speed: string;
    ipv4: string[];
    kind: string;
    vpn: boolean;
  }

  interface NetworkStatus {
    online: boolean;
    wifiSsid: string | null;
    vpn: string | null;
    adapters: Adapter[];
    checkedAt: number;
  }

  interface DownloadItem {
    id: number;
    url: string;
    name: string;
    path: string;
    received: number;
    total: number;
    done: boolean;
    error: string | null;
  }

  let mode = $state<"network" | "downloads">("network");
  let status = $state<NetworkStatus | null>(null);
  let loading = $state(false);
  let error = $state("");
  let copied = $state("");

  let downloads = $state<DownloadItem[]>([]);
  let url = $state("");
  let starting = $state(false);
  let downloadsBusy = $state(false);

  const activeAdapters = $derived(
    (status?.adapters ?? [])
      .filter((adapter) => adapter.status.toLowerCase() === "up" || adapter.ipv4.length)
      .sort((a, b) => Number(b.vpn) - Number(a.vpn) || Number(b.kind === "wifi") - Number(a.kind === "wifi")),
  );

  function formatBytes(value: number): string {
    if (!value) return "0 B";
    if (value < 1024) return `${value} B`;
    if (value < 1024 * 1024) return `${(value / 1024).toFixed(0)} KB`;
    if (value < 1024 * 1024 * 1024) return `${(value / 1024 / 1024).toFixed(1)} MB`;
    return `${(value / 1024 / 1024 / 1024).toFixed(2)} GB`;
  }

  function progress(item: DownloadItem): number {
    if (!item.total) return item.done ? 100 : 0;
    return Math.min(100, Math.round((item.received / item.total) * 100));
  }

  async function refresh(): Promise<void> {
    if (loading) return;
    loading = true;
    error = "";
    try {
      status = await invoke<NetworkStatus>("network_status");
    } catch (cause) {
      error = String(cause);
    } finally {
      loading = false;
    }
  }

  async function loadDownloads(): Promise<void> {
    try {
      downloads = await invoke<DownloadItem[]>("downloads_list");
    } catch (cause) {
      error = String(cause);
    }
  }

  async function startDownload(): Promise<void> {
    const target = url.trim();
    if (!target || starting) return;
    starting = true;
    error = "";
    try {
      await invoke("download_start", { url: target, name: null });
      url = "";
      await loadDownloads();
    } catch (cause) {
      error = String(cause);
    } finally {
      starting = false;
    }
  }

  async function cancelDownload(id: number): Promise<void> {
    await invoke("download_cancel", { id });
  }

  async function clearDownloads(): Promise<void> {
    downloadsBusy = true;
    try {
      await invoke("download_clear");
      await loadDownloads();
    } finally {
      downloadsBusy = false;
    }
  }

  async function openDownload(item: DownloadItem): Promise<void> {
    try {
      await invoke("open_path", { path: item.path });
    } catch (cause) {
      error = String(cause);
    }
  }

  async function openFolder(): Promise<void> {
    try {
      await invoke("downloads_open_folder");
    } catch (cause) {
      error = String(cause);
    }
  }

  async function openNetwork(path: string): Promise<void> {
    try {
      await invoke("open_path", { path });
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

  onMount(() => {
    void refresh();
    void loadDownloads();
    const offs: Array<Promise<() => void>> = [
      listen<DownloadItem>("download-progress", (event) => {
        const next = event.payload;
        const index = downloads.findIndex((item) => item.id === next.id);
        if (index >= 0) {
          downloads[index] = next;
          downloads = [...downloads];
        } else {
          downloads = [next, ...downloads];
        }
      }),
      listen("download-done", () => void loadDownloads()),
    ];
    return () => {
      for (const off of offs) void off.then((unlisten) => unlisten());
    };
  });
</script>

<div class="control">
  <div class="modes">
    <button class:active={mode === "network"} onclick={() => (mode = "network")}>
      <Icon name="wifi" size={12} />{t("network.tab")}
    </button>
    <button class:active={mode === "downloads"} onclick={() => (mode = "downloads")}>
      <Icon name="download" size={12} />{t("downloads.tab")}
      {#if downloads.some((item) => !item.done)}<span class="badge">{downloads.filter((item) => !item.done).length}</span>{/if}
    </button>
    <span class="spacer"></span>
    {#if mode === "network"}
      <button class="mini" onclick={refresh} disabled={loading} title={t("common.refresh")} aria-label={t("common.refresh")}>
        <Icon name="refresh" size={12} />
      </button>
    {:else}
      <button class="mini" onclick={clearDownloads} disabled={downloadsBusy || !downloads.some((item) => item.done)} title={t("downloads.clear")} aria-label={t("downloads.clear")}>
        <Icon name="trash" size={12} />
      </button>
      <button class="mini" onclick={openFolder} title={t("downloads.open_folder")} aria-label={t("downloads.open_folder")}>
        <Icon name="folder" size={12} />
      </button>
    {/if}
  </div>

  {#if mode === "network"}
    <div class="head">
      <span class="state" class:online={status?.online}>
        <span class="dot"></span>
        {status ? (status.online ? t("network.online") : t("network.offline")) : t("network.checking")}
      </span>
      {#if status?.wifiSsid}
        <span class="chip"><Icon name="wifi" size={11} />{status.wifiSsid}</span>
      {/if}
      {#if status?.vpn}
        <span class="chip vpn"><Icon name="globe" size={11} />{status.vpn}</span>
      {/if}
      <span class="spacer"></span>
      <button class="mini" onclick={() => openNetwork("ms-settings:network")} title={t("network.open_settings")} aria-label={t("network.open_settings")}>
        <Icon name="settings" size={12} />
      </button>
      <button class="mini" onclick={() => openNetwork("ncpa.cpl")} title={t("network.open_adapters")} aria-label={t("network.open_adapters")}>
        <Icon name="sliders" size={12} />
      </button>
    </div>

    <div class="list">
      {#if !status && loading}
        <div class="empty">{t("common.loading")}</div>
      {:else if status}
        {#each activeAdapters as adapter (adapter.name + adapter.description)}
          <div class="adapter">
            <span class="icon" class:vpn={adapter.vpn}>
              <Icon name={adapter.vpn ? "globe" : adapter.kind === "wifi" ? "wifi" : "terminal"} size={14} />
            </span>
            <div class="body">
              <div class="line">
                <span class="name">{adapter.name}</span>
                <span class="kind">{t(`network.kind_${adapter.kind}`)}</span>
                <span class="speed">{adapter.speed}</span>
              </div>
              <span class="desc" title={adapter.description}>{adapter.description}</span>
            </div>
            <div class="ips">
              {#each adapter.ipv4 as ip}
                <button class="ip" onclick={() => copy(ip)} title={t("network.copy_ip")}>
                  {copied === ip ? t("common.copied") : ip}
                </button>
              {/each}
            </div>
          </div>
        {:else}
          <div class="empty">{t("network.no_adapters")}</div>
        {/each}
      {:else}
        <div class="empty">{t("network.no_data")}</div>
      {/if}
    </div>

    <div class="foot">
      {#if status}
        <span>{t("network.checked", { time: new Date(status.checkedAt).toLocaleTimeString(locale(), { hour: "2-digit", minute: "2-digit" }) })}</span>
      {/if}
      <span class="spacer-line"></span>
      <span>{t("network.local_only")}</span>
    </div>
  {:else}
    <form class="download-add" onsubmit={(event) => { event.preventDefault(); void startDownload(); }}>
      <input
        bind:value={url}
        placeholder={t("downloads.placeholder")}
        aria-label={t("downloads.placeholder")}
        inputmode="url"
      />
      <button class="start" type="submit" disabled={starting || !url.trim()} aria-label={t("downloads.start")}>
        <Icon name="download" size={13} />
      </button>
    </form>

    <div class="list">
      {#each downloads as item (item.id)}
        <div class="download" class:done={item.done} class:failed={!!item.error}>
          <span class="file-icon"><Icon name={item.done && !item.error ? "check" : "download"} size={13} /></span>
          <div class="download-body">
            <div class="download-line">
              <span class="download-name" title={item.path}>{item.name}</span>
              <span class="download-size">
                {#if item.error}
                  {item.error === "cancelled" ? t("downloads.cancelled") : t("downloads.failed")}
                {:else if item.done}
                  {formatBytes(item.received)}
                {:else}
                  {formatBytes(item.received)}{item.total ? ` / ${formatBytes(item.total)}` : ""}
                {/if}
              </span>
            </div>
            <div class="bar"><span style:width={`${progress(item)}%`}></span></div>
          </div>
          {#if item.done && !item.error}
            <button class="row-action" onclick={() => openDownload(item)} title={t("downloads.open")} aria-label={t("downloads.open")}>
              <Icon name="external" size={12} />
            </button>
          {:else if !item.done}
            <button class="row-action" onclick={() => cancelDownload(item.id)} title={t("downloads.cancel")} aria-label={t("downloads.cancel")}>
              <Icon name="x" size={12} />
            </button>
          {/if}
        </div>
      {:else}
        <div class="empty">
          <Icon name="download" size={20} />
          <span>{t("downloads.empty")}</span>
          <span class="empty-hint">{t("downloads.hint")}</span>
        </div>
      {/each}
    </div>
  {/if}

  {#if error}<div class="error" role="alert">{error}</div>{/if}
</div>

<style>
  .control {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 4px;
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
    gap: 4px;
    border: 1px solid transparent;
    background: transparent;
    color: var(--muted);
    border-radius: 6px;
    padding: 3px 7px;
    font-size: 10.5px;
  }

  .modes button.active {
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

  .spacer {
    flex: 1;
  }

  .spacer-line {
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

  .mini:hover {
    color: var(--fg);
  }

  .mini:disabled {
    opacity: 0.4;
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

  .state .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--muted);
  }

  .state.online .dot {
    background: #22c55e;
    box-shadow: 0 0 6px #22c55e;
  }

  .state.online {
    color: var(--fg);
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--muted);
    border-radius: 999px;
    font-size: 9px;
    padding: 1px 7px;
    max-width: 150px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .chip.vpn {
    color: var(--accent);
    border-color: var(--accent);
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

  .adapter {
    display: flex;
    align-items: center;
    gap: 7px;
    border: 1px solid var(--border);
    background: var(--surface);
    border-radius: 8px;
    padding: 5px 7px;
  }

  .icon {
    width: 26px;
    height: 26px;
    border-radius: 7px;
    background: var(--surface-strong);
    display: grid;
    place-items: center;
    color: var(--muted);
    flex: none;
  }

  .icon.vpn {
    color: var(--accent);
  }

  .adapter .body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .line {
    display: flex;
    align-items: baseline;
    gap: 5px;
    min-width: 0;
  }

  .name {
    font-size: 10.5px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 160px;
  }

  .kind {
    font-size: 8px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--muted);
  }

  .speed {
    font-size: 8.5px;
    color: var(--muted);
    margin-left: auto;
    white-space: nowrap;
  }

  .desc {
    font-size: 8.5px;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .ips {
    display: flex;
    flex-direction: column;
    gap: 2px;
    align-items: flex-end;
    flex: none;
  }

  .ip {
    border: none;
    background: transparent;
    color: var(--muted);
    font-size: 9px;
    font-variant-numeric: tabular-nums;
    padding: 0;
    white-space: nowrap;
  }

  .ip:hover {
    color: var(--accent);
  }

  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 7px;
    color: var(--muted);
    font-size: 10.5px;
    padding: 14px;
    text-align: center;
  }

  .empty-hint {
    font-size: 9px;
    line-height: 1.4;
    max-width: 330px;
  }

  .foot {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 8.5px;
    color: var(--muted);
    flex: none;
  }

  .download-add {
    display: flex;
    align-items: center;
    gap: 4px;
    flex: none;
  }

  .download-add input {
    flex: 1;
    min-width: 0;
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--fg);
    border-radius: 6px;
    padding: 4px 7px;
    font-size: 10px;
    font-family: inherit;
  }

  .start {
    width: 27px;
    height: 25px;
    border: none;
    border-radius: 6px;
    background: var(--accent);
    color: #fff;
    display: grid;
    place-items: center;
    padding: 0;
  }

  .start:disabled {
    opacity: 0.4;
  }

  .download {
    display: flex;
    align-items: center;
    gap: 7px;
    border: 1px solid var(--border);
    background: var(--surface);
    border-radius: 8px;
    padding: 4px 7px;
    flex: none;
  }

  .download.failed {
    border-color: color-mix(in srgb, var(--danger) 45%, transparent);
  }

  .file-icon {
    width: 24px;
    height: 24px;
    border-radius: 6px;
    background: var(--surface-strong);
    display: grid;
    place-items: center;
    color: var(--muted);
    flex: none;
  }

  .download.done .file-icon {
    color: var(--ok);
  }

  .download-body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .download-line {
    display: flex;
    align-items: baseline;
    gap: 6px;
  }

  .download-name {
    font-size: 10px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
  }

  .download-size {
    font-size: 8.5px;
    color: var(--muted);
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }

  .bar {
    height: 3px;
    border-radius: 999px;
    background: var(--surface-strong);
    overflow: hidden;
  }

  .bar span {
    display: block;
    height: 100%;
    background: linear-gradient(to right, var(--accent), #93c5fd);
    border-radius: inherit;
    transition: width 120ms linear;
  }

  .download.done .bar span {
    background: var(--ok);
  }

  .download.failed .bar span {
    background: var(--danger);
  }

  .row-action {
    width: 22px;
    height: 22px;
    border: none;
    background: transparent;
    color: var(--muted);
    display: grid;
    place-items: center;
    padding: 0;
    border-radius: 5px;
    flex: none;
  }

  .row-action:hover {
    background: var(--surface-strong);
    color: var(--fg);
  }

  .error {
    font-size: 9px;
    color: var(--danger);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: none;
  }

  :global(:root[data-reduced-motion="true"]) .bar span {
    transition: none;
  }
</style>
