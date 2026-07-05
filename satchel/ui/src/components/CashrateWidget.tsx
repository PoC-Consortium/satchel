import { useEffect, useState } from "react";
import { Box, Switch, TextField, Tooltip, Typography } from "@mui/material";
import { useApp } from "../AppContext";
import { useFx } from "../fx";
import { useT } from "../i18n";
import { canonicalAmount, decimalSeparator, parseAmount, sanitizeAmountInput } from "../format";

// The Cashrate entry (issue #56), pinned in the sidebar footer above Settings:
// toggle + context-bound label + locale-aware rate box. The label follows the
// coin the current screen is about (the quote coin of the Corkboard/offer-form
// pair, via useFxContext); on screens with no coin context everything greys
// out but keeps showing the last coin's rate. Rates are remembered per coin
// (see fx.tsx) — switch the pair and the box recalls that coin's stored rate.
export default function CashrateWidget() {
  const t = useT();
  const { symOf } = useApp();
  const { enabled, setEnabled, context, lastCoin, rates, setRate } = useFx();

  // Which coin the label/value shows: the live context, else the last one
  // (visible but disabled, so the widget never goes blank between screens).
  const coin = context ?? lastCoin;
  const active = context != null;

  // Locale-entered draft (comma-decimal on comma locales); canonical
  // dot-decimal is what persists. Re-seeded whenever the bound coin changes —
  // deliberately NOT on `rates`, so typing is never clobbered by its own echo.
  const [draft, setDraft] = useState("");
  useEffect(() => {
    setDraft((rates[coin] || "").replace(".", decimalSeparator()));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [coin]);

  return (
    <Tooltip title={active ? t("fx.cashrateTip", { sym: symOf(coin) }) : t("fx.cashrateNoContext")} placement="right">
      <Box sx={{ px: 1.5, pt: 1, pb: 0.5, opacity: active ? 1 : 0.5 }}>
        <Box sx={{ display: "flex", alignItems: "center", gap: 0.5, mb: 0.5 }}>
          <Switch
            size="small"
            checked={enabled}
            disabled={!active}
            onChange={(_, on) => setEnabled(on)}
            inputProps={{ "aria-label": t("fx.cashrate", { sym: symOf(coin) }) }}
          />
          <Typography noWrap sx={{ fontSize: 12, fontWeight: 600, color: enabled ? "text.primary" : "text.secondary" }}>
            {t("fx.cashrate", { sym: symOf(coin) })}
          </Typography>
        </Box>
        <TextField
          size="small"
          fullWidth
          value={draft}
          disabled={!active || !enabled}
          inputMode="decimal"
          autoComplete="off"
          placeholder={`0${decimalSeparator()}0`}
          onChange={(e) => {
            const s = sanitizeAmountInput(e.target.value);
            setDraft(s);
            setRate(coin, parseAmount(s) > 0 ? canonicalAmount(s) : "");
          }}
          slotProps={{ htmlInput: { style: { fontSize: 13, padding: "5px 8px" } } }}
        />
      </Box>
    </Tooltip>
  );
}
