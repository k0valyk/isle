import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface SystemStats {
  cpu: number;
  gpu: number | null;
  ram: number;
  ramUsedGb: number;
  ramTotalGb: number;
  disk: number;
  diskUsedGb: number;
  diskTotalGb: number;
  diskLoad: number | null;
  diskTemp: number | null;
  netDown: number;
  netUp: number;
  cpuTemp: number | null;
  gpuTemp: number | null;
  refreshHz: number | null;
  screenWidth: number;
  screenHeight: number;
  cpuName: string;
  gpuName: string;
  motherboard: string;
  ramModules: string;
  osName: string;
  fps: number | null;
  fpsProcess: string;
  fpsError: string;
  checkedAt: number;
}

export const system = $state<{ data: SystemStats | null }>({ data: null });

/**
 * The UI frame rate. PresentMon only reports in-game FPS for fullscreen games,
 * so the island keeps its own always-on rAF meter for the rest of the time.
 */
export const perf = $state<{ fps: number; sampledAt: number }>({
  fps: 0,
  sampledAt: 0,
});

export const history = $state<{
  cpu: number[];
  gpu: number[];
  ram: number[];
  netDown: number[];
  netUp: number[];
}>({ cpu: [], gpu: [], ram: [], netDown: [], netUp: [] });

function startFpsMeter(): void {
  if (typeof window === "undefined" || typeof requestAnimationFrame !== "function") return;
  let frames = 0;
  let last = performance.now();
  const tick = (now: number): void => {
    frames += 1;
    const span = now - last;
    if (span >= 500) {
      // A hidden or blocked window produces one huge span on resume; start over
      // instead of publishing a fake 1 FPS reading.
      if (span > 1500) {
        frames = 0;
        last = now;
        requestAnimationFrame(tick);
        return;
      }
      const raw = (frames * 1000) / span;
      // Half-second buckets jitter around 58/62 on a stable 60 Hz panel; a short
      // EMA keeps the HUD still without hiding real drops.
      perf.fps = Math.round(perf.fps > 0 ? perf.fps * 0.6 + raw * 0.4 : raw);
      perf.sampledAt = Date.now();
      frames = 0;
      last = now;
    }
    requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
}

/** Prefer a real in-game reading, then the island's own frame rate. */
export function effectiveFps(stats: SystemStats | null = system.data): number | null {
  if (stats?.fps != null) return stats.fps;
  if (perf.fps > 0) return perf.fps;
  if (stats?.refreshHz) return stats.refreshHz;
  return null;
}

export function fpsLabel(stats: SystemStats | null = system.data): string {
  if (stats?.fps != null) return stats.fpsProcess || "game";
  if (perf.fps > 0) return "Isle";
  return "";
}

function push(target: number[], value: number): void {
  target.push(value);
  if (target.length > 36) target.shift();
}

let started = false;

export async function initStats(): Promise<void> {
  if (started) return;
  started = true;
  startFpsMeter();
  try {
    system.data = await invoke<SystemStats>("system_stats");
  } catch {
    /* stats are optional */
  }
  await listen<SystemStats>("system-stats", (event) => {
    const stats = event.payload;
    system.data = stats;
    push(history.cpu, stats.cpu);
    push(history.gpu, stats.gpu ?? 0);
    push(history.ram, stats.ram);
    push(history.netDown, stats.netDown);
    push(history.netUp, stats.netUp);
  });
}
