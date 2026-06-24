import { useCallback, useEffect, useMemo, useState } from "react";
import {
  Box,
  Button,
  Chip,
  Divider,
  Link,
  MenuItem,
  Select,
  Stack,
  ToggleButton,
  ToggleButtonGroup,
  Tooltip,
  Typography,
} from "@mui/material";
import AddIcon from "@mui/icons-material/Add";
import { useApp } from "../AppContext";
import { useDenom } from "../denom";
import { useNavigate } from "../ui/nav";
import { useT } from "../i18n";
import { errMsg, listCoinConfig, rpc } from "../api/tauri";
import { useTakeConfirm } from "../hooks/useTakeConfirm";
import { EmptyState } from "../components/StatusViews";
import CounterpartyTag from "../components/CounterpartyTag";
import {
  ago,
  baseQuote,
  bookEntry,
  DENOMS,
  denomLabel,
  fmtBareLocale,
  fmtDenom,
  fmtPriceDenom,
  freshness,
  hours,
  offerState,
  pairKey,
} from "../format";
import { C } from "../theme";
import type { BookSide, Denom, OfferState } from "../format";
import type { Offer, Pair } from "../api/types";

type Leg = (sats: number, coin: string) => string;

// The selected pair persists across navigation in localStorage (a pure view
// preference, like the denom toggle) — leaving and returning to the Corkboard
// keeps your pair instead of snapping back to the first one.
const PAIR_KEY = "satchel.corkboard.pair";

// One row from pactd `listmyoffers` — the maker's own offers from the local
// store (the `offer` envelope is Offer-shaped: swap_id / from / body).
interface MyOfferRow {
  offer_id: string;
  offer: Offer;
  state: string;
}

