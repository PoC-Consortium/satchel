import { Box, Button, Chip, Stack, Tooltip, Typography } from "@mui/material";
import { useApp } from "../AppContext";
import { useConfirm } from "../ui/ConfirmProvider";
import { useT } from "../i18n";
import { dumpSwap, errMsg, rpc } from "../api/tauri";
import { asset, fmtAmt, isActive, isFinalizing, swapParties } from "../format";
import { narrate } from "../screens/narrate";
import SwapProgressLine from "./SwapProgressLine";
import CounterpartyTag from "./CounterpartyTag";
import { C } from "../theme";
import type { Swap } from "../api/types";

// The "your active swaps" dock — a static strip sitting directly above the
// activity log (App.tsx), always in view rather than scrolling away with the
// page. It renders NOTHING when no swap is in flight, so it only takes space
// when there's something to act on. Swap LOGIC stays in pactd — these buttons
// just call its RPCs.

// Funding is automatic (--auto-fund), so there's no manual fund button — it was
// redundant and a double-fund footgun (a second click re-sends funds). Redeem
// stays as a manual "claim now" nudge; auto-redeem still runs and the engine
// guards it by state. Pre-funding abort = Cancel; post-funding exit = Refund.
function primaryAction(s: Swap): "redeem" | null {
  if (s.state === "funded_b") return "redeem";
  return null;
}
const canRefund = (s: Swap) => ["funded_a", "funded_b", "redeemed_b"].includes(s.state);
const canCancel = (s: Swap) =>
  ["created", "accepted"].includes(s.state) || (s.state === "funded_a" && s.role === "participant");

export default function ActiveSwaps() {
  const { swaps, refreshSwaps, log } = useApp();
  const confirm = useConfirm();
  const t = useT();
  const active = swaps.filter(isActive);

  async function act(action: string, id: string) {
    try {
      const params = action === "abort" ? [id, t("swaps.cancelReason")] : [id];
      await rpc(action, params);
      log(t("log.actionOk", { action, id }));
    } catch (e) {
      log(t("log.actionError", { action, id, err: errMsg(e) }));
    }
    void refreshSwaps();
  }

  async function cancel(id: string) {
    const ok = await confirm({
      title: t("swaps.cancelTitle"),
      body: t("swaps.cancelBody"),
      confirmLabel: t("swaps.cancelConfirm"),
      cancelLabel: t("swaps.cancelKeep"),
    });
    if (ok) void act("abort", id);
  }

  async function refund(id: string) {
    const ok = await confirm({
      title: t("swaps.refundTitle"),
      body: t("swaps.refundBody"),
      confirmLabel: t("swaps.refundConfirm"),
    });
    if (ok) void act("refund", id);
  }

  // RC2 #3b: copy a secret-free diagnostics bundle (record + log lines) for this
  // swap to the clipboard, for the user to paste to the devs.
  async function dump(id: string) {
    try {
      const d = await dumpSwap(id);
      await navigator.clipboard.writeText(JSON.stringify(d, null, 2));
      log(t("log.diagCopied", { id, count: d.log.length }));
    } catch (e) {
      log(t("log.dumpError", { id, err: errMsg(e) }));
    }
  }

  // Empty → render nothing so the dock collapses entirely.
  if (active.length === 0) return null;

  return (
    <Box
      component="section"
      sx={{
        flex: "none",
        borderTop: `1px solid ${C.line}`,
        bgcolor: "background.paper",
        maxHeight: "34vh",
        overflowY: "auto",
      }}
    >
      <Box
        sx={{
          position: "sticky",
          top: 0,
          zIndex: 1,
          display: "flex",
          alignItems: "center",
          gap: 1,
          px: 2,
          py: 0.5,
          bgcolor: "background.paper",
          borderBottom: `1px solid ${C.line}`,
        }}
      >
        <Typography
          sx={{ fontSize: 11, textTransform: "uppercase", letterSpacing: "0.08em", color: "text.secondary" }}
        >
          {t("corkboard.activeTitle")}
        </Typography>
        <Typography sx={{ fontSize: 11, color: "text.disabled" }}>{active.length}</Typography>
      </Box>

      <Box>
        {active.map((s, i) => (
          <ActiveSwapRow
            key={s.swap_id}
            s={s}
            first={i === 0}
            action={primaryAction(s)}
            onAction={(a) => void act(a, s.swap_id)}
            onCancel={() => void cancel(s.swap_id)}
            onRefund={() => void refund(s.swap_id)}
            onDump={() => void dump(s.swap_id)}
          />
        ))}
      </Box>
    </Box>
  );
}

