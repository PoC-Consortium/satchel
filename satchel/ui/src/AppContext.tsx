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
import { errMsg, getCoinIcon, inTauri, listMerchants, rpc } from "./api/tauri";
import { adaptorToSwap, fmtBare, isActive, pendingTakeToSwap, v1ToSwap } from "./format";
import { notifySwapEvents, updateTray } from "./notify";
import { tr, useI18n } from "./i18n";
import { COIN_ICON } from "./assets/coins";
import type {
  AdaptorSwapRecord,
  CoinInfo,
  Info,
  Merchant,
  PendingTake,
  RelayStatus,
  Swap,
  SwapProgress,
  V1SwapRecord,
} from "./api/types";

// The boot/connection state machine, ported from index.html `boot()`:
//   no-tauri  — not inside Satchel's webview (dev in a plain browser)
//   wizard    — no active merchant → first-run wizard
//   seed      — merchant active but its seed isn't provisioned yet
//   unlock    — encrypted merchant came up locked
//   disconnected — pactd unreachable / getinfo failed
//   ready     — connected, seed present + unlocked: usable (browsing is always
//               available; trading is gated per-action, not by a coin wall)
export type Phase =
  | "loading"
  | "no-tauri"
  | "wizard"
  | "seed"
  | "unlock"
  | "reimport"
  | "disconnected"
  | "ready";

export interface LogLine {
  time: string;
  msg: string;
}

/** Last-known wallet balance for one coin. `error` rides alongside the cached
 *  value when a refresh fails — the UI keeps showing the stale number. */
export interface Bal {
  text: string;
  sat?: number;
  error?: string;
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
  /** True once a swap list is showing — either the first listswaps resolved or
   *  the module cache seeded one — so the ledger can show skeletons on the
   *  first-ever load instead of claiming "no swaps" (#91/#131 pattern). */
  swapsLoaded: boolean;
  refreshSwaps: () => Promise<void>;

  /** Live `listcoins` (registry + configured + probed status), shared so the
   *  header status row + screens don't each re-poll. Empty until connected. */
  coins: CoinInfo[];
  /** True once a `listcoins` has resolved at least once — distinguishes
   *  "coins not loaded yet" from "loaded, none configured" (#139). */
  coinsLoaded: boolean;
  refreshCoins: () => Promise<void>;

  /** Bumped when something changed the board out-of-band (e.g. a post that
   *  completed after navigation, #153 follow-up) — the Corkboard reloads
   *  immediately instead of waiting for its next poll tick. */
  boardNonce: number;
  pokeBoard: () => void;

  /** coin_id → last-known wallet balance, cached app-wide so the Wallets page
   *  shows values instantly on every visit — stale, never blank (#91). */
  balances: Record<string, Bal>;
  refreshBalances: (coinIds: string[]) => Promise<void>;

  /** coin_id → icon data URL (or null when there's none), for file-coins
   *  without a bundled asset (e.g. ltc). Fetched once per id; the header and
   *  the screens share it so every surface shows the same icon. */
  coinIcons: Record<string, string | null>;

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

// Stale-never-blank cache for the swap list (#91/#131 pattern, single slot like
// the Corkboard's lastBoard): the last polled list, keyed by the merchant
// identity it was fetched under (null = boot-time, before identity resolved —
// treated as a wildcard). Seeds the initial state on a provider (re)mount so the
// ledger and the ActiveSwaps dock paint instantly instead of starting empty;
// boot() drops it when a different merchant logs in, so a switch starts fresh.
interface SwapsSnap {
  owner: string | null;
  swaps: Swap[];
}
let lastSwaps: SwapsSnap | null = null;

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
  const [swaps, setSwaps] = useState<Swap[]>(() => lastSwaps?.swaps ?? []);
  const [swapsLoaded, setSwapsLoaded] = useState(() => !!lastSwaps);
  const [coins, setCoins] = useState<CoinInfo[]>([]);
  const [coinsLoaded, setCoinsLoaded] = useState(false);
  const [boardNonce, setBoardNonce] = useState(0);
  const pokeBoard = useCallback(() => setBoardNonce((n) => n + 1), []);
  const [balances, setBalances] = useState<Record<string, Bal>>({});
  const [coinIcons, setCoinIcons] = useState<Record<string, string | null>>({});
  const [relays, setRelays] = useState<RelayStatus[]>([]);
  const [symbols, setSymbols] = useState<Record<string, string>>({});
  const [logLines, setLogLines] = useState<LogLine[]>([]);
  const [toast, setToast] = useState<string | null>(null);

