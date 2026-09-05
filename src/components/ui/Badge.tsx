import { type VariantProps, cva } from "class-variance-authority";

import { cn } from "@/lib/utils";

const badgeVariants = cva(
  "inline-flex items-center gap-1 rounded px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wide",
  {
    variants: {
      variant: {
        default: "bg-(--color-panel) text-(--color-muted)",
        accent: "bg-(--color-accent-bg) text-(--color-accent)",
        success: "bg-(--color-panel) text-(--color-success)",
        warning: "bg-(--color-panel) text-(--color-warning)",
        danger: "bg-(--color-panel) text-(--color-danger)",
      },
    },
    defaultVariants: { variant: "default" },
  },
);

export type BadgeProps = React.HTMLAttributes<HTMLSpanElement> & VariantProps<typeof badgeVariants>;

export function Badge({ className, variant, ...props }: BadgeProps) {
  return <span className={cn(badgeVariants({ variant }), className)} {...props} />;
}
