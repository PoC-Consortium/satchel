// Pure formatting helpers ported verbatim from the old index.html so numbers,
// rates, ages and freshness render identically to the vanilla UI.

import type {
  AdaptorSwapRecord,
  Capabilities,
  ChainRef,
  Offer,
  OfferBody,
  PendingTake,
  Swap,
  SwapState,
  V1SwapRecord,
} from "./api/types";
import { tr } from "./i18n";

/** sats → bare decimal string, trailing zeros trimmed (keeps one after the dot). */
export const fmtBare = (n: number): string =>
  (n / 1e8).toFixed(8).replace(/0+$/, "").replace(/\.$/, ".0");

/** sats → coin amount with ALWAYS 8 fraction digits (e.g. "0.00100000"), for
 *  fee displays where the full precision should be visible and not trimmed.
 *  Locale-aware separator + no grouping, matching the sat/vB rate line. */
export const fmtFee = (n: number): string =>
  new Intl.NumberFormat(undefined, {
    minimumFractionDigits: 8,
    maximumFractionDigits: 8,
    useGrouping: false,
  }).format(n / 1e8);

/** sats → "1.5 POCX" style with the asset symbol appended. */
export const fmtAmt = (n: number, asset: string): string =>
  `${fmtBare(n)} ${String(asset).toUpperCase()}`;

// ---- locale-aware amount entry --------------------------------------------
// Amount fields accept ONLY digits and the SYSTEM locale's decimal separator
// ("," in de-DE, "." in en-US) — that separator is the single legal non-digit.
// The other character is NOT treated as a grouping separator; it's simply
// illegal (dropped on input, rejected on parse), so on a comma-locale a "."
// does nothing rather than silently doing something. On submit we normalize the
// locale separator to "." for the engine wire (`coin:amount`, sendtoaddress).

let _decimalSep: string | undefined;
/** The current locale's decimal separator (cached) — the only non-digit
 *  character accepted in an amount field. */
export function decimalSeparator(): string {
  if (_decimalSep === undefined) {
    _decimalSep =
      new Intl.NumberFormat().formatToParts(1.1).find((p) => p.type === "decimal")?.value ?? ".";
  }
  return _decimalSep;
}

/** Keep an amount field to digits + a SINGLE locale decimal separator as the
 *  user types/pastes. Every other character — including a "." on a comma-locale
 *  and any second separator — is dropped, so the locale separator is the only
 *  legit non-digit and there is at most one. */
export function sanitizeAmountInput(s: string): string {
  const dec = decimalSeparator();
  let out = "";
  let hasSep = false;
  for (const ch of s) {
    if (ch >= "0" && ch <= "9") out += ch;
    else if (ch === dec && !hasSep) {
      out += ch;
      hasSep = true;
    }
  }
  return out;
}

/** Parse a locale-entered amount to a number. Only digits and the locale
 *  decimal separator are legal; a foreign separator (a "." on a comma-locale)
 *  or a second separator → NaN, so a malformed value fails validation loudly. */
export function parseAmount(s: string): number {
  const dec = decimalSeparator();
  const u = s.trim();
  if (u === "") return NaN;
  for (const ch of u) {
    if (!(ch >= "0" && ch <= "9") && ch !== dec) return NaN;
  }
  const dot = u.split(dec).join(".");
  if (!/^\d*\.?\d*$/.test(dot)) return NaN; // regex also caps it at one separator
  return parseFloat(dot);
}

/** Canonical dot-decimal string for the engine wire. Assumes `parseAmount`
 *  already accepted the input. */
export function canonicalAmount(s: string): string {
  let u = s.trim().split(decimalSeparator()).join(".");
  if (u.startsWith(".")) u = "0" + u;
  if (u.endsWith(".")) u = u.slice(0, -1);
  return u;
}

/** sats → bare decimal string in the SYSTEM locale (decimal separator swapped),
 *  for read-only computed amounts shown next to locale-aware inputs. */
export const fmtBareLocale = (n: number): string => fmtBare(n).replace(".", decimalSeparator());

// ---- offer protocol selection (mirrors engine::board_offer_protocol) -------
// A maker may pin a swap protocol on an offer; the board carries it in the offer
// body. The suite defaults to v1 (classic HTLC — auditable, battle-tested);
// v2 (Taproot/MuSig2 adaptor, "Private (Taproot)") is opt-in, offered whenever
// both legs are Taproot-capable — on EVERY network including mainnet (the engine
// allows it too: registry::ADAPTOR_MAINNET_ENABLED). Keep this in lockstep with
// the engine; if it ever diverges the daemon still rejects an illegal forced
// choice, so the worst case is a clear error rather than a bad swap.

