import { Check } from "lucide-react";
import { forwardRef } from "react";

import { cn } from "@/lib/utils";

type CheckboxProps = {
  checked: boolean;
  onCheckedChange: (checked: boolean) => void;
  label?: React.ReactNode;
  disabled?: boolean;
  id?: string;
  className?: string;
};

export const Checkbox = forwardRef<HTMLButtonElement, CheckboxProps>(
  ({ checked, onCheckedChange, label, disabled, id, className }, ref) => (
    <label
      htmlFor={id}
      className={cn(
        "inline-flex cursor-pointer select-none items-center gap-2 text-xs",
        disabled && "cursor-not-allowed opacity-50",
        className,
      )}
    >
      <button
        ref={ref}
        id={id}
        type="button"
        aria-checked={checked}
        aria-label="checkbox"
        disabled={disabled}
        onClick={() => onCheckedChange(!checked)}
        className={cn(
          "grid h-4 w-4 shrink-0 place-items-center rounded border transition-colors",
          checked
            ? "border-(--color-accent) bg-(--color-accent) text-zinc-950"
            : "border-(--color-border-strong) bg-(--color-bg) hover:border-(--color-accent-soft)",
          "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-(--color-accent) focus-visible:ring-offset-2 focus-visible:ring-offset-(--color-panel)",
        )}
      >
        {checked && <Check className="h-3 w-3" strokeWidth={3} />}
      </button>
      {label && <span className="text-(--color-text-soft)">{label}</span>}
    </label>
  ),
);
Checkbox.displayName = "Checkbox";
