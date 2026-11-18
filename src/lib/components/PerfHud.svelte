<script lang="ts">
  import AnimatedNumber from "./AnimatedNumber.svelte";
  import { effectiveFps, system } from "../stats.svelte";

  interface Props {
    compact?: boolean;
  }

  let { compact = true }: Props = $props();

  const stats = $derived(system.data);
  const fps = $derived(effectiveFps(stats));
  const cpu = $derived(stats ? Math.round(stats.cpu) : null);
  const temp = $derived(stats?.cpuTemp != null ? Math.round(stats.cpuTemp) : null);
  const hot = $derived((stats?.cpu ?? 0) > 80 || (stats?.cpuTemp ?? 0) > 85);
</script>

<div class="perf-hud" class:compact class:hot aria-hidden="true">
  <span class="cell fps">
    <span class="value">
      {#if fps === null}
        —
      {:else}
        <AnimatedNumber value={fps} />
      {/if}
    </span>
    <span class="label">FPS</span>
  </span>
  <span class="cell">
    <span class="value">
      {#if cpu === null}
        —
      {:else}
        <AnimatedNumber value={cpu} suffix="%" />
      {/if}
    </span>
    <span class="label">CPU</span>
  </span>
  {#if temp !== null}
    <span class="cell temp">
      <span class="value"><AnimatedNumber value={temp} suffix="°" /></span>
      <span class="label">TEMP</span>
    </span>
  {/if}
  <span class="pulse"></span>
</div>

<style>
  .perf-hud {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 20px;
    padding: 0 6px;
    border: 1px solid color-mix(in srgb, var(--live-accent, var(--accent)) 24%, var(--border));
    border-radius: 999px;
    background: color-mix(in srgb, var(--live-accent, var(--accent)) 8%, var(--surface));
    color: var(--fg);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    flex: none;
  }

  .cell {
    display: inline-flex;
    align-items: baseline;
    gap: 2px;
    line-height: 1;
  }

  .value {
    font-size: 10.5px;
    font-weight: 700;
    letter-spacing: -0.02em;
    color: var(--fg);
  }

  .fps .value {
    color: var(--live-accent, var(--accent));
  }

  .label {
    font-size: 6.5px;
    font-weight: 650;
    letter-spacing: 0.05em;
    color: var(--muted);
  }

  .temp .value {
    color: color-mix(in srgb, var(--fg) 82%, var(--warn));
  }

  .hot .temp .value {
    color: var(--warn);
  }

  .pulse {
    width: 4px;
    height: 4px;
    border-radius: 50%;
    background: var(--live-accent, var(--accent));
    box-shadow: 0 0 6px var(--live-accent, var(--accent));
    opacity: 0.85;
    animation: perf-pulse 2.4s ease-in-out infinite;
  }

  .compact {
    gap: 6px;
    padding: 0 5px;
  }

  @keyframes perf-pulse {
    0%,
    100% {
      opacity: 0.35;
      transform: scale(0.85);
    }
    50% {
      opacity: 1;
      transform: scale(1.1);
    }
  }

  :global(:root[data-reduced-motion="true"]) .pulse {
    animation: none;
  }
</style>
