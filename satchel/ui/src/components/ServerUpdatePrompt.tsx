import { useEffect, useState } from "react";
import { Button, Dialog, DialogActions, DialogContent, DialogTitle, Stack, Typography } from "@mui/material";
import { applyServerUpdates, pendingServerUpdates, type ServerUpdate } from "../api/tauri";
import { useApp } from "../AppContext";
import { useT } from "../i18n";

// One-shot startup prompt (Add / Ignore): when the shipped coins.toml default
// Electrum servers gain entries a configured nodeless coin isn't using yet, offer
// to add them. Additive only — a user's existing servers are kept; the backend
// records the reconciled default set either way, so declining ("Ignore") sticks
// until the defaults change again. This is how default-server updates reach users
// who already configured a coin (the setup form only pre-fills a NEW setup).
export default function ServerUpdatePrompt() {
  const t = useT();
  const { coins, refreshCoins } = useApp();
  const [pending, setPending] = useState<ServerUpdate[] | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    // Config is loaded by the time the webview mounts; a failure here (e.g. an
    // external/adopt mode without our state) just means no prompt.
    void pendingServerUpdates()
      .then((u) => setPending(u))
      .catch(() => setPending([]));
  }, []);

  async function resolve(add: boolean) {
    setBusy(true);
    try {
      await applyServerUpdates(add);
      if (add) await refreshCoins();
    } catch {
      // best-effort: never block the app on the reconcile
    } finally {
      setBusy(false);
      setPending([]);
    }
  }

  const open = (pending?.length ?? 0) > 0;
  const name = (id: string) => coins.find((c) => c.id === id)?.display_name ?? id;

  return (
    <Dialog open={open} onClose={() => void resolve(false)} maxWidth="xs" fullWidth>
      <DialogTitle>{t("serverSync.promptTitle")}</DialogTitle>
      <DialogContent>
        <Typography sx={{ fontSize: 14, mb: 1.5 }}>{t("serverSync.promptBody")}</Typography>
        <Stack spacing={0.5}>
          {(pending ?? []).map((u) => (
            <Typography key={u.coin_id} sx={{ fontSize: 13, color: "text.secondary" }}>
              {t("serverSync.coinLine", { coin: name(u.coin_id), count: u.new_servers.length })}
            </Typography>
          ))}
        </Stack>
      </DialogContent>
      <DialogActions>
        <Button color="inherit" disabled={busy} onClick={() => void resolve(false)}>
          {t("serverSync.ignore")}
        </Button>
        <Button variant="contained" disabled={busy} onClick={() => void resolve(true)}>
          {t("serverSync.add")}
        </Button>
      </DialogActions>
    </Dialog>
  );
}
