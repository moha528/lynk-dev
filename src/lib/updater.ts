import { relaunch } from "@tauri-apps/plugin-process";
import { type Update, check } from "@tauri-apps/plugin-updater";

/**
 * Auto-update integration (P5-T06).
 *
 * Strategy (chosen with the user) : **silent check + toast**. On startup we
 * quietly ask GitHub Releases whether a newer version exists ; if so, the
 * caller surfaces a non-intrusive toast with an "Install" action. The user
 * is never interrupted — they update on their own terms, or from the About
 * panel's manual "Check for updates" button.
 *
 * All functions degrade gracefully when the updater isn't wired (dev mode,
 * missing pubkey, offline) : they resolve to `null` / throw a readable error
 * rather than crashing the app.
 */

export type UpdateInfo = {
  version: string;
  notes: string | null;
  /** Opaque handle used by {@link installUpdate}. */
  handle: Update;
};

/**
 * Returns the available update, or `null` when the app is up to date.
 * In dev (no bundle) or when the endpoint is unreachable, this throws — the
 * caller decides whether to surface that (manual check) or swallow it
 * (silent startup check).
 */
export async function checkForUpdate(): Promise<UpdateInfo | null> {
  const update = await check();
  if (!update) return null;
  return {
    version: update.version,
    notes: update.body ?? null,
    handle: update,
  };
}

/**
 * Download + install the update, reporting progress via `onProgress`
 * (0..1). On success, relaunches the app so the new version takes over.
 */
export async function installUpdate(
  info: UpdateInfo,
  onProgress?: (fraction: number) => void,
): Promise<void> {
  let downloaded = 0;
  let total = 0;
  await info.handle.downloadAndInstall((event) => {
    switch (event.event) {
      case "Started":
        total = event.data.contentLength ?? 0;
        break;
      case "Progress":
        downloaded += event.data.chunkLength;
        if (total > 0 && onProgress) onProgress(downloaded / total);
        break;
      case "Finished":
        if (onProgress) onProgress(1);
        break;
    }
  });
  // The new binary is in place — relaunch to run it.
  await relaunch();
}
