import { Box, IconButton, Tooltip, Typography } from "@mui/material";
import ExpandMoreIcon from "@mui/icons-material/ExpandMore";
import ExpandLessIcon from "@mui/icons-material/ExpandLess";
import { useApp } from "../AppContext";
import { useT } from "../i18n";
import { C } from "../theme";

// UI-4: the activity log is a docked, collapsible footer of the main area — it
// stays put while long pages (Corkboard) scroll, with its own scroll. Newest
// line on top, monospace; a quiet running record of what Satchel/pactd just did.
// Collapsed/expanded is controlled by App.tsx (local state — persisting it would
// need an out-of-scope prefs field).
export default function LogPanel({
  collapsed,
  onToggle,
}: {
  collapsed: boolean;
  onToggle: () => void;
}) {
  const { logLines } = useApp();
  const t = useT();

  return (
    <Box
      component="footer"
      sx={{
        flex: "none",
        borderTop: `1px solid ${C.line}`,
        bgcolor: "background.paper",
      }}
    >
      {/* Title bar with the collapse toggle — always visible. */}
      <Box
        onClick={onToggle}
        sx={{
          display: "flex",
          alignItems: "center",
          gap: 1,
          px: 2,
          py: 0.5,
          cursor: "pointer",
          userSelect: "none",
          "&:hover": { bgcolor: "action.hover" },
        }}
      >
        <Typography
          sx={{ fontSize: 11, textTransform: "uppercase", letterSpacing: "0.08em", color: "text.secondary" }}
        >
          {t("log.title")}
        </Typography>
        {logLines.length > 0 && (
          <Typography sx={{ fontSize: 11, color: "text.disabled" }}>
            {t("log.count", { count: logLines.length })}
          </Typography>
        )}
        <Box sx={{ ml: "auto" }}>
          <Tooltip title={collapsed ? t("log.expand") : t("log.collapse")}>
            <IconButton
              size="small"
              onClick={(e) => {
                e.stopPropagation();
                onToggle();
              }}
              sx={{ color: "text.secondary" }}
              aria-label={collapsed ? t("log.expand") : t("log.collapse")}
            >
              {collapsed ? <ExpandLessIcon fontSize="small" /> : <ExpandMoreIcon fontSize="small" />}
            </IconButton>
          </Tooltip>
        </Box>
      </Box>

      {/* Scrolling log body — hidden when collapsed. */}
      {!collapsed && (
        <Box
          sx={{
            fontFamily: C.mono,
            fontSize: 11.5,
            lineHeight: 1.5,
            color: "text.secondary",
            px: 2,
            pb: 1,
            height: 108,
            overflowY: "auto",
            whiteSpace: "pre-wrap",
          }}
        >
          {logLines.length === 0
            ? t("log.empty")
            : logLines.map((l, i) => (
                <div key={i}>
                  {l.time}  {l.msg}
                </div>
              ))}
        </Box>
      )}
    </Box>
  );
}
