import { invoke } from "@tauri-apps/api/core";

export interface BatteryStatus {
  present: boolean;
  percent: number | null;
  charging: boolean;
  ac: boolean;
}

export const battery = $state<{ data: BatteryStatus | null }>({ data: null });

let started = false;

export async function initBattery(): Promise<void> {
  if (started) return;
  started = true;
  const refresh = async (): Promise<void> => {
    try {
      const next = await invoke<BatteryStatus>("battery_status");
      battery.data = next.present ? next : null;
    } catch {
      battery.data = null;
    }
  };
  void refresh();
  setInterval(() => void refresh(), 60_000);
}
