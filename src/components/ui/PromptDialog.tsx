import { useEffect, useState } from "react";

import { Button } from "./Button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "./Dialog";
import { Input } from "./Input";

type Props = {
  open: boolean;
  title: string;
  description?: string;
  label: string;
  initialValue?: string;
  confirmText?: string;
  onConfirm: (value: string) => Promise<void> | void;
  onOpenChange: (open: boolean) => void;
};

/**
 * Reusable one-field prompt (used for "New folder", "New file", "Rename").
 * Closes on success, surfaces backend errors inline on failure.
 */
export function PromptDialog({
  open,
  title,
  description,
  label,
  initialValue = "",
  confirmText = "Valider",
  onConfirm,
  onOpenChange,
}: Props) {
  const [value, setValue] = useState(initialValue);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (open) {
      setValue(initialValue);
      setError(null);
      setSubmitting(false);
    }
  }, [open, initialValue]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          {description && <DialogDescription>{description}</DialogDescription>}
        </DialogHeader>
        <form
          className="grid gap-3"
          onSubmit={async (e) => {
            e.preventDefault();
            setSubmitting(true);
            setError(null);
            try {
              await onConfirm(value.trim());
              onOpenChange(false);
            } catch (err) {
              setError(String(err));
            } finally {
              setSubmitting(false);
            }
          }}
        >
          <div className="grid gap-1.5 text-xs">
            <span className="font-medium text-(--color-muted)">{label}</span>
            <Input
              autoFocus
              value={value}
              onChange={(e) => setValue(e.currentTarget.value)}
              disabled={submitting}
              spellCheck={false}
            />
          </div>
          {error && (
            <div className="rounded-md border border-red-900/40 bg-red-950/30 px-3 py-2 text-xs text-red-400">
              {error}
            </div>
          )}
          <DialogFooter>
            <Button variant="outline" onClick={() => onOpenChange(false)} disabled={submitting}>
              Annuler
            </Button>
            <Button type="submit" disabled={submitting || !value.trim()}>
              {submitting ? "…" : confirmText}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