  // The active merchant id, reachable from the (stable) refreshSwaps callback —
  // notifySwapEvents keys its "seed silently, don't replay history" state on it
  // so a merchant switch never replays the other merchant's swap history as a
  // burst of OS notifications.
  const activeIdRef = useRef(activeId);
  activeIdRef.current = activeId;
  // The merchant identity, reachable from the (stable) refreshSwaps callback —
  // the swaps cache is keyed on it (null until boot resolves it).
  const identityRef = useRef(identity);
  identityRef.current = identity;

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

  // File-coin icons (e.g. ltc): fetch the data URL once per coin id for coins
  // without a bundled asset, so the header status row and every screen render
  // the same icon. Built-ins (btc/btcx) use COIN_ICON directly and are skipped.
  // null is recorded too (coin has no icon) so we never re-fetch a known miss.
  useEffect(() => {
    const missing = coins
      .map((c) => c.id)
      .filter((id) => !COIN_ICON[id] && coinIcons[id] === undefined);
    if (missing.length === 0) return;
    let cancelled = false;
    void Promise.all(
      missing.map((id) => getCoinIcon(id).catch(() => null).then((url) => [id, url] as const)),
    ).then((pairs) => {
      if (cancelled) return;
      setCoinIcons((prev) => {
        const next = { ...prev };
        for (const [id, url] of pairs) next[id] = url;
        return next;
      });
    });
    return () => {
      cancelled = true;
    };
  }, [coins, coinIcons]);

  // listswaps drives both the Swaps tab and the header's completed count, so it
  // polls globally (not per-tab) while we're connected — as in the old UI.
  // v2 (adaptor) swaps live in a separate pactd table; pending takes (post-take,
  // pre-record) in another. We fold all three into one array so the ledger,
  // header count, and active-swaps dock cover every protocol + the "initiating"
  // pre-swap. A pending take is dropped once its real record exists (same id).
  const refreshSwaps = useCallback(async () => {
    try {
      const [v1, v2, pend, prog] = await Promise.all([
        rpc<V1SwapRecord[]>("listswaps"),
        rpc<AdaptorSwapRecord[]>("listadaptorswaps"),
        rpc<PendingTake[]>("listpendingtakes"),
        // Live progress is observability-only; never let it break the refresh.
        rpc<SwapProgress[]>("swapprogress").catch(() => [] as SwapProgress[]),
      ]);
      const progById = new Map(prog.map((p) => [p.swap_id, p]));
      const real = [...v1.map(v1ToSwap), ...v2.map(adaptorToSwap)].map((s) => {
        const p = progById.get(s.swap_id);
        return p ? { ...s, progress: p } : s;
      });
      const realIds = new Set(real.map((s) => s.swap_id));
      const pending = pend.filter((p) => !realIds.has(p.offer_id)).map(pendingTakeToSwap);
      const next = [...real, ...pending];
      setSwaps(next);
      setSwapsLoaded(true);
      // Refresh the single-slot cache (even boot-time polls with a null
      // identity) so the next mount paints instantly with the last-known list.
      lastSwaps = { owner: identityRef.current, swaps: next };
      // OS notifications on milestone crossings (#55) — contained in its own
      // try so a bug in the (cosmetic) notification path can never masquerade
      // as pactd being unreachable.
      try {
        notifySwapEvents(next, activeIdRef.current);
      } catch {
        /* cosmetic path — never break the poll */
      }
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
      setCoinsLoaded(true);
      r.coins.forEach((c) => setSymbol(c.id, c.symbol));
      setConn(true);
    } catch {
      setConn(false);
    }
  }, [setConn, setSymbol]);

