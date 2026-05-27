import { Clock, Lock, ShieldCheck, ShieldOff } from "lucide-react";
import { useEffect, useState } from "react";

import { withToast } from "@/lib/feedback";
import { vaultApi } from "@/lib/ipc";
import { useSettingsStore } from "@/stores/useSettingsStore";
import { useVaultStore } from "@/stores/useVaultStore";

import { Button } from "./ui/Button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "./ui/Dialog";
import { Input } from "./ui/Input";

/**
 * Settings section: master password + auto-lock.
 *
 * Two independent toggles:
 *   - Enable/change/disable master password — the actual hash + verification
 *     live in the backend `vault` module (Argon2id).
 *   - Auto-lock after N minutes — purely client-side timer in MainLayout.
 *     Stored in settings KV so the value survives restarts.
 */
export function SecuritySection() {
  const hasMaster = useVaultStore((s) => s.hasMaster);
  const refresh = useVaultStore((s) => s.refresh);
  const lock = useVaultStore((s) => s.lock);
  const autoLockMinutes = useSettingsStore((s) => s.autoLockMinutes);
  const setSetting = useSettingsStore((s) => s.set);
  const [setupOpen, setSetupOpen] = useState(false);
  const [changeOpen, setChangeOpen] = useState(false);
  const [disableOpen, setDisableOpen] = useState(false);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return (
    <>
      <div className="flex flex-col gap-2">
        <div className="flex items-center justify-between gap-2 rounded-md border border-(--color-border) bg-(--color-bg-soft) px-2.5 py-1.5">
          <div className="flex min-w-0 items-center gap-2">
            <span className="grid h-6 w-6 shrink-0 place-items-center rounded-md bg-(--color-panel) text-(--color-accent)">
              {hasMaster ? <ShieldCheck className="h-3 w-3" /> : <ShieldOff className="h-3 w-3" />}
            </span>
            <div className="flex min-w-0 flex-col leading-tight">
              <span className="text-[11px] font-medium text-(--color-text)">PIN d'accès</span>
              <span className="text-[10px] text-(--color-muted)">
                {hasMaster
                  ? "Actif — demandé à chaque démarrage et après inactivité."
                  : "Non configuré — l'app s'ouvre librement."}
              </span>
            </div>
          </div>
          {hasMaster ? (
            <div className="flex shrink-0 items-center gap-1">
              <Button
                variant="outline"
                onClick={() => setChangeOpen(true)}
                className="h-6 px-2 text-[10px]"
              >
                Changer
              </Button>
              <Button
                variant="outline"
                onClick={() => setDisableOpen(true)}
                className="h-6 px-2 text-[10px]"
              >
                Désactiver
              </Button>
              <Button onClick={() => lock()} className="h-6 px-2 text-[10px]">
                <Lock className="h-2.5 w-2.5" />
                Lock now
              </Button>
            </div>
          ) : (
            <Button onClick={() => setSetupOpen(true)} className="h-6 shrink-0 px-2 text-[10px]">
              Activer
            </Button>
          )}
        </div>

        <div className="flex items-center justify-between gap-2 rounded-md border border-(--color-border) bg-(--color-bg-soft) px-2.5 py-1.5">
          <div className="flex min-w-0 items-center gap-2">
            <span className="grid h-6 w-6 shrink-0 place-items-center rounded-md bg-(--color-panel) text-(--color-accent)">
              <Clock className="h-3 w-3" />
            </span>
            <div className="flex min-w-0 flex-col leading-tight">
              <span className="text-[11px] font-medium text-(--color-text)">
                Verrouillage automatique
              </span>
              <span className="text-[10px] text-(--color-muted)">
                {hasMaster
                  ? "Verrouille après cette durée sans activité."
                  : "Sans effet tant qu'aucun PIN n'est défini."}
              </span>
            </div>
          </div>
          <select
            value={autoLockMinutes}
            onChange={(e) => setSetting("autoLockMinutes", Number(e.currentTarget.value))}
            className="h-7 shrink-0 rounded-md border border-(--color-border) bg-(--color-panel) px-1.5 text-[11px] outline-none focus:border-(--color-accent)"
          >
            <option value={0}>Désactivé</option>
            <option value={5}>5 min</option>
            <option value={15}>15 min</option>
            <option value={30}>30 min</option>
            <option value={60}>60 min</option>
          </select>
        </div>
      </div>

      <SetMasterDialog open={setupOpen} onOpenChange={setSetupOpen} onDone={() => refresh()} />
      <ChangeMasterDialog open={changeOpen} onOpenChange={setChangeOpen} onDone={() => refresh()} />
      <DisableMasterDialog
        open={disableOpen}
        onOpenChange={setDisableOpen}
        onDone={() => refresh()}
      />
    </>
  );
}

