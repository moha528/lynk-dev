import { Check, Lock, ShieldCheck } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { cn } from "@/lib/utils";
import { useSettingsStore } from "@/stores/useSettingsStore";
import { useVaultStore } from "@/stores/useVaultStore";

type Status = "idle" | "verifying" | "success" | "error";

const MIN_PIN = 4;
const MAX_PIN = 12;

/**
 * Full-screen blocking overlay shown whenever the vault is locked. Covers
 * every dialog (z-100) and intercepts pointer events.
 *
 * UX intent: every action the user takes should produce immediate, legible
 * feedback so the form feels alive even though it's a single text input
 * under the hood.
 *
 *   - **filled** slots animate in (`pin-pop`) so each keystroke registers
 *     visually
 *   - the **next empty slot** pulses with a caret (`pin-caret`) so the
 *     user always knows where typing will land
 *   - while we await the backend verification we switch the row to a
 *     dimmed pulsing state (`pin-verify`) and a "Vérification…" label
 *   - on success the boxes turn solid accent + a check icon flashes before
 *     the overlay tears down
 *   - on failure the row shakes red, the PIN clears and focus returns
 */
export function UnlockOverlay() {
  const locked = useVaultStore((s) => s.locked);
  const unlock = useVaultStore((s) => s.unlock);
  const pinLength = useSettingsStore((s) => s.pinLength);
  const setSetting = useSettingsStore((s) => s.set);
  const [pin, setPin] = useState("");
  const [status, setStatus] = useState<Status>("idle");
  const [error, setError] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement | null>(null);
  const [focused, setFocused] = useState(true);

  // Reset on each lock cycle and keep focus.
  useEffect(() => {
    if (locked) {
      setPin("");
      setStatus("idle");
      setError(null);
      // Defer focus until the overlay paints.
      requestAnimationFrame(() => inputRef.current?.focus());
    }
  }, [locked]);

  // Refocus whenever the input loses focus while we're still locked —
  // there's nothing else the user should be interacting with anyway.
  useEffect(() => {
    if (!locked || focused) return;
    const id = window.setTimeout(() => inputRef.current?.focus(), 50);
    return () => window.clearTimeout(id);
  }, [locked, focused]);

  if (!locked) return null;

  const submit = async (value: string) => {
    if (status === "verifying" || status === "success") return;
    if (value.length < MIN_PIN) return;
    setStatus("verifying");
    setError(null);
    try {
      const ok = await unlock(value);
      if (ok) {
        // Mémorise la longueur du PIN pour l'auto-validation des prochaines
        // fois (backfill pour les PIN configurés avant cette fonctionnalité).
        void setSetting("pinLength", value.length);
        setStatus("success");
        // Let the success animation breathe a bit before the overlay disappears.
        await new Promise((r) => setTimeout(r, 350));
        // setLocked happens inside unlock(); nothing else needed.
      } else {
        setStatus("error");
        setError("PIN incorrect");
        setPin("");
        setTimeout(() => {
          setStatus("idle");
          inputRef.current?.focus();
        }, 450);
      }
    } catch (err) {
      setStatus("error");
      setError(String(err));
      setTimeout(() => setStatus("idle"), 450);
    }
  };

  const slots = Math.max(MIN_PIN, Math.min(MAX_PIN, pin.length || MIN_PIN));

  return (
    <div className="fixed inset-0 z-[100] flex items-center justify-center bg-black/85 backdrop-blur-lg">
      <div className="w-full max-w-sm rounded-2xl border border-(--color-border-strong) bg-(--color-panel) p-8 shadow-2xl shadow-black/60">
        <div className="mb-6 flex flex-col items-center gap-3 text-center">
          <div
            className={cn(
              "relative grid h-14 w-14 place-items-center rounded-full bg-(--color-elevated) transition-colors duration-300",
              status === "success" ? "text-(--color-success)" : "text-(--color-accent)",
            )}
          >
            {status === "success" ? (
              <Check className="h-6 w-6" />
            ) : (
              <ShieldCheck className="h-6 w-6" />
            )}
            {status !== "success" && (
              <Lock className="absolute -bottom-1 -right-1 h-4 w-4 rounded-full bg-(--color-panel) p-0.5 text-(--color-accent)" />
            )}
          </div>
          <div>
            <h1 className="text-base font-semibold tracking-tight text-(--color-text)">
              {status === "success" ? "Déverrouillé" : "Application verrouillée"}
            </h1>
            <p className="mt-1 text-xs text-(--color-muted)">
              {status === "verifying"
                ? "Vérification…"
                : status === "success"
                  ? "Bon retour."
                  : "Entrez votre PIN pour déverrouiller."}
            </p>
          </div>
        </div>

        <form
          onSubmit={(e) => {
            e.preventDefault();
            void submit(pin);
          }}
          className="grid gap-4"
        >
          {/* Visible row of digit boxes. */}
          <button
            type="button"
            onClick={() => inputRef.current?.focus()}
            aria-label="Saisir le PIN"
            className={cn(
              "flex justify-center gap-1.5 outline-none",
              status === "error" && "animate-[shake_0.4s_ease-in-out]",
              status === "verifying" && "animate-[pin-verify_1s_ease-in-out_infinite]",
            )}
          >
            {Array.from({ length: slots }, (_, i) => {
              const filled = i < pin.length;
              const isNext = i === pin.length && focused && status === "idle";
              const isErrorState = status === "error";
              const isSuccess = status === "success";
              return (
                <span
                  // biome-ignore lint/suspicious/noArrayIndexKey: positional boxes, no reorder
                  key={i}
                  className={cn(
                    "grid h-11 w-9 place-items-center rounded-md border text-xl font-semibold transition-all duration-150",
                    filled
                      ? isSuccess
                        ? "border-(--color-success) bg-(--color-success)/20 text-(--color-success)"
                        : isErrorState
                          ? "border-red-500/60 bg-red-500/10 text-red-400"
                          : "border-(--color-accent) bg-(--color-accent-bg)/40 text-(--color-accent)"
                      : isNext
                        ? "border-(--color-accent) bg-(--color-bg-soft) shadow-[0_0_0_2px_rgba(56,189,248,0.18)]"
                        : "border-(--color-border) bg-(--color-bg-soft) text-(--color-muted-soft)",
                  )}
                >
                  {filled ? (
                    // Pop the dot in on type — the key includes pin.length
                    // so each new digit gets a fresh animation node.
                    <span
                      // biome-ignore lint/suspicious/noArrayIndexKey: indexed by position + length is intentional
                      key={`${i}-${pin.length}`}
                      className="inline-block animate-[pin-pop_180ms_ease-out]"
                    >
                      ●
                    </span>
                  ) : isNext ? (
                    <span className="inline-block h-5 w-px bg-(--color-accent) animate-[pin-caret_1s_ease-in-out_infinite]" />
                  ) : null}
                </span>
              );
            })}
          </button>

          {/* Hidden input that owns the state. */}
          <input
            ref={inputRef}
            type="password"
            inputMode="numeric"
            pattern="\d*"
            value={pin}
            maxLength={MAX_PIN}
            onChange={(e) => {
              const v = e.currentTarget.value.replace(/\D/g, "").slice(0, MAX_PIN);
              setPin(v);
              if (error) setError(null);
              if (status === "error") setStatus("idle");
              // Auto-validation dès que la longueur connue du PIN est atteinte.
              if (pinLength != null && pinLength >= MIN_PIN && v.length === pinLength) {
                void submit(v);
              }
            }}
            onFocus={() => setFocused(true)}
            onBlur={() => setFocused(false)}
            disabled={status === "verifying" || status === "success"}
            className="sr-only"
            autoComplete="off"
          />

          {/* Compact status line. Keeps height stable so the layout doesn't
              shift between idle/verify/error states. */}
          <p
            className={cn(
              "min-h-[16px] text-center text-xs transition-colors",
              status === "error"
                ? "text-red-400"
                : status === "success"
                  ? "text-(--color-success)"
                  : "text-(--color-muted)",
            )}
          >
            {error
              ? error
              : status === "success"
                ? "✓ Authentifié"
                : pin.length === 0
                  ? "En attente…"
                  : pin.length < MIN_PIN
                    ? `${pin.length} chiffre${pin.length > 1 ? "s" : ""} — au moins ${MIN_PIN}`
                    : `${pin.length} chiffres · Entrée pour valider`}
          </p>

          <button
            type="submit"
            disabled={status === "verifying" || status === "success" || pin.length < MIN_PIN}
            className={cn(
              "h-9 rounded-md bg-(--color-accent) text-sm font-medium text-(--color-bg)",
              "transition-all hover:brightness-110 disabled:cursor-not-allowed disabled:opacity-50",
            )}
          >
            {status === "verifying"
              ? "Vérification…"
              : status === "success"
                ? "✓ Déverrouillé"
                : "Déverrouiller"}
          </button>
        </form>
      </div>
    </div>
  );
}
