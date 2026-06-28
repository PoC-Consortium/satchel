import { useState } from "react";
import {
  Box,
  Button,
  Chip,
  Collapse,
  IconButton,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableRow,
  Tooltip,
  Typography,
} from "@mui/material";
import ExpandMoreIcon from "@mui/icons-material/ExpandMore";
import ContentCopyIcon from "@mui/icons-material/ContentCopy";
import { useApp } from "../AppContext";
import { useT } from "../i18n";
import { asset, fmtAmt, isActive, isFinalizing, isTerminal, settlementLeg, swapParties } from "../format";
import { dumpSwap } from "../api/tauri";
import { narrate } from "./narrate";
import SwapProgressLine from "../components/SwapProgressLine";
import CounterpartyTag from "../components/CounterpartyTag";
import ProtocolChip from "../components/ProtocolChip";
import { C } from "../theme";
import type { Swap, SwapState } from "../api/types";

// Swaps is the comprehensive book-keeping ledger (UI-2): in-flight swaps on top,
// walking their live states, then terminal trades below. The Corkboard's
// ActiveSwaps panel stays as the trading-view convenience with the fund/redeem
// buttons — here we only RENDER pactd's swap list (no swap logic, no actions).
// The scheduler ticks automatically (managed pactd launches with --tick-secs),
// and the e2e harness drives `tick` over RPC directly, so no manual button.
const STATE_COLOR: Partial<Record<SwapState, string>> = {
  completed: C.good,
  refunded: C.accent,
  aborted: C.bad,
};

// Newest first by `created_at` (C2, served by pactd). Records predating the
// field default to 0 → they sort last but keep their original list order
// relative to each other (Array.prototype.sort is stable).
const byNewest = (a: Swap, b: Swap) => (b.created_at ?? 0) - (a.created_at ?? 0);

export default function SwapsScreen() {
  const { swaps } = useApp();
  const t = useT();

  const active = swaps.filter(isActive).slice().sort(byNewest);
  const history = swaps.filter(isTerminal).slice().sort(byNewest);

  const empty = active.length === 0 && history.length === 0;

  return (
    <Box>
      <Typography variant="h1" sx={{ fontSize: 18, mb: 0.5 }}>
        {t("swaps.title")}
      </Typography>
      <Typography sx={{ color: "text.secondary", fontSize: 13, mb: 2 }}>
        {t("swaps.hint")}
      </Typography>

      {empty ? (
        <Typography sx={{ color: "text.secondary", fontSize: 13 }}>{t("swaps.none")}</Typography>
      ) : (
        <>
          {active.length > 0 && (
            <SwapSection title={t("swaps.activeTitle")} swaps={active} />
          )}
          {history.length > 0 && (
            <SwapSection
              title={t("swaps.historyTitle")}
              swaps={history}
              sx={{ mt: active.length > 0 ? 4 : 0 }}
            />
          )}
        </>
      )}
    </Box>
  );
}

function SwapSection({
  title,
  swaps,
  sx,
}: {
  title: string;
  swaps: Swap[];
  sx?: object;
}) {
  const t = useT();
  return (
    <Box sx={sx}>
      <Typography
        sx={{
          fontSize: 12,
          textTransform: "uppercase",
          letterSpacing: "0.08em",
          color: "text.secondary",
          mb: 1,
        }}
      >
        {title}
      </Typography>
      <Table size="small">
        <TableHead>
          <TableRow>
            {[
              t("swaps.col.swap"),
              t("swaps.maker"),
              t("swaps.taker"),
              t("swaps.col.amounts"),
              t("swaps.col.state"),
              t("swaps.col.when"),
              t("swaps.col.finalTx"),
            ].map((h, i) => (
              <TableCell
                key={i}
                sx={{ color: "text.secondary", textTransform: "uppercase", fontSize: 12, fontWeight: 500 }}
              >
                {h}
              </TableCell>
            ))}
          </TableRow>
        </TableHead>
        <TableBody>
          {swaps.map((s) => (
            <SwapRow key={s.swap_id} s={s} />
          ))}
        </TableBody>
      </Table>
    </Box>
  );
}

