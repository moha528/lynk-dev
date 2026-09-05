import { useEffect, useState } from "react";

import { type Line, highlightLines, languageOf } from "@/lib/highlight";
import { cn } from "@/lib/utils";
import { useSettingsStore } from "@/stores/useSettingsStore";

/**
 * Au-delà, on n'affiche plus tout.
 *
 * Ce n'est **pas** de la virtualisation : les lignes en trop ne sont simplement
 * pas rendues, et un pied de bloc le dit. Un fichier de 50 000 lignes bloquerait
 * la fenêtre plusieurs secondes à la coloration, pour un contenu que personne ne
 * lit d'un trait.
 */
const MAX_RENDERED_LINES = 2_000;

type Props = {
  code: string;
  /** Sert à déduire le langage. Sans lui, le texte reste brut. */
  path?: string;
  showLineNumbers?: boolean;
  className?: string;
};

export function CodeView({ code, path, showLineNumbers = true, className }: Props) {
  const theme = useSettingsStore((s) => s.appTheme);
  const [lines, setLines] = useState<Line[] | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLines(null);

    void highlightLines(code, path ? languageOf(path) : null, theme).then((result) => {
      if (!cancelled) setLines(result);
    });

    return () => {
      cancelled = true;
    };
  }, [code, path, theme]);

  const raw = code.split("\n");
  const total = raw.length;
  const shown = Math.min(total, MAX_RENDERED_LINES);

  return (
    <div className={cn("min-w-0 font-mono text-[11px] leading-relaxed", className)}>
      <pre className="whitespace-pre-wrap break-words">
        {Array.from({ length: shown }, (_, index) => (
          <div key={`${index}-${raw[index]}`} className="flex">
            {showLineNumbers && (
              <span className="mr-3 w-10 shrink-0 select-none text-right text-(--color-muted-soft)">
                {index + 1}
              </span>
            )}
            <span className="min-w-0 flex-1">
              {/* Tant que la coloration n'est pas revenue — ou si le langage
                  est inconnu — on affiche le texte brut. Un fichier illisible
                  serait pire qu'un fichier sans couleurs. */}
              {lines?.[index] ? <Tokens line={lines[index]} /> : raw[index] || " "}
            </span>
          </div>
        ))}
      </pre>

      {total > shown && (
        <p className="border-t border-(--color-border) px-3 py-2 text-(--color-muted)">
          {total - shown} lignes de plus, non affichées
        </p>
      )}
    </div>
  );
}

/** Rendu d'une ligne colorée. Exporté : le viewer de diff s'en sert aussi. */
export function Tokens({ line }: { line: Line }) {
  return (
    <>
      {line.map((token, index) => (
        <span
          key={`${index}-${token.content}`}
          style={{
            color: token.color,
            // `fontStyle` est un champ de bits : 1 = italique, 2 = gras.
            fontStyle: token.fontStyle && token.fontStyle & 1 ? "italic" : undefined,
            fontWeight: token.fontStyle && token.fontStyle & 2 ? 600 : undefined,
          }}
        >
          {token.content}
        </span>
      ))}
    </>
  );
}
