import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Code2, Coffee, Loader2, RefreshCw } from "lucide-react";
import { useEffect, useState } from "react";

import { withToast } from "@/lib/feedback";
import { checkForUpdate, installUpdate } from "@/lib/updater";
import { cn } from "@/lib/utils";
import { useSettingsStore } from "@/stores/useSettingsStore";

const REPO_URL = "https://github.com/moha528/lynk-dev";
const DONATE_URL = "https://buymeacoffee.com/mtseckdev";

/**
 * Settings → About (P5-T08).
 *
 * Shows the running version, project links, and the manual "Check for
 * updates" button (the silent startup check lives in MainLayout). Also hosts
 * the opt-in crash-reporting toggle — stored as a plain preference for now,
 * with an honest note that no telemetry backend is wired yet.
 */
export function AboutSection() {
  const [version, setVersion] = useState<string>("…");
  const [checking, setChecking] = useState(false);
  const crashOptIn = useSettingsStore((s) => s.crashReportingOptIn);
  const setSetting = useSettingsStore((s) => s.set);

  useEffect(() => {
    getVersion()
      .then(setVersion)
      .catch(() => setVersion("dev"));
  }, []);

  const handleCheck = async () => {
    setChecking(true);
    try {
      const info = await checkForUpdate();
      if (!info) {
        withToast(Promise.resolve(), {
          loading: "Vérification…",
          success: "Tu es déjà à jour 🎉",
        });
        return;
      }
      withToast(installUpdate(info), {
        loading: `Installation de la v${info.version}…`,
        success: "Mise à jour installée — redémarrage…",
      });
    } catch (e) {
      // Dev mode / offline / pubkey absente → message honnête.
      withToast(Promise.reject(e), {
        loading: "Vérification…",
        success: "",
        error: () => "Vérification impossible (mode dev, hors-ligne, ou updater non configuré).",
      });
    } finally {
      setChecking(false);
    }
  };

  return (
    <div className="flex flex-col gap-4">
      <header className="flex items-center gap-3">
        <Logo />
        <div className="flex flex-col">
          <span className="text-sm font-semibold text-(--color-text)">Lynk Dev</span>
          <span className="font-mono text-[11px] text-(--color-muted)">v{version}</span>
        </div>
      </header>

      <p className="text-xs text-(--color-muted)">
        Suite d'outils de développement desktop (Git, Dev, DB), construite avec Tauri 2, React et
        Rust. Licence FSL-1.1-MIT (source-available).
      </p>

      <div className="flex flex-wrap gap-2">
        <LinkButton onClick={() => void openUrl(REPO_URL)}>
          <Code2 className="h-3.5 w-3.5" />
          Code source
        </LinkButton>
        <LinkButton onClick={() => void openUrl(`${REPO_URL}/releases`)}>
          Notes de version
        </LinkButton>
        <LinkButton onClick={() => void openUrl(`${REPO_URL}/issues/new`)}>
          Signaler un bug
        </LinkButton>
        <LinkButton onClick={() => void openUrl(DONATE_URL)}>
          <Coffee className="h-3.5 w-3.5" />
          Offrir un café
        </LinkButton>
      </div>

      <button
        type="button"
        onClick={handleCheck}
        disabled={checking}
        className="flex items-center justify-center gap-2 rounded-md border border-(--color-border) bg-(--color-bg-soft) px-3 py-2 text-xs font-medium text-(--color-text) transition-colors hover:bg-(--color-panel-hover) disabled:opacity-50"
      >
        {checking ? (
          <Loader2 className="h-3.5 w-3.5 animate-spin" />
        ) : (
          <RefreshCw className="h-3.5 w-3.5" />
        )}
        Vérifier les mises à jour
      </button>

      <div className="border-t border-(--color-border) pt-3">
        <Toggle
          label="Rapports de crash anonymes"
          description="Aide à corriger les bugs. Désactivé par défaut. (Pas encore branché à un backend — préférence enregistrée pour une future version.)"
          checked={crashOptIn}
          onChange={(v) => setSetting("crashReportingOptIn", v)}
        />
      </div>

      <p className="text-[10px] text-(--color-muted-soft)">
        Astuce : si Windows affiche « Windows a protégé votre PC » à l'installation, c'est parce que
        l'installeur n'est pas encore signé — clique « Informations complémentaires » → « Exécuter
        quand même ».
      </p>
    </div>
  );
}

function LinkButton({ children, onClick }: { children: React.ReactNode; onClick: () => void }) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="inline-flex items-center gap-1.5 rounded-md border border-(--color-border) bg-(--color-bg-soft) px-2.5 py-1.5 text-[11px] text-(--color-text-soft) transition-colors hover:bg-(--color-panel-hover) hover:text-(--color-text)"
    >
      {children}
    </button>
  );
}

function Toggle({
  label,
  description,
  checked,
  onChange,
}: {
  label: string;
  description?: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <button
      type="button"
      onClick={() => onChange(!checked)}
      className="flex w-full items-center justify-between gap-2 rounded-md border border-(--color-border) bg-(--color-bg-soft) px-3 py-2 text-left hover:bg-(--color-panel-hover)"
    >
      <span className="flex min-w-0 flex-col">
        <span className="text-xs font-medium text-(--color-text)">{label}</span>
        {description && <span className="text-[10px] text-(--color-muted)">{description}</span>}
      </span>
      <span
        className={cn(
          "relative h-4 w-7 shrink-0 rounded-full transition-colors",
          checked ? "bg-(--color-accent)" : "bg-(--color-elevated)",
        )}
      >
        <span
          className={cn(
            "absolute top-0.5 h-3 w-3 rounded-full bg-white shadow-sm transition-all",
            checked ? "left-3.5" : "left-0.5",
          )}
        />
      </span>
    </button>
  );
}

function Logo() {
  return (
    <img src="/logo-mark.png" alt="Lynk Dev" className="h-10 w-10 select-none" draggable={false} />
  );
}