export default function CorkboardScreen() {
  const { identity, swaps, symOf, log, refreshSwaps, coins, watchOnly } = useApp();
  // A leg is tradeable only when its coin has a live ("ok") node. Used to gate
  // the Take button so you can't start a swap against a down chain (the engine
  // refuses too — this is the friendly up-front block).
  const coinLive = (id?: string) => !!coins.find((c) => c.id === id && c.status === "ok");
  const { denom, setDenom } = useDenom();
  const confirmTake = useTakeConfirm();
  const navigate = useNavigate();
  const t = useT();

  const [loaded, setLoaded] = useState(false);
  const [boardErr, setBoardErr] = useState<string | null>(null);
  const [offers, setOffers] = useState<Offer[]>([]);
  // Own offers shown optimistically but not yet seen back from a relay.
  const [stagedIds, setStagedIds] = useState<Set<string>>(() => new Set());
  const [available, setAvailable] = useState<Set<string>>(new Set());
  const [savedBoards, setSavedBoards] = useState("");
  const [pairFilter, setPairFilter] = useState<string>(() => {
    try {
      return localStorage.getItem(PAIR_KEY) || "";
    } catch {
      return "";
    }
  }); // pairKey; "" → first (persisted across navigation)
  const [boardSel, setBoardSel] = useState<string>(""); // selected board URL; "" → first
  const [mineOnly, setMineOnly] = useState(false); // All vs Mine (own offers)
  const [selected, setSelected] = useState<string | null>(null); // "side:priceKey"
  // Offers we just took, hidden optimistically so the line vanishes the instant
  // you confirm (before the take RPC + refresh land). Cleared if the take fails.
  const [justTook, setJustTook] = useState<Set<string>>(() => new Set());

  const myId = identity;
  // Correlate offers to our own swaps → "taken by us" without board-side
  // take tracking (a deliberate non-goal, see BACKEND_CONTRACTS C5).
  const mySwapIds = useMemo(() => new Set(swaps.map((s) => s.swap_id)), [swaps]);

  const loadOffers = useCallback(async () => {
    const avail = new Set<string>();
    try {
      const r = await rpc<{ pairs: Pair[] }>("listpairs");
      r.pairs.forEach((p) => {
        if (p.available) avail.add(pairKey(p.coin_a, p.coin_b));
      });
    } catch {
      /* no pairs yet */
    }
    setAvailable(avail);

    try {
      setSavedBoards(((await listCoinConfig()).board_urls || []).join(","));
    } catch {
      /* ok */
    }

    let list: Offer[];
    try {
      // Pass the selected board so we view that noticeboard (pactd lists from
      // the given board, else the first configured). Old pactd ignores the arg.
      list = (await rpc<{ offers?: Offer[] }>("boardlistoffers", boardSel ? [boardSel] : [])).offers || [];
      setBoardErr(null);
    } catch (e) {
      setBoardErr(errMsg(e));
      setOffers([]);
      setLoaded(true);
      return;
    }
    // Optimistically merge our OWN live offers from the local store so a posted
    // offer shows INSTANTLY — even before a relay round-trips it back into the
    // fetched board. "Staged" = not yet seen back from a relay (rendered
    // dimmer/italic until it goes live).
    const relayIds = new Set(list.map((o) => o.swap_id));
    const staged = new Set<string>();
    try {
      // listmyoffers returns a BARE array (unlike boardlistoffers' { offers }).
      const mine = (await rpc<MyOfferRow[]>("listmyoffers")) || [];
      for (const m of mine) {
        if (m.state !== "live" || !m.offer?.swap_id) continue;
        if (relayIds.has(m.offer.swap_id)) continue; // already live on the board
        list = [...list, { swap_id: m.offer.swap_id, from: m.offer.from, body: m.offer.body }];
        staged.add(m.offer.swap_id);
      }
    } catch {
      /* listmyoffers is optional (older pactd / no offers) */
    }
    setStagedIds(staged);
    setOffers(list);
    setLoaded(true);
  }, [boardSel]);

  // Re-sync on mount, when the active merchant changes (login / switch — the
  // board is per-identity and pactd only starts its relay poll once a merchant
  // is loaded), and then on a brisk 4s poll. The Nostr transport fills the local
  // offer cache a poll-tick after a relay round-trip, so a slow refresh made
  // fresh offers feel like they only appeared after navigating away and back.
  useEffect(() => {
    void loadOffers();
    const t2 = setInterval(() => void loadOffers(), 4000);
    return () => clearInterval(t2);
  }, [loadOffers, identity]);

  // Configured boards (from satchel.json). Default the selector to the first.
  const boards = useMemo(() => savedBoards.split(",").map((s) => s.trim()).filter(Boolean), [savedBoards]);
  useEffect(() => {
    if (!boardSel && boards.length) setBoardSel(boards[0]);
  }, [boards, boardSel]);

  // Supported pairs (from listpairs capabilities) → the pair selector options;
  // no "all" — the default pair is the first in the list. In watch-only there
  // are no configured pairs, so derive the selector from the pairs actually
  // present on the board — the whole point of the mode is to browse everything.
  const pairOptions = useMemo(() => {
    const keys = watchOnly
      ? [...new Set(offers.map((o) => pairKey(o.body.give_asset, o.body.get_asset)))]
      : [...available];
    return keys
      .map((key) => {
        const [a, b] = key.split("|");
        return { key, label: `${symOf(a)} ↔ ${symOf(b)}` };
      })
      .sort((x, y) => x.label.localeCompare(y.label));
  }, [watchOnly, offers, available, symOf]);
  // Default to the first pair when none is chosen, and fall back to it if a
  // persisted pair is no longer available (capabilities changed).
  useEffect(() => {
    if (!pairOptions.length) return;
    if (!pairOptions.some((p) => p.key === pairFilter)) setPairFilter(pairOptions[0].key);
  }, [pairOptions, pairFilter]);
  // Persist the chosen pair so it survives leaving/returning to the Corkboard.
  useEffect(() => {
    if (!pairFilter) return;
    try {
      localStorage.setItem(PAIR_KEY, pairFilter);
    } catch {
      /* best-effort persist */
    }
  }, [pairFilter]);
  const effectivePair = pairFilter || pairOptions[0]?.key || "";

  // Switching market/board/filter clears the open level so the detail pane never
  // shows offers from a market you're no longer looking at.
  useEffect(() => setSelected(null), [effectivePair, boardSel, mineOnly]);

  const selectLevel = useCallback((key: string) => {
    setSelected((cur) => (cur === key ? null : key));
  }, []);

  async function take(o: Offer) {
    const ok = await confirmTake(o.body, { from: o.from });
    if (!ok) return;
    // Hide the line the instant you confirm (optimistic); the refresh below makes
    // it truth-backed. If the take RPC fails, un-hide so the offer comes back.
    setJustTook((s) => new Set(s).add(o.swap_id));
    try {
      await rpc("boardtake", [o.swap_id]);
      log(t("log.tookOffer", { id: o.swap_id }));
      // Surface the pending take as an "initiating" pre-swap + flip this offer to
      // taken-by-us, without waiting for the next poll tick.
      void refreshSwaps();
      void loadOffers();
    } catch (e) {
      log(t("log.takeError", { err: errMsg(e) }));
      setJustTook((s) => {
        const n = new Set(s);
        n.delete(o.swap_id);
        return n;
      });
    }
  }

  async function revoke(o: Offer) {
    try {
      await rpc("boardrevoke", [o.swap_id]);
      log(t("log.offerWithdrawn", { id: o.swap_id }));
    } catch (e) {
      log(t("log.withdrawError", { err: errMsg(e) }));
    }
    void loadOffers();
  }

  if (!loaded) return null;

  if (boardErr !== null) {
    return (
      <EmptyState
        title={t("corkboard.noBoardTitle")}
        action={
          <Button variant="contained" onClick={() => navigate("settings")}>
            {t("corkboard.boardSettings")}
          </Button>
        }
      >
        {t("corkboard.noBoardBody")}
        <Typography sx={{ color: "text.secondary", fontSize: 12, mt: 2 }}>{boardErr}</Typography>
      </EmptyState>
    );
  }

  // Watch-only shows the whole board (no configured pairs to filter against);
  // otherwise only offers for pairs whose coins are connected are takeable, and
  // the rest fold into the "hidden" footer.
  const supported = watchOnly
    ? offers
    : offers.filter((o) => available.has(pairKey(o.body.give_asset, o.body.get_asset)));
  const hidden = watchOnly
    ? []
    : offers.filter((o) => !available.has(pairKey(o.body.give_asset, o.body.get_asset)));

  const inFilter = (o: Offer) =>
    !effectivePair || pairKey(o.body.give_asset, o.body.get_asset) === effectivePair;

  // Drop withdrawn notices; everything else is shown (expired offers are badged
  // + un-takeable in the revealed rows). The All/Mine toggle filters to our own
  // posted offers (manage/withdraw).
  const visible = supported
    .filter(inFilter)
    .filter((o) => !mineOnly || o.from === myId)
    // Drop withdrawn notices and anything we've taken — taken offers leave the
    // book immediately (optimistic justTook on click, then confirmed once the
    // pending take makes offerState "taken-by-us") rather than lingering badged.
    .filter((o) => !justTook.has(o.swap_id))
    .filter((o) => {
      const st = offerState(o, mySwapIds);
      return st !== "revoked" && st !== "taken-by-us";
    });

  // Pick a stable base/quote for the selected pair and project every offer onto
  // a single price axis (quote per base). Bids and asks are exact-rate levels;
  // this is purely a way to READ the board — pactd never matches or prioritises.
  const [ca, cb] = effectivePair ? effectivePair.split("|") : ["", ""];
  const { base, quote } = effectivePair ? baseQuote(ca, cb) : { base: "", quote: "" };
  const baseSym = symOf(base);
  const quoteSym = symOf(quote);
  const quoteUnit = quote ? denomLabel(quote, quoteSym, denom) : "";
  // Per-leg formatter: the quote coin follows the denomination toggle; the base
  // coin (already-friendly whole numbers) stays put.
  const fmtLeg: Leg = (sats, coin) =>
    coin === quote ? `${fmtDenom(sats, denom)} ${quoteUnit}` : `${fmtBareLocale(sats)} ${symOf(coin)}`;
  // An offer's base-coin amount (the size axis), for sorting offers within a level.
  const baseSizeOf = (o: Offer) => (o.body.give_asset === base ? o.body.give_amount : o.body.get_amount);

  const { bidLevels, askLevels, maxSize, bestBid, bestAsk } = buildBook(visible, base, quote);

  const haveBoth = bestBid != null && bestAsk != null;
  const mid = haveBoth ? (bestBid! + bestAsk!) / 2 : undefined;
  const spreadPct = haveBoth && mid ? ((bestAsk! - bestBid!) / mid) * 100 : undefined;
  const crossed = spreadPct != null && spreadPct < 0;
  const spreadLabel = haveBoth
    ? t("corkboard.book.spread", { pct: `${spreadPct!.toFixed(2)}%` })
    : t("corkboard.book.spreadOneSided");

  // The level whose offers fill the detail pane.
  const selLevel = (() => {
    if (!selected) return null;
    const i = selected.indexOf(":");
    const side = selected.slice(0, i) as BookSide;
    const key = selected.slice(i + 1);
    return (side === "bid" ? bidLevels : askLevels).find((l) => l.key === key) ?? null;
  })();
  const selSide: BookSide | null = selected ? (selected.slice(0, selected.indexOf(":")) as BookSide) : null;

  const colProps = {
    maxSize,
    baseSym,
    quoteSym,
    priceUnit: quoteUnit,
    denom,
    myId,
    stagedIds,
    selectedKey: selected,
    onSelect: selectLevel,
  };

  return (
    <>
      <Box sx={{ display: "flex", alignItems: "center", gap: 1.5, mb: 1.5, flexWrap: "wrap" }}>
        {/* LEFT: pair, the All/Mine filter, and the display-unit toggle. */}
        <Select
          size="small"
          value={pairOptions.some((p) => p.key === effectivePair) ? effectivePair : ""}
          onChange={(e) => setPairFilter(e.target.value)}
          displayEmpty
          sx={{ minWidth: 150 }}
        >
          {pairOptions.length === 0 && (
            <MenuItem value="" disabled>
              {t("corkboard.noPairs")}
            </MenuItem>
          )}
          {pairOptions.map((p) => (
            <MenuItem key={p.key} value={p.key}>
              {p.label}
            </MenuItem>
          ))}
        </Select>

        <ToggleButtonGroup
          exclusive
          size="small"
          value={mineOnly ? "mine" : "all"}
          onChange={(_, v) => v && setMineOnly(v === "mine")}
        >
          <ToggleButton value="all">{t("corkboard.filterAll")}</ToggleButton>
          <ToggleButton value="mine">{t("corkboard.filterMine")}</ToggleButton>
        </ToggleButtonGroup>

        {quote && (
          <Tooltip title={t("corkboard.book.denomTip", { coin: quoteSym })}>
            <ToggleButtonGroup
              exclusive
              size="small"
              value={denom}
              onChange={(_, v: Denom | null) => v && setDenom(v)}
            >
              {DENOMS.map((d) => (
                <ToggleButton key={d} value={d} sx={{ fontFamily: C.mono, fontSize: 11, px: 1 }}>
                  {denomLabel(quote, quoteSym, d)}
                </ToggleButton>
              ))}
            </ToggleButtonGroup>
          </Tooltip>
        )}

        <Box sx={{ flex: 1 }} />

        {/* RIGHT: noticeboard selection (switch between configured boards). */}
        {boards.length > 0 && (
          <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
            <Typography sx={{ fontSize: 12, color: "text.secondary" }}>{t("corkboard.board")}</Typography>
            <Select
              size="small"
              value={boards.includes(boardSel) ? boardSel : boards[0]}
              onChange={(e) => setBoardSel(e.target.value)}
              sx={{ minWidth: 180, "& .MuiSelect-select": { fontFamily: C.mono, fontSize: 12 } }}
            >
              {boards.map((b) => (
                <MenuItem key={b} value={b} sx={{ fontFamily: C.mono, fontSize: 12 }}>
                  {b.replace(/^https?:\/\//, "")}
                </MenuItem>
              ))}
            </Select>
          </Box>
        )}
      </Box>

      {visible.length === 0 ? (
        <EmptyState title={t("corkboard.noOffers")}>{t("corkboard.noOffersBody")}</EmptyState>
      ) : (
        <>
          <Box
            sx={{
              border: `1px solid ${C.line}`,
              borderRadius: 2,
              bgcolor: "background.paper",
              overflow: "hidden",
            }}
          >
            {/* Spread banner — relates the top bid to the top ask. A negative
                ("crossed") spread is possible because the board never matches. */}
            <Box
              sx={{
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                gap: 1.5,
                py: 0.875,
                px: 1.5,
                borderBottom: `1px solid ${C.line}`,
                bgcolor: "background.default",
                fontSize: 12,
                flexWrap: "wrap",
              }}
            >
              <Typography sx={{ fontSize: 13, fontWeight: 600 }}>
                {baseSym} <Box component="span" sx={{ color: "text.secondary", fontWeight: 400 }}>/ {quoteSym}</Box>
              </Typography>
              <Box sx={{ color: "text.secondary" }}>·</Box>
              {crossed ? (
                <Tooltip title={t("corkboard.book.crossedTip")}>
                  <Chip
                    size="small"
                    variant="outlined"
                    label={t("corkboard.book.crossed")}
                    sx={{ height: 18, fontSize: 10.5, color: C.bad, borderColor: C.badTintBorder, bgcolor: C.badTintBg, cursor: "help" }}
                  />
                </Tooltip>
              ) : (
                <Box sx={{ color: "text.secondary" }}>{spreadLabel}</Box>
              )}
              {mid != null && (
                <>
                  <Box sx={{ color: "text.secondary" }}>·</Box>
                  <Box sx={{ color: "text.secondary", fontFamily: C.mono }}>
                    {t("corkboard.book.mid", { price: `${fmtPriceDenom(mid, denom)} ${quoteUnit}` })}
                  </Box>
                </>
              )}
            </Box>

            <Box sx={{ display: "grid", gridTemplateColumns: { xs: "1fr", md: "1fr 1fr" } }}>
              <Box
                sx={{
                  borderRight: { md: `1px solid ${C.line}` },
                  borderBottom: { xs: `1px solid ${C.line}`, md: "none" },
                }}
              >
                <BookColumn side="bid" levels={bidLevels} {...colProps} />
              </Box>
              <BookColumn side="ask" levels={askLevels} {...colProps} />
            </Box>
          </Box>

          {/* Detail pane — the selected level's offers. Fixed below the book so
              the ladder never reflows and this stays in the same spot. */}
          <Box
            sx={{
              mt: 1.5,
              border: `1px solid ${C.line}`,
              borderRadius: 2,
              bgcolor: "background.paper",
              minHeight: 96,
              p: selLevel ? 1.5 : 0,
            }}
          >
            {!selLevel ? (
              <Box sx={{ display: "flex", alignItems: "center", justifyContent: "center", minHeight: 96, px: 2 }}>
                <Typography sx={{ fontSize: 13, color: "text.secondary", textAlign: "center" }}>
                  {t("corkboard.book.selectLevel")}
                </Typography>
              </Box>
            ) : (
              <Box sx={{ display: "flex", flexDirection: "column", gap: 1.25 }}>
                <Box sx={{ display: "flex", alignItems: "center", gap: 1, flexWrap: "wrap" }}>
                  <Chip
                    size="small"
                    label={selSide === "bid" ? t("corkboard.book.bids") : t("corkboard.book.asks")}
                    sx={{ height: 20, color: selSide === "bid" ? C.good : C.bad, borderColor: C.line, bgcolor: selSide === "bid" ? C.goodTintBg : C.badTintBg }}
                    variant="outlined"
                  />
                  <Typography sx={{ fontFamily: C.mono, fontSize: 13, fontWeight: 600 }}>
                    {t("corkboard.book.paneHeader", {
                      size: fmtBareLocale(selLevel.sizeSat),
                      base: baseSym,
                      price: fmtPriceDenom(selLevel.price, denom),
                      unit: quoteUnit,
                    })}
                  </Typography>
                  <Box sx={{ flex: 1 }} />
                  <Typography sx={{ fontSize: 11, color: "text.secondary" }}>
                    {t("corkboard.book.levelOffers", { count: selLevel.offers.length })}
                  </Typography>
                </Box>
                {/* Offers at this price, as a list — biggest size first. */}
                <Stack divider={<Divider flexItem />}>
                  {[...selLevel.offers]
                    .sort((a, b) => baseSizeOf(b) - baseSizeOf(a))
                    .map((o) => (
                      <OfferRow
                        key={o.swap_id}
                        o={o}
                        mine={o.from === myId}
                        staged={stagedIds.has(o.swap_id)}
                        state={offerState(o, mySwapIds)}
                        legDown={!coinLive(o.body.give_asset) || !coinLive(o.body.get_asset)}
                        watchOnly={watchOnly}
                        fmtLeg={fmtLeg}
                        onTake={() => void take(o)}
                        onRevoke={() => void revoke(o)}
                      />
                    ))}
                </Stack>
              </Box>
            )}
          </Box>
        </>
      )}

      {hidden.length > 0 && (
        <Typography sx={{ color: "text.secondary", fontSize: 12, mt: 1.5 }}>
          {t("corkboard.hiddenOffers", { count: hidden.length })}{" "}
          <Link component="button" onClick={() => navigate("settings")} underline="hover">
            {t("nav.settings")} → {t("nav.coins")}
          </Link>
        </Typography>
      )}

      {/* Active/taken swaps now live in a global dock above the activity log
          (App.tsx) so they stay in view from any screen. */}

    </>
  );
}

interface Level {
  key: string; // reduced rational price (exact-match grouping key)
  price: number; // quote per base
  sizeSat: number; // total base on offer at this price
  offers: Offer[];
}

/** Project the visible offers onto a two-sided book for the chosen base/quote.
 *  Exact-rate offers collapse into one level; bids sort high→low and asks
 *  low→high so each column's best price sits at the top, nearest the spread. */
function buildBook(visible: Offer[], base: string, quote: string) {
  const bids = new Map<string, Level>();
  const asks = new Map<string, Level>();
  for (const o of visible) {
    const e = bookEntry(o.body, base, quote);
    if (!e) continue;
    const m = e.side === "bid" ? bids : asks;
    let lvl = m.get(e.priceKey);
    if (!lvl) {
      lvl = { key: e.priceKey, price: e.price, sizeSat: 0, offers: [] };
      m.set(e.priceKey, lvl);
    }
    lvl.sizeSat += e.sizeSat;
    lvl.offers.push(o);
  }
  const bidLevels = [...bids.values()].sort((a, b) => b.price - a.price);
  const askLevels = [...asks.values()].sort((a, b) => a.price - b.price);
  const sizes = [...bidLevels, ...askLevels].map((l) => l.sizeSat);
  const maxSize = sizes.length ? Math.max(...sizes) : 1;
  return { bidLevels, askLevels, maxSize, bestBid: bidLevels[0]?.price, bestAsk: askLevels[0]?.price };
}

interface ColProps {
  side: BookSide;
  levels: Level[];
  maxSize: number;
  baseSym: string;
  quoteSym: string;
  priceUnit: string;
  denom: Denom;
  myId: string | null;
  stagedIds: Set<string>;
  selectedKey: string | null;
  onSelect: (key: string) => void;
}

// Only the top of book is usually relevant, so each side caps at this many
// price levels; the rest fold behind a "Show N more" toggle.
const DEPTH_CAP = 8;

function BookColumn({ side, levels, maxSize, baseSym, quoteSym, priceUnit, denom, myId, stagedIds, selectedKey, onSelect }: ColProps) {
  const t = useT();
  const [showAll, setShowAll] = useState(false);
  const isBid = side === "bid";
  const priceColor = isBid ? C.good : C.bad;
  const title = isBid ? t("corkboard.book.bids") : t("corkboard.book.asks");
  const hint = isBid
    ? t("corkboard.book.bidsHint", { base: baseSym, quote: quoteSym })
    : t("corkboard.book.asksHint", { base: baseSym, quote: quoteSym });

  const overflow = levels.length - DEPTH_CAP;
  const shown = showAll ? levels : levels.slice(0, DEPTH_CAP);

  return (
    <Box>
      <Box sx={{ px: 1.5, pt: 1.25, pb: 0.5, display: "flex", alignItems: "baseline", gap: 1 }}>
        <Typography sx={{ fontWeight: 600, fontSize: 13, color: priceColor }}>{title}</Typography>
        <Typography sx={{ fontSize: 11, color: "text.secondary" }}>{hint}</Typography>
      </Box>

      {/* Column header — Price (quote unit) / Size (base). Mirrored so price
          always hugs the centre divider. */}
      <Box
        sx={{
          display: "grid",
          gridTemplateColumns: "auto 1fr auto",
          alignItems: "center",
          gap: 1,
          px: 1.5,
          pb: 0.5,
          fontSize: 10.5,
          textTransform: "uppercase",
          letterSpacing: "0.06em",
          color: "text.secondary",
        }}
      >
        {isBid ? (
          <>
            <Box sx={{ width: 20 }} />
            <Box sx={{ textAlign: "left" }}>
              {t("corkboard.book.size")} ({baseSym})
            </Box>
            <Box sx={{ textAlign: "right" }}>
              {t("corkboard.book.price")} ({priceUnit})
            </Box>
          </>
        ) : (
          <>
            <Box sx={{ textAlign: "left" }}>
              {t("corkboard.book.price")} ({priceUnit})
            </Box>
            <Box sx={{ textAlign: "right" }}>
              {t("corkboard.book.size")} ({baseSym})
            </Box>
            <Box sx={{ width: 20 }} />
          </>
        )}
      </Box>

      {levels.length === 0 ? (
        <Typography sx={{ px: 1.5, py: 2, fontSize: 12, color: "text.secondary", textAlign: "center" }}>
          {isBid ? t("corkboard.book.noBids") : t("corkboard.book.noAsks")}
        </Typography>
      ) : (
        <Box sx={{ pb: 1 }}>
          {shown.map((lvl) => (
            <LevelRow
              key={lvl.key}
              side={side}
              lvl={lvl}
              maxSize={maxSize}
              denom={denom}
              priceColor={priceColor}
              baseSym={baseSym}
              mineHere={lvl.offers.some((o) => o.from === myId)}
              stagedHere={lvl.offers.some((o) => o.from === myId && stagedIds.has(o.swap_id))}
              selected={selectedKey === `${side}:${lvl.key}`}
              onSelect={() => onSelect(`${side}:${lvl.key}`)}
            />
          ))}
          {overflow > 0 && (
            <Box sx={{ px: 1.5, pt: 0.75 }}>
              <Link
                component="button"
                underline="hover"
                onClick={() => setShowAll((v) => !v)}
                sx={{ fontSize: 12, color: "text.secondary" }}
              >
                {showAll
                  ? t("corkboard.book.showLess", { count: DEPTH_CAP })
                  : t("corkboard.book.showMore", { count: overflow })}
              </Link>
            </Box>
          )}
        </Box>
      )}
    </Box>
  );
}

function LevelRow({
  side,
  lvl,
  maxSize,
  denom,
  priceColor,
  baseSym,
  mineHere,
  stagedHere,
  selected,
  onSelect,
}: {
  side: BookSide;
  lvl: Level;
  maxSize: number;
  denom: Denom;
  priceColor: string;
  baseSym: string;
  mineHere: boolean;
  /** This level's only own offer(s) are staged (not yet relay-confirmed). */
  stagedHere?: boolean;
  selected: boolean;
  onSelect: () => void;
}) {
  const t = useT();
  const isBid = side === "bid";
  const pct = Math.max(3, (lvl.sizeSat / maxSize) * 100);

  const price = (
    <Box sx={{ color: priceColor, fontWeight: 600, textAlign: isBid ? "right" : "left", fontVariantNumeric: "tabular-nums" }}>
      {fmtPriceDenom(lvl.price, denom)}
    </Box>
  );
  const size = (
    <Box sx={{ textAlign: isBid ? "left" : "right", fontVariantNumeric: "tabular-nums" }}>{fmtBareLocale(lvl.sizeSat)}</Box>
  );
  const meta = (
    <Box sx={{ display: "flex", alignItems: "center", justifyContent: isBid ? "flex-start" : "flex-end", gap: 0.5, width: 20 }}>
      {mineHere && (
        <Box
          sx={{
            width: 6,
            height: 6,
            borderRadius: "50%",
            // Hollow dot while staged (posted, not yet relay-confirmed); filled
            // once it's live on the board.
            ...(stagedHere ? { border: `1px solid ${C.dim}` } : { bgcolor: C.dim }),
          }}
        />
      )}
      <Typography sx={{ fontSize: 11, color: "text.secondary" }}>{lvl.offers.length}</Typography>
    </Box>
  );

  return (
    <Tooltip
      title={t("corkboard.book.depthTip", { sym: `${fmtBareLocale(lvl.sizeSat)} ${baseSym}`, count: lvl.offers.length })}
      placement={isBid ? "left" : "right"}
    >
      <Box
        onClick={onSelect}
        sx={{
          position: "relative",
          cursor: "pointer",
          bgcolor: selected ? C.raised : "transparent",
          [isBid ? "borderRight" : "borderLeft"]: selected ? `2px solid ${priceColor}` : "2px solid transparent",
          "&:hover": { bgcolor: C.raised },
        }}
      >
        <Box
          sx={{
            position: "absolute",
            top: 2,
            bottom: 2,
            [isBid ? "right" : "left"]: 0,
            width: `${pct}%`,
            bgcolor: isBid ? C.goodTintBg : C.badTintBg,
          }}
        />
        <Box
          sx={{
            position: "relative",
            display: "grid",
            gridTemplateColumns: "auto 1fr auto",
            alignItems: "center",
            gap: 1,
            px: 1.5,
            py: 0.7,
            fontFamily: C.mono,
            fontSize: 13,
          }}
        >
          {isBid ? (
            <>
              {meta}
              {size}
              {price}
            </>
          ) : (
            <>
              {price}
              {size}
              {meta}
            </>
          )}
        </Box>
      </Box>
    </Tooltip>
  );
}

function OfferStateChip({ state }: { state: OfferState }) {
  const t = useT();
  if (state === "open") return null; // open is the default, no badge needed
  const map: Record<Exclude<OfferState, "open">, { label: string; color: string; bg: string; border: string }> = {
    "taken-by-us": { label: t("corkboard.states.takenByUs"), color: C.accent, bg: C.warnTintBg, border: C.warnTintBorder },
    revoked: { label: t("corkboard.states.revoked"), color: C.dim, bg: "transparent", border: C.line },
    expired: { label: t("corkboard.states.expired"), color: C.dim, bg: "transparent", border: C.line },
  };
  const s = map[state];
  return (
    <Chip size="small" variant="outlined" label={s.label} sx={{ height: 22, color: s.color, bgcolor: s.bg, borderColor: s.border }} />
  );
}

function OfferRow({
  o,
  mine,
  staged,
  state,
  legDown,
  watchOnly,
  fmtLeg,
  onTake,
  onRevoke,
}: {
  o: Offer;
  mine: boolean;
  /** Our own offer, shown optimistically but not yet seen back from a relay. */
  staged?: boolean;
  state: OfferState;
  /** One of the offer's legs has a down/unconfigured node — can't take it. */
  legDown?: boolean;
  /** Watch-only viewer: taking is disabled (withdrawing our own still works). */
  watchOnly?: boolean;
  fmtLeg: Leg;
  onTake: () => void;
  onRevoke: () => void;
}) {
  const t = useT();
  const b = o.body;
  const expiry = b.created ? b.created + (b.ttl_secs || 24 * 3600) : 0;
  const f = freshness(b.created || 0, expiry);
  const freshColor = f.cls === "expiring" ? C.bad : C.dim;
  const takeable = !mine && state === "open" && !legDown && !watchOnly;

  // One list row: who · state · amounts · timing · action.
  return (
    <Box
      sx={{
        display: "flex",
        alignItems: "center",
        flexWrap: "wrap",
        gap: 1.25,
        py: 1,
        px: 0.5,
        opacity: staged ? 0.65 : state === "expired" ? 0.6 : 1,
        fontStyle: staged ? "italic" : "normal",
        bgcolor: mine ? C.mineBg : "transparent",
      }}
    >
      {mine ? (
        <Tooltip title={staged ? t("corkboard.offerStagedTip") : ""} disableHoverListener={!staged}>
          <Chip
            size="small"
            variant="outlined"
            label={staged ? t("corkboard.offerStaged") : t("corkboard.yourOffer")}
            sx={{ height: 22, fontStyle: staged ? "italic" : "normal" }}
          />
        </Tooltip>
      ) : (
        <CounterpartyTag id={o.from} />
      )}
      <OfferStateChip state={state} />
      {/* Every offer shows its swap type: v2 adaptor is primary-accented,
          v1 HTLC is a muted "Standard" chip. */}
      {b.protocol === "pact-htlc-v2" ? (
        <Tooltip title={t("coins.protoPrivateTip")}>
          <Chip
            size="small"
            variant="outlined"
            label={t("coins.protoPrivate")}
            sx={{ height: 22, color: "primary.main", borderColor: "primary.main", cursor: "help" }}
          />
        </Tooltip>
      ) : (
        <Tooltip title={t("coins.protoHtlcTip")}>
          <Chip
            size="small"
            variant="outlined"
            label={t("makeOffer.protoStandard")}
            sx={{ height: 22, color: "text.secondary", borderColor: "divider", cursor: "help" }}
          />
        </Tooltip>
      )}

      <Typography sx={{ fontFamily: C.mono, fontSize: 13, color: "text.primary", whiteSpace: "nowrap" }}>
        {mine
          ? `${fmtLeg(b.give_amount, b.give_asset)} → ${fmtLeg(b.get_amount, b.get_asset)}`
          : `${fmtLeg(b.get_amount, b.get_asset)} → ${fmtLeg(b.give_amount, b.give_asset)}`}
      </Typography>

      <Tooltip title={t("corkboard.safetyRefundTip")}>
        <Box component="span" sx={{ fontSize: 12, color: "text.secondary", cursor: "help", whiteSpace: "nowrap" }}>
          ⏱ <Box component="span" sx={{ fontFamily: C.mono, color: "text.primary" }}>{hours(b.t2_secs)}h / {hours(b.t1_secs)}h</Box>
        </Box>
      </Tooltip>

      <Tooltip title={t("format.posted", { age: ago(b.created || 0) })}>
        <Box sx={{ display: "inline-flex", alignItems: "center", gap: 0.625, fontSize: 12, color: freshColor, whiteSpace: "nowrap" }}>
          <Box sx={{ width: 6, height: 6, borderRadius: "50%", bgcolor: f.cls === "expiring" ? C.bad : C.good }} />
          {f.label}
        </Box>
      </Tooltip>

      <Box sx={{ flex: 1 }} />

      {mine ? (
        <Tooltip title={t("corkboard.withdrawTip")}>
          <Button size="small" variant="outlined" color="inherit" onClick={onRevoke}>
            {t("corkboard.withdraw")}
          </Button>
        </Tooltip>
      ) : watchOnly && state === "open" ? (
        <Tooltip title={t("watchOnly.takeBlockedTip")}>
          <span>
            <Button size="small" variant="contained" startIcon={<AddIcon />} disabled>
              {t("corkboard.take")}
            </Button>
          </span>
        </Tooltip>
      ) : legDown && state === "open" ? (
        <Tooltip title={t("corkboard.legDown")}>
          <span>
            <Button size="small" variant="contained" startIcon={<AddIcon />} disabled>
              {t("corkboard.take")}
            </Button>
          </span>
        </Tooltip>
      ) : (
        <Button size="small" variant="contained" startIcon={<AddIcon />} onClick={onTake} disabled={!takeable}>
          {t("corkboard.take")}
        </Button>
      )}
    </Box>
  );
}
