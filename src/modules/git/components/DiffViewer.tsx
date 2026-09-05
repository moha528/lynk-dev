import { useEffect, useState } from "react";

import { cn } from "@/lib/utils";

import { gitApi } from "../ipc";

type Props = {
  repoPath: string;
  filePath: string;
  staged: boolean;
};

/**
 * Affichage d'un diff unifié.
 *
 * Coloration **par nature de ligne** seulement (ajout / retrait / en-tête) : la
 * coloration syntaxique du langage arrive au lot 2.2, avec Shiki. Ce composant
 * sera alors remplacé, pas retouché.
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

  return (
    <pre className="overflow-auto p-3 font-mono text-[11px] leading-relaxed">
      {diff.split("\n").map((line, index) => (
        <div
          // Les lignes d'un diff n'ont pas d'identité : l'index est stable tant
          // que le diff ne change pas, et il change en bloc.
          key={`${index}-${line}`}
          className={cn("whitespace-pre-wrap break-all", lineClass(line))}
        >
          {line || " "}
        </div>
      ))}
    </pre>
  );
}

function lineClass(line: string): string {
  // L'ordre compte : `+++` et `---` sont des en-têtes, pas des ajouts.
  if (line.startsWith("+++") || line.startsWith("---")) return "text-(--color-muted-soft)";
  if (line.startsWith("@@")) return "text-(--color-accent)";
  if (line.startsWith("+")) return "text-(--color-success)";
  if (line.startsWith("-")) return "text-(--color-danger)";
  if (line.startsWith("diff ") || line.startsWith("index ")) return "text-(--color-muted-soft)";
  return "text-(--color-text-soft)";
}
