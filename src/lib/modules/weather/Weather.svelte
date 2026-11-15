<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "../../components/Icon.svelte";
  import { store, t, updateSettings, formatDayName, formatTimeOfDay } from "../../store.svelte";
  import {
    ensureWeather,
    searchCity,
    temperature,
    weather,
    weatherIcon,
    weatherKey,
    type GeoCity,
  } from "../../weather.svelte";

  let query = $state("");
  let results = $state<GeoCity[]>([]);
  let searching = $state(false);
  let searchError = $state<string | null>(null);
  let searchTimer: ReturnType<typeof setTimeout> | undefined;

  const current = $derived(weather.data?.current ?? null);
  const hourly = $derived(weather.data?.hourly ?? []);
  const daily = $derived(weather.data?.daily ?? []);
  const today = $derived(daily[0] ?? null);
  const place = $derived(store.settings.weather_city || t("weather.city"));

  function search(): void {
    if (searchTimer) clearTimeout(searchTimer);
    if (query.trim().length < 2) {
      results = [];
      return;
    }
    searchTimer = setTimeout(async () => {
      searching = true;
      searchError = null;
      results = await searchCity(query.trim());
      if (!results.length) searchError = t("weather.no_data");
      searching = false;
    }, 260);
  }

  function choose(city: GeoCity): void {
    updateSettings({
      weather_city: city.name,
      weather_lat: city.latitude,
      weather_lon: city.longitude,
    });
    results = [];
    query = "";
    void ensureWeather(true);
  }

  function windDirection(degrees: number): string {
    const arrows = ["↓", "↙", "←", "↖", "↑", "↗", "→", "↘"];
    return arrows[Math.round(degrees / 45) % 8];
  }

  function uvLabel(value: number): string {
    if (value < 3) return "1";
    if (value < 6) return "2";
    if (value < 8) return "3";
    if (value < 11) return "4";
    return "5";
  }

  onMount(() => {
    void ensureWeather();
    return () => {
      if (searchTimer) clearTimeout(searchTimer);
    };
  });
</script>

