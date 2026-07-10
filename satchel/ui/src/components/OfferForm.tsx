// The shared offer-builder form. The user picks a CANONICAL pair (base/quote,
// ordered the same everywhere — see `baseQuote`), a direction (Sell/Buy the
// base), the base-coin amount, and a unit price that is ALWAYS quote-per-base
// (e.g. BTC per BTCX) — invariant to direction, so flipping Sell↔Buy never
// changes the price. The price unit is a user-chosen denom (BTC/mBTC/µBTC/sat),
// defaulting to milli for a BTCX base and coin otherwise. It hands the validated
// (give, want, t1, t2, protocol, ttl) to `onSubmit`. Used by the public "Post an
// offer" dialog (→ boardpostoffer) and the private "Create slip" screen
// (→ makeprivateoffer). No swap logic lives here.

import { useEffect, useMemo, useState, type ReactNode } from "react";
import {
  Box,
  Button,
  Divider,
  MenuItem,
  Select,
  TextField,
  ToggleButton,
  ToggleButtonGroup,
  Typography,
} from "@mui/material";
import { useApp } from "../AppContext";
import { useConfirm } from "../ui/ConfirmProvider";
import { useDenom } from "../denom";
import { useFx, useFxContext } from "../fx";
import { useT } from "../i18n";
import { usePrefs } from "../prefs";
import { rpc } from "../api/tauri";
import {
  baseQuote,
  canonicalAmount,
  decimalSeparator,
  denomLabel,
  denomUnitSats,
  DENOMS,
  fmtBare,
  fmtCash,
  priceCash,
  fmtPrice,
  hours,
  offerCash,
  offerProtocols,
  pairKey,
  parseAmount,
  sanitizeAmountInput,
  PROTOCOL_V2,
  type Denom,
} from "../format";
import FeePreview from "./FeePreview";
import { LockFundsGate } from "./InsufficientFunds";
import { C } from "../theme";
import type { Pair } from "../api/types";

// Timelock presets — raw T1/T2 hours are too low-level (and dangerous) to ask a
// user for, so we offer Short/Medium/Long with safe 2:1 gaps (T1 = 2×T2). Every
// preset clears the engine's spec §7.4 action margins. Medium default.
export const TERMS = {
  short: { t1: 12 * 3600, t2: 6 * 3600 },
  medium: { t1: 24 * 3600, t2: 12 * 3600 },
  long: { t1: 36 * 3600, t2: 18 * 3600 },
} as const;
export type Term = keyof typeof TERMS;

// "Valid for" presets (offer lifetime, minutes) — the common lifetimes as one
// tap; Custom reveals the raw minutes field. The last choice (preset or custom
// minutes) persists in satchel.json (ui.offer_ttl_min), so the form reopens the
// way you left it; stored minutes map back to their preset chip, else Custom.
const TTL_PRESETS = [60, 240, 480, 1440, 10080] as const;
const TTL_LABEL_KEY: Record<number, string> = {
  60: "makeOffer.ttl1h",
  240: "makeOffer.ttl4h",
  480: "makeOffer.ttl8h",
  1440: "makeOffer.ttl24h",
  10080: "makeOffer.ttl1w",
};

// The Corkboard persists its selected pair here; the form defaults to it so you
// land on the pair you were just looking at.
const CORKBOARD_PAIR_KEY = "satchel.corkboard.pair";
// localStorage key the Corkboard denom (useDenom) persists under — we read it to
// decide whether to honour it or fall back to our per-base default.
const CORKBOARD_DENOM_KEY = "satchel.denom";
// Fallback price-unit when the Corkboard denom was never set: milli for a BTCX
// base (its quote price is tiny → reads better), coin for everything else.
const defaultDenom = (base: string): Denom => (base === "btcx" ? "milli" : "coin");

type Side = "sell" | "buy"; // sell = give the base coin; buy = get the base coin

