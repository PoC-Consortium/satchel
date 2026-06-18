import { useState } from "react";
import {
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
  TextField,
  Typography,
} from "@mui/material";
import { errMsg, rpc } from "../api/tauri";
import { useT } from "../i18n";

// Encrypted merchant came up locked: prompt for the session passphrase.
// Satchel passes it to pactd's `unlock` RPC and never persists it.
export default function Unlock({
  onDone,
  onSwitch,
}: {
  onDone: () => void | Promise<void>;
  onSwitch: () => void;
}) {
  const t = useT();
  const [pass, setPass] = useState("");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState("");

  async function doUnlock() {
    setBusy(true);
    setErr("");
    try {
      await rpc("unlock", [pass]);
      await onDone();
    } catch (e) {
      setErr(errMsg(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog open maxWidth="sm" fullWidth disableEscapeKeyDown>
      <DialogTitle>{t("unlock.title")}</DialogTitle>
      <DialogContent>
        <DialogContentText sx={{ mb: 2 }}>{t("unlock.body")}</DialogContentText>
        <TextField
          label={t("seed.passphraseLabel")}
          type="password"
          value={pass}
          onChange={(e) => setPass(e.target.value)}
          autoFocus
          fullWidth
          onKeyDown={(e) => {
            if (e.key === "Enter" && !busy) void doUnlock();
          }}
        />
        {err && <Typography sx={{ color: "error.main", fontSize: 13, mt: 1.25 }}>{err}</Typography>}
      </DialogContent>
      <DialogActions sx={{ px: 3, pb: 2 }}>
        <Button color="inherit" onClick={onSwitch} sx={{ mr: "auto" }}>
          {t("unlock.switchMerchant")}
        </Button>
        <Button variant="contained" disabled={busy} onClick={() => void doUnlock()}>
          {t("unlock.unlock")}
        </Button>
      </DialogActions>
    </Dialog>
  );
}
