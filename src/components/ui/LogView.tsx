import { ArrowDownToLine, Copy, Search, Trash2 } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";

import { LEVEL_CLASS, detectLevel, parseAnsi, stripAnsi } from "@/lib/ansi";
import { cn } from "@/lib/utils";

import { Input } from "./Input";

export type LogStream = "stdout" | "stderr" | "system";

export type LogLine = {
  timestamp: number;
  stream: LogStream;
  text: string;
};

type Props = {
  lines: LogLine[];
  onClear: () => void;
  emptyLabel?: string;
};

const STREAMS: { id: LogStream; label: string }[] = [
  { id: "stdout", label: "sortie" },
  { id: "stderr", label: "erreurs" },
  { id: "system", label: "système" },
];

/** Distance au bas en dessous de laquelle on considère l'utilisateur « en bas ». */
const STICK_THRESHOLD = 40;

/**
 * Visionneuse de logs : séquences ANSI interprétées, niveaux mis en évidence,
 * filtre par flux, recherche surlignée et suivi automatique.
 */
export function LogView({ lines, onClear, emptyLabel = "Aucun log" }: Props) {
  const [query, setQuery] = useState("");
  const [hidden, setHidden] = useState<Set<LogStream>>(new Set());
  const [follow, setFollow] = useState(true);
  const scroller = useRef<HTMLDivElement>(null);

  const needle = query.trim().toLowerCase();

  const visible = useMemo(
    () =>
      lines.filter((line) => {
        if (hidden.has(line.stream)) return false;
        if (!needle) return true;
        // La recherche porte sur le texte **sans** séquences ANSI : sinon un
        // mot coloré est introuvable, coupé en deux par un code d'échappement.
        return stripAnsi(line.text).toLowerCase().includes(needle);
      }),
    [lines, hidden, needle],
  );

  // Suivi automatique : on ne colle en bas que si l'utilisateur y est déjà.
  // Sans ce garde-fou, lire une ligne ancienne pendant qu'un service démarre
  // est impossible — l'écran saute à chaque arrivée.
  useEffect(() => {
    if (!follow || visible.length === 0) return;
    const element = scroller.current;
    if (element) element.scrollTop = element.scrollHeight;
  }, [follow, visible.length]);

  const onScroll = () => {
    const element = scroller.current;
    if (!element) return;
    const atBottom =
      element.scrollHeight - element.scrollTop - element.clientHeight < STICK_THRESHOLD;
    if (atBottom !== follow) setFollow(atBottom);
  };

  const toggleStream = (stream: LogStream) => {
    setHidden((current) => {
      const next = new Set(current);
      if (next.has(stream)) next.delete(stream);
      else next.add(stream);
      return next;
    });
  };

  const copyAll = () => {
    // On copie du texte propre : coller des codes d'échappement dans un ticket
    // ne rend service à personne.
    void navigator.clipboard.writeText(visible.map((line) => stripAnsi(line.text)).join("\n"));
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex flex-wrap items-center gap-2 border-b border-(--color-border) px-2 py-1.5">
        <div className="relative min-w-40 flex-1">
          <Search className="pointer-events-none absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-(--color-muted-soft)" />
          <Input
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Rechercher"
            className="h-7 pl-8 text-xs"
          />
        </div>

        <div className="flex items-center gap-1">
          {STREAMS.map((stream) => (
            <button
              key={stream.id}
              type="button"
              onClick={() => toggleStream(stream.id)}
              className={cn(
                "rounded px-1.5 py-0.5 text-[10px] uppercase tracking-wide transition-colors",
                hidden.has(stream.id)
                  ? "text-(--color-muted-soft) line-through"
                  : "bg-(--color-panel) text-(--color-text-soft)",
              )}
            >
              {stream.label}
            </button>
          ))}
        </div>

        <span className="font-mono text-[10px] text-(--color-muted-soft)">{visible.length}</span>

        <div className="flex items-center gap-0.5">
          <IconButton
            label={follow ? "Suivi actif" : "Reprendre le suivi"}
            active={follow}
            onClick={() => setFollow(true)}
          >
            <ArrowDownToLine className="h-3.5 w-3.5" />
          </IconButton>
          <IconButton label="Copier" onClick={copyAll}>
            <Copy className="h-3.5 w-3.5" />
          </IconButton>
          <IconButton label="Effacer" onClick={onClear}>
            <Trash2 className="h-3.5 w-3.5" />
          </IconButton>
        </div>
      </div>

      <div
        ref={scroller}
        onScroll={onScroll}
        className="min-h-0 flex-1 overflow-auto bg-(--color-bg) px-3 py-2"
      >
        {visible.length === 0 ? (
          <p className="pt-6 text-center text-xs text-(--color-muted)">
            {lines.length === 0 ? emptyLabel : "Aucune ligne ne correspond"}
          </p>
        ) : (
          <pre className="whitespace-pre-wrap break-words font-mono text-[11px] leading-relaxed">
            {visible.map((line, index) => (
              <Row key={`${line.timestamp}-${index}`} line={line} needle={needle} />
            ))}
          </pre>
        )}
      </div>
    </div>
  );
}

function Row({ line, needle }: { line: LogLine; needle: string }) {
  const level = detectLevel(line.text);

  // Le flux prime sur le niveau : `stderr` est rouge même sans mot-clé, et une
  // ligne « système » vient de Lynk Dev, pas du service.
  const base =
    line.stream === "stderr"
      ? "text-(--color-danger)"
      : line.stream === "system"
        ? "italic text-(--color-muted)"
        : level
          ? LEVEL_CLASS[level]
          : "text-(--color-text-soft)";

  // Recherche et couleurs ANSI ne cohabitent pas : quand on cherche, le
  // surlignage prime et la ligne est rendue en texte propre. Chercher, c'est
  // vouloir *trouver*, pas admirer les couleurs.
  if (needle) {
    return <div className={base}>{highlight(stripAnsi(line.text), needle)}</div>;
  }

  const spans = parseAnsi(line.text);
  const colored = spans.some((span) => span.className);
  if (!colored) {
    return <div className={base}>{stripAnsi(line.text) || " "}</div>;
  }

  return (
    <div>
      {spans.map((span, index) => (
        <span key={`${index}-${span.text}`} className={span.className || base}>
          {span.text}
        </span>
      ))}
    </div>
  );
}

/** Met en évidence les occurrences de `needle`, sans dépendance ni regex. */
function highlight(text: string, needle: string) {
  const lower = text.toLowerCase();
  const parts: React.ReactNode[] = [];
  let cursor = 0;

  for (;;) {
    const found = lower.indexOf(needle, cursor);
    if (found === -1) break;
    if (found > cursor) parts.push(text.slice(cursor, found));
    parts.push(
      <mark
        key={`${found}-${parts.length}`}
        className="rounded-sm bg-(--color-accent) px-0.5 text-zinc-950"
      >
        {text.slice(found, found + needle.length)}
      </mark>,
    );
    cursor = found + needle.length;
  }
  if (cursor < text.length) parts.push(text.slice(cursor));
  return parts.length > 0 ? parts : text || " ";
}

function IconButton({
  label,
  active,
  onClick,
  children,
}: {
  label: string;
  active?: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      title={label}
      aria-label={label}
      onClick={onClick}
      className={cn(
        "rounded p-1 transition-colors",
        active
          ? "text-(--color-accent)"
          : "text-(--color-muted) hover:bg-(--color-panel-hover) hover:text-(--color-text)",
      )}
    >
      {children}
    </button>
  );
}
