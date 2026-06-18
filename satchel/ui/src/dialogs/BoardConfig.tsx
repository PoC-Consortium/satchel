import { useState } from "react";
import {
  Box,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
  Divider,
  TextField,
  Typography,
} from "@mui/material";
import { errMsg, saveBoard, saveNostrRelays } from "../api/tauri";
import { useApp } from "../AppContext";
import { useT } from "../i18n";

// Configure the machine-level noticeboard URL(s) and the optional Nostr relay
// transport. Both are passed to pactd at launch, so saving relaunches the
// active merchant's pactd to pick them up.
export default function BoardConfig({
  initialUrls,
  initialRelays,
  recommendedRelays,
  onClose,
  onSaved,
}: {
  initialUrls: string;
  initialRelays: string;
  recommendedRelays: string[];
  onClose: () => void;
  onSaved: () => void | Promise<void>;
}) {
  const { log } = useApp();
  const t = useT();
  const [urls, setUrls] = useState(initialUrls);
  const [relays, setRelays] = useState(initialRelays);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState("");

  async function save() {
    setBusy(true);
    setErr("Saving & reconnecting…");
    try {
      await saveBoard(urls.trim());
      await saveNostrRelays(relays.trim());
      log("noticeboard updated");
      onClose();
      await onSaved();
    } catch (e) {
      setBusy(false);
      setErr(errMsg(e));
    }
  }

  return (
    <Dialog open onClose={busy ? undefined : onClose} maxWidth="sm" fullWidth>
      <DialogTitle>{t("boards.configTitle")}</DialogTitle>
      <DialogContent>
        <DialogContentText sx={{ mb: 2 }}>{t("boards.configIntro")}</DialogContentText>
        <TextField
          label={t("boards.urlLabel")}
          placeholder="http://host:port"
          value={urls}
          onChange={(e) => setUrls(e.target.value)}
          fullWidth
          autoFocus
        />

        <Divider sx={{ my: 2.5 }} />

        <Typography sx={{ fontWeight: 600, mb: 0.5 }}>{t("boards.nostrHeading")}</Typography>
        <DialogContentText sx={{ mb: 1.5 }}>{t("boards.nostrIntro")}</DialogContentText>
        <TextField
          label={t("boards.nostrLabel")}
          placeholder="wss://relay.example.com"
          value={relays}
          onChange={(e) => setRelays(e.target.value)}
          fullWidth
        />
        {recommendedRelays.length > 0 && (
          <Box sx={{ mt: 1, display: "flex", alignItems: "center", gap: 1 }}>
            <Button
              size="small"
              variant="text"
              disabled={busy}
              onClick={() => setRelays(recommendedRelays.join(","))}
              sx={{ fontSize: 13, textTransform: "none", p: 0, minWidth: 0 }}
            >
              {t("boards.nostrRecommend")}
            </Button>
            <Typography component="span" sx={{ fontSize: 12, color: "text.secondary" }}>
              ({recommendedRelays.join(", ")})
            </Typography>
          </Box>
        )}

        {err && <Typography sx={{ color: "error.main", fontSize: 13, mt: 1.25 }}>{err}</Typography>}
      </DialogContent>
      <DialogActions sx={{ px: 3, pb: 2 }}>
        <Button color="inherit" onClick={onClose} disabled={busy} sx={{ mr: "auto" }}>
          {t("common.cancel")}
        </Button>
        <Button variant="contained" onClick={() => void save()} disabled={busy}>
          {t("boards.save")}
        </Button>
      </DialogActions>
    </Dialog>
  );
}
