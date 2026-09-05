import { useEffect, useMemo, useState } from "react";

import { type Line, highlightLines, languageOf } from "@/lib/highlight";
import { cn } from "@/lib/utils";
import { useSettingsStore } from "@/stores/useSettingsStore";

import { Tokens } from "./CodeView";

type Kind = "add" | "del" | "context" | "hunk" | "meta";

type DiffLine = {
  kind: Kind;
  /** Ligne telle qu'elle apparaît dans le diff, marqueur compris. */
  raw: string;
  /** Contenu sans le marqueur — c'est lui qu'on colore. */
  content: string;
};

/**
 * Découpe un diff unifié.
 *
 * ⚠️ L'ordre des tests compte : `+++` et `---` sont des **en-têtes de fichier**,
 * pas un ajout et une suppression. Les confondre teinte tout l'en-tête en vert
 * et rouge, ce qui est le premier défaut visible d'un afficheur de diff naïf.
 */
export function parseDiff(diff: string): DiffLine[] {
  return diff.split("\n").map((raw) => {
    if (raw.startsWith("+++") || raw.startsWith("---")) {
      return { kind: "meta", raw, content: raw };
    }
    if (raw.startsWith("@@")) return { kind: "hunk", raw, content: raw };
    if (raw.startsWith("diff ") || raw.startsWith("index ") || raw.startsWith("new file")) {
      return { kind: "meta", raw, content: raw };
    }
    if (raw.startsWith("+")) return { kind: "add", raw, content: raw.slice(1) };
    if (raw.startsWith("-")) return { kind: "del", raw, content: raw.slice(1) };
    // Une ligne de contexte commence par une espace ; une ligne vide en fin de
    // diff n'a même pas ça.
    return { kind: "context", raw, content: raw.startsWith(" ") ? raw.slice(1) : raw };
  });
}

const ROW_CLASS: Record<Kind, string> = {
  add: "bg-(--color-success)/10",
  del: "bg-(--color-danger)/10",
  context: "",
  hunk: "bg-(--color-panel)",
  meta: "",
};

const MARKER_CLASS: Record<Kind, string> = {
  add: "text-(--color-success)",
  del: "text-(--color-danger)",
  context: "text-(--color-muted-soft)",
  hunk: "text-(--color-accent)",
  meta: "text-(--color-muted-soft)",
};

type Props = {
  diff: string;
  /** Sert à déduire le langage coloré à l'intérieur du diff. */
  path?: string;
  className?: string;
};

/**
 * Affichage d'un diff unifié **avec la coloration du langage**.
 *
 * La grammaire `diff` de Shiki colorerait les `+` et les `-`, mais laisserait le
 * code en gris. On fait l'inverse : on retire les marqueurs, on colore le
 * contenu dans son vrai langage, puis on repose le marqueur et le fond. C'est
 * ce qui permet de lire *ce qui change*, pas seulement *qu'il y a un
 * changement*.
 */
export function DiffView({ diff, path, className }: Props) {
  const theme = useSettingsStore((s) => s.appTheme);
  const lines = useMemo(() => parseDiff(diff), [diff]);
  const [colored, setColored] = useState<Line[] | null>(null);

  // On colore le contenu **démarqué** d'un seul tenant : la grammaire a besoin
  // du contexte des lignes voisines pour fermer ses chaînes et ses commentaires.
  const body = useMemo(
    () =>
      lines
        .map((line) =>
          line.kind === "add" || line.kind === "del" || line.kind === "context" ? line.content : "",
        )
        .join("\n"),
    [lines],
  );

  useEffect(() => {
    let cancelled = false;
    setColored(null);

    void highlightLines(body, path ? languageOf(path) : null, theme).then((result) => {
      if (!cancelled) setColored(result);
    });

    return () => {
      cancelled = true;
    };
  }, [body, path, theme]);

  return (
    <pre
      className={cn(
        "min-w-0 whitespace-pre-wrap break-words font-mono text-[11px] leading-relaxed",
        className,
      )}
    >
      {lines.map((line, index) => {
        const isCode = line.kind === "add" || line.kind === "del" || line.kind === "context";
        const marker = line.kind === "add" ? "+" : line.kind === "del" ? "-" : " ";
        return (
          <div
            // Les lignes d'un diff n'ont pas d'identité propre ; l'index est
            // stable tant que le diff ne change pas, et il change en bloc.
            key={`${index}-${line.raw}`}
            className={cn("flex", ROW_CLASS[line.kind])}
          >
            <span className={cn("w-4 shrink-0 select-none", MARKER_CLASS[line.kind])}>
              {isCode ? marker : ""}
            </span>
            <span className="min-w-0 flex-1">
              {isCode && colored?.[index] ? (
                <Tokens line={colored[index]} />
              ) : (
                <span className={isCode ? undefined : MARKER_CLASS[line.kind]}>
                  {(isCode ? line.content : line.raw) || " "}
                </span>
              )}
            </span>
          </div>
        );
      })}
    </pre>
  );
}
