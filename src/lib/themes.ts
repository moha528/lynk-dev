/** Loose terminal palette type (xterm.js ITheme-compatible), kept for modules
 * that may embed a terminal later. */
export type TerminalPalette = { [key: string]: string | undefined };

/**
 * Application themes that drive both:
 * - the entire app chrome (sidebar, dialogs, tabs, panels …) via CSS variables
 * - the embedded xterm.js terminals via `terminal`
 *
 * Switching theme calls [`applyAppTheme`] which writes every `app.*` entry
 * onto `document.documentElement`, so React only re-renders the parts that
 * read the CSS variables (which is essentially every component using
 * `text-(--color-*)` / `bg-(--color-*)` Tailwind classes).
 */

export type ThemeId =
  | "catppuccin-mocha"
  | "catppuccin-latte"
  | "tokyo-night"
  | "dracula"
  | "solarized-dark"
  | "solarized-light"
  | "gruvbox-dark";

export type AppPalette = {
  /** Outermost surface (window body). */
  "--color-bg": string;
  /** Slightly raised surface (sidebar background). */
  "--color-bg-soft": string;
  /** Header bars, popups. */
  "--color-panel": string;
  /** Hover state over `--color-panel`. */
  "--color-panel-hover": string;
  /** Highest elevation (menus). */
  "--color-elevated": string;
  /** Thin dividers. */
  "--color-border": string;
  /** Stronger dividers for floating surfaces. */
  "--color-border-strong": string;
  /** Primary foreground text. */
  "--color-text": string;
  /** Slightly de-emphasised text. */
  "--color-text-soft": string;
  /** Secondary text (labels, captions). */
  "--color-muted": string;
  /** Hint text, placeholders. */
  "--color-muted-soft": string;
  /** Brand accent (primary buttons, focus rings). */
  "--color-accent": string;
  /** Darker accent for borders and hovers. */
  "--color-accent-soft": string;
  /** Translucent accent background (chips, selection). */
  "--color-accent-bg": string;
  /** Status colors. */
  "--color-success": string;
  "--color-warning": string;
  "--color-danger": string;
};

export type AppTheme = {
  id: ThemeId;
  name: string;
  /** "dark" sets the OS title bar dark mode (DwmSetWindowAttribute on Win). */
  appearance: "dark" | "light";
  app: AppPalette;
  terminal: TerminalPalette;
};

