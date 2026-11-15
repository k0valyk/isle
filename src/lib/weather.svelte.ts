import { invoke } from "@tauri-apps/api/core";
import { store } from "./store.svelte";

export interface CurrentWeather {
  time: string;
  temperature: number;
  apparent: number;
  humidity: number;
  wind: number;
  wind_direction: number;
  code: number;
  is_day: boolean;
  precipitation: number;
  pressure: number;
}

export interface HourlyWeather {
  time: string;
  temperature: number;
  code: number;
  precipitation_probability: number;
}

export interface DailyWeather {
  date: string;
  code: number;
  tmax: number;
  tmin: number;
  sunrise: string;
  sunset: string;
  uv: number;
  precipitation_probability: number;
}

interface WeatherPayload {
  current: CurrentWeather;
  hourly: HourlyWeather[];
  daily: DailyWeather[];
  timezone: string;
}

interface WeatherState {
  data: WeatherPayload | null;
  loading: boolean;
  error: string | null;
  updated: number;
}

export const weather = $state<WeatherState>({
  data: null,
  loading: false,
  error: null,
  updated: 0,
});

const REFRESH_MS = 15 * 60 * 1000;
let inflight: Promise<void> | null = null;

export async function ensureWeather(force = false): Promise<void> {
  if (inflight) return inflight;
  const fresh = Date.now() - weather.updated < REFRESH_MS;
  if (!force && fresh && weather.data) return;
  inflight = fetchWeather();
  try {
    await inflight;
  } finally {
    inflight = null;
  }
}

async function fetchWeather(): Promise<void> {
  weather.loading = true;
  weather.error = null;
  try {
    const lat = store.settings.weather_lat ?? 50.4501;
    const lon = store.settings.weather_lon ?? 30.5234;
    const units = store.settings.weather_units || "c";
    const data = await invoke<WeatherPayload>("fetch_weather", {
      lat,
      lon,
      units,
    });
    weather.data = data;
    weather.updated = Date.now();
  } catch (error) {
    weather.error = String(error);
  } finally {
    weather.loading = false;
  }
}

export interface GeoCity {
  name: string;
  country: string;
  admin1?: string;
  latitude: number;
  longitude: number;
}

export async function searchCity(query: string): Promise<GeoCity[]> {
  if (query.trim().length < 2) return [];
  try {
    return await invoke<GeoCity[]>("geocode_city", { query });
  } catch (error) {
    console.warn("Geocoding failed", error);
    return [];
  }
}

export function weatherIcon(code: number, isDay = true): string {
  if (code === 0) return isDay ? "sun" : "moon";
  if (code === 1 || code === 2) return isDay ? "cloud-sun" : "cloud-moon";
  if (code === 3) return "cloud";
  if (code === 45 || code === 48) return "fog";
  if (code >= 51 && code <= 57) return "cloud-drizzle";
  if (code >= 61 && code <= 67) return "cloud-rain";
  if (code >= 71 && code <= 77) return "cloud-snow";
  if (code >= 80 && code <= 82) return "cloud-rain";
  if (code === 85 || code === 86) return "cloud-snow";
  if (code >= 95) return "cloud-lightning";
  return "cloud";
}

export function weatherKey(code: number): string {
  if (code === 0) return "weather.clear";
  if (code === 1 || code === 2) return "weather.partly_cloudy";
  if (code === 3) return "weather.cloudy";
  if (code === 45 || code === 48) return "weather.fog";
  if (code >= 51 && code <= 57) return "weather.drizzle";
  if (code >= 61 && code <= 67) return "weather.rain";
  if (code >= 71 && code <= 77) return "weather.snow";
  if (code >= 80 && code <= 82) return "weather.showers";
  if (code === 85 || code === 86) return "weather.snow";
  if (code >= 95) return "weather.thunder";
  return "weather.unknown";
}

export function temperature(value: number): string {
  return `${Math.round(value)}°`;
}
