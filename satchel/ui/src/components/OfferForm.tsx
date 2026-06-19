// The shared offer-builder form: pick give/get coins, enter a unit price and an
// amount (price + either side fills the other, exchange-style), pick a swap type
// when the pair allows more than one, and a Short/Medium/Long timelock preset.
// It hands the validated (give, want, t1, t2, protocol) to `onSubmit`. Used by
// the public "Post an offer" dialog (→ boardpostoffer) and the private "Create
// slip" screen (→ makeprivateoffer). No swap logic lives here.

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
import { useT } from "../i18n";
import { rpc } from "../api/tauri";
import {
  canonicalAmount,
  decimalSeparator,
  fmtBare,
  fmtBareLocale,
  hours,
  offerProtocols,
  parseAmount,
  sanitizeAmountInput,
  PROTOCOL_V2,
} from "../format";
import FeePreview from "./FeePreview";
import { C } from "../theme";

// Timelock presets — raw T1/T2 hours are too low-level (and dangerous) to ask a
// user for, so we offer Short/Medium/Long with safe 2:1 gaps (T1 = 2×T2). Every
// preset clears the engine's spec §7.4 action margins: Bob must fund before
// T2−3h and Alice must reveal before T2−2h, so T2 needs real headroom. The old
// 3h/6h "short" (gap 3h) violated both the §7.4 fund margin and the 4h structural
// gap (T1−T2 ≥ 4h) and was rejected at take time — lifted here. T2 ≥ 6h now leaves
// a comfortable funding+reveal window; gaps (≥ 6h) sit well above the 4h minimum.
// Longer = safer margin but slower auto-refund if a trade stalls. Medium default.
export const TERMS = {
  short: { t1: 12 * 3600, t2: 6 * 3600 },
  medium: { t1: 24 * 3600, t2: 12 * 3600 },
  long: { t1: 36 * 3600, t2: 18 * 3600 },
} as const;
export type Term = keyof typeof TERMS;

