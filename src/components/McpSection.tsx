import {
  Check,
  ClipboardCopy,
  Eye,
  EyeOff,
  Plug,
  RefreshCw,
  Trash2,
  TriangleAlert,
} from "lucide-react";
import { useCallback, useEffect, useState } from "react";

import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { formatError, toastError, toastSuccess } from "@/lib/feedback";
import {
  type McpCall,
  type McpStatus,
  clientConfig,
  formatDuration,
  mcpApi,
  onMcpCall,
} from "@/lib/mcp";
import { cn } from "@/lib/utils";

/**
 * Réglages du serveur MCP.
 *
 * Trois choses seulement se règlent ici : marche/arrêt, le port, et le jeton.
 * Tout le reste — quels outils, quel périmètre — est fixé par le backend et
 * n'a pas à être configurable : un serveur dont l'utilisateur peut élargir les
 * pouvoirs n'a plus de garde-fou.
 *
 * ⚠️ **`enabled` et `running` sont deux choses différentes.** Le premier est ce
 * qu'on a demandé, le second ce qui est vrai. Ils divergent quand le port est
 * déjà pris, et l'écran doit le montrer plutôt que d'afficher un état inventé.
 */
export function McpSection() {
  const [status, setStatus] = useState<McpStatus | null>(null);
  const [token, setToken] = useState<string | null>(null);
  const [revealed, setRevealed] = useState(false);
  const [port, setPort] = useState("");
  const [busy, setBusy] = useState(false);
  const [calls, setCalls] = useState<McpCall[]>([]);

  const load = useCallback(async () => {
    const next = await mcpApi.status();
    setStatus(next);
    setPort(String(next.port));
    return next;
  }, []);

  useEffect(() => {
    void load().catch((error: unknown) => toastError(formatError(error)));
    void mcpApi
      .calls()
      .then(setCalls)
      .catch(() => setCalls([]));
    void mcpApi
      .token()
      .then(setToken)
      .catch(() => setToken(null));
  }, [load]);

  // Le journal se remplit sous les yeux : un appel qui arrive pendant qu'on
  // regarde l'écran n'a aucune raison d'attendre un rechargement.
  useEffect(() => {
    const unlisten = onMcpCall((call) => setCalls((current) => [call, ...current].slice(0, 200)));
    return () => {
      void unlisten.then((stop) => stop());
    };
  }, []);

  const guard = async (action: () => Promise<unknown>) => {
    setBusy(true);
    try {
      await action();
    } catch (error) {
      toastError(formatError(error));
      // L'état affiché doit rester celui du backend, même après un échec.
      await load().catch(() => undefined);
    } finally {
      setBusy(false);
    }
  };

  const toggle = () =>
    guard(async () => {
      const next = await mcpApi.setEnabled(!status?.enabled);
      setStatus(next);
      setPort(String(next.port));
      if (next.running && !token) setToken(await mcpApi.token());
    });

  const applyPort = () =>
    guard(async () => {
      const parsed = Number.parseInt(port, 10);
      if (!Number.isInteger(parsed) || parsed < 1 || parsed > 65535) {
        throw new Error("port attendu entre 1 et 65535");
      }
      if (parsed === status?.port) return;
      setStatus(await mcpApi.setPort(parsed));
      toastSuccess(`Port ${parsed}`);
    });

  const regenerate = () =>
    guard(async () => {
      setToken(await mcpApi.regenerateToken());
      setRevealed(true);
      toastSuccess("Nouveau jeton — l'ancien n'est plus accepté");
    });

  const copy = (text: string, label: string) => {
    void navigator.clipboard.writeText(text).then(() => toastSuccess(label));
  };

  if (!status) {
    return <p className="text-xs text-(--color-muted)">Chargement…</p>;
  }

  return (
    <div className="flex flex-col gap-5">
      {status.keychainError && (
        <p className="flex items-start gap-2 rounded-md border border-(--color-danger)/40 bg-(--color-danger)/10 px-2.5 py-2 text-[11px] text-(--color-danger)">
          <TriangleAlert className="mt-px h-3.5 w-3.5 shrink-0" />
          <span>{status.keychainError}</span>
        </p>
      )}

      <section className="flex flex-col gap-2">
        <div className="flex items-center justify-between gap-2 rounded-md border border-(--color-border) bg-(--color-bg-soft) px-2.5 py-1.5">
          <div className="flex min-w-0 items-center gap-2">
            <span
              className={cn(
                "grid h-6 w-6 shrink-0 place-items-center rounded-md bg-(--color-panel)",
                status.running ? "text-(--color-success)" : "text-(--color-muted)",
              )}
            >
              <Plug className="h-3 w-3" />
            </span>
            <div className="flex min-w-0 flex-col leading-tight">
              <span className="text-[11px] font-medium text-(--color-text)">Serveur MCP</span>
              <span className="text-[10px] text-(--color-muted)">
                {status.running
                  ? `En écoute sur ${status.url}`
                  : status.enabled
                    ? "Demandé, mais pas en écoute — port occupé ?"
                    : "Arrêté"}
              </span>
            </div>
          </div>
          <Button
            size="sm"
            variant={status.running ? "outline" : "default"}
            disabled={busy}
            onClick={() => void toggle()}
            className="h-6 shrink-0 px-2 text-[10px]"
          >
            {status.running ? "Arrêter" : "Démarrer"}
          </Button>
        </div>

        <div className="flex items-center gap-2">
          <label
            htmlFor="mcp-port"
            className="text-[10px] font-semibold uppercase tracking-wider text-(--color-muted)"
          >
            Port
          </label>
          <Input
            id="mcp-port"
            value={port}
            inputMode="numeric"
            onChange={(event) => setPort(event.target.value.replace(/\D/g, ""))}
            onBlur={() => void applyPort()}
            onKeyDown={(event) => {
              if (event.key === "Enter") void applyPort();
            }}
            className="h-7 w-24 font-mono text-xs"
          />
          {status.running && (
            <Button
              size="sm"
              variant="ghost"
              onClick={() => copy(status.url, "URL copiée")}
              className="h-7 px-2 text-[10px]"
            >
              <ClipboardCopy className="h-3 w-3" />
              Copier l'URL
            </Button>
          )}
        </div>
      </section>

      <section className="flex flex-col gap-2">
        <h3 className="text-[10px] font-semibold uppercase tracking-wider text-(--color-muted)">
          Jeton
        </h3>
        <div className="flex gap-2">
          <div className="relative flex-1">
            <Input
              readOnly
              value={token ?? ""}
              type={revealed ? "text" : "password"}
              placeholder="Généré au premier démarrage"
              className="h-8 pr-8 font-mono text-[11px]"
            />
            <button
              type="button"
              aria-label={revealed ? "Masquer le jeton" : "Afficher le jeton"}
              onClick={() => setRevealed((current) => !current)}
              className="absolute right-2 top-1/2 -translate-y-1/2 text-(--color-muted) hover:text-(--color-text)"
            >
              {revealed ? <EyeOff className="h-3.5 w-3.5" /> : <Eye className="h-3.5 w-3.5" />}
            </button>
          </div>
          <Button
            size="sm"
            variant="outline"
            disabled={busy || !token}
            onClick={() => token && copy(token, "Jeton copié")}
          >
            <ClipboardCopy className="h-3.5 w-3.5" />
          </Button>
          <Button size="sm" variant="outline" disabled={busy} onClick={() => void regenerate()}>
            <RefreshCw className="h-3.5 w-3.5" />
            Régénérer
          </Button>
        </div>
        <Button
          size="sm"
          variant="outline"
          disabled={!token}
          onClick={() =>
            token && copy(clientConfig(status.url, token), "Configuration client copiée")
          }
          className="w-fit"
        >
          <ClipboardCopy className="h-3.5 w-3.5" />
          Copier la configuration client
        </Button>
        <p className="text-[11px] text-(--color-muted)">
          Jeton conservé dans le trousseau du système. Le fichier de configuration du client, lui,
          le porte en clair.
        </p>
      </section>

      <section className="flex min-h-0 flex-col gap-2">
        <div className="flex items-center gap-2">
          <h3 className="text-[10px] font-semibold uppercase tracking-wider text-(--color-muted)">
            Appels
          </h3>
          <span className="font-mono text-[10px] text-(--color-muted-soft)">{calls.length}</span>
          <Button
            size="sm"
            variant="ghost"
            disabled={calls.length === 0}
            onClick={() => void mcpApi.clearCalls().then(() => setCalls([]))}
            className="ml-auto h-6 px-2 text-[10px]"
          >
            <Trash2 className="h-3 w-3" />
            Vider
          </Button>
        </div>

        {calls.length === 0 ? (
          <p className="rounded-md border border-(--color-border) px-3 py-4 text-center text-[11px] text-(--color-muted)">
            Aucun appel depuis le démarrage.
          </p>
        ) : (
          <ul className="max-h-64 overflow-y-auto rounded-md border border-(--color-border)">
            {calls.map((call) => (
              <li
                key={`${call.at}-${call.tool}-${call.durationMs}`}
                className="flex items-start gap-2 border-b border-(--color-border) px-2 py-1.5 last:border-b-0"
              >
                <span className="mt-px shrink-0">
                  {call.ok ? (
                    <Check className="h-3 w-3 text-(--color-success)" />
                  ) : (
                    <TriangleAlert className="h-3 w-3 text-(--color-danger)" />
                  )}
                </span>
                <div className="min-w-0 flex-1 leading-tight">
                  <div className="flex items-baseline gap-1.5">
                    <span className="font-mono text-[11px] text-(--color-text-soft)">
                      {call.tool}
                    </span>
                    {call.target && (
                      <span className="truncate text-[10px] text-(--color-accent)">
                        {call.target}
                      </span>
                    )}
                    <span className="ml-auto shrink-0 font-mono text-[10px] text-(--color-muted-soft)">
                      {new Date(call.at).toLocaleTimeString()} · {formatDuration(call.durationMs)}
                    </span>
                  </div>
                  <p
                    className={cn(
                      "truncate text-[10px]",
                      call.ok ? "text-(--color-muted)" : "text-(--color-danger)",
                    )}
                    title={call.detail}
                  >
                    {call.detail}
                  </p>
                </div>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