export const PROTOCOL_V1 = "pact-htlc-v1";
export const PROTOCOL_V2 = "pact-htlc-v2";

/** Which protocols a maker can post for a give/get pair, and the preferred
 *  (default) one — given each leg's capabilities. */
export function offerProtocols(
  give: Capabilities | undefined,
  get: Capabilities | undefined,
): { options: string[]; preferred: string | null } {
  const htlc = !!give?.cltv && !!give?.segwit_v0 && !!get?.cltv && !!get?.segwit_v0;
  const adaptor = !!give?.taproot && !!get?.taproot;
  // HTLC first so it's the default-selected option; v2 is the opt-in alternative.
  const options: string[] = [];
  if (htlc) options.push(PROTOCOL_V1);
  if (adaptor) options.push(PROTOCOL_V2);
  return { options, preferred: htlc ? PROTOCOL_V1 : adaptor ? PROTOCOL_V2 : null };
}

/** A swap leg is a ChainRef { coin_id, network }; older builds used `asset`. */
export const asset = (chain?: ChainRef): string =>
  chain ? chain.coin_id || chain.asset || "?" : "?";

/** Implied price: get per 1 give, with enough significant figures to be exact. */
export function impliedRate(giveSat: number, getSat: number): string {
  if (!giveSat) return "—";
  const r = getSat / giveSat;
  const s = r >= 1 ? r.toFixed(r >= 100 ? 2 : 4) : r.toPrecision(4);
  return s.replace(/0+$/, "").replace(/\.$/, "");
}

/** Hours from seconds, dropping the decimal when it's a whole number of hours. */
export const hours = (secs: number): string => (secs / 3600).toFixed(secs % 3600 ? 1 : 0);

