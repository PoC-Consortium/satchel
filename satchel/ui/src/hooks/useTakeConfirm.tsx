// The single take-confirmation flow, shared by the Corkboard "Take" button and
// the "Paste a slip" entry (PRIVATE_OFFERS.md §6) so both show the IDENTICAL
// summary: who you'd trade with, the amounts you give and
// receive, the safety-timelock timing, the maker-funds-first note, and the
// network-cost breakdown (moved here from the offer rows so the board stays
// scannable — fees belong on the decision screen). The only difference is which
// RPC actually takes the offer (boardtake vs takeoffer).

import { useCallback, type ReactNode } from "react";
import { Box, Divider, Typography } from "@mui/material";
import { useConfirm } from "../ui/ConfirmProvider";
import { useApp } from "../AppContext";
import { useT } from "../i18n";
import { useDenom } from "../denom";
import CounterpartyTag from "../components/CounterpartyTag";
import FeePreview from "../components/FeePreview";
import { ago, baseQuote, denomLabel, fmtBare, fmtDenom, hours } from "../format";
import { C } from "../theme";
import type { OfferBody } from "../api/types";

/** Optional counterparty context so the summary can name who you'd trade with
 *  (the maker id, from the offer or slip). */
export interface TakeCtx {
  from?: string | null;
}

export type ConfirmTake = (b: OfferBody, ctx?: TakeCtx) => Promise<boolean>;

/** Returns `confirmTake(body, ctx?)`: opens the shared summary dialog and
 *  resolves to the user's decision. Callers do the actual RPC. */
export function useTakeConfirm(): ConfirmTake {
  const { symOf } = useApp();
  const { denom } = useDenom();
  const confirm = useConfirm();
  const t = useT();

  return useCallback<ConfirmTake>(
    async (b, ctx) => {
      // Denominate the quote coin (BTC-side) to match the board's unit toggle;
      // the base coin stays in whole units.
      const { quote } = baseQuote(b.give_asset, b.get_asset);
      const leg = (sats: number, coin: string) =>
        coin === quote
          ? `${fmtDenom(sats, denom)} ${denomLabel(coin, symOf(coin), denom)}`
          : `${fmtBare(sats)} ${symOf(coin)}`;

      // From the taker's perspective: you give what the maker WANTS (get_*),
      // and you receive what the maker OFFERS (give_*).
      const youGive = leg(b.get_amount, b.get_asset);
      const youGet = leg(b.give_amount, b.give_asset);
      const posted = b.created ? `posted ${ago(b.created)}` : null;

      const row = (label: string, value: ReactNode, valueColor = "text.primary") => (
        <>
          <Typography sx={{ fontSize: 12, color: "text.secondary" }}>{label}</Typography>
          <Box sx={{ textAlign: "right", color: valueColor, fontFamily: C.mono, fontSize: 13.5 }}>{value}</Box>
        </>
      );

      return confirm({
        title: t("takeConfirm.title"),
        wide: true,
        confirmLabel: t("takeConfirm.confirm"),
        body: (
          <Box sx={{ display: "flex", flexDirection: "column", gap: 1.5 }}>
            {/* Who you'd trade with. */}
            {ctx?.from && (
              <Box sx={{ display: "flex", alignItems: "center", gap: 1, flexWrap: "wrap" }}>
                <Typography sx={{ fontSize: 12, color: "text.secondary", mr: 0.5 }}>
                  {t("takeConfirm.counterparty")}
                </Typography>
                <CounterpartyTag id={ctx.from} />
              </Box>
            )}

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
              {row(t("takeConfirm.youGive"), youGive)}
              {row(t("takeConfirm.youReceive"), youGet, "primary.main")}
              <Box sx={{ gridColumn: "1 / -1", my: 0.25 }}>
                <Divider />
              </Box>
              {row(
                t("takeConfirm.safetyRefund"),
                <>
                  {hours(b.t2_secs)}h / {hours(b.t1_secs)}h
                </>,
              )}
              {posted && row(t("takeConfirm.offerAge"), <span style={{ color: C.dim }}>{posted}</span>)}
            </Box>

            <Typography sx={{ fontSize: 12.5, color: "text.secondary" }}>
              {t("takeConfirm.makerFundsFirst", { sym: symOf(b.give_asset) })}
            </Typography>

            {/* Network-cost summary — taker-perspective legs (you fund the coin
                you give, redeem the coin you get). Corkboard charges nothing. */}
            <FeePreview giveCoin={b.get_asset} getCoin={b.give_asset} />
          </Box>
        ),
      });
    },
    [confirm, symOf, denom, t],
  );
}
