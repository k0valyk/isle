import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface MediaSnapshot {
  active: boolean;
  title: string;
  artist: string;
  album: string;
  status: "playing" | "paused" | "stopped" | "changing" | "closed";
  position: number;
  duration: number;
  artwork: string | null;
  sourceApp: string | null;
}

export interface LyricLine {
  time: number;
  text: string;
}

export type LyricsState = "idle" | "loading" | "ready" | "none" | "error";

export const audio = $state<{ bands: number[]; level: number }>({
  bands: [],
  level: 0,
});

let lastAudioAt = 0;

interface MediaStore {
  current: MediaSnapshot;
  position: number;
  lyrics: LyricLine[];
  lyricsPlain: string;
  lyricsState: LyricsState;
  instrumental: boolean;
  accent: string | null;
}

const EMPTY: MediaSnapshot = {
  active: false,
  title: "",
  artist: "",
  album: "",
  status: "closed",
  position: 0,
  duration: 0,
  artwork: null,
  sourceApp: null,
};

export const media = $state<MediaStore>({
  current: { ...EMPTY },
  position: 0,
  lyrics: [],
  lyricsPlain: "",
  lyricsState: "idle",
  instrumental: false,
  accent: null,
});

let started = false;
let basePosition = 0;
let baseTime = 0;
let lyricsKey = "";
let lyricsRequest = 0;
let lyricsTimer: ReturnType<typeof setTimeout> | undefined;
let coverKey = "";
let coverRequest = 0;
let accentRequest = 0;
const coverCache = new Map<string, string>();

export function trackKey(snapshot: MediaSnapshot = media.current): string {
  return `${snapshot.title}|${snapshot.artist}|${snapshot.album}`;
}

export function isPlaying(): boolean {
  return media.current.active && media.current.status === "playing";
}

function parseSynced(source: string): LyricLine[] {
  const lines: LyricLine[] = [];
  for (const raw of source.split(/\r?\n/)) {
    const tags = [...raw.matchAll(/\[(\d{1,2}):(\d{2})(?:[.:](\d{1,3}))?\]/g)];
    if (!tags.length) continue;
    const text = raw.replace(/\[[^\]]*\]/g, "").trim();
    if (!text) continue;
    for (const tag of tags) {
      const minutes = Number(tag[1]);
      const seconds = Number(tag[2]);
      const fraction = tag[3] ? Number(tag[3].padEnd(3, "0")) / 1000 : 0;
      lines.push({ time: minutes * 60 + seconds + fraction, text });
    }
  }
  lines.sort((a, b) => a.time - b.time);
  return lines;
}

export function activeLyricIndex(position: number): number {
  if (!media.lyrics.length) return -1;
  let low = 0;
  let high = media.lyrics.length - 1;
  let found = -1;
  while (low <= high) {
    const middle = (low + high) >> 1;
    if (media.lyrics[middle].time <= position + 0.15) {
      found = middle;
      low = middle + 1;
    } else {
      high = middle - 1;
    }
  }
  return found;
}

export function currentLyricText(): string {
  if (!media.lyrics.length) return "";
  const index = activeLyricIndex(media.position);
  if (index >= 0) return media.lyrics[index].text;
  return media.lyrics[0]?.text ?? "";
}