  // Wallet balances, fetched in parallel (pactd serializes on its lock anyway,
  // but each card updates as its coin lands instead of queuing behind the
  // slowest one). A failed refresh keeps the cached value and records the
  // error alongside it — never blanks a number the user already saw.
  const refreshBalances = useCallback(async (coinIds: string[]) => {
    await Promise.all(
      coinIds.map(async (id) => {
        try {
          const r = await rpc<{ balance_sat: number }>("getbalance", [id]);
          setBalances((b) => ({
            ...b,
            [id]: { text: fmtBare(r.balance_sat), sat: r.balance_sat },
          }));
        } catch (e) {
          setBalances((b) => ({
            ...b,
            [id]: b[id] ? { ...b[id], error: errMsg(e) } : { text: "—", error: errMsg(e) },
          }));
        }
      }),
    );
  }, []);

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
      log(tr("log.noTauri"));
      setPhase("no-tauri");
      return;
    }
    let m;
    try {
      m = await listMerchants();
    } catch (e) {
      log(tr("log.startupError", { err: errMsg(e) }));
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
      log(tr("log.notConnected", { err: errMsg(e) }));
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
    // #133: the seed file exists but this machine's OS-keystore key can no
    // longer decrypt it (data dir moved to a new machine / keychain reset).
    // Without this the app lands in "ready" looking healthy and only fails on
    // the first seed-using action, with no guided recovery.
    if (gi.needs_reimport) {
      setPhase("reimport");
      return;
    }

    // A merchant switch starts the ledger fresh: a swap list cached/showing
    // under a DIFFERENT identity is dropped here, where the new identity first
    // resolves (a null-owner boot-time snapshot passes — it can only be this
    // machine's own history).
    if (lastSwaps?.owner != null && gi.identity && lastSwaps.owner !== gi.identity) {
      lastSwaps = null;
      setSwaps([]);
      setSwapsLoaded(false);
    }
    setIdentity(gi.identity || null);
    setNetwork(gi.network || null);
    // C10: pactd owns the merchant registry and backfills each merchant's
    // identity into its own manifest on load/seed-provision — no Satchel-side
    // identity cache to keep in sync anymore (this supersedes C1).
    log(tr("log.connected", { version: gi.version ?? "?", protocol: gi.protocol ?? "?" }));

    // Browsing is always available: after unlock we land straight in the app,
    // regardless of how many coins are configured. There is no hard coin wall to
    // get trapped in (#119) — trading is gated PER-ACTION (each screen prompts to
    // set up the pair's coins) and enforced server-side (ensure_chains_live).
    // Load coins up front so the wallet/header/board render honestly; a fresh
    // merchant typically has none, and the empty states nudge coin setup.
    try {
      const cl = await rpc<{ coins: CoinInfo[] }>("listcoins");
      setCoins(cl.coins);
      setCoinsLoaded(true);
      cl.coins.forEach((c) => setSymbol(c.id, c.symbol));
    } catch (e) {
      log(tr("log.listcoinsError", { err: errMsg(e) }));
    }

    setPhase("ready");
    void refreshSwaps();
    void refreshCoins();
  }, [log, refreshSwaps, refreshCoins, setConn, setSymbol]);

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

  // Tray tooltip = the header's live-swap count; menu labels follow the UI
  // language (#55). Effect on (swaps, lang) so a language switch relabels the
  // tray immediately, not on the next poll; updateTray itself no-ops when
  // nothing changed.
  const { lang } = useI18n();
  useEffect(() => {
    updateTray(swaps.filter(isActive).length);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [swaps, lang]);

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
    swapsLoaded,
    refreshSwaps,
    coins,
    coinsLoaded,
    boardNonce,
    pokeBoard,
    refreshCoins,
    balances,
    refreshBalances,
    coinIcons,
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
