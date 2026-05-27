import { create } from "zustand";

import { vaultApi } from "@/lib/ipc";

/**
 * Vault state: PIN presence + lock status.
 *
 * `locked` is the authoritative gate for the UI. While true the
 * `<UnlockOverlay />` covers everything and blocks input.
 */
type VaultState = {
  /** Has the user configured a PIN? */
  hasMaster: boolean;
  /** Currently locked — render the unlock overlay only. */
  locked: boolean;
  /** Hydrate `hasMaster` + initial lock state from the backend. */
  hydrate: () => Promise<void>;
  /** Verify PIN + unlock if correct. */
  unlock: (pin: string) => Promise<boolean>;
  /** Force lock now. */
  lock: () => Promise<void>;
  /** Re-read whether a PIN is configured (after a Settings change). */
  refresh: () => Promise<void>;
};

export const useVaultStore = create<VaultState>((set, get) => ({
  hasMaster: false,
  locked: false,

  async hydrate() {
    try {
      const hasMaster = await vaultApi.hasPin();
      // If a PIN is set, the app starts locked.
      set({ hasMaster, locked: hasMaster });
    } catch (e) {
      console.warn("vault hydrate:", e);
    }
  },

  async unlock(pin) {
    const ok = await vaultApi.verify(pin);
    if (ok) set({ locked: false });
    return ok;
  },

  async lock() {
    if (!get().hasMaster || get().locked) return;
    set({ locked: true });
  },

  async refresh() {
    const hasMaster = await vaultApi.hasPin();
    set({ hasMaster });
  },
}));