function ActiveSwapRow({
  s,
  first,
  action,
  onAction,
  onCancel,
  onRefund,
  onDump,
}: {
  s: Swap;
  first: boolean;
  action: "redeem" | null;
  onAction: (a: string) => void;
  onCancel: () => void;
  onRefund: () => void;
  onDump: () => void;
}) {
  const t = useT();
  const { identity } = useApp();
  const { maker, taker } = swapParties(s, identity);
  const refundAt = s.role === "initiator" ? s.t1 : s.t2;
  // While finalizing the state is `completed` but it isn't done — show "finalizing".
  const stateLabel = isFinalizing(s) ? "finalizing" : s.state;
  return (
    <Box sx={{ borderTop: first ? "none" : `1px solid ${C.line}` }}>
      <Box
        sx={{
          display: "flex",
          alignItems: "center",
          gap: 1.25,
          px: 2,
          pt: 0.875,
          pb: s.progress ? 0.25 : 0.875,
          flexWrap: "wrap",
        }}
      >
      <Chip label={stateLabel} size="small" sx={{ height: 20, bgcolor: "action.selected", fontSize: 11 }} />
      {/* maker (left) ↔ taker (right); the arrow tooltip spells out the sides. */}
      <Box sx={{ display: "flex", alignItems: "center", gap: 0.75 }}>
        <CounterpartyTag id={maker.id} you={maker.you} size={18} />
        <Tooltip title={`${t("swaps.maker")} ↔ ${t("swaps.taker")}`}>
          <Typography sx={{ color: "text.disabled", fontSize: 15, cursor: "help" }}>↔</Typography>
        </Tooltip>
        <CounterpartyTag id={taker.id} you={taker.you} size={18} />
      </Box>
      <Typography sx={{ fontFamily: C.mono, fontWeight: 600, fontSize: 13 }}>
        {fmtAmt(s.amount_a, asset(s.chain_a))} → {fmtAmt(s.amount_b, asset(s.chain_b))}
      </Typography>

      {/* The plain-language swap story (frontend narrate()) — kept verbatim,
          truncated with the full text in the tooltip to keep the dock compact. */}
      <Tooltip title={narrate(s)}>
        <Typography
          sx={{ flex: "1 1 240px", minWidth: 0, fontSize: 12, color: "text.primary", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}
        >
          {narrate(s)}
        </Typography>
      </Tooltip>

      <Typography sx={{ fontSize: 11, color: "text.secondary", fontFamily: C.mono, whiteSpace: "nowrap" }}>
        {t("swaps.refundAt", { when: new Date(refundAt * 1000).toLocaleString() })}
      </Typography>

      <Stack direction="row" spacing={0.75}>
        {action && (
          <Button size="small" variant="contained" onClick={() => onAction(action)}>
            {action}
          </Button>
        )}
        {canCancel(s) && (
          <Button size="small" variant="outlined" color="inherit" onClick={onCancel}>
            {t("swaps.cancel")}
          </Button>
        )}
        {canRefund(s) && (
          <Button size="small" variant="outlined" color="inherit" onClick={onRefund}>
            {t("swaps.refund")}
          </Button>
        )}
        <Tooltip title={t("swaps.dumpHint")}>
          <Button size="small" variant="text" color="inherit" onClick={onDump}>
            {t("swaps.dump")}
          </Button>
        </Tooltip>
      </Stack>
      </Box>
      {/* Live progress (observability) — a compact second line; the top row is
          already horizontally full. */}
      {s.progress && (
        <Box sx={{ px: 2, pb: 0.875 }}>
          <SwapProgressLine p={s.progress} />
        </Box>
      )}
    </Box>
  );
}
