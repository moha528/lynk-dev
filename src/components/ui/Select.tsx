import * as SelectPrimitive from "@radix-ui/react-select";
import { Check, ChevronDown } from "lucide-react";
import { forwardRef } from "react";

import { cn } from "@/lib/utils";

/**
 * Liste déroulante.
 *
 * Remplace les `<select>` natifs : sous Windows, ceux-ci s'affichent avec le
 * chrome du système, ignorent le thème de l'application et deviennent
 * illisibles dès qu'il y a plus d'une poignée d'entrées — sans parler des
 * groupes, qu'un `<optgroup>` ne met pas vraiment en valeur.
 */
export const Select = SelectPrimitive.Root;
export const SelectValue = SelectPrimitive.Value;

export const SelectTrigger = forwardRef<
  React.ElementRef<typeof SelectPrimitive.Trigger>,
  React.ComponentPropsWithoutRef<typeof SelectPrimitive.Trigger>
>(({ className, children, ...props }, ref) => (
  <SelectPrimitive.Trigger
    ref={ref}
    className={cn(
      "flex h-8 w-full items-center justify-between gap-2 rounded-md border border-(--color-border)",
      "bg-(--color-bg) px-2 text-xs text-(--color-text) transition-colors",
      "hover:border-(--color-border-strong)",
      "focus-visible:border-(--color-accent) focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-(--color-accent)",
      "disabled:cursor-not-allowed disabled:opacity-50",
      "data-[placeholder]:text-(--color-muted-soft)",
      className,
    )}
    {...props}
  >
    <span className="min-w-0 truncate text-left">{children}</span>
    <SelectPrimitive.Icon asChild>
      <ChevronDown className="h-3.5 w-3.5 shrink-0 text-(--color-muted)" />
    </SelectPrimitive.Icon>
  </SelectPrimitive.Trigger>
));
SelectTrigger.displayName = "SelectTrigger";

export const SelectContent = forwardRef<
  React.ElementRef<typeof SelectPrimitive.Content>,
  React.ComponentPropsWithoutRef<typeof SelectPrimitive.Content>
>(({ className, children, position = "popper", ...props }, ref) => (
  <SelectPrimitive.Portal>
    <SelectPrimitive.Content
      ref={ref}
      position={position}
      className={cn(
        "z-50 max-h-80 min-w-[var(--radix-select-trigger-width)] overflow-hidden",
        "rounded-md border border-(--color-border-strong) bg-(--color-panel)",
        "shadow-xl shadow-black/40",
        position === "popper" && "mt-1",
        className,
      )}
      {...props}
    >
      <SelectPrimitive.Viewport className="p-1">{children}</SelectPrimitive.Viewport>
    </SelectPrimitive.Content>
  </SelectPrimitive.Portal>
));
SelectContent.displayName = "SelectContent";

export const SelectItem = forwardRef<
  React.ElementRef<typeof SelectPrimitive.Item>,
  React.ComponentPropsWithoutRef<typeof SelectPrimitive.Item>
>(({ className, children, ...props }, ref) => (
  <SelectPrimitive.Item
    ref={ref}
    className={cn(
      "relative flex cursor-pointer select-none items-center gap-2 rounded px-2 py-1.5 text-xs outline-none",
      "text-(--color-text-soft)",
      "data-[highlighted]:bg-(--color-panel-hover) data-[highlighted]:text-(--color-text)",
      "data-[state=checked]:text-(--color-text)",
      "data-[disabled]:pointer-events-none data-[disabled]:opacity-50",
      className,
    )}
    {...props}
  >
    <span className="flex h-3 w-3 shrink-0 items-center justify-center">
      <SelectPrimitive.ItemIndicator>
        <Check className="h-3 w-3 text-(--color-accent)" />
      </SelectPrimitive.ItemIndicator>
    </span>
    <SelectPrimitive.ItemText>{children}</SelectPrimitive.ItemText>
  </SelectPrimitive.Item>
));
SelectItem.displayName = "SelectItem";

export function SelectGroup({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <SelectPrimitive.Group>
      <SelectPrimitive.Label className="px-2 pb-0.5 pt-2 text-[10px] font-semibold uppercase tracking-wider text-(--color-muted)">
        {label}
      </SelectPrimitive.Label>
      {children}
    </SelectPrimitive.Group>
  );
}
