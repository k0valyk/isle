<script lang="ts">
  import { store, locale } from "../store.svelte";
  import { clock, initClock } from "../clock.svelte";

  initClock();

  const time = $derived(
    clock.now.toLocaleTimeString(locale(), {
      hour: "2-digit",
      minute: "2-digit",
      second: store.settings.show_seconds ? "2-digit" : undefined,
      hour12: false,
    }),
  );
</script>

{#key time}
  <span class="clock" aria-label={time}>{time}</span>
{/key}

<style>
  .clock {
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.3px;
    white-space: nowrap;
    animation: clock-in 0.26s cubic-bezier(0.22, 1, 0.36, 1);
  }

  @keyframes clock-in {
    from {
      opacity: 0.55;
      transform: translateY(1.5px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  :global(:root[data-reduced-motion="true"]) .clock {
    animation: none;
  }
</style>
