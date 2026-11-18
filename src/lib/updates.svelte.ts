import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export type UpdatePhase =
  | "idle"
  | "checking"
  | "current"
  | "available"
  | "downloading"
  | "ready"
  | "error";

/**
 * In-app updates through GitHub Releases. The updater downloads the signed
 * NSIS package, installs it and restarts Isle, so the user never has to visit
 * the repository or reinstall by hand.
 */
export const updates = $state<{
  phase: UpdatePhase;
  version: string;
  notes: string;
  progress: number;
  error: string;
}>({
  phase: "idle",
  version: "",
  notes: "",
  progress: 0,
  error: "",
});

let pending: Update | null = null;

export async function checkForUpdates(): Promise<void> {
  if (updates.phase === "checking" || updates.phase === "downloading") return;
  updates.phase = "checking";
  updates.error = "";
  try {
    const found = await check();
    if (!found) {
      pending = null;
      updates.version = "";
      updates.notes = "";
      updates.phase = "current";
      return;
    }
    pending = found;
    updates.version = found.version;
    updates.notes = found.body ?? "";
    updates.phase = "available";
  } catch (error) {
    updates.phase = "error";
    updates.error = String(error);
  }
}

export async function installUpdate(): Promise<void> {
  const update = pending;
  if (!update) {
    await checkForUpdates();
    return;
  }
  updates.phase = "downloading";
  updates.progress = 0;
  let total = 0;
  let downloaded = 0;
  try {
    await update.downloadAndInstall((event) => {
      if (event.event === "Started") {
        total = event.data.contentLength ?? 0;
      } else if (event.event === "Progress") {
        downloaded += event.data.chunkLength;
        updates.progress = total > 0 ? Math.min(1, downloaded / total) : 0;
      } else if (event.event === "Finished") {
        updates.progress = 1;
      }
    });
    updates.phase = "ready";
    await relaunch();
  } catch (error) {
    updates.phase = "error";
    updates.error = String(error);
  }
}
