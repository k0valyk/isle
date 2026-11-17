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

export const history = $state<{
  cpu: number[];
  gpu: number[];
  ram: number[];
  netDown: number[];
  netUp: number[];
}>({ cpu: [], gpu: [], ram: [], netDown: [], netUp: [] });

function push(target: number[], value: number): void {
  target.push(value);
  if (target.length > 36) target.shift();
}

let started = false;

export async function initStats(): Promise<void> {
  if (started) return;
  started = true;
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
