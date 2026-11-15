<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import Icon from "./Icon.svelte";

  interface BatteryStatus {
    present: boolean;
    percent: number | null;
    charging: boolean;
    ac: boolean;
  }

  let status = $state<BatteryStatus | null>(null);

  async function refresh(): Promise<void> {
    try {
      const next = await invoke<BatteryStatus>("battery_status");
      status = next.present ? next : null;
    } catch {
      status = null;
    }
  }

  onMount(() => {
    void refresh();
    const timer = setInterval(refresh, 60_000);
    return () => clearInterval(timer);
  });
</script>

{#if status?.percent !== null && status?.percent !== undefined}
  <span class="battery" title={`${status.percent}%`} aria-label={`${status.percent}%`}>
    <Icon name="battery" size={14} />
    <span>{status.percent}%</span>
    {#if status.charging}<span class="bolt">⚡</span>{/if}
  </span>
{/if}

<style>
  .battery {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    font-variant-numeric: tabular-nums;
    font-size: 11px;
    color: var(--muted);
  }

  .bolt {
    font-size: 9px;
  }
</style>
