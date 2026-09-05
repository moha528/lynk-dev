import { invoke } from "@tauri-apps/api/core";

/**
 * Assistance par modèle — surface IPC et types.
 *
 * Partagée par les deux modules : le Git Manager rédige des messages de commit
 * et explique des diffs, le Dev Manager résume des logs. D'où sa place dans
 * `lib/` plutôt que dans l'un des deux.
 */

export type AiConfig = {
  /** Une clé est enregistrée. **Sa valeur ne sort jamais du backend.** */
  apiKeySet: boolean;
  model: string | null;
};

export type ModelInfo = {
  id: string;
  name: string;
  /** Dollars par million de jetons d'entrée. `null` si le tarif est absent. */
  promptPrice: number | null;
  completionPrice: number | null;
  contextLength: number | null;
  free: boolean;
};

export type Usage = {
  promptTokens: number;
  completionTokens: number;
  totalTokens: number;
};

export type Completion = {
  text: string;
  usage: Usage;
  model: string;
};

export const aiApi = {
  getConfig: () => invoke<AiConfig>("ai_config_get"),
  setConfig: (config: { apiKey?: string; model?: string }) =>
    invoke<void>("ai_config_set", { apiKey: config.apiKey, model: config.model }),
  /** `apiKey` permet d'éprouver une clé **avant** de l'enregistrer. */
  listModels: (apiKey?: string) => invoke<ModelInfo[]>("ai_list_models", { apiKey }),

  commitMessage: (repoPath: string) => invoke<Completion>("ai_commit_message", { repoPath }),
  explainDiff: (diff: string) => invoke<Completion>("ai_explain_diff", { diff }),
  summarizeLogs: (logs: string) => invoke<Completion>("ai_summarize_logs", { logs }),
};

/**
 * Tarif lisible : `gratuit`, `0,10 $/M` ou `—`.
 *
 * Le million de jetons est la seule échelle où ces nombres veulent dire quelque
 * chose : par jeton, tout vaut « 0,0000001 ».
 */
export function formatPrice(price: number | null): string {
  if (price === null) return "—";
  if (price === 0) return "gratuit";
  if (price < 0.01) return `${price.toFixed(4)} $/M`;
  return `${price.toFixed(2)} $/M`;
}

/** Coût d'un appel, quand les deux tarifs sont connus. */
export function estimateCost(usage: Usage, model: ModelInfo | undefined): number | null {
  if (!model || model.promptPrice === null || model.completionPrice === null) return null;
  return (
    (usage.promptTokens * model.promptPrice + usage.completionTokens * model.completionPrice) /
    1_000_000
  );
}