async function ensureLyrics(snapshot: MediaSnapshot): Promise<void> {
  if (!snapshot.active || !snapshot.title.trim()) {
    lyricsKey = "";
    media.lyrics = [];
    media.lyricsPlain = "";
    media.lyricsState = snapshot.active ? "none" : "idle";
    media.instrumental = false;
    return;
  }

  const key = `${snapshot.title}|${snapshot.artist}|${snapshot.album}|${Math.round(snapshot.duration)}`;
  if (key === lyricsKey) return;
  lyricsKey = key;
  const request = ++lyricsRequest;

  if (lyricsTimer) clearTimeout(lyricsTimer);
  media.lyricsState = "loading";
  media.lyrics = [];
  media.lyricsPlain = "";

  lyricsTimer = setTimeout(async () => {
    try {
      const result = await invoke<{
        synced: string | null;
        plain: string | null;
        instrumental: boolean;
      }>("fetch_lyrics", {
        title: snapshot.title,
        artist: snapshot.artist,
        album: snapshot.album || null,
        duration: snapshot.duration || null,
      });
      if (request !== lyricsRequest) return;
      media.instrumental = result.instrumental;
      media.lyrics = result.synced ? parseSynced(result.synced) : [];
      media.lyricsPlain = result.plain ?? "";
      media.lyricsState =
        media.lyrics.length || media.lyricsPlain || result.instrumental ? "ready" : "none";
    } catch {
      if (request !== lyricsRequest) return;
      media.lyrics = [];
      media.lyricsPlain = "";
      media.lyricsState = "error";
    }
  }, 350);
}

async function updateAccent(artwork: string | null): Promise<void> {
  const request = ++accentRequest;
  if (!artwork) {
    if (request === accentRequest) media.accent = null;
    return;
  }
  try {
    const image = new Image();
    image.decoding = "async";
    image.src = artwork;
    await image.decode();
    if (request !== accentRequest) return;
    const canvas = document.createElement("canvas");
    canvas.width = 20;
    canvas.height = 20;
    const context = canvas.getContext("2d", { willReadFrequently: true });
    if (!context) return;
    context.drawImage(image, 0, 0, 20, 20);
    const pixels = context.getImageData(0, 0, 20, 20).data;
    let r = 0;
    let g = 0;
    let b = 0;
    let weight = 0;
    for (let i = 0; i < pixels.length; i += 4) {
      const red = pixels[i];
      const green = pixels[i + 1];
      const blue = pixels[i + 2];
      const max = Math.max(red, green, blue);
      const min = Math.min(red, green, blue);
      const saturation = (max - min) / 255;
      const brightness = max / 255;
      const w = 0.05 + saturation * brightness;
      r += red * w;
      g += green * w;
      b += blue * w;
      weight += w;
    }
    if (weight <= 0) return;
    r /= weight;
    g /= weight;
    b /= weight;
    const top = Math.max(r, g, b) || 1;
    const boost = 225 / top;
    const clamp = (value: number) => Math.max(0, Math.min(255, Math.round(value)));
    media.accent = `rgb(${clamp(r * boost)}, ${clamp(g * boost)}, ${clamp(b * boost)})`;
  } catch {
    /* artwork may be an unsupported format; the theme accent stays */
  }
}

async function ensureCover(snapshot: MediaSnapshot): Promise<void> {
  if (!snapshot.active || !snapshot.title.trim() || snapshot.artwork) return;
  const key = trackKey(snapshot);
  if (key === coverKey) return;
  coverKey = key;
  const request = ++coverRequest;
  const cached = coverCache.get(key);
  if (cached) {
    if (trackKey(media.current) === key) {
      media.current = { ...media.current, artwork: cached };
      void updateAccent(cached);
    }
    return;
  }
  try {
    const cover = await invoke<string | null>("fetch_cover", {
      title: snapshot.title,
      artist: snapshot.artist,
      album: snapshot.album || null,
    });
    if (request !== coverRequest || !cover) return;
    coverCache.set(key, cover);
    if (coverCache.size > 40) {
      const oldest = coverCache.keys().next().value;
      if (oldest) coverCache.delete(oldest);
    }
    if (trackKey(media.current) === key && !media.current.artwork) {
      media.current = { ...media.current, artwork: cover };
      void updateAccent(cover);
    }
  } catch {
    /* cover lookup is optional */
  }
}

