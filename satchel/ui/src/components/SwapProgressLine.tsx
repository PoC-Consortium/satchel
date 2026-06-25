import { Box, LinearProgress, Typography } from "@mui/material";
import { useT } from "../i18n";
import { C } from "../theme";
import type { SwapProgress } from "../api/types";

// The live progress line for an active swap (observability). Shown beneath the
// verbatim narrate() story — confirmation depth + the current settlement feerate
// + the latest scheduler action. Fed from pactd's `swapprogress` snapshot; never
// drives any swap logic. Renders nothing if there's nothing to report.
//
// i18n note: only the leading LABEL is translatable (via t()). The counts and
// feerate are pure data composed in JS (not JSX text), so they sit outside the
// no-literal-string rule and need no keys.
const LABELS: Record<SwapProgress["watching"], string> = {
  settlement: "progress.settlement",
  their_funding: "progress.theirFunding",
  ours_funding: "progress.oursFunding",
};

export default function SwapProgressLine({ p }: { p: SwapProgress }) {
  const t = useT();

  const counts = `${p.confs}/${p.needed}`;
  const fee = p.feerate_sat_vb != null ? ` · ${p.feerate_sat_vb} sat/vB` : "";
  const pct = p.needed > 0 ? Math.min(100, Math.round((p.confs / p.needed) * 100)) : 0;

  const reorg = p.last_action === "reorg-alert";
  const bumped = p.last_action === "fee-bump";

  // A snapshot older than a few scheduler ticks (e.g. the daemon detached) is
  // greyed so it doesn't read as live.
  const stale = Date.now() / 1000 - p.updated_at > 90;

  const head = reorg
    ? t("progress.reorg")
    : `${bumped ? t("progress.feeBumped") : t(LABELS[p.watching])} · ${counts}${fee}`;

  const color = reorg ? "warning.main" : bumped ? C.accent : "text.secondary";

  return (
    <Box sx={{ opacity: stale ? 0.5 : 1, maxWidth: 360 }}>
      <Typography sx={{ fontSize: 11, color, fontFamily: C.mono, whiteSpace: "nowrap" }}>
        {head}
      </Typography>
      {!reorg && (
        <LinearProgress
          variant="determinate"
          value={pct}
          sx={{ height: 3, mt: 0.25, borderRadius: 2 }}
        />
      )}
    </Box>
  );
}
