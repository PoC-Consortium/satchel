import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { errMsg, inTauri, listMerchants, rpc } from "./api/tauri";
import { adaptorToSwap, pendingTakeToSwap, v1ToSwap } from "./format";
import type {
  AdaptorSwapRecord,
  CoinInfo,
  Info,
  Merchant,
  PendingTake,
  RelayStatus,
  Swap,
  V1SwapRecord,
} from "./api/types";

// The boot/connection state machine, ported from index.html `boot()`:
//   no-tauri  — not inside Satchel's webview (dev in a plain browser)
//   wizard    — no active merchant → first-run wizard
//   seed      — merchant active but its seed isn't provisioned yet
//   unlock    — encrypted merchant came up locked
//   disconnected — pactd unreachable / getinfo failed
//   ready     — connected, seed present + unlocked: the app is usable
export type Phase = "loading" | "no-tauri" | "wizard" | "seed" | "unlock" | "disconnected" | "ready";

export interface LogLine {
  time: string;
  msg: string;
}

interface AppCtx {
  phase: Phase;
  ready: boolean;
  connOk: boolean;
  setConn: (ok: boolean) => void;

  merchants: Merchant[];
  activeId: string | null;
  activeMerchant: Merchant | null;

  info: Info | null;
  identity: string | null;
  network: string | null;

  swaps: Swap[];
  refreshSwaps: () => Promise<void>;

  /** Live `listcoins` (registry + configured + probed status), shared so the
   *  header status row + screens don't each re-poll. Empty until connected. */
  coins: CoinInfo[];
  refreshCoins: () => Promise<void>;

  /** Nostr relay connectivity (pactd `boardstatus`), for the header indicator.
   *  Empty ⇒ no relays configured (header hides the dot). */
  relays: RelayStatus[];

  /** coin_id → symbol, shared so the board can label legs without a node probe. */
  symbols: Record<string, string>;
  setSymbol: (id: string, symbol: string) => void;
  symOf: (id: string) => string;

  boot: () => Promise<void>;

  logLines: LogLine[];
  log: (msg: string) => void;

  toast: string | null;
  showToast: (msg: string) => void;
  clearToast: () => void;
}

const Ctx = createContext<AppCtx | null>(null);

export function useApp(): AppCtx {
  const c = useContext(Ctx);
  if (!c) throw new Error("useApp outside AppProvider");
  return c;
}

