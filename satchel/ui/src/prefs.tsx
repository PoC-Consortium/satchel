import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { getUiPrefs, inTauri, setUiPrefs } from "./api/tauri";
import type { UiPrefs } from "./api/types";

// Per-install UI preferences (UI-1). These used to live in the webview's
// localStorage; they now live in satchel.json so the UI persists nothing of its
// own (the same "Satchel owns config" principle C10 applies to merchants).
//
// Flow: load once from `get_ui_prefs` on mount (defaults outside Tauri / before
// the call returns), and write through `set_ui_prefs` on every change. Theme +
// language consumers (MUI color scheme, i18n) sync off `prefs` once it loads.

const DEFAULTS: UiPrefs = {
  theme: "system",
  language: "en",
  nav_open: true,
  onboarded: false,
  offer_ttl_min: 60,
  notify: {
    enabled: true,
    swap_started: true,
    locks: true,
    completed: true,
    failed: true,
    reorg: true,
  },
};

interface PrefsCtx {
  prefs: UiPrefs;
  /** True once the persisted prefs have been read (so consumers can wait before
   *  syncing the MUI color scheme / language and avoid a flash of the default). */
  loaded: boolean;
  update: (patch: Partial<UiPrefs>) => void;
}

const Ctx = createContext<PrefsCtx | null>(null);

export function PrefsProvider({ children }: { children: ReactNode }) {
  const [prefs, setPrefs] = useState<UiPrefs>(DEFAULTS);
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    let alive = true;
    (async () => {
      if (!inTauri()) {
        // Plain-browser dev: no Satchel backend — keep the in-memory defaults.
        if (alive) setLoaded(true);
        return;
      }
      try {
        const p = await getUiPrefs();
        if (alive && p) setPrefs({ ...DEFAULTS, ...p });
      } catch {
        /* keep defaults */
      } finally {
        if (alive) setLoaded(true);
      }
    })();
    return () => {
      alive = false;
    };
  }, []);

  const update = useCallback((patch: Partial<UiPrefs>) => {
    setPrefs((prev) => ({ ...prev, ...patch }));
    // Best-effort persist; the in-memory value already updated the UI.
    if (inTauri()) void setUiPrefs(patch).catch(() => {});
  }, []);

  const value = useMemo(() => ({ prefs, loaded, update }), [prefs, loaded, update]);
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function usePrefs(): PrefsCtx {
  const c = useContext(Ctx);
  if (!c) throw new Error("usePrefs outside PrefsProvider");
  return c;
}
