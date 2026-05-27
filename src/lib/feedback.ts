import { toast } from "sonner";

/**
 * Wrap an async backend call with a loading/success/error toast.
 *
 * Returns the same promise the caller passed in (after attaching the toast
 * lifecycle), so call sites can `await` it as usual:
 *
 * ```ts
 * await withToast(adapter.mkdir(path), {
 *   loading: "Création du dossier…",
 *   success: "Dossier créé",
 * });
 * ```
 *
 * `error` defaults to a generic "Échec — {message}" — pass a custom formatter
 * when the action deserves a more specific message.
 */
export function withToast<T>(
  promise: Promise<T>,
  opts: {
    loading: string;
    success: string | ((value: T) => string);
    error?: string | ((e: unknown) => string);
  },
): Promise<T> {
  toast.promise(promise, {
    loading: opts.loading,
    success: opts.success,
    error: opts.error ?? ((e) => `Échec — ${formatError(e)}`),
  });
  return promise;
}

/** Pretty-print whatever the backend or `throw` returned. */
export function formatError(e: unknown): string {
  if (e instanceof Error) return e.message;
  if (typeof e === "string") return e;
  return String(e);
}

/** Fire-and-forget success toast (no async). */
export function toastSuccess(message: string): void {
  toast.success(message);
}

/** Fire-and-forget error toast. */
export function toastError(message: string): void {
  toast.error(message);
}

/** Fire-and-forget info toast. */
export function toastInfo(message: string): void {
  toast.info(message);
}