function SetMasterDialog({
  open,
  onOpenChange,
  onDone,
}: {
  open: boolean;
  onOpenChange: (o: boolean) => void;
  onDone: () => void;
}) {
  const [pw1, setPw1] = useState("");
  const [pw2, setPw2] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (open) {
      setPw1("");
      setPw2("");
      setError(null);
      setSubmitting(false);
    }
  }, [open]);

  const onSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (pw1.length < 4 || pw1.length > 12) {
      setError("Le PIN doit faire entre 4 et 12 chiffres.");
      return;
    }
    if (pw1 !== pw2) {
      setError("Les deux PIN ne correspondent pas.");
      return;
    }
    setSubmitting(true);
    setError(null);
    try {
      await withToast(vaultApi.setPin(pw1), {
        loading: "Activation…",
        success: "PIN activé",
      });
      void useSettingsStore.getState().set("pinLength", pw1.length);
      onDone();
      onOpenChange(false);
    } catch (err) {
      setError(String(err));
      setSubmitting(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Activer le PIN d'accès</DialogTitle>
          <DialogDescription>
            4 à 12 chiffres. Il sera demandé au démarrage et après le délai d'inactivité.
            Conservez-le en lieu sûr : il n'est pas récupérable.
          </DialogDescription>
        </DialogHeader>
        <form onSubmit={onSubmit} className="grid gap-3">
          <Input
            type="password"
            inputMode="numeric"
            pattern="\d*"
            maxLength={12}
            // biome-ignore lint/a11y/noAutofocus: user just clicked "Activer"
            autoFocus
            placeholder="Nouveau PIN"
            value={pw1}
            onChange={(e) => setPw1(e.currentTarget.value.replace(/\D/g, "").slice(0, 12))}
            disabled={submitting}
            className="text-center font-mono text-lg tracking-[0.4em]"
          />
          <Input
            type="password"
            inputMode="numeric"
            pattern="\d*"
            maxLength={12}
            placeholder="Confirmer le PIN"
            value={pw2}
            onChange={(e) => setPw2(e.currentTarget.value.replace(/\D/g, "").slice(0, 12))}
            disabled={submitting}
            className="text-center font-mono text-lg tracking-[0.4em]"
          />
          {error && (
            <div className="rounded-md border border-red-900/40 bg-red-950/30 px-3 py-2 text-xs text-red-400">
              {error}
            </div>
          )}
          <DialogFooter>
            <Button variant="outline" onClick={() => onOpenChange(false)} disabled={submitting}>
              Annuler
            </Button>
            <Button type="submit" disabled={submitting || !pw1 || !pw2}>
              {submitting ? "Activation…" : "Activer"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

function ChangeMasterDialog({
  open,
  onOpenChange,
  onDone,
}: {
  open: boolean;
  onOpenChange: (o: boolean) => void;
  onDone: () => void;
}) {
  const [cur, setCur] = useState("");
  const [pw1, setPw1] = useState("");
  const [pw2, setPw2] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (open) {
      setCur("");
      setPw1("");
      setPw2("");
      setError(null);
      setSubmitting(false);
    }
  }, [open]);

  const onSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (pw1.length < 4 || pw1.length > 12) {
      setError("Le PIN doit faire entre 4 et 12 chiffres.");
      return;
    }
    if (pw1 !== pw2) {
      setError("Les deux nouveaux PIN ne correspondent pas.");
      return;
    }
    setSubmitting(true);
    setError(null);
    try {
      await withToast(vaultApi.changePin(cur, pw1), {
        loading: "Modification…",
        success: "PIN modifié",
      });
      void useSettingsStore.getState().set("pinLength", pw1.length);
      onDone();
      onOpenChange(false);
    } catch (err) {
      setError(String(err));
      setSubmitting(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Changer le PIN d'accès</DialogTitle>
        </DialogHeader>
        <form onSubmit={onSubmit} className="grid gap-3">
          <Input
            type="password"
            inputMode="numeric"
            pattern="\d*"
            maxLength={12}
            // biome-ignore lint/a11y/noAutofocus: user just clicked "Changer"
            autoFocus
            placeholder="PIN actuel"
            value={cur}
            onChange={(e) => setCur(e.currentTarget.value.replace(/\D/g, "").slice(0, 12))}
            disabled={submitting}
            className="text-center font-mono text-lg tracking-[0.4em]"
          />
          <Input
            type="password"
            inputMode="numeric"
            pattern="\d*"
            maxLength={12}
            placeholder="Nouveau PIN"
            value={pw1}
            onChange={(e) => setPw1(e.currentTarget.value.replace(/\D/g, "").slice(0, 12))}
            disabled={submitting}
            className="text-center font-mono text-lg tracking-[0.4em]"
          />
          <Input
            type="password"
            inputMode="numeric"
            pattern="\d*"
            maxLength={12}
            placeholder="Confirmer le nouveau"
            value={pw2}
            onChange={(e) => setPw2(e.currentTarget.value.replace(/\D/g, "").slice(0, 12))}
            disabled={submitting}
            className="text-center font-mono text-lg tracking-[0.4em]"
          />
          {error && (
            <div className="rounded-md border border-red-900/40 bg-red-950/30 px-3 py-2 text-xs text-red-400">
              {error}
            </div>
          )}
          <DialogFooter>
            <Button variant="outline" onClick={() => onOpenChange(false)} disabled={submitting}>
              Annuler
            </Button>
            <Button type="submit" disabled={submitting || !cur || !pw1 || !pw2}>
              {submitting ? "Modification…" : "Changer"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

function DisableMasterDialog({
  open,
  onOpenChange,
  onDone,
}: {
  open: boolean;
  onOpenChange: (o: boolean) => void;
  onDone: () => void;
}) {
  const [cur, setCur] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (open) {
      setCur("");
      setError(null);
      setSubmitting(false);
    }
  }, [open]);

  const onSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setSubmitting(true);
    setError(null);
    try {
      await withToast(vaultApi.disablePin(cur), {
        loading: "Désactivation…",
        success: "PIN désactivé",
      });
      void useSettingsStore.getState().set("pinLength", null);
      onDone();
      onOpenChange(false);
    } catch (err) {
      setError(String(err));
      setSubmitting(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Désactiver le PIN d'accès</DialogTitle>
          <DialogDescription>
            L'application s'ouvrira sans demande de PIN. Tu peux le réactiver à tout moment.
          </DialogDescription>
        </DialogHeader>
        <form onSubmit={onSubmit} className="grid gap-3">
          <Input
            type="password"
            inputMode="numeric"
            pattern="\d*"
            maxLength={12}
            // biome-ignore lint/a11y/noAutofocus: user just clicked "Désactiver"
            autoFocus
            placeholder="PIN actuel"
            value={cur}
            onChange={(e) => setCur(e.currentTarget.value.replace(/\D/g, "").slice(0, 12))}
            disabled={submitting}
            className="text-center font-mono text-lg tracking-[0.4em]"
          />
          {error && (
            <div className="rounded-md border border-red-900/40 bg-red-950/30 px-3 py-2 text-xs text-red-400">
              {error}
            </div>
          )}
          <DialogFooter>
            <Button variant="outline" onClick={() => onOpenChange(false)} disabled={submitting}>
              Annuler
            </Button>
            <Button type="submit" disabled={submitting || !cur}>
              {submitting ? "Désactivation…" : "Désactiver"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
