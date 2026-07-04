import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from "react";

// The manual USD reference (issue #56) — ONE user-entered FX anchor (USD per
// 1 BTC) from which every USD-equivalent shown in the UI is derived. It is
// deliberately manual and display-only: BTCX is unlisted (no API can price it,
// so no feed could honestly price the flagship BTCX↔BTC pair) and the UI makes
// zero external calls — the anchor is "your rate, for reference", never a
// market feed. Auto-filling it from a live price stream is a future extension
// only (it would need Tor + multi-source aggregation to keep the privacy
// posture). Like the denom toggle this is a pure view preference, so it
// persists in the webview's localStorage (see denom.tsx for the rationale).

const ENABLED_KEY = "satchel.fx.enabled";
const ANCHOR_KEY = "satchel.fx.usdPerBtc"; // canonical dot-decimal string

function loadEnabled(): boolean {
  try {
    return localStorage.getItem(ENABLED_KEY) === "1";
  } catch {
    return false; // no localStorage (SSR / locked-down webview) — default off
  }
}

function loadAnchor(): string {
  try {
    const v = localStorage.getItem(ANCHOR_KEY);
    if (v && isFinite(parseFloat(v)) && parseFloat(v) > 0) return v;
  } catch {
    /* no localStorage — fall through */
  }
  return "";
}

interface FxCtx {
  /** The opt-in switch — no USD renders anywhere while off (the default). */
  enabled: boolean;
  setEnabled: (on: boolean) => void;
  /** Canonical dot-decimal anchor string ("" = unset), for the Settings field. */
  anchor: string;
  setAnchor: (s: string) => void;
  /** USD per 1 BTC — null while unset/invalid (displays degrade to "—"). */
  usdPerBtc: number | null;
}

const Ctx = createContext<FxCtx | null>(null);

export function FxProvider({ children }: { children: ReactNode }) {
  const [enabled, setEnabledState] = useState<boolean>(loadEnabled);
  const [anchor, setAnchorState] = useState<string>(loadAnchor);

  const setEnabled = useCallback((on: boolean) => {
    setEnabledState(on);
    try {
      localStorage.setItem(ENABLED_KEY, on ? "1" : "0");
    } catch {
      /* best-effort persist */
    }
  }, []);

  const setAnchor = useCallback((s: string) => {
    setAnchorState(s);
    try {
      if (s) localStorage.setItem(ANCHOR_KEY, s);
      else localStorage.removeItem(ANCHOR_KEY);
    } catch {
      /* best-effort persist */
    }
  }, []);

  const value = useMemo(() => {
    const n = parseFloat(anchor);
    return {
      enabled,
      setEnabled,
      anchor,
      setAnchor,
      usdPerBtc: isFinite(n) && n > 0 ? n : null,
    };
  }, [enabled, setEnabled, anchor, setAnchor]);

  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function useFx(): FxCtx {
  const c = useContext(Ctx);
  if (!c) throw new Error("useFx outside FxProvider");
  return c;
}
