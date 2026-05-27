import { useEffect, useMemo } from "react";
import { Toaster } from "sonner";

import { MainLayout } from "./components/MainLayout";
import { DEFAULT_THEME, TERMINAL_THEMES, type ThemeId, applyAppTheme } from "./lib/themes";
import { useSettingsStore } from "./stores/useSettingsStore";
import "./App.css";

function App() {
  const appTheme = useSettingsStore((s) => s.appTheme);

  useEffect(() => {
    applyAppTheme(appTheme);
  }, [appTheme]);

  // Drive the toaster theme from the active app palette so toasts always
  // match the surrounding UI without us having to restyle each variant.
  const toastTheme = useMemo<"dark" | "light">(() => {
    const palette =
      TERMINAL_THEMES[(appTheme as ThemeId) ?? DEFAULT_THEME] ?? TERMINAL_THEMES[DEFAULT_THEME];
    return palette.appearance;
  }, [appTheme]);

  return (
    <>
      <MainLayout />
      <Toaster
        theme={toastTheme}
        position="bottom-right"
        richColors
        closeButton
        toastOptions={{
          style: {
            background: "var(--color-elevated)",
            color: "var(--color-text)",
            border: "1px solid var(--color-border-strong)",
            fontSize: "13px",
          },
        }}
      />
    </>
  );
}

export default App;
