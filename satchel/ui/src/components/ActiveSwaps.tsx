import { Box, Button, Chip, Stack, Tooltip, Typography } from "@mui/material";
import { useApp } from "../AppContext";
import { useConfirm } from "../ui/ConfirmProvider";
import { useT } from "../i18n";
import { dumpSwap, errMsg, rpc } from "../api/tauri";
import { asset, fmtAmt, isActive, isFinalizing, swapParties } from "../format";
import { narrate } from "../screens/narrate";
import SwapProgressLine from "./SwapProgressLine";
import CounterpartyTag from "./CounterpartyTag";
import ProtocolChip from "./ProtocolChip";
import { C } from "../theme";
import type { Swap } from "../api/types";

// The "your active swaps" dock — a static strip sitting directly above the
// activity log (App.tsx), always in view rather than scrolling away with the
// page. It renders NOTHING when no swap is in flight, so it only takes space
// when there's something to act on. Swap LOGIC stays in pactd — these buttons
// just call its RPCs.

// Funding, redeem and refund are all automatic (--auto-fund + the scheduler) and
// chain-gated (confirmations, timelocks) — a manual button can never make them
// happen sooner or differently, only fail or double-act. So the only genuine human
// action is backing out BEFORE any funds are committed (Cancel = abort the
// handshake); everything else is surfaced as status (the live progress line + the
// "refunds at X" time). Redeem/Refund buttons were removed: auto-redeem fires the
// instant it's safe, and pact auto-refunds anything past its timelock.
//
// Cancel is gated on "nothing locked yet" (no funding on EITHER leg) rather than a
// state name, so it's correct for both v1 (htlc_*) and v2 (funding_*, which can lock
// a leg while the record is still `accepted`). Dump (diagnostics) stays always-on.
const canCancel = (s: Swap) => !s.fund_a_txid && !s.fund_b_txid;

export default function ActiveSwaps() {
  const { swaps, refreshSwaps, log } = useApp();
  const confirm = useConfirm();
  const t = useT();
  const active = swaps.filter(isActive);
  // Multi-machine (#122): swaps this machine drives vs. ones another machine on
  // the same seed drives (we only follow those read-only). Foreign swaps group
  // per originating machine so one "Take over" adopts all of that machine's work.
  const local = active.filter((s) => s.source !== "foreign");
  const foreign = active.filter((s) => s.source === "foreign");
  const foreignGroups = new Map<string, Swap[]>();
  for (const s of foreign) {
    const key = s.machine_label ?? "?";
    const g = foreignGroups.get(key);
    if (g) g.push(s);
    else foreignGroups.set(key, [s]);
  }

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

  // Take over EVERY in-flight swap of one stopped machine (§4). One confirm
  // asserts "that machine is stopped" — the whole safety model — then each swap
  // is adopted (pactd `takeover`) and starts being driven here. Never do this
  // while the other machine is alive: two drivers can double-spend.
  async function takeover(group: Swap[], machine: string) {
    const ok = await confirm({
      title: t("swaps.takeoverTitle"),
      body: t("swaps.takeoverBody", { machine }),
      confirmLabel: t("swaps.takeoverConfirm"),
      cancelLabel: t("swaps.takeoverCancel"),
      danger: true,
    });
    if (!ok) return;
    for (const s of group) {
      try {
        await rpc("takeover", [s.swap_id]);
        log(t("log.actionOk", { action: "takeover", id: s.swap_id }));
      } catch (e) {
        log(t("log.actionError", { action: "takeover", id: s.swap_id, err: errMsg(e) }));
      }
    }
    void refreshSwaps();
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
        {local.map((s, i) => (
          <ActiveSwapRow
            key={s.swap_id}
            s={s}
            first={i === 0}
            onCancel={() => void cancel(s.swap_id)}
            onDump={() => void dump(s.swap_id)}
          />
        ))}
      </Box>

      {/* Foreign swaps: one read-only group per other machine on this seed, with
          a single "Take over" that adopts all of that machine's in-flight work. */}
      {[...foreignGroups.entries()].map(([machine, rows]) => (
        <Box key={machine}>
          <Box
            sx={{
              display: "flex",
              alignItems: "center",
              gap: 1,
              px: 2,
              py: 0.5,
              borderTop: `1px solid ${C.line}`,
              bgcolor: "action.hover",
            }}
          >
            <Typography
              sx={{ fontSize: 11, textTransform: "uppercase", letterSpacing: "0.08em", color: "text.secondary" }}
            >
              {t("swaps.foreignGroup", { machine })}
            </Typography>
            <Typography sx={{ fontSize: 11, color: "text.disabled" }}>{rows.length}</Typography>
            <Box sx={{ flex: 1 }} />
            <Tooltip title={t("swaps.takeoverHint")}>
              <Button
                size="small"
                variant="outlined"
                color="warning"
                onClick={() => void takeover(rows, machine)}
              >
                {t("swaps.takeover")}
              </Button>
            </Tooltip>
          </Box>
          {rows.map((s, i) => (
            <ActiveSwapRow key={s.swap_id} s={s} first={i === 0} readOnly onDump={() => void dump(s.swap_id)} />
          ))}
        </Box>
      ))}
    </Box>
  );
}

function ActiveSwapRow({
  s,
  first,
  onCancel,
  onDump,
  readOnly = false,
}: {
  s: Swap;
  first: boolean;
  // Absent for a read-only (followed) row — foreign swaps expose no drive action.
  onCancel?: () => void;
  onDump: () => void;
  readOnly?: boolean;
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
      {/* Swap type — Standard (HTLC) or Private (Taproot) — same badge the
          Swaps ledger and Corkboard show, so the dock labels both. */}
      <ProtocolChip protocol={s.protocol} />
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
        {!readOnly && canCancel(s) && onCancel && (
          <Button size="small" variant="outlined" color="inherit" onClick={onCancel}>
            {t("swaps.cancel")}
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
