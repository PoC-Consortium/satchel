import { Box, Tooltip } from "@mui/material";
import { useApp } from "../AppContext";
import { useT } from "../i18n";

// Companion to NetworkStamp: a worn-stamp marker that this session is watch-only
// — it can browse the board and withdraw its own offers, but can't post, take,
// or fund. Renders nothing outside watch-only. When a network stamp is also
// showing, this sits ABOVE it (both centered) — the Header stacks them.
const COLOR = "#c77800"; // amber — distinct from the network-stamp palette

export default function WatchOnlyStamp() {
  const { watchOnly } = useApp();
  const t = useT();
  if (!watchOnly) return null;

  return (
    <Tooltip title={t("watchOnly.badgeTip")}>
      <Box
        sx={{
          display: "inline-block",
          px: 0.9,
          py: 0.15,
          color: COLOR,
          border: `2px double ${COLOR}`,
          borderRadius: "6px",
          fontFamily: '"Courier New", Courier, monospace',
          fontWeight: 700,
          fontSize: 12,
          letterSpacing: "0.06em",
          textTransform: "uppercase",
          opacity: 0.92,
          userSelect: "none",
          cursor: "help",
        }}
      >
        {t("watchOnly.badge")}
      </Box>
    </Tooltip>
  );
}
