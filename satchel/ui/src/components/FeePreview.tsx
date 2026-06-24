import { useEffect, useState } from "react";
import { Box, Chip, Tooltip, Typography } from "@mui/material";
import { rpc } from "../api/tauri";
import { useApp } from "../AppContext";
import { useT } from "../i18n";
import { commas, fmtFee } from "../format";
import { C } from "../theme";
import type { FeeSide, SwapFees } from "../api/types";

// Pre-trade cost transparency. The shape of a swap's cost — 2 on-chain txs you
// pay for, 2 chains, 2 coins, and CORKBOARD CHARGES NOTHING — is always shown.
// The numbers come from pactd's estimateswapfees (C3); it never errors (a down
// node yields a conservative fallback rate, flagged so we mark it a guess). If
// the method is missing entirely (older pactd) we fall back to the static note.
export default function FeePreview({
  giveCoin,
  getCoin,
}: {
  giveCoin: string;
  getCoin: string;
}) {
  const { symOf } = useApp();
  const t = useT();
  const [fees, setFees] = useState<SwapFees | null>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    let alive = true;
    rpc<SwapFees>("estimateswapfees", [giveCoin, getCoin])
      .then((f) => alive && setFees(f))
      .catch(() => alive && setFailed(true));
    return () => {
      alive = false;
    };
  }, [giveCoin, getCoin]);

  const anyFallback = !!fees && (fees.give.fee_rate_is_fallback || fees.get.fee_rate_is_fallback);

  return (
    <Box sx={{ border: `1px dashed ${C.line}`, borderRadius: 1.5, p: 1.25, bgcolor: "background.default" }}>
      <Box sx={{ display: "flex", alignItems: "center", gap: 1, mb: 0.75 }}>
        <Typography sx={{ fontSize: 12, fontWeight: 600 }}>{t("fees.title")}</Typography>
        {anyFallback && (
          <Tooltip title={t("fees.fallbackTip")}>
            <Chip
              size="small"
              variant="outlined"
              label={t("fees.estimated")}
              sx={{ height: 18, fontSize: 10, color: C.accent, borderColor: C.warnTintBorder, bgcolor: C.warnTintBg, cursor: "help" }}
            />
          </Tooltip>
        )}
      </Box>

      <Typography sx={{ fontSize: 12, color: "text.secondary" }}>{t("fees.summary")}</Typography>

      {fees ? (
        <Box sx={{ mt: 1, display: "flex", flexDirection: "column", gap: 0.75 }}>
          <FeeSideRows side={fees.give} symbol={symOf(fees.give.coin_id)} />
          <FeeSideRows side={fees.get} symbol={symOf(fees.get.coin_id)} />
        </Box>
      ) : (
        failed && (
          <Typography sx={{ fontSize: 11, color: "text.secondary", mt: 0.75, fontStyle: "italic" }}>
            {t("fees.provisionalNote")}
          </Typography>
        )
      )}
    </Box>
  );
}

function FeeSideRows({ side, symbol }: { side: FeeSide; symbol: string }) {
  const t = useT();
  return (
    <Box>
      <Typography sx={{ fontSize: 10.5, color: "text.secondary", letterSpacing: "0.04em" }}>
        {symbol} · {commas(side.fee_rate_sat_per_vb)} sat/vB
      </Typography>
      {side.legs.map((leg) => (
        <Box
          key={`${side.coin_id}-${leg.name}`}
          sx={{ display: "flex", justifyContent: "space-between", fontSize: 12, fontFamily: C.mono }}
        >
          <span style={{ opacity: leg.name === "refund" ? 0.6 : 1 }}>
            {leg.name === "refund" ? `${leg.name} ${t("fees.ifItStalls")}` : leg.name}
          </span>
          <span style={{ opacity: leg.name === "refund" ? 0.6 : 1 }}>
            {fmtFee(leg.fee_sat)} {symbol}
          </span>
        </Box>
      ))}
    </Box>
  );
}