export function AppProvider({ children }: { children: ReactNode }) {
  const [phase, setPhase] = useState<Phase>("loading");
  const [connOk, setConnOk] = useState(false);
  const [merchants, setMerchants] = useState<Merchant[]>([]);
  const [activeId, setActiveId] = useState<string | null>(null);
  const [info, setInfo] = useState<Info | null>(null);
  const [identity, setIdentity] = useState<string | null>(null);
  const [network, setNetwork] = useState<string | null>(null);
  const [swaps, setSwaps] = useState<Swap[]>([]);
  const [coins, setCoins] = useState<CoinInfo[]>([]);
  const [relays, setRelays] = useState<RelayStatus[]>([]);
  const [symbols, setSymbols] = useState<Record<string, string>>({});
  const [logLines, setLogLines] = useState<LogLine[]>([]);
  const [toast, setToast] = useState<string | null>(null);

  const log = useCallback((msg: string) => {
    const time = new Date().toLocaleTimeString();
    setLogLines((prev) => [{ time, msg }, ...prev].slice(0, 200));
    setToast(msg);
  }, []);

  const showToast = useCallback((msg: string) => setToast(msg), []);
  const clearToast = useCallback(() => setToast(null), []);
  const setConn = useCallback((ok: boolean) => setConnOk(ok), []);

  const setSymbol = useCallback(
    (id: string, symbol: string) => setSymbols((m) => (m[id] === symbol ? m : { ...m, [id]: symbol })),
    [],
  );
  const symbolsRef = useRef(symbols);
  symbolsRef.current = symbols;
  const symOf = useCallback((id: string) => symbolsRef.current[id] || String(id).toUpperCase(), []);

  const ready = phase === "ready";

  // listswaps drives both the Swaps tab and the header's completed count, so it
  // polls globally (not per-tab) while we're connected — as in the old UI.
  // v2 (adaptor) swaps live in a separate pactd table; pending takes (post-take,
  // pre-record) in another. We fold all three into one array so the ledger,
  // header count, and active-swaps dock cover every protocol + the "initiating"
  // pre-swap. A pending take is dropped once its real record exists (same id).
  const refreshSwaps = useCallback(async () => {
    try {
      const [v1, v2, pend] = await Promise.all([
        rpc<V1SwapRecord[]>("listswaps"),
        rpc<AdaptorSwapRecord[]>("listadaptorswaps"),
        rpc<PendingTake[]>("listpendingtakes"),
      ]);
      const real = [...v1.map(v1ToSwap), ...v2.map(adaptorToSwap)];
      const realIds = new Set(real.map((s) => s.swap_id));
      const pending = pend.filter((p) => !realIds.has(p.offer_id)).map(pendingTakeToSwap);
      setSwaps([...real, ...pending]);
      setConn(true);
    } catch {
      setConn(false);
    }
  }, [setConn]);

  // listcoins drives the header per-coin health glyphs (live-probed status +
  // tip). Polled globally while connected so the indicators stay fresh on any
  // tab; coin symbols are cached for leg labels.
  const refreshCoins = useCallback(async () => {
    try {
      const r = await rpc<{ coins: CoinInfo[] }>("listcoins");
      setCoins(r.coins);
      r.coins.forEach((c) => setSymbol(c.id, c.symbol));
      setConn(true);
    } catch {
      setConn(false);
    }
  }, [setConn, setSymbol]);

  // Nostr relay connectivity for the header dot. Cheap (a local pactd call); the
  // RELAY poll cadence (pactd tick) is separate — this only reads pactd's view.
  const refreshRelays = useCallback(async () => {
    try {
      const r = await rpc<{ relays: RelayStatus[] }>("boardstatus");
      setRelays(r.relays || []);
    } catch {
      /* leave last-known; the engine dot already covers pactd being down */
    }
  }, []);

  const boot = useCallback(async () => {
    if (!inTauri()) {
      log("not running inside Satchel — this UI needs the Tauri bridge");
      setPhase("no-tauri");
      return;
    }
    let m;
    try {
      m = await listMerchants();
    } catch (e) {
      log("startup: " + errMsg(e));
    }
    setMerchants(m?.merchants ?? []);
    setActiveId(m?.active ?? null);

    if (!m || !m.active || !(m.merchants || []).length) {
      setConn(false);
      setPhase("wizard");
      return;
    }

    let gi: Info;
    try {
      gi = await rpc<Info>("getinfo");
      setConn(true);
    } catch (e) {
      setConn(false);
      setInfo(null);
      log("not connected: " + errMsg(e));
      setPhase("disconnected");
      return;
    }
    setInfo(gi);

    if (!gi.seed_exists) {
      setPhase("seed");
      return;
    }
    if (gi.locked) {
      setPhase("unlock");
      return;
    }

    setIdentity(gi.identity || null);
    setNetwork(gi.network || null);
    // C10: pactd owns the merchant registry and backfills each merchant's
    // identity into its own manifest on load/seed-provision — no Satchel-side
    // identity cache to keep in sync anymore (this supersedes C1).
    log(`connected to pactd ${gi.version ?? "?"} (${gi.protocol ?? "?"})`);
    setPhase("ready");
    void refreshSwaps();
    void refreshCoins();
  }, [log, refreshSwaps, refreshCoins, setConn]);

  // Initial boot.
  useEffect(() => {
    void boot();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Global swaps poll (every 4s) while connected — state moves fast.
  useEffect(() => {
    if (!ready) return;
    const t = setInterval(() => void refreshSwaps(), 4000);
    return () => clearInterval(t);
  }, [ready, refreshSwaps]);

  // Coin health refreshes more slowly (node probes are heavier, status changes
  // rarely) — enough to keep the header glyphs honest.
  useEffect(() => {
    if (!ready) return;
    const t = setInterval(() => void refreshCoins(), 10000);
    return () => clearInterval(t);
  }, [ready, refreshCoins]);

  // Relay connectivity for the header dot (every 8s while connected).
  useEffect(() => {
    if (!ready) return;
    void refreshRelays();
    const t = setInterval(() => void refreshRelays(), 8000);
    return () => clearInterval(t);
  }, [ready, refreshRelays]);

  const activeMerchant = useMemo(
    () => merchants.find((x) => x.id === activeId) ?? null,
    [merchants, activeId],
  );
  const value: AppCtx = {
    phase,
    ready,
    connOk,
    setConn,
    merchants,
    activeId,
    activeMerchant,
    info,
    identity,
    network,
    swaps,
    refreshSwaps,
    coins,
    refreshCoins,
    relays,
    symbols,
    setSymbol,
    symOf,
    boot,
    logLines,
    log,
    toast,
    showToast,
    clearToast,
  };

  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}
