import { Check, Loader2, Search } from "lucide-react";
import { useEffect, useMemo, useState } from "react";

import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { type AiConfig, type ModelInfo, aiApi, formatPrice } from "@/lib/ai";
import { formatError, toastError, toastSuccess } from "@/lib/feedback";
import { cn } from "@/lib/utils";

/**
 * Réglages de l'assistance par modèle.
 *
 * Deux décisions visibles à l'écran :
 *
 * - **La clé ne se relit pas.** On sait qu'elle est enregistrée, on peut la
 *   remplacer ou l'effacer, jamais la voir. Un champ qui réaffiche un secret
 *   finit sur une capture d'écran.
 * - **Le catalogue est chargé en direct**, trié du moins cher au plus cher, avec
 *   le tarif affiché. Figer « le modèle pas cher du moment » dans le code, c'est
 *   garantir qu'il sera périmé dans trois mois.
 */
export function AiSection() {
  const [config, setConfig] = useState<AiConfig | null>(null);
  const [apiKey, setApiKey] = useState("");
  const [models, setModels] = useState<ModelInfo[] | null>(null);
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    void aiApi
      .getConfig()
      .then(setConfig)
      .catch((error: unknown) => toastError(formatError(error)));
  }, []);

  const visible = useMemo(() => {
    if (!models) return [];
    const needle = query.trim().toLowerCase();
    const matching = needle
      ? models.filter(
          (model) =>
            model.id.toLowerCase().includes(needle) || model.name.toLowerCase().includes(needle),
        )
      : models;
    // Le catalogue compte des centaines d'entrées : sans filtre, on n'en montre
    // qu'une tranche, sinon la liste est ingérable.
    return matching.slice(0, 60);
  }, [models, query]);

  const loadModels = async () => {
    setLoading(true);
    try {
      // La clé saisie prime : elle permet d'éprouver le catalogue avant même
      // d'enregistrer quoi que ce soit.
      setModels(await aiApi.listModels(apiKey.trim() || undefined));
    } catch (error) {
      toastError(formatError(error));
    } finally {
      setLoading(false);
    }
  };

  const saveKey = async () => {
    try {
      await aiApi.setConfig({ apiKey: apiKey.trim() });
      setApiKey("");
      setConfig(await aiApi.getConfig());
      toastSuccess("Clé enregistrée");
    } catch (error) {
      toastError(formatError(error));
    }
  };

  const chooseModel = async (id: string) => {
    try {
      await aiApi.setConfig({ model: id });
      setConfig(await aiApi.getConfig());
    } catch (error) {
      toastError(formatError(error));
    }
  };

  return (
    <div className="flex flex-col gap-5">
      <section className="flex flex-col gap-2">
        <h3 className="text-[10px] font-semibold uppercase tracking-wider text-(--color-muted)">
          Clé OpenRouter
        </h3>
        <div className="flex gap-2">
          <Input
            type="password"
            value={apiKey}
            onChange={(event) => setApiKey(event.target.value)}
            placeholder={config?.apiKeySet ? "Clé enregistrée — saisir pour remplacer" : "sk-or-…"}
            className="font-mono text-xs"
          />
          <Button
            size="sm"
            variant="outline"
            disabled={!apiKey.trim()}
            onClick={() => void saveKey()}
          >
            Enregistrer
          </Button>
        </div>
        {config?.apiKeySet && (
          <button
            type="button"
            onClick={() => {
              void aiApi
                .setConfig({ apiKey: "" })
                .then(() => aiApi.getConfig())
                .then((next) => {
                  setConfig(next);
                  toastSuccess("Clé effacée");
                })
                .catch((error: unknown) => toastError(formatError(error)));
            }}
            className="w-fit text-[11px] text-(--color-muted) hover:text-(--color-danger)"
          >
            Effacer la clé
          </button>
        )}
        <p className="text-[11px] text-(--color-warning)">
          Enregistrée en clair dans la base locale de l'application.
        </p>
      </section>

      <section className="flex min-h-0 flex-col gap-2">
        <div className="flex items-center gap-2">
          <h3 className="text-[10px] font-semibold uppercase tracking-wider text-(--color-muted)">
            Modèle
          </h3>
          {config?.model && (
            <span className="font-mono text-[11px] text-(--color-accent)">{config.model}</span>
          )}
          <Button
            size="sm"
            variant="outline"
            className="ml-auto"
            disabled={loading}
            onClick={() => void loadModels()}
          >
            {loading ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : null}
            {models ? "Recharger" : "Charger le catalogue"}
          </Button>
        </div>

        {models && (
          <>
            <div className="relative">
              <Search className="pointer-events-none absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-(--color-muted-soft)" />
              <Input
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                placeholder={`Filtrer parmi ${models.length} modèles`}
                className="h-8 pl-8 text-xs"
              />
            </div>

            <div className="max-h-72 overflow-y-auto rounded-md border border-(--color-border)">
              {visible.map((model) => {
                const active = model.id === config?.model;
                return (
                  <button
                    key={model.id}
                    type="button"
                    onClick={() => void chooseModel(model.id)}
                    className={cn(
                      "flex w-full items-center gap-2 border-b border-(--color-border) px-2 py-1.5 text-left last:border-b-0",
                      active ? "bg-(--color-accent-bg)" : "hover:bg-(--color-panel-hover)",
                    )}
                  >
                    <span className="w-3 shrink-0">
                      {active && <Check className="h-3 w-3 text-(--color-accent)" />}
                    </span>
                    <span className="min-w-0 flex-1">
                      <span className="block truncate text-xs text-(--color-text-soft)">
                        {model.name}
                      </span>
                      <span className="block truncate font-mono text-[10px] text-(--color-muted-soft)">
                        {model.id}
                      </span>
                    </span>
                    <span
                      className={cn(
                        "shrink-0 font-mono text-[10px]",
                        model.free ? "text-(--color-success)" : "text-(--color-muted)",
                      )}
                    >
                      {formatPrice(model.promptPrice)}
                    </span>
                  </button>
                );
              })}
              {visible.length === 0 && (
                <p className="p-4 text-center text-xs text-(--color-muted)">Aucun modèle</p>
              )}
            </div>
          </>
        )}
      </section>
    </div>
  );
}
