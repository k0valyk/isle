<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { store } from "../store.svelte";
  import { isPlaying } from "../media.svelte";

  let host: HTMLDivElement | undefined = $state();
  let offsetX = $state(0);
  let offsetY = $state(0);

  const variant = $derived(store.settings.character || "eyes");
  const playing = $derived(isPlaying());

  function onMove(clientX: number, clientY: number): void {
    if (!host || variant === "none") return;
    const rect = host.getBoundingClientRect();
    const cx = rect.left + rect.width / 2;
    const cy = rect.top + rect.height / 2;
    const dx = clientX - cx;
    const dy = clientY - cy;
    const distance = Math.hypot(dx, dy) || 1;
    const strength = Math.min(1, distance / 180) * 2.6 * (store.settings.character_speed || 1);
    offsetX = (dx / distance) * strength;
    offsetY = (dy / distance) * strength;
  }

  function onMouseMove(event: MouseEvent): void {
    onMove(event.clientX, event.clientY);
  }

  function reset(): void {
    offsetX = 0;
    offsetY = 0;
  }

  onMount(() => {
    window.addEventListener("mousemove", onMouseMove, { passive: true });
    window.addEventListener("mouseleave", reset);
    const unlisten: Array<() => void> = [];
    const win = getCurrentWindow();
    let windowPosition = { x: 0, y: 0 };

    const refreshPosition = async (): Promise<void> => {
      try {
        const position = await win.outerPosition();
        windowPosition = { x: position.x, y: position.y };
      } catch {
        /* window is closing */
      }
    };
    void refreshPosition();
    void win.onMoved(() => void refreshPosition()).then((off) => unlisten.push(off));
    void win.onScaleChanged(() => void refreshPosition()).then((off) => unlisten.push(off));

    void listen<{ x: number; y: number }>("cursor-move", (event) => {
      if (!host) return;
      const dpr = window.devicePixelRatio || 1;
      onMove(
        (event.payload.x - windowPosition.x) / dpr,
        (event.payload.y - windowPosition.y) / dpr,
      );
    }).then((off) => unlisten.push(off));

    return () => {
      window.removeEventListener("mousemove", onMouseMove);
      window.removeEventListener("mouseleave", reset);
      for (const off of unlisten) off();
    };
  });
</script>

{#if variant !== "none"}
  <div
    class="character"
    class:playing
    bind:this={host}
    data-variant={variant}
    aria-hidden="true"
    style:--look-x={`${offsetX}px`}
    style:--look-y={`${offsetY}px`}
  >
    {#if variant === "dots"}
      <span class="dot"></span>
      <span class="dot"></span>
      <span class="dot"></span>
    {:else if variant === "pill"}
      <span class="pill">
        <span class="pupil"></span>
        <span class="pupil"></span>
      </span>
    {:else}
      <span class="eye">
        <span class="pupil"></span>
      </span>
      <span class="eye">
        <span class="pupil"></span>
      </span>
    {/if}
  </div>
{/if}

<style>
  .character {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 20px;
    padding: 0 2px;
  }

  .character.playing {
    animation: groove 0.62s ease-in-out infinite;
  }

  .character.playing .eye {
    box-shadow: 0 0 8px color-mix(in srgb, var(--accent) 45%, transparent);
  }

  .character.playing .dot {
    animation-duration: 0.62s;
  }

  @keyframes groove {
    0%, 100% {
      transform: translateY(0) scale(1);
    }
    50% {
      transform: translateY(-1.5px) scale(1.04);
    }
  }

  .eye {
    width: 10px;
    height: 13px;
    border-radius: 6px;
    background: #f4f4f5;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    animation: blink 6s infinite;
  }

  .pupil {
    width: 4px;
    height: 5px;
    border-radius: 50%;
    background: #101012;
    transform: translate(var(--look-x), var(--look-y));
    transition: transform 0.12s ease-out;
  }

  .dot {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: #f4f4f5;
    animation: bob 2.6s ease-in-out infinite;
  }

  .dot:nth-child(2) {
    animation-delay: 0.18s;
  }

  .dot:nth-child(3) {
    animation-delay: 0.36s;
  }

  .dot:nth-child(1) {
    transform: translate(var(--look-x), var(--look-y));
  }

  .pill {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
    width: 24px;
    height: 15px;
    border-radius: 999px;
    background: #f4f4f5;
  }

  .pill .pupil {
    width: 3.4px;
    height: 4.4px;
  }

  @keyframes blink {
    0%,
    92%,
    100% {
      transform: scaleY(1);
    }
    95% {
      transform: scaleY(0.15);
    }
  }

  @keyframes bob {
    0%,
    100% {
      transform: translateY(0);
    }
    50% {
      transform: translateY(-1.6px);
    }
  }

  :global(:root[data-reduced-motion="true"]) .eye,
  :global(:root[data-reduced-motion="true"]) .dot,
  :global(:root[data-reduced-motion="true"]) .character.playing {
    animation: none;
  }
</style>
