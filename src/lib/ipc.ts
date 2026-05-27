import { invoke } from "@tauri-apps/api/core";

/**
 * Typed wrappers around the backend Tauri commands.
 *
 * Template baseline: only the generic vault (PIN) commands are wired. Each
 * Lynk Dev module (Git / Dev / DB) adds its own `*Api` object here.
 */
export const vaultApi = {
  hasPin: () => invoke<boolean>("vault_has_pin"),
  verify: (pin: string) => invoke<boolean>("vault_verify_pin", { pin }),
  setPin: (newPin: string) => invoke<void>("vault_set_pin", { newPin }),
  changePin: (currentPin: string, newPin: string) =>
    invoke<void>("vault_change_pin", { currentPin, newPin }),
  disablePin: (currentPin: string) => invoke<void>("vault_disable_pin", { currentPin }),
};
