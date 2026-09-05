import { createHighlighterCore } from "shiki/core";
import type { HighlighterCore, LanguageInput, ThemeInput, ThemedToken } from "shiki/core";
import { createOnigurumaEngine } from "shiki/engine/oniguruma";

import type { ThemeId } from "./themes";

/**
 * Coloration syntaxique partagée.
 *
 * **Une seule instance** pour toute l'application : chaque `createHighlighter`
 * recharge le moteur WebAssembly et les grammaires, ce qui coûte des dizaines
 * de mégaoctets et une bonne seconde. Les grammaires de langage sont chargées
 * **à la demande**, jamais toutes d'un coup.
 *
 * Les thèmes, eux, sont chargés d'emblée : ils sont minuscules, et l'utilisateur
 * peut changer de palette à tout moment depuis les réglages.
 */

/**
 * Correspondance thème de l'app → thème Shiki.
 *
 * Les sept palettes de Lynk Dev existent telles quelles dans le catalogue
 * Shiki : c'est ce qui a fait retenir Shiki plutôt qu'un autre surligneur — il
 * n'y a aucune palette à réinventer, et un diff prend exactement les couleurs
 * du reste de la fenêtre.
 */
const THEME_OF: Record<ThemeId, string> = {
  "catppuccin-mocha": "catppuccin-mocha",
  "catppuccin-latte": "catppuccin-latte",
  "tokyo-night": "tokyo-night",
  dracula: "dracula",
  "solarized-dark": "solarized-dark",
  "solarized-light": "solarized-light",
  "gruvbox-dark": "gruvbox-dark-medium",
};

const THEME_LOADERS: Record<string, () => ThemeInput> = {
  "catppuccin-mocha": () => import("shiki/themes/catppuccin-mocha.mjs"),
  "catppuccin-latte": () => import("shiki/themes/catppuccin-latte.mjs"),
  "tokyo-night": () => import("shiki/themes/tokyo-night.mjs"),
  dracula: () => import("shiki/themes/dracula.mjs"),
  "solarized-dark": () => import("shiki/themes/solarized-dark.mjs"),
  "solarized-light": () => import("shiki/themes/solarized-light.mjs"),
  "gruvbox-dark-medium": () => import("shiki/themes/gruvbox-dark-medium.mjs"),
};

/**
 * Grammaires disponibles, par identifiant Shiki.
 *
 * Une carte explicite plutôt qu'un import dynamique calculé : le bundler doit
 * savoir statiquement quels fichiers produire. Ajouter un langage = ajouter une
 * ligne ici et son extension dans [`LANGUAGE_OF_EXTENSION`].
 */
const LANGUAGE_LOADERS: Record<string, () => LanguageInput> = {
  bash: () => import("shiki/langs/bash.mjs"),
  csharp: () => import("shiki/langs/csharp.mjs"),
  css: () => import("shiki/langs/css.mjs"),
  docker: () => import("shiki/langs/docker.mjs"),
  go: () => import("shiki/langs/go.mjs"),
  html: () => import("shiki/langs/html.mjs"),
  ini: () => import("shiki/langs/ini.mjs"),
  java: () => import("shiki/langs/java.mjs"),
  javascript: () => import("shiki/langs/javascript.mjs"),
  json: () => import("shiki/langs/json.mjs"),
  jsx: () => import("shiki/langs/jsx.mjs"),
  kotlin: () => import("shiki/langs/kotlin.mjs"),
  markdown: () => import("shiki/langs/markdown.mjs"),
  php: () => import("shiki/langs/php.mjs"),
  python: () => import("shiki/langs/python.mjs"),
  ruby: () => import("shiki/langs/ruby.mjs"),
  rust: () => import("shiki/langs/rust.mjs"),
  scss: () => import("shiki/langs/scss.mjs"),
  sql: () => import("shiki/langs/sql.mjs"),
  toml: () => import("shiki/langs/toml.mjs"),
  tsx: () => import("shiki/langs/tsx.mjs"),
  typescript: () => import("shiki/langs/typescript.mjs"),
  xml: () => import("shiki/langs/xml.mjs"),
  yaml: () => import("shiki/langs/yaml.mjs"),
};

