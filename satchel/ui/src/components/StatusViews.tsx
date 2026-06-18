import { Box, Button, Paper, Typography } from "@mui/material";
import { useApp } from "../AppContext";
import { useT } from "../i18n";

// Shared empty/error placeholder — a dashed panel with a headline + body and
// optional action, matching the old `.empty` block.
export function EmptyState({
  title,
  children,
  action,
}: {
  title: string;
  children?: React.ReactNode;
  action?: React.ReactNode;
}) {
  return (
    <Paper
      variant="outlined"
      sx={{ textAlign: "center", color: "text.secondary", py: 5, px: 3, borderStyle: "dashed" }}
    >
      <Typography sx={{ fontSize: 15, color: "text.primary", mb: 0.75 }}>{title}</Typography>
      <Box sx={{ fontSize: 14 }}>{children}</Box>
      {action && <Box sx={{ mt: 2 }}>{action}</Box>}
    </Paper>
  );
}

/** Engine unreachable (getinfo failed at boot): explain + offer a retry. */
export function Disconnected() {
  const { boot } = useApp();
  const t = useT();
  return (
    <EmptyState
      title={t("status.notConnectedTitle")}
      action={
        <Button variant="contained" onClick={() => void boot()}>
          {t("common.retry")}
        </Button>
      }
    >
      {t("status.disconnectedBody")}
    </EmptyState>
  );
}

/** Running outside the Tauri webview (e.g. a plain `vite dev` in a browser). */
export function NoTauri() {
  const t = useT();
  return (
    <EmptyState title={t("status.openInSatchel")}>{t("status.noTauriBody")}</EmptyState>
  );
}
