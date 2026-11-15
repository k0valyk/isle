<script lang="ts">
  import { t } from "../store.svelte";

  interface Props {
    value: string;
    label: string;
    onchange: (value: string) => void;
  }

  let { value, label, onchange }: Props = $props();
  let recording = $state(false);
  let element: HTMLButtonElement | undefined = $state();

  const MODIFIERS = ["Control", "Alt", "Shift", "Meta"];

  function normalizeKey(key: string): string {
    if (key === " ") return "Space";
    if (key === "ArrowUp") return "Up";
    if (key === "ArrowDown") return "Down";
    if (key === "ArrowLeft") return "Left";
    if (key === "ArrowRight") return "Right";
    if (key.length === 1) return key.toUpperCase();
    return key;
  }

  function onKeydown(event: KeyboardEvent): void {
    if (!recording) return;
    event.preventDefault();
    event.stopPropagation();
    if (event.key === "Escape") {
      recording = false;
      return;
    }
    if (event.key === "Backspace" || event.key === "Delete") {
      onchange("");
      recording = false;
      return;
    }
    if (MODIFIERS.includes(event.key)) return;

    const key = normalizeKey(event.key);
    const functionKey = /^F([1-9]|1[0-9]|2[0-4])$/.test(key);
    if (!event.ctrlKey && !event.altKey && !event.shiftKey && !event.metaKey && !functionKey) {
      return;
    }

    const parts: string[] = [];
    if (event.ctrlKey) parts.push("Ctrl");
    if (event.altKey) parts.push("Alt");
    if (event.shiftKey) parts.push("Shift");
    if (event.metaKey) parts.push("Super");
    parts.push(key);
    onchange(parts.join("+"));
    recording = false;
  }
</script>

<button
  bind:this={element}
  type="button"
  class="hotkey"
  class:recording
  aria-label={label}
  onclick={() => {
    recording = true;
    element?.focus();
  }}
  onkeydown={onKeydown}
  onblur={() => (recording = false)}
>
  {#if recording}
    <span class="pulse"></span>
    {t("settings.hotkey_recording")}
  {:else if value}
    <kbd>{value}</kbd>
  {:else}
    <span class="unset">{t("settings.hotkey_unset")}</span>
  {/if}
</button>

<style>
  .hotkey {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    min-width: 128px;
    min-height: 30px;
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--fg);
    border-radius: 8px;
    padding: 4px 10px;
    font-size: 11px;
    transition: border-color 0.15s ease, background 0.15s ease;
  }

  .hotkey:hover {
    background: var(--surface-strong);
  }

  .hotkey.recording {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 12%, transparent);
  }

  kbd {
    font-family: inherit;
    font-size: 11px;
  }

  .unset {
    color: var(--muted);
    font-size: 10px;
  }

  .pulse {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
    animation: pulse 1s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 0.4; }
    50% { opacity: 1; }
  }

  :global(:root[data-reduced-motion="true"]) .pulse {
    animation: none;
  }
</style>