const LANGUAGE_OF_EXTENSION: Record<string, string> = {
  ts: "typescript",
  mts: "typescript",
  cts: "typescript",
  tsx: "tsx",
  js: "javascript",
  mjs: "javascript",
  cjs: "javascript",
  jsx: "jsx",
  json: "json",
  jsonc: "json",
  rs: "rust",
  java: "java",
  kt: "kotlin",
  kts: "kotlin",
  py: "python",
  go: "go",
  rb: "ruby",
  php: "php",
  cs: "csharp",
  sql: "sql",
  yml: "yaml",
  yaml: "yaml",
  toml: "toml",
  xml: "xml",
  html: "html",
  htm: "html",
  css: "css",
  scss: "scss",
  sass: "scss",
  md: "markdown",
  markdown: "markdown",
  sh: "bash",
  bash: "bash",
  zsh: "bash",
  env: "ini",
  ini: "ini",
  properties: "ini",
  conf: "ini",
};

/** Fichiers reconnus à leur nom plutôt qu'à leur extension. */
const LANGUAGE_OF_FILENAME: Record<string, string> = {
  dockerfile: "docker",
  ".env": "ini",
  ".gitignore": "ini",
  makefile: "bash",
  "cargo.lock": "toml",
  "cargo.toml": "toml",
};

/** Langage Shiki d'un chemin, ou `null` si on ne sait pas le colorer. */
export function languageOf(path: string): string | null {
  const file = path.split(/[\\/]/).pop()?.toLowerCase() ?? "";
  if (file in LANGUAGE_OF_FILENAME) return LANGUAGE_OF_FILENAME[file];
  // `.env.local` et `.env.production` comptent aussi.
  if (file.startsWith(".env")) return "ini";

  const extension = file.includes(".") ? file.split(".").pop() : undefined;
  if (!extension) return null;
  return LANGUAGE_OF_EXTENSION[extension] ?? null;
}

let instance: Promise<HighlighterCore> | null = null;
const loadedLanguages = new Set<string>();

function highlighter(): Promise<HighlighterCore> {
  if (!instance) {
    instance = createHighlighterCore({
      themes: Object.values(THEME_LOADERS).map((load) => load()),
      langs: [],
      engine: createOnigurumaEngine(import("shiki/wasm")),
    });
  }
  return instance;
}

async function ensureLanguage(core: HighlighterCore, language: string): Promise<boolean> {
  if (loadedLanguages.has(language)) return true;
  const load = LANGUAGE_LOADERS[language];
  if (!load) return false;
  try {
    await core.loadLanguage(load());
    loadedLanguages.add(language);
    return true;
  } catch (error) {
    console.warn(`shiki: grammaire ${language} indisponible`, error);
    return false;
  }
}

export type Line = ThemedToken[];

/**
 * Découpe `code` en lignes de jetons colorés.
 *
 * Rend `null` quand le langage est inconnu ou la grammaire absente : l'appelant
 * affiche alors le texte brut. **Ne jamais faire échouer un affichage à cause
 * d'une coloration** — un fichier illisible est pire qu'un fichier sans
 * couleurs.
 */
export async function highlightLines(
  code: string,
  language: string | null,
  theme: ThemeId,
): Promise<Line[] | null> {
  if (!language) return null;
  try {
    const core = await highlighter();
    if (!(await ensureLanguage(core, language))) return null;
    const { tokens } = core.codeToTokens(code, {
      lang: language,
      theme: THEME_OF[theme],
    });
    return tokens;
  } catch (error) {
    console.warn("shiki: coloration impossible", error);
    return null;
  }
}