function apply(snapshot: MediaSnapshot): void {
  const sameTrack =
    media.current.title === snapshot.title &&
    media.current.artist === snapshot.artist &&
    media.current.album === snapshot.album;
  media.current = {
    ...snapshot,
    artwork: snapshot.artwork ?? (sameTrack ? media.current.artwork : null),
  };
  basePosition = snapshot.position;
  baseTime = performance.now();
  media.position = snapshot.position;
  if (snapshot.artwork) {
    void updateAccent(snapshot.artwork);
  } else if (!sameTrack) {
    media.accent = null;
  }
  void ensureLyrics(snapshot);
  void ensureCover(snapshot);
}

export async function initMedia(): Promise<void> {
  if (started) return;
  started = true;
  await listen<MediaSnapshot>("media-update", (event) => apply(event.payload));
  await listen<{ bands: number[]; level: number }>("audio-spectrum", (event) => {
    audio.bands = event.payload.bands;
    audio.level = event.payload.level;
    lastAudioAt = performance.now();
  });
  // If the frontend reloads while a track is already playing, the incremental
  // media-update stream may have already sent its artwork. Seed from the
  // backend snapshot so the cover and lyrics appear immediately.
  try {
    const snapshot = await invoke<MediaSnapshot>("media_current");
    if (snapshot.active) apply(snapshot);
  } catch {
    /* the media service may still be starting */
  }
  setInterval(() => {
    if (performance.now() - lastAudioAt > 1200 && audio.bands.some((value) => value > 0.01)) {
      audio.bands = audio.bands.map((value) => value * 0.85);
      audio.level *= 0.85;
    }
    if (!isPlaying()) return;
    const next = basePosition + (performance.now() - baseTime) / 1000;
    media.position = media.current.duration
      ? Math.min(media.current.duration, next)
      : next;
  }, 250);
}

export async function mediaControl(
  action: "play_pause" | "next" | "previous" | "seek",
  seconds?: number,
): Promise<void> {
  try {
    await invoke("media_control", { action, seconds: seconds ?? null });
  } catch (error) {
    console.warn("Media command failed", error);
  }
}

export const volume = $state<{ level: number; muted: boolean; ready: boolean }>({
  level: 0.6,
  muted: false,
  ready: false,
});

interface VolumeSnapshot {
  level: number;
  muted: boolean;
}

function applyVolume(snapshot: VolumeSnapshot | null): void {
  if (!snapshot) return;
  volume.level = Math.max(0, Math.min(1, snapshot.level));
  volume.muted = snapshot.muted;
  volume.ready = true;
}

export async function refreshVolume(): Promise<void> {
  try {
    applyVolume(await invoke<VolumeSnapshot>("system_volume"));
  } catch {
    /* the audio service is optional */
  }
}

export async function setVolume(level: number): Promise<void> {
  const next = Math.max(0, Math.min(1, level));
  const wasMuted = volume.muted;
  volume.level = next;
  volume.muted = next === 0 ? volume.muted : false;
  try {
    const snapshot = await invoke<VolumeSnapshot>("set_system_volume", { level: next });
    applyVolume(snapshot);
    // Raising the slider on a muted output must actually unmute it, otherwise
    // the level looks restored but nothing is audible.
    if (wasMuted && next > 0) {
      applyVolume(await invoke<VolumeSnapshot>("set_system_mute", { muted: false }));
    }
  } catch {
    await refreshVolume();
  }
}

export async function toggleMute(): Promise<void> {
  const next = !volume.muted;
  volume.muted = next;
  try {
    applyVolume(await invoke<VolumeSnapshot>("set_system_mute", { muted: next }));
  } catch {
    await refreshVolume();
  }
}

export function formatTrackTime(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) return "0:00";
  const total = Math.floor(seconds);
  const minutes = Math.floor(total / 60);
  const rest = total % 60;
  return `${minutes}:${rest.toString().padStart(2, "0")}`;
}

export function resetLyrics(): void {
  lyricsKey = "";
  media.lyrics = [];
  media.lyricsPlain = "";
  media.lyricsState = "idle";
}
