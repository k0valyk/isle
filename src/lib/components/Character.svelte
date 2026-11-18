<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { store } from "../store.svelte";
  import { isPlaying } from "../media.svelte";
  import PerfHud from "./PerfHud.svelte";

  let host: HTMLDivElement | undefined = $state();
  let offsetX = $state(0);
  let offsetY = $state(0);

  const variant = $derived(store.settings.character || "eyes");
  const playing = $derived(isPlaying());

  function onMove(clientX: number, clientY: number): void {
    if (!host || variant === "none" || variant === "hud") return;
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
    {:else if variant === "hud"}
      <PerfHud />
    {:else if variant === "pill"}
      <span class="pill">
        <span class="pupil"></span>
        <span class="pupil"></span>
      </span>
    {:else if variant === "cat"}
      <span class="cat">
        <span class="ear left"></span>
        <span class="ear right"></span>
        <span class="cat-eye"><span class="pupil"></span></span>
        <span class="cat-eye"><span class="pupil"></span></span>
        <span class="cat-tail"></span>
      </span>
    {:else if variant === "orb"}
      <span class="orb">
        <span class="orb-core"></span>
        <span class="orb-ring"></span>
        <span class="orb-moon"></span>
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

  .character[data-variant="hud"].playing {
    animation: none;
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
    transition: transform 0.2s cubic-bezier(0.22, 1, 0.36, 1);
    will-change: transform;
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

  .cat {
    position: relative;
    width: 24px;
    height: 15px;
    border-radius: 8px 8px 10px 10px;
    background: #151519;
    box-shadow:
      inset 0 0 0 1px rgba(255, 255, 255, 0.55),
      0 0 9px color-mix(in srgb, var(--accent) 40%, transparent);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
  }

  .ear {
    position: absolute;
    top: -4px;
    width: 6px;
    height: 7px;
    background: #151519;
    clip-path: polygon(50% 0, 100% 100%, 0 100%);
    filter: drop-shadow(0 -1px 0 rgba(255, 255, 255, 0.45));
  }

  .ear.left {
    left: 2px;
    transform: rotate(-10deg);
  }

  .ear.right {
    right: 2px;
    transform: rotate(10deg);
  }

  .cat-eye {
    width: 4px;
    height: 6px;
    border-radius: 50%;
    background: #f4f4f5;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    animation: blink 5.4s infinite;
  }

  .cat-eye .pupil {
    width: 2.2px;
    height: 3.4px;
    background: #101012;
  }

  .cat-tail {
    position: absolute;
    right: -8px;
    bottom: 0;
    width: 8px;
    height: 9px;
    border-right: 2px solid #f4f4f5;
    border-bottom: 2px solid #f4f4f5;
    border-radius: 0 0 8px 0;
    transform-origin: bottom left;
    animation: cat-tail 2.2s ease-in-out infinite;
  }

  @keyframes cat-tail {
    0%,
    100% {
      transform: rotate(-10deg);
    }
    50% {
      transform: rotate(14deg);
    }
  }

  .orb {
    position: relative;
    width: 22px;
    height: 22px;
    display: inline-grid;
    place-items: center;
  }

  .orb-core {
    width: 13px;
    height: 13px;
    border-radius: 50%;
    background: radial-gradient(circle at 32% 28%, #fff, var(--accent) 55%, #1e3a8a);
    box-shadow: 0 0 10px color-mix(in srgb, var(--accent) 75%, transparent);
    animation: orb-pulse 2.6s ease-in-out infinite;
  }

  .orb-ring {
    position: absolute;
    inset: 1px;
    border-radius: 50%;
    border: 1px solid color-mix(in srgb, var(--accent) 65%, transparent);
    transform: rotateX(64deg);
    animation: orb-spin 4.2s linear infinite;
  }

  .orb-moon {
    position: absolute;
    width: 3px;
    height: 3px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 0 5px #fff;
    animation: orb-orbit 3.6s linear infinite;
  }

  @keyframes orb-pulse {
    0%,
    100% {
      transform: scale(1);
      filter: brightness(1);
    }
    50% {
      transform: scale(1.08);
      filter: brightness(1.25);
    }
  }

  @keyframes orb-spin {
    to {
      transform: rotateX(64deg) rotate(360deg);
    }
  }

  @keyframes orb-orbit {
    from {
      transform: rotate(0deg) translateX(9px) rotate(0deg);
    }
    to {
      transform: rotate(360deg) translateX(9px) rotate(-360deg);
    }
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
  :global(:root[data-reduced-motion="true"]) .cat-eye,
  :global(:root[data-reduced-motion="true"]) .cat-tail,
  :global(:root[data-reduced-motion="true"]) .orb-core,
  :global(:root[data-reduced-motion="true"]) .orb-ring,
  :global(:root[data-reduced-motion="true"]) .orb-moon,
  :global(:root[data-reduced-motion="true"]) .character.playing {
    animation: none;
  }
</style>
