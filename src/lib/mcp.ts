import { invoke } from "@tauri-apps/api/core";
import { type UnlistenFn, listen } from "@tauri-apps/api/event";

/**
 * Serveur MCP — surface IPC et types.
 *
 * Le serveur vit dans le backend et pilote le **même** superviseur que l'écran
 * du Dev Manager. Ici on ne fait que l'allumer, l'éteindre et regarder ce qu'il
 * a fait.
 */

export type McpStatus = {
  /** Ce que l'utilisateur a demandé. */
  enabled: boolean;
  /** Ce qui est vrai — les deux divergent quand le port est déjà pris. */
  running: boolean;
  port: number;
  url: string;
  /** Sans trousseau, pas de jeton, donc pas de serveur. */
  keychainError: string | null;
};

export type McpCall = {
  /** Millisecondes depuis l'époque Unix. */
  at: number;
  tool: string;
  target: string | null;
  ok: boolean;
  detail: string;
  durationMs: number;
};

export const mcpApi = {
  status: () => invoke<McpStatus>("mcp_status"),
  setEnabled: (enabled: boolean) => invoke<McpStatus>("mcp_set_enabled", { enabled }),
  setPort: (port: number) => invoke<McpStatus>("mcp_set_port", { port }),
  /** Le jeton est **relisible** : il est fait pour être collé dans un client. */
  token: () => invoke<string | null>("mcp_token"),
  regenerateToken: () => invoke<string>("mcp_regenerate_token"),
  calls: () => invoke<McpCall[]>("mcp_calls"),
  clearCalls: () => invoke<void>("mcp_clear_calls"),
};

/** Chaque appel d'outil arrive en direct — le journal se remplit sous les yeux. */
export function onMcpCall(handler: (call: McpCall) => void): Promise<UnlistenFn> {
  return listen<McpCall>("mcp:call", (event) => handler(event.payload));
}

/**
 * La configuration à coller côté client IA.
 *
 * Le jeton y figure en clair : c'est le but du bouton « copier ». ⚠️ Le fichier
 * de configuration qui la reçoit devient donc lui aussi un porteur de secret —
 * d'où la possibilité de régénérer.
 */
export function clientConfig(url: string, token: string): string {
  return JSON.stringify(
    {
      mcpServers: {
        "lynk-dev": {
          type: "http",
          url,
          headers: { Authorization: `Bearer ${token}` },
        },
      },
    },
    null,
    2,
  );
}

/** `52 ms`, `1,2 s` — la durée d'un appel, dans l'unité qui se lit. */
export function formatDuration(ms: number): string {
  if (ms < 1_000) return `${ms} ms`;
  return `${(ms / 1_000).toFixed(1).replace(".", ",")} s`;
}
