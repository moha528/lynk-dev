import { type VariantProps, cva } from "class-variance-authority";
import { forwardRef } from "react";

import { cn } from "@/lib/utils";

const buttonVariants = cva(
  cn(
    "inline-flex items-center justify-center gap-2 rounded-md text-sm font-medium",
    "transition-all duration-150",
    "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-(--color-accent) focus-visible:ring-offset-2 focus-visible:ring-offset-(--color-bg)",
    "disabled:pointer-events-none disabled:opacity-50",
    "active:translate-y-px",
  ),
  {
    variants: {
      variant: {
        default:
          "bg-(--color-accent) text-zinc-950 hover:brightness-110 shadow-[0_1px_0_rgba(0,0,0,0.3),inset_0_1px_0_rgba(255,255,255,0.15)]",
        outline:
          "border border-(--color-border-strong) bg-(--color-panel) text-(--color-text-soft) hover:bg-(--color-panel-hover) hover:text-(--color-text)",
        ghost: "text-(--color-text-soft) hover:bg-(--color-panel-hover) hover:text-(--color-text)",
        destructive:
          "bg-(--color-danger) text-white hover:brightness-110 shadow-[0_1px_0_rgba(0,0,0,0.3),inset_0_1px_0_rgba(255,255,255,0.15)]",
      },
      size: {
        default: "h-9 px-4",
        sm: "h-8 px-3 text-xs",
        icon: "h-8 w-8",
      },
    },
    defaultVariants: { variant: "default", size: "default" },
  },
);

type ButtonProps = React.ButtonHTMLAttributes<HTMLButtonElement> &
  VariantProps<typeof buttonVariants>;

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, ...props }, ref) => (
    <button
      ref={ref}
      type={props.type ?? "button"}
      className={cn(buttonVariants({ variant, size }), className)}
      {...props}
    />
  ),
);
Button.displayName = "Button";
