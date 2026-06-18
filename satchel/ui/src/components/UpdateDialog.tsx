import {
  Box,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  Typography,
} from "@mui/material";
import { openExternal } from "../api/tauri";
import { useT } from "../i18n";
import { useUpdate } from "../update";
import { C } from "../theme";

// The update dialog (Phoenix-style), opened from the sidebar version badge. When
// an update is available it shows installed → latest + release notes and a link
// to the release page; otherwise it confirms you're up to date.
export default function UpdateDialog() {
  const t = useT();
  const { info, dialogOpen, closeDialog, dismiss, version } = useUpdate();
  const available = !!info?.available && !!info.latestVersion;

  return (
    <Dialog open={dialogOpen} onClose={closeDialog} maxWidth="sm" fullWidth>
      <DialogTitle>{available ? t("update.title") : t("update.upToDate")}</DialogTitle>
      <DialogContent>
        <Box sx={{ display: "flex", gap: 3, alignItems: "baseline", mb: available ? 2 : 0 }}>
          <Box>
            <Typography sx={{ fontSize: 11, color: "text.secondary", textTransform: "uppercase", letterSpacing: "0.06em" }}>
              {t("update.current")}
            </Typography>
            <Typography sx={{ fontFamily: C.mono, fontSize: 15 }}>
              v{info?.currentVersion ?? version}
            </Typography>
          </Box>
          {available && (
            <Box>
              <Typography sx={{ fontSize: 11, color: "primary.main", textTransform: "uppercase", letterSpacing: "0.06em" }}>
                {t("update.latest")}
              </Typography>
              <Typography sx={{ fontFamily: C.mono, fontSize: 15, color: "primary.main", fontWeight: 600 }}>
                v{info?.latestVersion}
              </Typography>
            </Box>
          )}
        </Box>

        {available && info?.releaseNotes && (
          <>
            <Typography sx={{ fontSize: 12, color: "text.secondary", mb: 0.5 }}>
              {t("update.notesTitle")}
            </Typography>
            <Box
              sx={{
                maxHeight: 240,
                overflowY: "auto",
                whiteSpace: "pre-wrap",
                fontFamily: C.mono,
                fontSize: 12,
                color: "text.primary",
                bgcolor: "background.default",
                border: `1px solid ${C.line}`,
                borderRadius: 1.5,
                p: 1.25,
              }}
            >
              {info.releaseNotes}
            </Box>
          </>
        )}
      </DialogContent>
      <DialogActions sx={{ px: 3, pb: 2 }}>
        {available ? (
          <>
            <Button onClick={() => { dismiss(); closeDialog(); }} color="inherit">
              {t("update.dismiss")}
            </Button>
            <Button
              variant="contained"
              onClick={() => {
                if (info?.releaseUrl) void openExternal(info.releaseUrl);
              }}
            >
              {t("update.get")}
            </Button>
          </>
        ) : (
          <Button onClick={closeDialog} variant="contained">
            {t("update.close")}
          </Button>
        )}
      </DialogActions>
    </Dialog>
  );
}
