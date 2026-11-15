<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "./Icon.svelte";
  import { t } from "../store.svelte";
  import {
    ensureWeather,
    temperature,
    weather,
    weatherIcon,
    weatherKey,
  } from "../weather.svelte";

  interface Props {
    compact?: boolean;
  }

  let { compact = true }: Props = $props();

  onMount(() => {
    void ensureWeather();
  });

  const current = $derived(weather.data?.current ?? null);
  const icon = $derived(current ? weatherIcon(current.code, current.is_day) : "cloud");
  const label = $derived(current ? t(weatherKey(current.code)) : t("weather.no_data"));
</script>

<span class="weather" title={label} aria-label={label}>
  <Icon name={icon} size={compact ? 14 : 18} />
  <span class="temp">
    {#if current}
      {temperature(current.temperature)}
    {:else if weather.loading}
      …
    {:else}
      --
    {/if}
  </span>
</span>

<style>
  .weather {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .temp {
    font-size: inherit;
  }
</style>
