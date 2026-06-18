import React, { useEffect } from "react";
import ReactDOM from "react-dom/client";
import CssBaseline from "@mui/material/CssBaseline";
import { ThemeProvider, useColorScheme } from "@mui/material/styles";
import { theme } from "./theme";
import { I18nProvider } from "./i18n";
import { AppProvider } from "./AppContext";
import { PrefsProvider, usePrefs } from "./prefs";
import App from "./App";

// Push the persisted theme (UI-1, from satchel.json) into MUI's color scheme
// once prefs have loaded. Lives inside ThemeProvider so `useColorScheme` works.
function ThemeSync() {
  const { setMode } = useColorScheme();
  const { prefs, loaded } = usePrefs();
  useEffect(() => {
    if (loaded) setMode(prefs.theme);
  }, [loaded, prefs.theme, setMode]);
  return null;
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <PrefsProvider>
      <ThemeProvider theme={theme} defaultMode="system">
        <CssBaseline enableColorScheme />
        <ThemeSync />
        <I18nProvider>
          <AppProvider>
            <App />
          </AppProvider>
        </I18nProvider>
      </ThemeProvider>
    </PrefsProvider>
  </React.StrictMode>,
);
