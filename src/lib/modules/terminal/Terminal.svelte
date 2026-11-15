<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { FitAddon } from "@xterm/addon-fit";
  import { Terminal } from "@xterm/xterm";
  import "@xterm/xterm/css/xterm.css";
  import Icon from "../../components/Icon.svelte";
  import { store, t } from "../../store.svelte";

  let host: HTMLDivElement | undefined = $state();
  let sessionId = $state(0);
  let error = $state("");
  let term: Terminal | undefined;
  let fit: FitAddon | undefined;
  let resizeObserver: ResizeObserver | undefined;
  let unlisten: Array<() => void> = [];

  const decoder = new TextDecoder();

  function themeColors() {
    const root = getComputedStyle(document.documentElement);
    const read = (name: string, fallback: string) =>
      root.getPropertyValue(name).trim() || fallback;
    return {
      background: read("--bg", "#000000"),
      foreground: read("--fg", "#f4f4f5"),
      cursor: read("--accent", "#3b82f6"),
      selectionBackground: read("--surface-strong", "rgba(255,255,255,0.18)"),
      black: "#1a1a1e",
      brightBlack: read("--muted", "#a1a1aa"),
    };
  }

  async function clearSession(): Promise<void> {
    if (sessionId) {
      try {
        await invoke("terminal_close", { id: sessionId });
      } catch {
        /* session may already be gone */
      }
      sessionId = 0;
    }
  }

  async function start(): Promise<void> {
    if (!host || !term || !fit) return;
    await clearSession();
    fit.fit();
    try {
      const id = await invoke<number>("terminal_open", {
        cols: term.cols,
        rows: term.rows,
      });
      sessionId = id;
      error = "";
      term.focus();
    } catch (cause) {
      error = String(cause);
    }
  }

  async function restart(): Promise<void> {
    if (term) {
      term.reset();
      term.clear();
    }
    await start();
  }

  function clearScreen(): void {
    term?.clear();
    term?.focus();
  }

  onMount(() => {
    if (!host) return;
    term = new Terminal({
      fontFamily:
        '"Cascadia Mono", "Consolas", "Segoe UI Mono", ui-monospace, monospace',
      fontSize: 11,
      lineHeight: 1.15,
      cursorBlink: !store.settings.reduced_motion,
      scrollback: 1200,
      allowTransparency: true,
      theme: themeColors(),
      convertEol: true,
    });
    fit = new FitAddon();
    term.loadAddon(fit);
    term.open(host);
    fit.fit();

    const offData = term.onData((data) => {
      if (!sessionId) return;
      void invoke("terminal_write", { id: sessionId, data }).catch((cause) => {
        error = String(cause);
      });
    });

    void listen<{ id: number; data: string }>("terminal-output", (event) => {
      if (!term || event.payload.id !== sessionId) return;
      const raw = atob(event.payload.data);
      const bytes = new Uint8Array(raw.length);
      for (let index = 0; index < raw.length; index += 1) {
        bytes[index] = raw.charCodeAt(index);
      }
      term.write(decoder.decode(bytes, { stream: true }));
    }).then((off) => unlisten.push(off));

    void listen<{ id: number }>("terminal-exit", (event) => {
      if (event.payload.id === sessionId) {
        term?.write("\r\n\x1b[90m[process exited]\x1b[0m\r\n");
        sessionId = 0;
      }
    }).then((off) => unlisten.push(off));

    resizeObserver = new ResizeObserver(() => {
      if (!fit || !term || !host) return;
      if (host.clientWidth < 20 || host.clientHeight < 20) return;
      fit.fit();
      if (sessionId) {
        void invoke("terminal_resize", {
          id: sessionId,
          cols: term.cols,
          rows: term.rows,
        }).catch(() => {});
      }
    });
    resizeObserver.observe(host);

    void start();

    return () => {
      offData.dispose();
      resizeObserver?.disconnect();
      for (const off of unlisten) off();
      unlisten = [];
      void clearSession();
      term?.dispose();
      term = undefined;
    };
  });
</script>

<div class="terminal" data-session={sessionId}>
  <div class="bar">
    <span class="shell"><Icon name="terminal" size={12} />PowerShell</span>
    <span class="spacer"></span>
    <button class="mini" onclick={clearScreen} title={t("common.clear")} aria-label={t("common.clear")}>
      <Icon name="trash" size={11} />
    </button>
    <button class="mini" onclick={restart} title={t("terminal.restart")} aria-label={t("terminal.restart")}>
      <Icon name="refresh" size={11} />
    </button>
  </div>
  <div class="screen" bind:this={host}></div>
  {#if error}<div class="error" role="alert">{error}</div>{/if}
</div>

<style>
  .terminal {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-height: 0;
  }

  .bar {
    display: flex;
    align-items: center;
    gap: 4px;
    flex: none;
  }

  .shell {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 9px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--muted);
  }

  .spacer {
    flex: 1;
  }

  .mini {
    width: 20px;
    height: 18px;
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

  .screen {
    flex: 1;
    min-height: 0;
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 4px 6px;
    background: var(--bg);
    overflow: hidden;
  }

  .screen :global(.xterm) {
    height: 100%;
  }

  .screen :global(.xterm-viewport) {
    scrollbar-width: thin;
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