// Display a derived price ratio (receive per give) compactly, in the locale.
function fmtPriceLocale(ratio: number): string {
  const s = (ratio >= 1 ? ratio.toFixed(6) : ratio.toPrecision(6))
    .replace(/0+$/, "")
    .replace(/\.$/, "");
  return s.replace(".", decimalSeparator());
}

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
  const { coins, symOf, network } = useApp();
  const confirm = useConfirm();
  const t = useT();
  const configured = useMemo(() => coins.filter((c) => c.configured), [coins]);

  const [give, setGive] = useState("");
  const [want, setWant] = useState("");
  const [giveAmt, setGiveAmt] = useState("");
  const [wantAmt, setWantAmt] = useState("");
  const [price, setPrice] = useState(""); // receive-coin per give-coin
  const [proto, setProto] = useState<string | null>(null); // explicit override
  const [term, setTerm] = useState<Term>("medium");
  const [validMin, setValidMin] = useState("60"); // offer lifetime (minutes)
  const [balances, setBalances] = useState<Record<string, string>>({});

  // Sensible defaults once coins load: give = first, want = a different one.
  useEffect(() => {
    if (!give && configured[0]) setGive(configured[0].id);
    if (!want && configured[1]) setWant(configured[1].id);
  }, [configured, give, want]);

  // Live wallet balances per configured coin (shown under each amount field).
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

  // Price + either amount fills the other; `give` is the size anchor. Amounts are
  // computed in sats so the posted values stay exact.
  const computeWantFrom = (g: string, p: string) => {
    const gn = parseAmount(g);
    const pn = parseAmount(p);
    if (gn > 0 && pn > 0) setWantAmt(fmtBareLocale(Math.round(gn * pn * 1e8)));
  };
  const onGiveAmt = (v: string) => {
    const s = sanitizeAmountInput(v);
    setGiveAmt(s);
    computeWantFrom(s, price);
  };
  const onPrice = (v: string) => {
    const s = sanitizeAmountInput(v);
    setPrice(s);
    computeWantFrom(giveAmt, s);
  };
  const onWantAmt = (v: string) => {
    const s = sanitizeAmountInput(v);
    setWantAmt(s);
    const gn = parseAmount(giveAmt);
    const wn = parseAmount(s);
    if (gn > 0 && wn > 0) setPrice(fmtPriceLocale(wn / gn));
  };

  const sameCoin = !!give && give === want;

  // Chain-up gate: don't let the user post/create an offer whose own node is
  // down — the engine refuses it too, this is the friendly up-front block.
  const coinLive = (id: string) => coins.find((c) => c.id === id)?.status === "ok";
  const legDown = !sameCoin && ((!!give && !coinLive(give)) || (!!want && !coinLive(want)));

  // Which swap protocols this pair+network allows, and the preferred default.
  const giveCaps = useMemo(() => configured.find((c) => c.id === give)?.capabilities, [configured, give]);
  const wantCaps = useMemo(() => configured.find((c) => c.id === want)?.capabilities, [configured, want]);
  const { options: protoOptions, preferred } = useMemo(
    () => (sameCoin ? { options: [], preferred: null } : offerProtocols(giveCaps, wantCaps, network)),
    [giveCaps, wantCaps, network, sameCoin],
  );
  const effProto = proto && protoOptions.includes(proto) ? proto : preferred;
  const protoLabel = (p: string) => (p === PROTOCOL_V2 ? t("coins.protoPrivate") : t("makeOffer.protoStandard"));

  const valid =
    !!give &&
    !!want &&
    !sameCoin &&
    !legDown &&
    parseAmount(giveAmt) > 0 &&
    parseAmount(wantAmt) > 0 &&
    !!effProto;

  async function submit() {
    if (!valid || busy || !effProto) return;
    const { t1, t2 } = TERMS[term];
    const validForMin = Math.max(1, Math.round(Number(validMin) || 60));
    const ttlSecs = validForMin * 60;
    const lbl = { fontSize: 12, color: "text.secondary" } as const;
    const val = { textAlign: "right", fontFamily: C.mono, fontSize: 13.5 } as const;
    // Review/confirm step (same shape as the take-offer summary): the trade, the
    // swap type, the refund window, the plain-language note, and the network-cost
    // breakdown all live here — the decision screen, not the form.
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
            <Box sx={val}>{giveAmt} {symOf(give)}</Box>
            <Typography sx={lbl}>{t("makeOffer.want")}</Typography>
            <Box sx={{ ...val, color: "primary.main" }}>{wantAmt} {symOf(want)}</Box>
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
          <FeePreview giveCoin={give} getCoin={want} />
        </Box>
      ),
    });
    if (!ok) return;
    // Only force a protocol when the user picked the non-preferred one; otherwise
    // let the engine apply its own default.
    const forced = effProto === preferred ? undefined : effProto;
    onSubmit(
      `${give}:${canonicalAmount(giveAmt)}`,
      `${want}:${canonicalAmount(wantAmt)}`,
      t1,
      t2,
      forced,
      ttlSecs,
    );
  }

  return (
    <Box
      component="form"
      onSubmit={(e) => {
        e.preventDefault();
        submit();
      }}
      sx={{ display: "flex", flexDirection: "column", gap: 1.5 }}
    >
      <MoneyRow
        label={t("makeOffer.give")}
        coins={configured}
        coin={give}
        amount={giveAmt}
        balance={give ? balances[give] : undefined}
        symOf={symOf}
        onCoin={setGive}
        onAmount={onGiveAmt}
        t={t}
      />

      {/* Unit price — receive falls out of give × price (and vice versa). */}
      <TextField
        label={t("makeOffer.price")}
        size="small"
        fullWidth
        placeholder={t("makeOffer.pricePlaceholder")}
        value={price}
        onChange={(e) => onPrice(e.target.value)}
        inputMode="decimal"
        autoComplete="off"
        helperText={give && want && !sameCoin ? t("makeOffer.priceUnit", { quote: symOf(want), give: symOf(give) }) : " "}
      />

      <MoneyRow
        label={t("makeOffer.want")}
        coins={configured}
        coin={want}
        amount={wantAmt}
        balance={want ? balances[want] : undefined}
        symOf={symOf}
        onCoin={setWant}
        onAmount={onWantAmt}
        t={t}
      />

      {sameCoin && (
        <Typography sx={{ color: "error.main", fontSize: 12 }}>{t("makeOffer.sameCoin")}</Typography>
      )}

      {!sameCoin && legDown && (
        <Typography sx={{ color: "error.main", fontSize: 12 }}>{t("makeOffer.legDown")}</Typography>
      )}

      {/* Swap type — a dropdown (scales to future protocols); a static line when
          the pair+network only supports one. */}
      {!sameCoin && effProto && (
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

      {/* Timelock preset (Short/Medium/Long) instead of raw T1/T2. */}
      <Box>
        <Typography sx={{ fontSize: 12, color: "text.secondary", mb: 0.75 }}>{t("makeOffer.term")}</Typography>
        <ToggleButtonGroup
          exclusive
          fullWidth
          size="small"
          value={term}
          onChange={(_, v) => v && setTerm(v as Term)}
        >
          <ToggleButton value="short">{t("makeOffer.termShort")}</ToggleButton>
          <ToggleButton value="medium">{t("makeOffer.termMedium")}</ToggleButton>
          <ToggleButton value="long">{t("makeOffer.termLong")}</ToggleButton>
        </ToggleButtonGroup>
        <Typography sx={{ fontSize: 11.5, color: "text.secondary", mt: 0.75 }}>
          {t(`makeOffer.termHint.${term}`)}
        </Typography>
      </Box>

      {/* Offer validity (minutes): while you're online the engine keeps the
          listing alive (refreshing its short relay TTL); after this window it
          stops and the offer expires. */}
      <TextField
        label={t("makeOffer.validFor")}
        size="small"
        type="number"
        value={validMin}
        onChange={(e) => setValidMin(e.target.value)}
        inputProps={{ min: 1 }}
        helperText={t("makeOffer.validForHint")}
      />

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

// One coin+amount line: a compact coin picker beside an amount field whose
// floating label names the side and whose helper text shows the live balance.
function MoneyRow({
  label,
  coins,
  coin,
  amount,
  balance,
  symOf,
  onCoin,
  onAmount,
  t,
}: {
  label: string;
  coins: { id: string; symbol: string }[];
  coin: string;
  amount: string;
  balance: string | undefined;
  symOf: (id: string) => string;
  onCoin: (id: string) => void;
  onAmount: (v: string) => void;
  t: (k: string, vars?: Record<string, string | number>) => string;
}) {
  return (
    <Box sx={{ display: "flex", gap: 1, alignItems: "flex-start" }}>
      <Select
        size="small"
        value={coins.some((c) => c.id === coin) ? coin : ""}
        onChange={(e) => onCoin(e.target.value)}
        displayEmpty
        sx={{ minWidth: 104 }}
      >
        {coins.length === 0 && (
          <MenuItem value="" disabled>
            {t("makeOffer.noCoins")}
          </MenuItem>
        )}
        {coins.map((c) => (
          <MenuItem key={c.id} value={c.id}>
            {symOf(c.id)}
          </MenuItem>
        ))}
      </Select>
      <TextField
        label={label}
        size="small"
        fullWidth
        placeholder={`0${decimalSeparator()}0`}
        value={amount}
        onChange={(e) => onAmount(e.target.value)}
        inputMode="decimal"
        autoComplete="off"
        helperText={
          coin
            ? balance !== undefined
              ? t("makeOffer.balance", { amt: balance, sym: symOf(coin) })
              : t("makeOffer.balanceLoading")
            : " "
        }
      />
    </Box>
  );
}
