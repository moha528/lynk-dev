import { ArrowDownToLine, Copy, Search, Trash2 } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";

import { Input } from "@/components/ui/Input";
import { cn } from "@/lib/utils";

import type { LogEntry, LogStream, ServiceRuntime } from "../types";

type Props = {
  runtime: ServiceRuntime;
  onClear: () => void;
};

const STREAMS: { id: LogStream; label: string }[] = [
  { id: "stdout", label: "sortie" },
  { id: "stderr", label: "erreurs" },
  { id: "system", label: "système" },
];

const STREAM_CLASS: Record<LogStream, string> = {
  stdout: "text-(--color-text-soft)",
  stderr: "text-(--color-danger)",
  system: "italic text-(--color-muted)",
};

/** Distance au bas en dessous de laquelle on considère l'utilisateur « en bas ». */
const STICK_THRESHOLD = 40;

export function LogPanel({ runtime, onClear }: Props) {
  const [query, setQuery] = useState("");
  const [hidden, setHidden] = useState<Set<LogStream>>(new Set());
  const [follow, setFollow] = useState(true);
  const scroller = useRef<HTMLDivElement>(null);

  const lines = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return runtime.logs.filter((entry) => {
      if (hidden.has(entry.stream)) return false;
      if (!needle) return true;
      return entry.text.toLowerCase().includes(needle);
    });
  }, [runtime.logs, query, hidden]);

  // Suivi automatique : on ne colle en bas que si l'utilisateur y est déjà.
  // Sans ce garde-fou, lire un log ancien pendant qu'un service démarre est
  // impossible — l'écran saute à chaque ligne.
  useEffect(() => {
    if (!follow || lines.length === 0) return;
    const element = scroller.current;
    if (element) element.scrollTop = element.scrollHeight;
  }, [follow, lines.length]);

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
    void navigator.clipboard.writeText(lines.map((entry) => entry.text).join("\n"));
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex flex-wrap items-center gap-2 border-b border-(--color-border) px-2 py-1.5">
        <div className="relative min-w-40 flex-1">
          <Search className="pointer-events-none absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-(--color-muted-soft)" />
          <Input
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Rechercher dans les logs"
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

        <span className="font-mono text-[10px] text-(--color-muted-soft)">{lines.length}</span>

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
        {lines.length === 0 ? (
          <p className="pt-6 text-center text-xs text-(--color-muted)">
            {runtime.logs.length === 0 ? "Aucun log" : "Aucune ligne ne correspond"}
          </p>
        ) : (
          <pre className="whitespace-pre-wrap break-words font-mono text-[11px] leading-relaxed">
            {lines.map((entry, index) => (
              <LogLine
                // Les lignes n'ont pas d'identité propre ; l'horodatage et
                // l'index suffisent et restent stables tant qu'on n'efface pas.
                key={`${entry.timestamp}-${index}`}
                entry={entry}
                query={query.trim()}
              />
            ))}
          </pre>
        )}
      </div>
    </div>
  );
}

function LogLine({ entry, query }: { entry: LogEntry; query: string }) {
  return <div className={STREAM_CLASS[entry.stream]}>{highlight(entry.text, query)}</div>;
}

/** Met en évidence les occurrences de `query`, sans dépendance ni regex. */
function highlight(text: string, query: string) {
  if (!query) return text;
  const lower = text.toLowerCase();
  const needle = query.toLowerCase();
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
  return parts;
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
