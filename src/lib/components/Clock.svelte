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

<span class="clock" aria-label={time}>{time}</span>

<style>
  .clock {
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.3px;
    white-space: nowrap;
  }
</style>