<div class="weather">
  <div class="top">
    <section class="current card">
      <div class="city-row">
        <label class="city-search">
          <Icon name="search" size={13} />
          <input
            bind:value={query}
            oninput={search}
            placeholder={t("weather.search_city")}
            aria-label={t("weather.search_city")}
          />
        </label>
        <button class="btn icon-only" onclick={() => void ensureWeather(true)} aria-label={t("common.refresh")}>
          <Icon name="refresh" size={14} />
        </button>
      </div>

      {#if results.length}
        <div class="results">
          {#each results as city}
            <button class="result" onclick={() => choose(city)}>
              <span>{city.name}</span>
              <span class="muted">{city.country}{city.admin1 ? `, ${city.admin1}` : ""}</span>
            </button>
          {/each}
        </div>
      {:else if searching}
        <div class="searching">{t("common.loading")}</div>
      {:else if searchError}
        <div class="searching">{searchError}</div>
      {/if}

      {#if current}
        <div class="main-reading">
          <Icon name={weatherIcon(current.code, current.is_day)} size={40} />
          <span class="temp">{temperature(current.temperature)}</span>
          <div class="reading-meta">
            <span class="place">{place}</span>
            <span class="desc">{t(weatherKey(current.code))}</span>
            <span class="muted">{t("weather.feels_like")} {temperature(current.apparent)}</span>
          </div>
        </div>
        <div class="tiles">
          <div class="tile">
            <Icon name="wind" size={14} />
            <span class="tile-value">{Math.round(current.wind)} {windDirection(current.wind_direction)}</span>
            <span class="tile-label">{t("weather.wind")}</span>
          </div>
          <div class="tile">
            <Icon name="droplet" size={14} />
            <span class="tile-value">{Math.round(current.humidity)}%</span>
            <span class="tile-label">{t("weather.humidity")}</span>
          </div>
          <div class="tile">
            <Icon name="gauge" size={14} />
            <span class="tile-value">{Math.round(current.pressure)}</span>
            <span class="tile-label">{t("weather.pressure")}</span>
          </div>
          {#if today}
            <div class="tile">
              <Icon name="sun" size={14} />
              <span class="tile-value">{formatTimeOfDay(today.sunrise)}</span>
              <span class="tile-label">{t("weather.sunrise")}</span>
            </div>
            <div class="tile">
              <Icon name="moon" size={14} />
              <span class="tile-value">{formatTimeOfDay(today.sunset)}</span>
              <span class="tile-label">{t("weather.sunset")}</span>
            </div>
            <div class="tile">
              <Icon name="sparkles" size={14} />
              <span class="tile-value">{uvLabel(today.uv)}</span>
              <span class="tile-label">{t("weather.uv")}</span>
            </div>
          {/if}
        </div>
      {:else if weather.loading}
        <div class="state">{t("common.loading")}</div>
      {:else if weather.error}
        <div class="state">
          <span>{t("weather.error")}</span>
          <button class="btn" onclick={() => void ensureWeather(true)}>{t("common.retry")}</button>
        </div>
      {:else}
        <div class="state">{t("weather.no_data")}</div>
      {/if}
    </section>

    <section class="daily card">
      <div class="section-label">{t("weather.daily")}</div>
      <div class="daily-list">
        {#each daily as day, index}
          <div class="day">
            <span class="day-name">{formatDayName(day.date, index)}</span>
            <Icon name={weatherIcon(day.code, true)} size={15} />
            <span class="day-temp">
              <span>{temperature(day.tmax)}</span>
              <span class="muted">{temperature(day.tmin)}</span>
            </span>
          </div>
        {/each}
        {#if !daily.length}
          <div class="state small">{t("weather.no_data")}</div>
        {/if}
      </div>
    </section>
  </div>

  <section class="hourly card">
    <div class="section-label">{t("weather.hourly")}</div>
    <div class="hourly-row">
      {#each hourly.slice(0, 24) as hour}
        <div class="hour">
          <span class="hour-time">{formatTimeOfDay(hour.time)}</span>
          <Icon name={weatherIcon(hour.code, true)} size={16} />
          <span class="hour-temp">{temperature(hour.temperature)}</span>
          <span class="hour-rain">{hour.precipitation_probability}%</span>
        </div>
      {/each}
    </div>
  </section>

  {#if weather.data}
    <div class="updated">
      {t("weather.updated")} · {new Date(weather.updated).toLocaleTimeString(undefined, {
        hour: "2-digit",
        minute: "2-digit",
      })}
    </div>
  {/if}
</div>

<style>
  .weather {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-height: 0;
    position: relative;
  }

  .top {
    display: grid;
    grid-template-columns: 1.4fr 1fr;
    gap: 8px;
    min-height: 0;
    flex: 1;
  }

  .card {
    padding: 10px 12px;
    border-radius: 14px;
    background: var(--surface);
    border: 1px solid var(--border);
    min-height: 0;
  }

  .current {
    display: flex;
    flex-direction: column;
    gap: 8px;
    overflow-y: auto;
  }

  .city-row {
    display: flex;
    align-items: center;
    gap: 4px;
    flex: none;
  }

  .city-search {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 0 7px;
    border-radius: 8px;
    background: var(--input);
    border: 1px solid var(--border);
    color: var(--muted);
  }

  .city-search input {
    flex: 1;
    border: none;
    background: transparent;
    padding: 5px 0;
    font-size: 11px;
    outline: none;
  }

  .results {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border);
    border-radius: 9px;
    overflow: hidden;
    flex: none;
  }

  .result {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    border: none;
    background: var(--bg-elevated);
    color: var(--fg);
    padding: 5px 8px;
    font-size: 11px;
    text-align: left;
  }

  .result:hover {
    background: var(--surface-strong);
  }

  .searching {
    font-size: 10.5px;
    color: var(--muted);
    flex: none;
  }

  .main-reading {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .temp {
    font-size: 40px;
    font-weight: 650;
    line-height: 1;
    font-variant-numeric: tabular-nums;
    letter-spacing: -1.5px;
  }

  .reading-meta {
    display: flex;
    flex-direction: column;
    gap: 1px;
    font-size: 11px;
    min-width: 0;
  }

  .place {
    font-weight: 600;
    font-size: 12px;
  }

  .desc {
    color: var(--muted);
  }

  .tiles {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 4px;
  }

  .tile {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1px;
    padding: 5px 4px;
    border-radius: 8px;
    background: var(--surface);
    border: 1px solid var(--border);
    color: var(--muted);
  }

  .tile-value {
    font-size: 11px;
    color: var(--fg);
    font-variant-numeric: tabular-nums;
  }

  .tile-label {
    font-size: 8.5px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    text-align: center;
  }

  .daily {
    display: flex;
    flex-direction: column;
    gap: 6px;
    overflow: hidden;
  }

  .section-label {
    font-size: 9.5px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--muted);
    flex: none;
  }

  .daily-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    overflow-y: auto;
    min-height: 0;
  }

  .day {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 3px 4px;
    border-radius: 7px;
    font-size: 11px;
  }

  .day:hover {
    background: var(--surface);
  }

  .day-name {
    flex: 1;
    color: var(--muted);
    text-transform: capitalize;
  }

  .day-temp {
    display: flex;
    gap: 6px;
    font-variant-numeric: tabular-nums;
  }

  .hourly {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .hourly-row {
    display: flex;
    gap: 4px;
    overflow-x: auto;
    padding-bottom: 2px;
  }

  .hour {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    min-width: 42px;
    padding: 4px 3px;
    border-radius: 8px;
    background: var(--surface);
    font-size: 10px;
    font-variant-numeric: tabular-nums;
  }

  .hour-time {
    color: var(--muted);
  }

  .hour-rain {
    font-size: 9px;
    color: var(--muted);
  }

  .state {
    margin: auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    color: var(--muted);
    font-size: 11px;
    padding: 16px;
    text-align: center;
  }

  .state.small {
    padding: 6px;
  }

  .updated {
    font-size: 9.5px;
    color: var(--muted);
    text-align: right;
    flex: none;
  }
</style>
