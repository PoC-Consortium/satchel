import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from "react";
import type { Denom } from "./format";

// The display-unit (denomination) preference — see format.ts. It is view-only
// (never touches amounts, which stay in sats), so unlike the daemon-level UI
// prefs in satchel.json (UI-1) it persists in the webview's localStorage. That
// keeps it a pure frontend concern; promoting it into satchel.json later is a
// one-field addition to get_ui_prefs / set_ui_prefs.

const KEY = "satchel.denom";
const VALID: Denom[] = ["coin", "milli", "micro", "sat"];

function load(): Denom {
  try {
    const v = localStorage.getItem(KEY) as Denom | null;
    if (v && VALID.includes(v)) return v;
  } catch {
    /* no localStorage (SSR / locked-down webview) — fall through */
  }
  return "coin";
}

interface DenomCtx {
  denom: Denom;
  setDenom: (d: Denom) => void;
}

const Ctx = createContext<DenomCtx | null>(null);

export function DenomProvider({ children }: { children: ReactNode }) {
  const [denom, set] = useState<Denom>(load);
  const setDenom = useCallback((d: Denom) => {
    set(d);
    try {
      localStorage.setItem(KEY, d);
    } catch {
      /* best-effort persist */
    }
  }, []);
  const value = useMemo(() => ({ denom, setDenom }), [denom, setDenom]);
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function useDenom(): DenomCtx {
  const c = useContext(Ctx);
  if (!c) throw new Error("useDenom outside DenomProvider");
  return c;
}