function SwapRow({ s }: { s: Swap }) {
  const t = useT();
  const { identity } = useApp();
  const { maker, taker } = swapParties(s, identity);
  const [open, setOpen] = useState(false);
  // While finalizing, the state is `completed` but it isn't done — show
  // "finalizing" and not the terminal (green) colour.
  const fin = isFinalizing(s);
  const stateLabel = fin ? "finalizing" : s.state;
  const stateColor = fin ? undefined : STATE_COLOR[s.state];
  return (
    <>
      <TableRow sx={{ "& td": { borderBottom: "none" } }}>
        <TableCell sx={{ fontFamily: C.mono, fontSize: 13 }}>
          <Box sx={{ display: "flex", alignItems: "center", gap: 0.5 }}>
            <IconButton
              size="small"
              aria-label={t("swaps.audit.toggle")}
              onClick={() => setOpen((v) => !v)}
              sx={{ p: 0.25 }}
            >
              <ExpandMoreIcon
                sx={{ fontSize: 18, transition: "transform .15s", transform: open ? "rotate(180deg)" : "none" }}
              />
            </IconButton>
            {s.swap_id}
            {/* Every swap shows its type — Standard (HTLC) or Private
                (Taproot) — mirroring the Corkboard offer rows. */}
            <ProtocolChip protocol={s.protocol} />
          </Box>
        </TableCell>
        <TableCell>
          <CounterpartyTag id={maker.id} you={maker.you} />
        </TableCell>
        <TableCell>
          <CounterpartyTag id={taker.id} you={taker.you} />
        </TableCell>
        <TableCell>
          {fmtAmt(s.amount_a, asset(s.chain_a))} → {fmtAmt(s.amount_b, asset(s.chain_b))}
        </TableCell>
        <TableCell>
          <Chip
            label={stateLabel}
            size="small"
            sx={{
              height: 20,
              bgcolor: stateColor ? `${stateColor}22` : "action.selected",
              color: stateColor ?? "text.primary",
              fontSize: 12,
            }}
          />
        </TableCell>
        <TableCell sx={{ fontFamily: C.mono, fontSize: 13 }}>
          {s.created_at ? new Date(s.created_at * 1000).toLocaleString() : "—"}
        </TableCell>
        <TableCell sx={{ fontFamily: C.mono, fontSize: 13 }}>
          {s.final_txid ? s.final_txid.slice(0, 16) + "…" : "—"}
        </TableCell>
      </TableRow>
      <TableRow>
        <TableCell colSpan={7} sx={{ color: "text.secondary", fontSize: 12, pt: 0 }}>
          {/* pactd narration is shown VERBATIM (do not rewrite). */}
          <Typography sx={{ fontSize: 12, color: "text.secondary" }}>{narrate(s)}</Typography>
          {/* Live progress (observability) — additive, never replaces narrate(). */}
          {s.progress && (
            <Box sx={{ mt: 0.5 }}>
              <SwapProgressLine p={s.progress} />
            </Box>
          )}
          <Collapse in={open} unmountOnExit>
            <SwapAudit s={s} />
          </Collapse>
        </TableCell>
      </TableRow>
    </>
  );
}

// The on-chain audit trail for one swap: both funding txs and OUR settlement,
// grouped per leg. We never show the counterparty's settlement tx or the swap
// secret — this is a self-contained record of how YOUR funds moved.
function SwapAudit({ s }: { s: Swap }) {
  const t = useT();
  const { showToast } = useApp();
  const copy = async (txid: string) => {
    try {
      await navigator.clipboard.writeText(txid);
      showToast(t("swaps.audit.copied"));
    } catch {
      /* clipboard blocked — the id is selectable inline as a fallback */
    }
  };
  // RC2 #3b: copy a secret-free diagnostics bundle (record + log lines) to the
  // clipboard, for the user to paste to the developers.
  const dump = async () => {
    try {
      const d = await dumpSwap(s.swap_id);
      await navigator.clipboard.writeText(JSON.stringify(d, null, 2));
      showToast(t("swaps.dumpCopied"));
    } catch {
      showToast(t("swaps.dumpFailed"));
    }
  };
  const settleOn = settlementLeg(s.role, s.state);
  const settleLabel = s.state === "refunded" ? t("swaps.audit.refunded") : t("swaps.audit.received");
  const legs = [
    { key: "a" as const, coin: asset(s.chain_a), amount: s.amount_a, fund: s.fund_a_txid, mine: s.role === "initiator" },
    { key: "b" as const, coin: asset(s.chain_b), amount: s.amount_b, fund: s.fund_b_txid, mine: s.role === "participant" },
  ];
  return (
    <Box sx={{ mt: 1, pl: 0.5, borderLeft: `2px solid ${C.line}` }}>
      <Typography
        sx={{ fontSize: 11, textTransform: "uppercase", letterSpacing: "0.06em", color: "text.secondary", mb: 0.5, pl: 1 }}
      >
        {t("swaps.audit.title")}
      </Typography>
      {legs.map((leg) => (
        <Box key={leg.key} sx={{ pl: 1, mb: 0.75 }}>
          <Typography sx={{ fontSize: 12, color: "text.primary" }}>
            {fmtAmt(leg.amount, leg.coin)} · {leg.mine ? t("swaps.audit.youLocked") : t("swaps.audit.theyLocked")}
          </Typography>
          <TxLine label={t("swaps.audit.funding")} txid={leg.fund} copy={copy} />
          {s.final_txid && settleOn === leg.key && (
            <TxLine label={settleLabel} txid={s.final_txid} copy={copy} />
          )}
        </Box>
      ))}
      <Box sx={{ pl: 1, mt: 0.5 }}>
        <Tooltip title={t("swaps.dumpHint")}>
          <Button size="small" variant="text" color="inherit" onClick={() => void dump()}>
            {t("swaps.dump")}
          </Button>
        </Tooltip>
      </Box>
    </Box>
  );
}

function TxLine({
  label,
  txid,
  copy,
}: {
  label: string;
  txid?: string | null;
  copy: (txid: string) => void;
}) {
  const t = useT();
  return (
    <Box sx={{ display: "flex", alignItems: "center", gap: 1, py: 0.1 }}>
      <Typography sx={{ fontSize: 12, color: "text.secondary", minWidth: 64 }}>{label}</Typography>
      {txid ? (
        <>
          <Typography sx={{ fontFamily: C.mono, fontSize: 12, color: "text.primary", wordBreak: "break-all" }}>
            {txid}
          </Typography>
          <Tooltip title={t("swaps.audit.copy")}>
            <IconButton size="small" aria-label={t("swaps.audit.copy")} onClick={() => copy(txid)} sx={{ p: 0.25 }}>
              <ContentCopyIcon sx={{ fontSize: 14 }} />
            </IconButton>
          </Tooltip>
        </>
      ) : (
        <Typography sx={{ fontSize: 12, color: "text.secondary", fontStyle: "italic" }}>
          {t("swaps.audit.pending")}
        </Typography>
      )}
    </Box>
  );
}
