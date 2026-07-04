import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";

// The manual Cashrate (issue #56) — user-entered FX anchors, one per coin, from
// which every "~Cash" figure in the UI derives. Deliberately manual and
// currency-NEUTRAL: the user thinks in whatever money they think in (EUR, USD,
// RMB, …) and Satchel never names it — BTCX is unlisted (no API could price
// it), the UI makes zero external calls, and the rate is "your rate, for
// reference", never a market feed. Auto-filling from a live price stream is a
// future extension only (it would need Tor + multi-source aggregation to keep
// the privacy posture).
//
// The rate entry lives in the sidebar (above Settings) and binds to the coin
// the current screen is about — the quote coin of the Corkboard/offer-form
// pair — via useFxContext. Rates are remembered per coin across sessions, so
// switching a BTCX/BTC board to BTCX/LTC recalls your LTC rate while your BTC
// rate stays stored. Like the denom toggle this is a pure view preference, so
// it persists in the webview's localStorage (see denom.tsx for the rationale).

const ENABLED_KEY = "satchel.fx.enabled";
const RATES_KEY = "satchel.fx.rates"; // JSON: coin id → canonical dot-decimal
const LAST_COIN_KEY = "satchel.fx.lastCoin"; // widget label fallback on boot

function loadEnabled(): boolean {
  try {
    return localStorage.getItem(ENABLED_KEY) === "1";
  } catch {
    return false; // no localStorage (SSR / locked-down webview) — default off
  }
}

function loadRates(): Record<string, string> {
  try {
    const raw = localStorage.getItem(RATES_KEY);
    if (!raw) return {};
    const obj = JSON.parse(raw) as unknown;
    if (obj && typeof obj === "object") {
      const out: Record<string, string> = {};
      for (const [k, v] of Object.entries(obj as Record<string, unknown>)) {
        if (typeof v === "string" && isFinite(parseFloat(v)) && parseFloat(v) > 0) out[k] = v;
      }
      return out;
    }
  } catch {
    /* no localStorage / corrupt JSON — fall through */
  }
  return {};
}

function loadLastCoin(): string {
  try {
    return localStorage.getItem(LAST_COIN_KEY) || "btc";
  } catch {
    return "btc";
  }
}

interface FxCtx {
  /** The opt-in switch — no ~Cash renders anywhere while off (the default). */
  enabled: boolean;
  setEnabled: (on: boolean) => void;
  /** Coin the current screen binds the rate entry to (quote coin of the pair
   *  in view), or null on screens with no coin context (widget greys out). */
  context: string | null;
  setContext: (coin: string | null) => void;
  /** Last coin that had context — keeps the widget's label/value meaningful
   *  (though disabled) on context-less screens. Survives sessions. */
  lastCoin: string;
  /** Per-coin remembered rates (canonical dot-decimal strings). */
  rates: Record<string, string>;
  /** Store/clear a coin's rate ("" clears). `s` must be canonical dot-decimal. */
  setRate: (coin: string, s: string) => void;
  /** A coin's rate as a number — null when unset/invalid (renders "—"). */
  rateOf: (coin: string) => number | null;
}

const Ctx = createContext<FxCtx | null>(null);

export function FxProvider({ children }: { children: ReactNode }) {
  const [enabled, setEnabledState] = useState<boolean>(loadEnabled);
  const [rates, setRates] = useState<Record<string, string>>(loadRates);
  const [context, setContextState] = useState<string | null>(null);
  const [lastCoin, setLastCoin] = useState<string>(loadLastCoin);

  const setEnabled = useCallback((on: boolean) => {
    setEnabledState(on);
    try {
      localStorage.setItem(ENABLED_KEY, on ? "1" : "0");
    } catch {
      /* best-effort persist */
    }
  }, []);

  const setContext = useCallback((coin: string | null) => {
    setContextState(coin);
    if (coin) {
      setLastCoin(coin);
      try {
        localStorage.setItem(LAST_COIN_KEY, coin);
      } catch {
        /* best-effort persist */
      }
    }
  }, []);

  const setRate = useCallback((coin: string, s: string) => {
    setRates((cur) => {
      const next = { ...cur };
      if (s) next[coin] = s;
      else delete next[coin];
      try {
        localStorage.setItem(RATES_KEY, JSON.stringify(next));
      } catch {
        /* best-effort persist */
      }
      return next;
    });
  }, []);

  const rateOf = useCallback(
    (coin: string): number | null => {
      const n = parseFloat(rates[coin] ?? "");
      return isFinite(n) && n > 0 ? n : null;
    },
    [rates],
  );

  const value = useMemo(
    () => ({ enabled, setEnabled, context, setContext, lastCoin, rates, setRate, rateOf }),
    [enabled, setEnabled, context, setContext, lastCoin, rates, setRate, rateOf],
  );

  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function useFx(): FxCtx {
  const c = useContext(Ctx);
  if (!c) throw new Error("useFx outside FxProvider");
  return c;
}

/** Bind the sidebar rate entry to a coin while the calling screen is mounted —
 *  pass the quote coin of the pair in view (or null/"" when none is selected).
 *  Cleared on unmount, so context-less screens grey the widget out. */
export function useFxContext(coin?: string | null) {
  const { setContext } = useFx();
  const c = coin || null;
  useEffect(() => {
    setContext(c);
    return () => setContext(null);
  }, [c, setContext]);
}