/** Bytes → "1.2 MB" style (for the Relays monitor traffic counters). */
export function formatBytes(n: number | undefined): string {
  const b = n ?? 0;
  if (b < 1024) return `${b} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let v = b / 1024;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i += 1;
  }
  return `${v.toFixed(v < 10 ? 1 : 0)} ${units[i]}`;
}

/** Compact uptime since a unix timestamp: "1h 03m" / "12m" / "45s" / "—". For
 *  the Relays monitor (how long a relay has held its current connection). */
export function uptimeSince(unix: number | null | undefined): string {
  if (!unix) return "—";
  const s = Math.max(0, Math.floor(Date.now() / 1000) - unix);
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60);
  if (m < 60) return `${m}m`;
  const h = Math.floor(m / 60);
  if (h < 24) return `${h}h ${String(m % 60).padStart(2, "0")}m`;
  return `${Math.floor(h / 24)}d ${h % 24}h`;
}

export function ago(unix: number): string {
  if (!unix) return tr("format.ageUnknown");
  const s = Math.max(0, Math.floor(Date.now() / 1000) - unix);
  if (s < 60) return tr("format.justNow");
  const m = Math.floor(s / 60);
  if (m < 60) return tr("format.minutesAgo", { n: m });
  const h = Math.floor(m / 60);
  if (h < 24) return tr("format.hoursAgo", { n: h });
  return tr("format.daysAgo", { n: Math.floor(h / 24) });
}

/** Coarse time-UNTIL a future unix timestamp: "in ~3h" / "in ~45m" / "soon".
 *  `ago()` clamps negatives to "just now", so it must NOT be used for expiries
 *  (a future time would always read "just now"); use this instead. */
export function until(unix: number): string {
  if (!unix) return "—";
  const s = unix - Math.floor(Date.now() / 1000);
  if (s <= 0) return tr("format.expiryNow");
  if (s < 60) return tr("format.expirySoon");
  if (s < 3600) return tr("format.inMinutes", { n: Math.round(s / 60) });
  if (s < 36 * 3600) return tr("format.inHours", { n: Math.round(s / 3600) });
  return tr("format.inDays", { n: Math.round(s / 86400) });
}

/** Freshness: "" = fresh, "stale" = aging, "expiring" = within the hour. */
export function freshness(
  created: number,
  expiry: number,
): { cls: "" | "stale" | "expiring"; label: string } {
  const now = Math.floor(Date.now() / 1000);
  // Always surface the exact time-to-expiry (e.g. "expires in ~42m"), not a
  // coarse "<1h" bucket — short-TTL offers live and die in minutes.
  const exp = expiry ? ` · ${tr("format.expires", { time: until(expiry) })}` : "";
  const label = `${tr("format.posted", { age: ago(created) })}${exp}`;
  if (!created) return { cls: "stale", label };
  if (expiry && expiry - now <= 3600) return { cls: "expiring", label };
  if (now - created > 6 * 3600) return { cls: "stale", label };
  return { cls: "", label };
}

export const pairKey = (a: string, b: string): string => [a, b].sort().join("|");

// ---- order-book base/quote convention (DISPLAY ONLY) --------------------
// The ladder is purely a way to read the noticeboard — it never matches or
// orders by priority (load-bearing for the MiCA position). To draw one price
// axis with bids on one side and asks on the other we must pick, per pair, a
// stable base (the thing being priced) and quote (the unit of price). Rule:
// the higher-ranked coin is the quote; price = quote per 1 base. PoCX sits
// lowest, so PoCX markets read "PoCX in BTC" — the question a holder asks.

/** Quote priority, low → high rank. Higher rank wins the quote slot. Coins not
 *  listed rank below all of these (so a known major stays the quote against an
 *  unknown), with ties broken alphabetically for determinism. */
const QUOTE_PRIORITY = ["btcx", "doge", "ltc", "btc"];

export const quoteRank = (coinId: string): number => QUOTE_PRIORITY.indexOf(coinId);

/** Deterministic base/quote split for a pair (order of args is irrelevant). */
export function baseQuote(a: string, b: string): { base: string; quote: string } {
  const ra = quoteRank(a);
  const rb = quoteRank(b);
  const quote = ra !== rb ? (ra > rb ? a : b) : a > b ? a : b;
  return { base: quote === a ? b : a, quote };
}

export type BookSide = "bid" | "ask";

export interface BookEntry {
  side: BookSide;
  /** reduced rational quote/base — exact key for grouping offers into a level. */
  priceKey: string;
  /** numeric quote-per-base, for sorting and display. */
  price: number;
  /** base-coin size in sats (what the level's depth bar measures). */
  sizeSat: number;
}

const gcd = (a: number, b: number): number => (b ? gcd(b, a % b) : a);

/** Place an offer on the base/quote book. Both sides are expressed as
 *  quote-per-base so bids and asks share one price axis.
 *    bid = maker gives QUOTE to get BASE (buying base); size = base they get.
 *    ask = maker gives BASE to get QUOTE (selling base); size = base they give.
 *  Returns null if the offer isn't this market (shouldn't happen post-filter)
 *  or has a zero leg. */
export function bookEntry(b: OfferBody, base: string, quote: string): BookEntry | null {
  let side: BookSide;
  let numSat: number; // quote
  let denSat: number; // base
  let sizeSat: number;
  if (b.give_asset === quote && b.get_asset === base) {
    side = "bid";
    numSat = b.give_amount;
    denSat = b.get_amount;
    sizeSat = b.get_amount;
  } else if (b.give_asset === base && b.get_asset === quote) {
    side = "ask";
    numSat = b.get_amount;
    denSat = b.give_amount;
    sizeSat = b.give_amount;
  } else {
    return null;
  }
  if (!denSat || !numSat) return null;
  const n = Math.round(numSat);
  const d = Math.round(denSat);
  const g = gcd(n, d) || 1;
  return { side, priceKey: `${n / g}/${d / g}`, price: numSat / denSat, sizeSat };
}

/** Format a quote-per-base price with enough significant figures to be exact,
 *  trimming trailing zeros. Handles both tiny (BTC/PoCX) and large ratios. */
export function fmtPrice(p: number): string {
  if (!isFinite(p) || p <= 0) return "—";
  // Locale-aware throughout — the decimal separator AND grouping follow the
  // locale, so a grouped "1.000"/"100.000" (de-DE) can't be misread as a decimal
  // sitting next to a "0,001". Precision is tiered: integers for large ratios,
  // up to 4 fraction digits mid-range, 5 significant figures for tiny (BTC/PoCX)
  // prices. Intl trims trailing zeros for us.
  const opts: Intl.NumberFormatOptions =
    p >= 1000
      ? { maximumFractionDigits: 0 }
      : p >= 1
        ? { maximumFractionDigits: 4 }
        : { maximumSignificantDigits: 5 };
  return new Intl.NumberFormat(undefined, opts).format(p);
}

// ---- denomination (display unit) ----------------------------------------
// A view-only preference: amounts are always stored/handled in sats; this only
// changes how the quote coin is shown (tiny BTC decimals read better as sat /
// bits). The base coin keeps whole units (its numbers are already friendly).

export type Denom = "coin" | "milli" | "micro" | "sat";

/** Toolbar order, whole → smallest. */
export const DENOMS: Denom[] = ["coin", "milli", "micro", "sat"];

/** sats in one unit of each denomination (coin = 1e8 sats). */
const DENOM_SATS: Record<Denom, number> = { coin: 1e8, milli: 1e5, micro: 1e2, sat: 1 };

/** Sats in one unit of denomination `d`. For converting a price/amount typed in
 *  a chosen denom back to exact sats (offer form's unit'd price field). */
export const denomUnitSats = (d: Denom): number => DENOM_SATS[d];

/** Per-coin unit label. BTC uses the familiar bits/sat names; other coins fall
 *  back to m-/µ- prefixes on their symbol (atomic unit shown as "sat"). */
export function denomLabel(coinId: string, sym: string, d: Denom): string {
  const btc = coinId === "btc";
  if (d === "coin") return sym;
  if (d === "milli") return "m" + sym;
  if (d === "micro") return btc ? "bits" : "µ" + sym;
  return "sat";
}

/** A sats amount as a bare number string in the chosen denomination. */
export function fmtDenom(sats: number, d: Denom): string {
  const v = sats / DENOM_SATS[d];
  // Locale-aware (decimal separator + grouping): sat is a grouped integer, finer
  // units keep up to 8 fraction digits with trailing zeros trimmed by Intl.
  return new Intl.NumberFormat(undefined, {
    maximumFractionDigits: d === "sat" ? 0 : 8,
  }).format(v);
}

/** A quote-per-base price (in whole quote coin) re-expressed in denomination d
 *  of the quote, then formatted. e.g. 0.0000436 BTC/PoCX → "4,360" in sat. */
export function fmtPriceDenom(price: number, d: Denom): string {
  return fmtPrice(price * (1e8 / DENOM_SATS[d]));
}

export const commas = (n: number | undefined): string => Number(n ?? 0).toLocaleString();

export const COIN_GLYPH: Record<string, string> = { btc: "₿", btcx: "◈" };
export const glyph = (c: { id: string; symbol?: string }): string =>
  COIN_GLYPH[c.id] || (c.symbol || "?").slice(0, 1);

// ---- network ------------------------------------------------------------

/** Network is one coherent mode for the running client; mainnet is the only
 *  "real funds" mode, everything else must read as visually unmistakable. */
export const isMainnet = (n: string | null | undefined): boolean => n === "mainnet" || n === "main";

// ---- swap lifecycle -----------------------------------------------------

/** Terminal states: the swap is finished one way or another (history). */
export const TERMINAL_STATES: SwapState[] = ["completed", "refunded", "aborted"];
/** Finalizing = our claim is broadcast but still burying. The state reads
 *  `completed`, yet the scheduler is still nursing it to depth (a `settlement`
 *  progress bar is present) and the refund stays armed — so it is NOT done: the
 *  funds aren't final and the app must stay open. The taker hits this because it
 *  goes straight to `completed` on broadcast (the maker stays in `redeemed_b`
 *  until buried, so it never lands here). */
export const isFinalizing = (s: Swap): boolean =>
  s.state === "completed" && s.progress?.watching === "settlement";
/** Terminal = finished AND final (history). Finalizing is excluded — it is still
 *  in flight until the claim buries. */
export const isTerminal = (s: Swap): boolean =>
  TERMINAL_STATES.includes(s.state) && !isFinalizing(s);
/** Active = in flight: the scheduler still has work / funds may be exposed.
 *  (Drives the active dock, the in-flight count, and the exit-gate warning.) */
export const isActive = (s: Swap): boolean => !isTerminal(s);

/** Which leg carries OUR settlement tx. We fund one leg and claim the other; a
 *  refund instead reclaims the leg we funded. So on success the settlement is on
 *  the leg we receive, on refund it's on the leg we funded. (Initiator funds A /
 *  receives B; participant funds B / receives A.) */
export function settlementLeg(role: Swap["role"], state: SwapState): "a" | "b" {
  const fundedLeg = role === "initiator" ? "a" : "b";
  const receiveLeg = role === "initiator" ? "b" : "a";
  return state === "refunded" ? fundedLeg : receiveLeg;
}

/** Market-facing role label. The offer maker initiates the swap (funds first);
 *  the taker participates — a 1:1 map of the protocol role (post → initiator =
 *  maker, take → participant = taker). Not localized (the role was never
 *  translated; same as before). Used for the Maker/Taker column headers. */
export function roleLabel(role: Swap["role"]): string {
  return role === "initiator" ? "Maker" : "Taker";
}

/** Who fills the maker and taker slots of a swap, given your own identity.
 *  Maker = initiator (posts/funds first), Taker = participant. Each slot is the
 *  party's pubkey + whether it's you — so the UI can label both sides explicitly
 *  ("Maker: you · Taker: 5278…") instead of the ambiguous "your role + their
 *  hash". Rendered via CounterpartyTag (its `you` mode marks your side). */
export function swapParties(s: Swap, youId: string | null | undefined) {
  const me = { id: youId ?? null, you: true };
  const them = { id: s.counterparty_identity ?? null, you: false };
  return s.role === "initiator" ? { maker: me, taker: them } : { maker: them, taker: me };
}

/** Normalize a v1 `listswaps` record: copy the HTLC funding txids into the
 *  canonical `fund_*` fields. `final_txid` is already our settlement. */
export function v1ToSwap(r: V1SwapRecord): Swap {
  return { ...r, fund_a_txid: r.htlc_a_txid ?? null, fund_b_txid: r.htlc_b_txid ?? null };
}

/** Fold a pending take (post-boardtake, pre-record) into the unified Swap shape
 *  as an "initiating" pre-swap. `swap_id = offer_id` (== the eventual record id)
 *  so it dedupes against the real swap once it lands. We're the taker
 *  (participant); leg A is the maker's give, leg B is what we give — matching how
 *  the real record renders, so the row doesn't jump when it resolves. */
export function pendingTakeToSwap(p: PendingTake): Swap {
  return {
    swap_id: p.offer_id,
    role: "participant",
    state: "initiating",
    chain_a: { coin_id: p.body.give_asset },
    chain_b: { coin_id: p.body.get_asset },
    amount_a: p.body.give_amount,
    amount_b: p.body.get_amount,
    t1: 0,
    t2: 0,
    created_at: p.created_at,
    protocol: p.body.protocol,
    counterparty_identity: p.from,
  };
}

/** Fold a v2 `listadaptorswaps` record into the unified `Swap` shape, tagging it
 *  `pact-htlc-v2`. Carries both funding txids; for settlement we surface ONLY
 *  the leg we redeemed/refunded (never the counterparty's). */
export function adaptorToSwap(r: AdaptorSwapRecord): Swap {
  const mine = settlementLeg(r.role, r.state);
  const mySettle = mine === "a" ? r.final_txid_a : r.final_txid_b;
  return {
    swap_id: r.swap_id,
    role: r.role,
    state: r.state,
    chain_a: r.chain_a,
    chain_b: r.chain_b,
    amount_a: r.amount_a,
    amount_b: r.amount_b,
    t1: r.t1,
    t2: r.t2,
    created_at: r.created_at,
    fund_a_txid: r.funding_a_txid ?? null,
    fund_b_txid: r.funding_b_txid ?? null,
    final_txid: mySettle ?? null,
    protocol: "pact-htlc-v2",
    counterparty_identity: r.counterparty_identity ?? null,
  };
}

// ---- offer lifecycle (derived client-side per BACKEND_CONTRACTS) ---------

export type OfferState = "open" | "taken-by-us" | "revoked" | "expired";

/** Derive an offer's lifecycle state from the board envelope + our own swaps.
 *  The board never tracks who-took-what (deliberate non-goal C5); "taken by us"
 *  is correlated locally from our swap list. */
export function offerState(o: Offer, mySwapIds: Set<string>): OfferState {
  if (mySwapIds.has(o.swap_id)) return "taken-by-us";
  if (o.revoked) return "revoked";
  const expiry = o.body.created ? o.body.created + (o.body.ttl_secs || 24 * 3600) : 0;
  if (expiry && expiry < Math.floor(Date.now() / 1000)) return "expired";
  return "open";
}
