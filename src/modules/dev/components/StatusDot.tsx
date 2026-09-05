import { cn } from "@/lib/utils";

import { STATUS_LABEL, TONE_BG, isTransient, statusTone } from "../status";
import type { ServiceStatus } from "../types";

type Props = {
  status: ServiceStatus;
  className?: string;
};

/**
 * Pastille d'état.
 *
 * Les états transitoires pulsent : c'est le seul mouvement de l'écran, donc il
 * se lit comme « ça travaille » sans avoir besoin d'un texte à côté.
 */
export function StatusDot({ status, className }: Props) {
  const tone = statusTone(status);
  return (
    <span
      title={STATUS_LABEL[status]}
      className={cn("relative grid h-2.5 w-2.5 shrink-0 place-items-center", className)}
    >
      {isTransient(status) && (
        <span
          className={cn("absolute h-2.5 w-2.5 animate-ping rounded-full opacity-60", TONE_BG[tone])}
        />
      )}
      <span className={cn("h-2 w-2 rounded-full", TONE_BG[tone])} />
    </span>
  );
}