export default function OfferForm({
  submitLabel,
  submitIcon,
  confirmTitle,
  busy,
  error,
  onSubmit,
}: {
  submitLabel: string;
  submitIcon?: ReactNode;
  /** Title for the review/confirm dialog shown before posting. */
  confirmTitle: string;
  busy: boolean;
  error: string | null;
  onSubmit: (
    give: string,
    want: string,
    t1: number,
    t2: number,
    protocol?: string,
    ttlSecs?: number,
  ) => void;
}) {
  const { coins, symOf } = useApp();
  const confirm = useConfirm();
  const t = useT();
  const { prefs, update: updatePrefs } = usePrefs();
  const configured = useMemo(() => coins.filter((c) => c.configured), [coins]);

  // Tradable pairs (capability-derived, from listpairs) → canonical base/quote.
  const [pairs, setPairs] = useState<{ key: string; base: string; quote: string }[]>([]);
  const [pairKeySel, setPairKeySel] = useState<string>(() => {
    try {
      return localStorage.getItem(CORKBOARD_PAIR_KEY) || "";
    } catch {
      return "";
    }
  });
  const [side, setSide] = useState<Side>("sell");
  const [baseAmt, setBaseAmt] = useState("");
  const [price, setPrice] = useState(""); // quote-per-base, in the chosen denom
  // Price unit is the SAME shared preference the Corkboard uses (useDenom) — the
  // form always mirrors it, so the two can never drift. We only SEED it (below)
  // when it was never chosen, from our per-base default.
  const { denom, setDenom } = useDenom();
  const onDenom = (d: Denom) => setDenom(d);
  const [proto, setProto] = useState<string | null>(null); // explicit override
  const [term, setTerm] = useState<Term>("medium");
  // Offer lifetime — seeded from the persisted last choice (60 = the 1h default
  // on a fresh install); a stored value off the preset grid opens as Custom.
  const storedTtl = Math.max(1, Math.round(prefs.offer_ttl_min || 60));
  const [ttlSel, setTtlSel] = useState<number | "custom">(() =>
    (TTL_PRESETS as readonly number[]).includes(storedTtl) ? storedTtl : "custom",
  );
  const [validMin, setValidMin] = useState(String(storedTtl)); // custom minutes
  const [balances, setBalances] = useState<Record<string, string>>({});

  // Load the capability-derived pairs (same source as the Corkboard).
  useEffect(() => {
    let alive = true;
    (async () => {
      try {
        const r = await rpc<{ pairs: Pair[] }>("listpairs");
        const opts = r.pairs
          .filter((p) => p.available)
          .map((p) => {
            const { base, quote } = baseQuote(p.coin_a, p.coin_b);
            return { key: pairKey(p.coin_a, p.coin_b), base, quote };
          })
          .sort((x, y) => symOf(x.base).localeCompare(symOf(y.base)));
        if (alive) setPairs(opts);
      } catch {
        /* none yet */
      }
    })();
    return () => {
      alive = false;
    };
  }, [symOf]);

  // Default to the Corkboard's pair; fall back to the first if it's gone.
  useEffect(() => {
    if (!pairs.length) return;
    if (!pairs.some((p) => p.key === pairKeySel)) setPairKeySel(pairs[0].key);
  }, [pairs, pairKeySel]);

  const pair = useMemo(() => pairs.find((p) => p.key === pairKeySel), [pairs, pairKeySel]);
  const base = pair?.base ?? "";
  const quote = pair?.quote ?? "";
  // Bind the sidebar Cashrate entry to this form's quote coin (issue #56).
  useFxContext(quote);

  // Seed the shared denom once, from our per-base default (mBTC for a BTCX base,
  // coin else), only if it was never chosen — so BTCX prices read well on a fresh
  // setup AND the Corkboard immediately mirrors it. Thereafter it's one shared
  // preference the user controls from either screen.
  useEffect(() => {
    if (!base) return;
    try {
      if (localStorage.getItem(CORKBOARD_DENOM_KEY) == null) setDenom(defaultDenom(base));
    } catch {
      /* ignore */
    }
  }, [base, setDenom]);

  // Live wallet balances per configured coin.
  useEffect(() => {
    let alive = true;
    (async () => {
      for (const c of configured) {
        try {
          const r = await rpc<{ balance_sat: number }>("getbalance", [c.id]);
          if (alive) setBalances((b) => ({ ...b, [c.id]: fmtBare(r.balance_sat) }));
        } catch {
          /* node maybe down — leave blank */
        }
      }
    })();
    return () => {
      alive = false;
    };
  }, [configured]);

  // The give/get coins follow the direction; the SIZE is always the base coin and
  // the PRICE is always quote-per-base, so neither changes when you flip.
  const giveCoin = side === "sell" ? base : quote;
  const wantCoin = side === "sell" ? quote : base;

  const baseNum = parseAmount(baseAmt);
  const priceNum = parseAmount(price);
  // quote sats = baseCoin × (quote-sats per base-coin); price is in `denom` units.
  const quoteSats =
    baseNum > 0 && priceNum > 0 ? Math.round(baseNum * priceNum * denomUnitSats(denom)) : 0;
  const baseSats = baseNum > 0 ? Math.round(baseNum * 1e8) : 0;
  // The raw quote-per-base in whole quote coin, for the "= 0.0000043 BTC/BTCX" hint.
  const rawPriceCoin = priceNum > 0 ? (priceNum * denomUnitSats(denom)) / 1e8 : 0;

  const giveStr = side === "sell" ? canonicalAmount(baseAmt) : fmtBare(quoteSats);
  const wantStr = side === "sell" ? fmtBare(quoteSats) : canonicalAmount(baseAmt);
  const giveSat = side === "sell" ? baseSats : quoteSats;
  const wantSat = side === "sell" ? quoteSats : baseSats;

  // Muted cash-equivalent of the offer from the user's own Cashrate (issue
  // #56); both legs are equal-valued at the entered price, so one figure
  // annotates both summary lines. No rate set reads "—".
  const { enabled: fxOn, rateOf } = useFx();
  // Unit-price cash equivalent for the price hint line (rc10 review): cash per
  // 1 base coin through the quote coin's rate — same derivation as the ladder's
  // ~Cash price columns. Omitted (not "—") when no rate is set: the hint line
  // is prose, not a table cell.
  const unitCash = fxOn && rawPriceCoin > 0 ? priceCash(rawPriceCoin, rateOf(quote)) : null;
  const cashLeg = fxOn
    ? fmtCash(
        offerCash(
          { give_asset: giveCoin, give_amount: giveSat, get_asset: wantCoin, get_amount: wantSat },
          rateOf,
        ),
      )
    : null;

  // Chain-up gate: refuse to post when a leg's own node is down (the engine does
  // too — this is the friendly up-front block).
  const coinLive = (id: string) => coins.find((c) => c.id === id)?.status === "ok";
  const legDown = !!base && !!quote && (!coinLive(base) || !coinLive(quote));

  // Which swap protocols this pair allows (capabilities are symmetric, so pass
  // base+quote regardless of direction), and the preferred default.
  const baseCaps = useMemo(() => configured.find((c) => c.id === base)?.capabilities, [configured, base]);
  const quoteCaps = useMemo(() => configured.find((c) => c.id === quote)?.capabilities, [configured, quote]);
  const { options: protoOptions, preferred } = useMemo(
    () => (base && quote ? offerProtocols(baseCaps, quoteCaps) : { options: [], preferred: null }),
    [baseCaps, quoteCaps, base, quote],
  );
  const effProto = proto && protoOptions.includes(proto) ? proto : preferred;
  const protoLabel = (p: string) => (p === PROTOCOL_V2 ? t("coins.protoPrivate") : t("makeOffer.protoStandard"));

  const valid = !!base && !!quote && !legDown && baseNum > 0 && quoteSats > 0 && !!effProto;

  const unitLabel = quote ? denomLabel(quote, symOf(quote), denom) : "";

  // Effective offer lifetime in minutes (preset chip, or the custom field).
  const ttlMin = ttlSel === "custom" ? Math.max(1, Math.round(Number(validMin) || 60)) : ttlSel;
  // Both handlers write the choice through to satchel.json so it survives
  // restarts; custom keystrokes only persist once they parse to a real minute.
  const onTtlPreset = (v: number | "custom") => {
    setTtlSel(v);
    updatePrefs({ offer_ttl_min: v === "custom" ? Math.max(1, Math.round(Number(validMin) || 60)) : v });
  };
  const onTtlCustom = (raw: string) => {
    setValidMin(raw);
    const mins = Math.round(Number(raw));
    if (Number.isFinite(mins) && mins >= 1) updatePrefs({ offer_ttl_min: mins });
  };

  async function submit() {
    if (!valid || busy || !effProto) return;
    const { t1, t2 } = TERMS[term];
    const validForMin = ttlMin;
    const ttlSecs = validForMin * 60;
    const lbl = { fontSize: 12, color: "text.secondary" } as const;
    const val = { textAlign: "right", fontFamily: C.mono, fontSize: 13.5 } as const;
    // The funds pre-check streams INSIDE the dialog body (LockFundsGate below),
    // so the review opens instantly instead of stalling on chain-touching calls.
    const ok = await confirm({
      title: confirmTitle,
      wide: true,
      confirmLabel: submitLabel,
      body: (
        <Box sx={{ display: "flex", flexDirection: "column", gap: 1.5 }}>
          <Box
            sx={{
              display: "grid",
              gridTemplateColumns: "auto 1fr",
              rowGap: 0.6,
              columnGap: 1.5,
              border: `1px solid ${C.line}`,
              borderRadius: 1.5,
              p: 1.25,
              alignItems: "center",
            }}
          >
            <Typography sx={lbl}>{t("makeOffer.give")}</Typography>
            <Box sx={val}>{giveStr} {symOf(giveCoin)}</Box>
            <Typography sx={lbl}>{t("makeOffer.want")}</Typography>
            <Box sx={{ ...val, color: "primary.main" }}>{wantStr} {symOf(wantCoin)}</Box>
            <Typography sx={lbl}>{t("makeOffer.price")}</Typography>
            <Box sx={val}>
              {price} {unitLabel} / {symOf(base)}
            </Box>
            <Box sx={{ gridColumn: "1 / -1", my: 0.25 }}>
              <Divider />
            </Box>
            <Typography sx={lbl}>{t("makeOffer.protocol")}</Typography>
            <Box sx={{ textAlign: "right", fontSize: 13 }}>{protoLabel(effProto)}</Box>
            <Typography sx={lbl}>{t("takeConfirm.safetyRefund")}</Typography>
            <Box sx={val}>
              {hours(t2)}h / {hours(t1)}h
            </Box>
            <Typography sx={lbl}>{t("makeOffer.validFor")}</Typography>
            <Box sx={val}>{t("makeOffer.validForMins", { mins: validForMin })}</Box>
          </Box>
          <Typography sx={{ fontSize: 12, color: "text.secondary" }}>{t("makeOffer.note")}</Typography>
          <FeePreview giveCoin={giveCoin} getCoin={wantCoin} />
          <LockFundsGate lockCoin={giveCoin} otherCoin={wantCoin} amountSat={giveSat} />
        </Box>
      ),
    });
    if (!ok) return;
    const forced = effProto === preferred ? undefined : effProto;
    onSubmit(`${giveCoin}:${giveStr}`, `${wantCoin}:${wantStr}`, t1, t2, forced, ttlSecs);
  }

  const noPairs = pairs.length === 0;

  return (
    <Box
      component="form"
      onSubmit={(e) => {
        e.preventDefault();
        submit();
      }}
      sx={{ display: "flex", flexDirection: "column", gap: 1.75 }}
    >
      {/* Pair */}
      <TextField
        select
        label={t("makeOffer.pair")}
        size="small"
        fullWidth
        value={noPairs ? "" : pairKeySel}
        onChange={(e) => setPairKeySel(e.target.value)}
        disabled={noPairs}
        helperText={noPairs ? t("makeOffer.noPairs") : " "}
      >
        {noPairs && (
          <MenuItem value="" disabled>
            {t("makeOffer.noCoins")}
          </MenuItem>
        )}
        {pairs.map((p) => (
          <MenuItem key={p.key} value={p.key}>
            {symOf(p.base)} / {symOf(p.quote)}
          </MenuItem>
        ))}
      </TextField>

      {/* Direction — Sell/Buy the base coin (this is the give↔get swap). */}
      <ToggleButtonGroup
        exclusive
        fullWidth
        size="small"
        value={side}
        onChange={(_, v) => v && setSide(v as Side)}
      >
        <ToggleButton value="sell">{t("makeOffer.sell", { sym: symOf(base) })}</ToggleButton>
        <ToggleButton value="buy">{t("makeOffer.buy", { sym: symOf(base) })}</ToggleButton>
      </ToggleButtonGroup>

      {/* Amount — always in the base coin. */}
      <TextField
        label={t("makeOffer.amount")}
        size="small"
        fullWidth
        placeholder={`0${decimalSeparator()}0`}
        value={baseAmt}
        onChange={(e) => setBaseAmt(sanitizeAmountInput(e.target.value))}
        inputMode="decimal"
        autoComplete="off"
        InputProps={{ endAdornment: <Typography sx={{ color: "text.secondary", fontSize: 13 }}>{symOf(base)}</Typography> }}
        helperText={
          base
            ? balances[base] !== undefined
              ? t("makeOffer.balance", { amt: balances[base], sym: symOf(base) })
              : t("makeOffer.balanceLoading")
            : " "
        }
      />

      {/* Price — quote per base, invariant; unit is the user's denom choice. */}
      <Box>
        <Box sx={{ display: "flex", gap: 1, alignItems: "flex-start" }}>
          <TextField
            label={t("makeOffer.price")}
            size="small"
            fullWidth
            placeholder={t("makeOffer.pricePlaceholder")}
            value={price}
            onChange={(e) => setPrice(sanitizeAmountInput(e.target.value))}
            inputMode="decimal"
            autoComplete="off"
          />
          <Select size="small" value={denom} onChange={(e) => onDenom(e.target.value as Denom)} sx={{ minWidth: 96 }}>
            {DENOMS.map((d) => (
              <MenuItem key={d} value={d}>
                {quote ? denomLabel(quote, symOf(quote), d) : d}
              </MenuItem>
            ))}
          </Select>
        </Box>
        <Typography sx={{ fontSize: 11.5, color: "text.secondary", mt: 0.5 }}>
          {base && quote
            ? t("makeOffer.priceUnit", { unit: unitLabel, base: symOf(base) }) +
              (rawPriceCoin > 0 ? `  ·  ${fmtPrice(rawPriceCoin)} ${symOf(quote)} / ${symOf(base)}` : "") +
              (unitCash != null ? `  ·  ${fmtCash(unitCash)} / ${symOf(base)}` : "")
            : " "}
        </Typography>
      </Box>

      {/* Derived give/get summary. */}
      {valid && (
        <Box sx={{ display: "flex", flexDirection: "column", gap: 0.25, px: 0.5 }}>
          <Typography sx={{ fontSize: 12.5 }}>
            <Box component="span" sx={{ color: "text.secondary" }}>{t("makeOffer.youGive")}: </Box>
            <Box component="span" sx={{ fontFamily: C.mono }}>{giveStr} {symOf(giveCoin)}</Box>
            {cashLeg != null && (
              <Box component="span" sx={{ fontFamily: C.mono, color: "text.secondary" }}>
                {" "}· {cashLeg}
              </Box>
            )}
          </Typography>
          <Typography sx={{ fontSize: 12.5 }}>
            <Box component="span" sx={{ color: "text.secondary" }}>{t("makeOffer.youGet")}: </Box>
            <Box component="span" sx={{ fontFamily: C.mono, color: "primary.main" }}>{wantStr} {symOf(wantCoin)}</Box>
            {cashLeg != null && (
              <Box component="span" sx={{ fontFamily: C.mono, color: "text.secondary" }}>
                {" "}· {cashLeg}
              </Box>
            )}
          </Typography>
        </Box>
      )}

      {legDown && (
        <Typography sx={{ color: "error.main", fontSize: 12 }}>{t("makeOffer.legDown")}</Typography>
      )}

      {/* Swap type — dropdown when the pair allows more than one, else a line. */}
      {base && quote && effProto && (
        protoOptions.length > 1 ? (
          <TextField
            select
            label={t("makeOffer.protocol")}
            size="small"
            fullWidth
            value={effProto}
            onChange={(e) => setProto(e.target.value)}
          >
            {protoOptions.map((p) => (
              <MenuItem key={p} value={p}>
                {protoLabel(p)}
              </MenuItem>
            ))}
          </TextField>
        ) : (
          <Typography sx={{ fontSize: 12, color: "text.secondary" }}>
            {t("makeOffer.protocol")}: {protoLabel(effProto)}
          </Typography>
        )
      )}

      {/* Timelock preset. */}
      <Box>
        <Typography sx={{ fontSize: 12, color: "text.secondary", mb: 0.75 }}>{t("makeOffer.term")}</Typography>
        <ToggleButtonGroup exclusive fullWidth size="small" value={term} onChange={(_, v) => v && setTerm(v as Term)}>
          <ToggleButton value="short">{t("makeOffer.termShort")}</ToggleButton>
          <ToggleButton value="medium">{t("makeOffer.termMedium")}</ToggleButton>
          <ToggleButton value="long">{t("makeOffer.termLong")}</ToggleButton>
        </ToggleButtonGroup>
        <Typography sx={{ fontSize: 11.5, color: "text.secondary", mt: 0.75 }}>
          {t(`makeOffer.termHint.${term}`)}
        </Typography>
      </Box>

      {/* Offer validity — preset chips (last choice persists across restarts);
          Custom reveals the raw minutes field. */}
      <Box>
        <Typography sx={{ fontSize: 12, color: "text.secondary", mb: 0.75 }}>
          {t("makeOffer.validForTitle")}
        </Typography>
        <ToggleButtonGroup
          exclusive
          fullWidth
          size="small"
          value={ttlSel}
          onChange={(_, v) => v != null && onTtlPreset(v as number | "custom")}
        >
          {TTL_PRESETS.map((m) => (
            <ToggleButton key={m} value={m}>
              {t(TTL_LABEL_KEY[m])}
            </ToggleButton>
          ))}
          <ToggleButton value="custom">{t("makeOffer.ttlCustom")}</ToggleButton>
        </ToggleButtonGroup>
        {ttlSel === "custom" ? (
          <TextField
            label={t("makeOffer.validFor")}
            size="small"
            fullWidth
            type="number"
            value={validMin}
            onChange={(e) => onTtlCustom(e.target.value)}
            inputProps={{ min: 1 }}
            helperText={t("makeOffer.validForHint")}
            sx={{ mt: 1 }}
          />
        ) : (
          <Typography sx={{ fontSize: 11.5, color: "text.secondary", mt: 0.75 }}>
            {t("makeOffer.validForHint")}
          </Typography>
        )}
      </Box>

      {error && (
        <Typography sx={{ color: "error.main", fontSize: 13, whiteSpace: "pre-wrap" }}>{error}</Typography>
      )}

      <Box sx={{ display: "flex", justifyContent: "flex-end" }}>
        <Button type="submit" variant="contained" startIcon={submitIcon} disabled={!valid || busy}>
          {submitLabel}
        </Button>
      </Box>
    </Box>
  );
}
