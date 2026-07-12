import { Box, LinearProgress, Typography } from "@mui/material";
import { useT } from "../i18n";
import { C } from "../theme";
import type { SwapProgress } from "../api/types";

// The live progress line for an active swap (observability). Shown beneath the
// verbatim narrate() story. Two display kinds, fed from pactd's `swapprogress`:
//   - awaiting_lock / awaiting_claim → waiting on the COUNTERPARTY (no target);
//     an indeterminate bar + "+N blocks" liveness count.
//   - their_lock / our_lock / settlement → a wait that is OURS (their lock
//     burying, our own lock burying toward the taker's required depth, or our
//     own claim burying); a determinate bar + "confs/needed".
// Never drives swap logic; renders nothing when there's nothing to report.
//
// i18n note: only the LABEL is translatable (via t()). Counts, feerate and the
// "+N blocks" number are data composed in JS (not JSX text), outside the
// no-literal-string rule.
const LABELS: Record<SwapProgress["watching"], string> = {
  awaiting_lock: "progress.awaitingLock",
  awaiting_our_lock: "progress.awaitingOurLock",
  awaiting_claim: "progress.awaitingClaim",
  their_lock: "progress.theirLock",
  our_lock: "progress.ourLock",
  settlement: "progress.securing", // rendered with {coin} via the special case below
  funding: "progress.funding", // ditto — our own funding pending/retrying (#3)
};

export default function SwapProgressLine({ p }: { p: SwapProgress }) {
  const t = useT();

  const reorg = p.last_action === "reorg-alert";
  const bumped = p.last_action === "fee-bump";
  // "funding" is also a no-target liveness wait (our own funding pending/retrying,
  // #3): an indeterminate bar + "+N blocks", where a growing count flags a stall.
  const awaiting =
    p.watching === "awaiting_lock" ||
    p.watching === "awaiting_our_lock" ||
    p.watching === "awaiting_claim" ||
    p.watching === "funding";

  // A snapshot older than a few scheduler ticks (e.g. the daemon detached) is
  // greyed so it doesn't read as live.
  const stale = Date.now() / 1000 - p.updated_at > 90;

  const label = bumped
    ? t("progress.feeBumped")
    : p.watching === "settlement"
      ? t("progress.securing", { coin: p.coin })
      : p.watching === "funding"
        ? t("progress.funding", { coin: p.coin })
        : t(LABELS[p.watching]);

  // Awaiting → a "+N blocks" liveness count (no target). Confirming → confs/needed
  // (+ feerate on our settlement).
  const tail = awaiting
    ? ` · ${t("progress.blocks", { n: p.blocks_elapsed ?? 0 })}`
    : ` · ${p.confs}/${p.needed}${p.feerate_sat_vb != null ? ` · ${p.feerate_sat_vb} sat/vB` : ""}`;

  const head = reorg ? t("progress.reorg") : `${label}${tail}`;
  const color = reorg ? "warning.main" : bumped ? C.accent : "text.secondary";
  const pct = p.needed > 0 ? Math.min(100, Math.round((p.confs / p.needed) * 100)) : 0;

  return (
    <Box sx={{ opacity: stale ? 0.5 : 1, maxWidth: 360 }}>
      <Typography sx={{ fontSize: 11, color, fontFamily: C.mono, whiteSpace: "nowrap" }}>
        {head}
      </Typography>
      {!reorg && (
        <LinearProgress
          variant={awaiting ? "indeterminate" : "determinate"}
          value={awaiting ? undefined : pct}
          sx={{ height: 3, mt: 0.25, borderRadius: 2 }}
        />
      )}
    </Box>
  );
}