export const TERMINAL_THEMES: Record<ThemeId, AppTheme> = {
  "catppuccin-mocha": {
    id: "catppuccin-mocha",
    name: "Catppuccin Mocha",
    appearance: "dark",
    app: {
      "--color-bg": "#1e1e2e",
      "--color-bg-soft": "#181825",
      "--color-panel": "#27273a",
      "--color-panel-hover": "#313244",
      "--color-elevated": "#45475a",
      "--color-border": "#313244",
      "--color-border-strong": "#45475a",
      "--color-text": "#cdd6f4",
      "--color-text-soft": "#bac2de",
      "--color-muted": "#9399b2",
      "--color-muted-soft": "#7f849c",
      "--color-accent": "#89b4fa",
      "--color-accent-soft": "#74a8d9",
      "--color-accent-bg": "#1e283d",
      "--color-success": "#a6e3a1",
      "--color-warning": "#f9e2af",
      "--color-danger": "#f38ba8",
    },
    terminal: {
      background: "#1e1e2e",
      foreground: "#cdd6f4",
      cursor: "#f5e0dc",
      cursorAccent: "#1e1e2e",
      selectionBackground: "#585b7080",
      black: "#45475a",
      red: "#f38ba8",
      green: "#a6e3a1",
      yellow: "#f9e2af",
      blue: "#89b4fa",
      magenta: "#f5c2e7",
      cyan: "#94e2d5",
      white: "#bac2de",
      brightBlack: "#585b70",
      brightRed: "#f38ba8",
      brightGreen: "#a6e3a1",
      brightYellow: "#f9e2af",
      brightBlue: "#89b4fa",
      brightMagenta: "#f5c2e7",
      brightCyan: "#94e2d5",
      brightWhite: "#a6adc8",
    },
  },
  "catppuccin-latte": {
    id: "catppuccin-latte",
    name: "Catppuccin Latte",
    appearance: "light",
    app: {
      "--color-bg": "#eff1f5",
      "--color-bg-soft": "#e6e9ef",
      "--color-panel": "#ccd0da",
      "--color-panel-hover": "#bcc0cc",
      "--color-elevated": "#acb0be",
      "--color-border": "#ccd0da",
      "--color-border-strong": "#acb0be",
      "--color-text": "#4c4f69",
      "--color-text-soft": "#5c5f77",
      "--color-muted": "#6c6f85",
      "--color-muted-soft": "#8c8fa1",
      "--color-accent": "#1e66f5",
      "--color-accent-soft": "#1953c4",
      "--color-accent-bg": "#cad7f7",
      "--color-success": "#40a02b",
      "--color-warning": "#df8e1d",
      "--color-danger": "#d20f39",
    },
    terminal: {
      background: "#eff1f5",
      foreground: "#4c4f69",
      cursor: "#dc8a78",
      cursorAccent: "#eff1f5",
      selectionBackground: "#acb0be80",
      black: "#5c5f77",
      red: "#d20f39",
      green: "#40a02b",
      yellow: "#df8e1d",
      blue: "#1e66f5",
      magenta: "#ea76cb",
      cyan: "#179299",
      white: "#acb0be",
      brightBlack: "#6c6f85",
      brightRed: "#d20f39",
      brightGreen: "#40a02b",
      brightYellow: "#df8e1d",
      brightBlue: "#1e66f5",
      brightMagenta: "#ea76cb",
      brightCyan: "#179299",
      brightWhite: "#bcc0cc",
    },
  },
  "tokyo-night": {
    id: "tokyo-night",
    name: "Tokyo Night",
    appearance: "dark",
    app: {
      "--color-bg": "#1a1b26",
      "--color-bg-soft": "#1f2030",
      "--color-panel": "#24283b",
      "--color-panel-hover": "#2e334d",
      "--color-elevated": "#3b425a",
      "--color-border": "#292e42",
      "--color-border-strong": "#3b425a",
      "--color-text": "#c0caf5",
      "--color-text-soft": "#a9b1d6",
      "--color-muted": "#7882ab",
      "--color-muted-soft": "#565f89",
      "--color-accent": "#7aa2f7",
      "--color-accent-soft": "#5e7ec9",
      "--color-accent-bg": "#2a3656",
      "--color-success": "#9ece6a",
      "--color-warning": "#e0af68",
      "--color-danger": "#f7768e",
    },
    terminal: {
      background: "#1a1b26",
      foreground: "#c0caf5",
      cursor: "#c0caf5",
      black: "#15161e",
      red: "#f7768e",
      green: "#9ece6a",
      yellow: "#e0af68",
      blue: "#7aa2f7",
      magenta: "#bb9af7",
      cyan: "#7dcfff",
      white: "#a9b1d6",
      brightBlack: "#414868",
      brightRed: "#f7768e",
      brightGreen: "#9ece6a",
      brightYellow: "#e0af68",
      brightBlue: "#7aa2f7",
      brightMagenta: "#bb9af7",
      brightCyan: "#7dcfff",
      brightWhite: "#c0caf5",
    },
  },
  dracula: {
    id: "dracula",
    name: "Dracula",
    appearance: "dark",
    app: {
      "--color-bg": "#282a36",
      "--color-bg-soft": "#2e3142",
      "--color-panel": "#343746",
      "--color-panel-hover": "#44475a",
      "--color-elevated": "#4c4f63",
      "--color-border": "#44475a",
      "--color-border-strong": "#565973",
      "--color-text": "#f8f8f2",
      "--color-text-soft": "#e0e0d8",
      "--color-muted": "#8895c2",
      "--color-muted-soft": "#6272a4",
      "--color-accent": "#bd93f9",
      "--color-accent-soft": "#9d7ad9",
      "--color-accent-bg": "#44345f",
      "--color-success": "#50fa7b",
      "--color-warning": "#f1fa8c",
      "--color-danger": "#ff5555",
    },
    terminal: {
      background: "#282a36",
      foreground: "#f8f8f2",
      cursor: "#f8f8f0",
      black: "#21222c",
      red: "#ff5555",
      green: "#50fa7b",
      yellow: "#f1fa8c",
      blue: "#bd93f9",
      magenta: "#ff79c6",
      cyan: "#8be9fd",
      white: "#f8f8f2",
      brightBlack: "#6272a4",
      brightRed: "#ff6e6e",
      brightGreen: "#69ff94",
      brightYellow: "#ffffa5",
      brightBlue: "#d6acff",
      brightMagenta: "#ff92df",
      brightCyan: "#a4ffff",
      brightWhite: "#ffffff",
    },
  },
  "solarized-dark": {
    id: "solarized-dark",
    name: "Solarized Dark",
    appearance: "dark",
    app: {
      "--color-bg": "#002b36",
      "--color-bg-soft": "#073642",
      "--color-panel": "#0b4654",
      "--color-panel-hover": "#0d5667",
      "--color-elevated": "#14687c",
      "--color-border": "#135263",
      "--color-border-strong": "#1c6580",
      "--color-text": "#fdf6e3",
      "--color-text-soft": "#eee8d5",
      "--color-muted": "#93a1a1",
      "--color-muted-soft": "#586e75",
      "--color-accent": "#268bd2",
      "--color-accent-soft": "#1d6c9e",
      "--color-accent-bg": "#103e5c",
      "--color-success": "#859900",
      "--color-warning": "#b58900",
      "--color-danger": "#dc322f",
    },
    terminal: {
      background: "#002b36",
      foreground: "#839496",
      cursor: "#93a1a1",
      black: "#073642",
      red: "#dc322f",
      green: "#859900",
      yellow: "#b58900",
      blue: "#268bd2",
      magenta: "#d33682",
      cyan: "#2aa198",
      white: "#eee8d5",
      brightBlack: "#586e75",
      brightRed: "#cb4b16",
      brightGreen: "#586e75",
      brightYellow: "#657b83",
      brightBlue: "#839496",
      brightMagenta: "#6c71c4",
      brightCyan: "#93a1a1",
      brightWhite: "#fdf6e3",
    },
  },
  "solarized-light": {
    id: "solarized-light",
    name: "Solarized Light",
    appearance: "light",
    app: {
      "--color-bg": "#fdf6e3",
      "--color-bg-soft": "#f5eed1",
      "--color-panel": "#eee8d5",
      "--color-panel-hover": "#e0d8b0",
      "--color-elevated": "#d6cd9d",
      "--color-border": "#d6cd9d",
      "--color-border-strong": "#b8aa6c",
      "--color-text": "#073642",
      "--color-text-soft": "#1b4753",
      "--color-muted": "#586e75",
      "--color-muted-soft": "#93a1a1",
      "--color-accent": "#268bd2",
      "--color-accent-soft": "#1d6c9e",
      "--color-accent-bg": "#cee5f3",
      "--color-success": "#859900",
      "--color-warning": "#b58900",
      "--color-danger": "#dc322f",
    },
    terminal: {
      background: "#fdf6e3",
      foreground: "#657b83",
      cursor: "#586e75",
      black: "#073642",
      red: "#dc322f",
      green: "#859900",
      yellow: "#b58900",
      blue: "#268bd2",
      magenta: "#d33682",
      cyan: "#2aa198",
      white: "#eee8d5",
      brightBlack: "#002b36",
      brightRed: "#cb4b16",
      brightGreen: "#586e75",
      brightYellow: "#657b83",
      brightBlue: "#839496",
      brightMagenta: "#6c71c4",
      brightCyan: "#93a1a1",
      brightWhite: "#fdf6e3",
    },
  },
  "gruvbox-dark": {
    id: "gruvbox-dark",
    name: "Gruvbox Dark",
    appearance: "dark",
    app: {
      "--color-bg": "#282828",
      "--color-bg-soft": "#32302f",
      "--color-panel": "#3c3836",
      "--color-panel-hover": "#504945",
      "--color-elevated": "#665c54",
      "--color-border": "#504945",
      "--color-border-strong": "#665c54",
      "--color-text": "#ebdbb2",
      "--color-text-soft": "#d5c4a1",
      "--color-muted": "#a89984",
      "--color-muted-soft": "#7c6f64",
      "--color-accent": "#fabd2f",
      "--color-accent-soft": "#d79921",
      "--color-accent-bg": "#503e2f",
      "--color-success": "#b8bb26",
      "--color-warning": "#fe8019",
      "--color-danger": "#fb4934",
    },
    terminal: {
      background: "#282828",
      foreground: "#ebdbb2",
      cursor: "#ebdbb2",
      black: "#282828",
      red: "#cc241d",
      green: "#98971a",
      yellow: "#d79921",
      blue: "#458588",
      magenta: "#b16286",
      cyan: "#689d6a",
      white: "#a89984",
      brightBlack: "#928374",
      brightRed: "#fb4934",
      brightGreen: "#b8bb26",
      brightYellow: "#fabd2f",
      brightBlue: "#83a598",
      brightMagenta: "#d3869b",
      brightCyan: "#8ec07c",
      brightWhite: "#ebdbb2",
    },
  },
};

export const DEFAULT_THEME: ThemeId = "catppuccin-mocha";

/**
 * Push every CSS variable from the active theme onto `:root` so the entire
 * app re-renders with the new palette. Idempotent.
 */
export function applyAppTheme(id: string | undefined): void {
  const theme = TERMINAL_THEMES[(id as ThemeId) ?? DEFAULT_THEME] ?? TERMINAL_THEMES[DEFAULT_THEME];
  const root = document.documentElement;
  for (const [k, v] of Object.entries(theme.app)) {
    root.style.setProperty(k, v);
  }
  root.dataset.theme = theme.id;
  root.classList.toggle("theme-light", theme.appearance === "light");
}
