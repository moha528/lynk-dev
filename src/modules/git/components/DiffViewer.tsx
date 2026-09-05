import { useEffect, useState } from "react";

import { DiffView } from "@/components/ui/DiffView";

import { gitApi } from "../ipc";

type Props = {
  repoPath: string;
  filePath: string;
  staged: boolean;
};

/**
 * Charge le diff d'un fichier et le confie a `DiffView`.
 *
 * Ce composant ne fait plus que l'acces au depot : la coloration, elle, est
 * partagee avec le reste de l'application.
 */
export function DiffViewer({ repoPath, filePath, staged }: Props) {
  const [diff, setDiff] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setDiff(null);
    setError(null);

    gitApi
      .diff(repoPath, filePath, staged)
      .then((value) => {
        if (!cancelled) setDiff(value);
      })
      .catch((err: unknown) => {
        if (!cancelled) setError(err instanceof Error ? err.message : String(err));
      });

    return () => {
      cancelled = true;
    };
  }, [repoPath, filePath, staged]);

  if (error) {
    return <p className="p-4 font-mono text-[11px] text-(--color-danger)">{error}</p>;
  }
  if (diff === null) {
    return <p className="p-4 text-xs text-(--color-muted)">…</p>;
  }
  if (diff.trim() === "") {
    return (
      <p className="p-4 text-xs text-(--color-muted)">
        Aucune différence — fichier binaire ou nouveau fichier non indexé.
      </p>
    );
  }

  return <DiffView diff={diff} path={filePath} className="p-3" />;
}
