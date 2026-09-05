/**
 * Interprétation des séquences ANSI d'une ligne de log.
 *
 * Les outils de développement colorent leur sortie : `mvn`, `vite`, `cargo` et
 * `docker` émettent tous des séquences SGR. Sans les interpréter, on affiche
 * des codes d'échappement au milieu du texte ; sans les retirer, la ligne
 * devient illisible.
 *
 * Les couleurs sont ramenées sur **la palette de l'application** plutôt que sur
 * les couleurs ANSI brutes : un rouge de terminal serait criard à côté du reste
 * de la fenêtre, et ne suivrait pas le changement de thème.
 */

export type AnsiSpan = {
  text: string;
  className: string;
};

/** Codes SGR de couleur d'avant-plan → classe de la palette. */
const FOREGROUND: Record<number, string> = {
  30: "text-(--color-muted-soft)",
  31: "text-(--color-danger)",
  32: "text-(--color-success)",
  33: "text-(--color-warning)",
  34: "text-(--color-accent)",
  35: "text-(--color-accent-soft)",
  36: "text-(--color-accent)",
  37: "text-(--color-text)",
  // Variantes « vives » : même palette, l'app n'a pas deux nuances par teinte.
  90: "text-(--color-muted)",
  91: "text-(--color-danger)",
  92: "text-(--color-success)",
  93: "text-(--color-warning)",
  94: "text-(--color-accent)",
  95: "text-(--color-accent-soft)",
  96: "text-(--color-accent)",
  97: "text-(--color-text)",
};

/**
 * Séquence de contrôle : `ESC [ … lettre`.
 *
 * L'octet ESC est écrit `\u001b` plutôt qu'en clair : un caractère de contrôle
 * littéral dans le source est invisible à la relecture, et le linter le refuse.
 *
 * Une instance **neuve à chaque appel** : une expression `g` porte un
 * `lastIndex`, et la partager entre `exec` et `replace` produit des sauts
 * impossibles à reproduire.
 *
 * L'expression contient délibérément un caractère de contrôle, d'où la
 * suppression de règle : celle-ci protège des caractères glissés par accident,
 * pas de celui qui est l'objet même du motif. Elle doit rester **collée** à la
 * ligne visée — Biome n'applique une suppression qu'à la ligne suivante.
 */
function controlPattern(): RegExp {
  // biome-ignore lint/suspicious/noControlCharactersInRegex: ESC est l'objet meme de l'expression
  return /\u001b\[([0-9;]*)([A-Za-z])/g;
}

/**
 * Découpe une ligne en segments colorés.
 *
 * Toute séquence non reconnue (déplacement de curseur, effacement…) est
 * **retirée** sans effet : on affiche du texte, pas un terminal.
 */
export function parseAnsi(line: string): AnsiSpan[] {
  const spans: AnsiSpan[] = [];
  let color = "";
  let bold = false;
  let cursor = 0;

  const push = (text: string) => {
    if (!text) return;
    const className = [color, bold ? "font-semibold" : ""].filter(Boolean).join(" ");
    const previous = spans.at(-1);
    // Fusionne les segments consécutifs de même style : moins de nœuds DOM sur
    // une sortie très colorée.
    if (previous && previous.className === className) previous.text += text;
    else spans.push({ text, className });
  };

  const pattern = controlPattern();
  let match = pattern.exec(line);
  while (match !== null) {
    push(line.slice(cursor, match.index));
    cursor = match.index + match[0].length;

    if (match[2] === "m") {
      // Une séquence `m` sans paramètre vaut `0` : remise à zéro.
      const codes = match[1] === "" ? [0] : match[1].split(";").map(Number);
      for (const code of codes) {
        if (code === 0) {
          color = "";
          bold = false;
        } else if (code === 1) {
          bold = true;
        } else if (code === 22) {
          bold = false;
        } else if (code === 39) {
          color = "";
        } else if (code in FOREGROUND) {
          color = FOREGROUND[code];
        }
      }
    }

    match = pattern.exec(line);
  }

  push(line.slice(cursor));
  return spans;
}

/** Retire toute séquence ANSI — pour la recherche et le copier-coller. */
export function stripAnsi(line: string): string {
  return line.replace(controlPattern(), "");
}

/** Niveau de journalisation reconnu en tête de ligne. */
export type LogLevel = "error" | "warn" | "info" | "debug" | "trace";

const LEVEL_PATTERNS: [LogLevel, RegExp][] = [
  ["error", /\b(ERROR|ERREUR|FATAL|SEVERE|PANIC)\b/],
  ["warn", /\b(WARN|WARNING|ATTENTION)\b/],
  ["info", /\bINFO\b/],
  ["debug", /\bDEBUG\b/],
  ["trace", /\bTRACE\b/],
];

/**
 * Niveau d'une ligne, ou `null`.
 *
 * Sert à teinter la ligne entière : dans un flot de logs, repérer les erreurs à
 * la couleur est bien plus rapide qu'à la lecture.
 */
export function detectLevel(line: string): LogLevel | null {
  // On ne regarde que le début de la ligne : le mot « ERROR » cité au milieu
  // d'un message ne fait pas de ce message une erreur.
  const head = stripAnsi(line).slice(0, 120);
  for (const [level, pattern] of LEVEL_PATTERNS) {
    if (pattern.test(head)) return level;
  }
  return null;
}

export const LEVEL_CLASS: Record<LogLevel, string> = {
  error: "text-(--color-danger)",
  warn: "text-(--color-warning)",
  info: "text-(--color-text-soft)",
  debug: "text-(--color-muted)",
  trace: "text-(--color-muted-soft)",
};
