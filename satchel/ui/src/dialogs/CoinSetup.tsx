import { useState } from "react";
import {
  Alert,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
  Stack,
  TextField,
  Typography,
} from "@mui/material";
import ChoiceCard from "../components/ChoiceCard";
import { errMsg, rpc, saveCoin } from "../api/tauri";
import { useApp } from "../AppContext";
import { useT } from "../i18n";
import { commas } from "../format";
import { C } from "../theme";
import type { CoinConn, CoinInfo } from "../api/types";

type Verdict =
  | null
  | { kind: "checking" }
  | { kind: "ok"; tip_height?: number; genesis_hash?: string }
  | { kind: "bad"; msg: string };

// Coin setup: enter backend URL(s) → validate against the node (genesis-hash
// check) → save. Nothing is persisted until the genesis check passes, so funds
// can never be pointed at the wrong chain. Editing the URL invalidates a prior
// check (you can't validate one node and save another).
export default function CoinSetup({
  coin,
  saved,
  onClose,
  onSaved,
}: {
  coin: CoinInfo;
  saved: CoinConn | undefined;
  onClose: () => void;
  onSaved: () => void | Promise<void>;
}) {
  const { log } = useApp();
  const t = useT();
  const [url, setUrl] = useState(saved?.chain_data ?? "");
  const [funding] = useState(saved?.funding_wallet ?? "core-rpc");
  const [confs, setConfs] = useState(
    saved?.confirmations != null ? String(saved.confirmations) : "",
  );
  const [validated, setValidated] = useState(false);
  const [verdict, setVerdict] = useState<Verdict>(null);
  const [err, setErr] = useState("");
  const [busy, setBusy] = useState(false);

  function onEdit(v: string) {
    setUrl(v);
    setValidated(false);
    setVerdict(null);
  }

  async function validate() {
    const u = url.trim();
    if (!u) {
      setErr("Enter your node's URL first.");
      return;
    }
    setErr("");
    setBusy(true);
    setVerdict({ kind: "checking" });
    try {
      const r = await rpc<{ tip_height?: number; genesis_hash?: string }>("validatecoin", [coin.id, u]);
      setValidated(true);
      setVerdict({ kind: "ok", tip_height: r.tip_height, genesis_hash: r.genesis_hash });
    } catch (e) {
      setValidated(false);
      setVerdict({ kind: "bad", msg: errMsg(e) });
    } finally {
      setBusy(false);
    }
  }

  async function save() {
    if (!validated) {
      setErr("Validate the node before saving.");
      return;
    }
    setErr("Saving & reconnecting…");
    setBusy(true);
    try {
      const parsed = parseInt(confs.trim(), 10);
      const confValue = Number.isFinite(parsed) && parsed >= 1 ? parsed : null;
      await saveCoin(coin.id, url.trim(), funding, confValue);
      log(`${coin.id} connected`);
      onClose();
      await onSaved();
    } catch (e) {
      setErr(errMsg(e));
      setBusy(false);
    }
  }

  return (
    <Dialog open onClose={busy ? undefined : onClose} maxWidth="sm" fullWidth>
      <DialogTitle>{t("coins.setupTitle", { coin: coin.display_name })}</DialogTitle>
      <DialogContent>
        <DialogContentText sx={{ mb: 2 }}>
          {t("coins.setupIntro", { sym: coin.symbol })}
        </DialogContentText>

        <TextField
          label={t("coins.backendUrlLabel")}
          placeholder="http://user:pass@127.0.0.1:port/wallet/yourwallet"
          value={url}
          onChange={(e) => onEdit(e.target.value)}
          fullWidth
          multiline
          minRows={2}
          slotProps={{ htmlInput: { style: { fontFamily: C.mono } } }}
        />
        <Typography sx={{ color: "text.secondary", fontSize: 12, mt: 1 }}>
          {t("coins.backendUrlHint")}
        </Typography>

        <Typography
          sx={{ fontSize: 12, textTransform: "uppercase", letterSpacing: "0.08em", color: "text.secondary", mt: 2 }}
        >
          {t("coins.fundingWallet")}
        </Typography>
        <Stack direction="row" spacing={1.5} sx={{ mt: 1 }}>
          <ChoiceCard
            title={t("coins.backendCoreTitle")}
            desc={t("coins.backendCoreDesc")}
            selected
          />
          <ChoiceCard
            title={t("coins.backendHardwareTitle")}
            badge={t("coins.backendLater")}
            desc={t("coins.backendHardwareDesc")}
            disabled
          />
        </Stack>

        <TextField
          label={t("coins.confirmationsLabel")}
          type="number"
          value={confs}
          onChange={(e) => setConfs(e.target.value)}
          placeholder={String(coin.default_confirmations ?? coin.confirmations ?? "")}
          fullWidth
          sx={{ mt: 2, maxWidth: 220 }}
          slotProps={{ htmlInput: { min: 1, step: 1 }, inputLabel: { shrink: true } }}
        />
        <Typography sx={{ color: "text.secondary", fontSize: 12, mt: 1 }}>
          {t("coins.confirmationsHint", { default: coin.default_confirmations ?? coin.confirmations ?? "" })}
        </Typography>

        {verdict && <VerdictBlock v={verdict} />}
        {err && <Typography sx={{ color: "error.main", fontSize: 13, mt: 1.25 }}>{err}</Typography>}
      </DialogContent>
      <DialogActions sx={{ px: 3, pb: 2 }}>
        <Button color="inherit" onClick={onClose} disabled={busy} sx={{ mr: "auto" }}>
          {t("common.cancel")}
        </Button>
        <Button color="inherit" variant="outlined" onClick={() => void validate()} disabled={busy}>
          {t("coins.validateNode")}
        </Button>
        <Button variant="contained" onClick={() => void save()} disabled={!validated || busy}>
          {t("common.save")}
        </Button>
      </DialogActions>
    </Dialog>
  );
}

function VerdictBlock({ v }: { v: Exclude<Verdict, null> }) {
  const t = useT();
  if (v.kind === "checking") {
    return (
      <Alert icon={false} variant="outlined" severity="info" sx={{ mt: 1.5 }}>
        {t("coins.checking")}
      </Alert>
    );
  }
  if (v.kind === "ok") {
    return (
      <Alert icon={false} variant="outlined" severity="success" sx={{ mt: 1.5 }}>
        <Typography sx={{ fontWeight: 600 }}>✓ {t("coins.genesisOk")}</Typography>
        <Typography sx={{ color: "text.secondary", fontFamily: C.mono, fontSize: 12, mt: 0.75, wordBreak: "break-all" }}>
          {t("coins.genesisDetail", { tip: commas(v.tip_height), hash: (v.genesis_hash || "").slice(0, 24) })}
        </Typography>
      </Alert>
    );
  }
  return (
    <Alert icon={false} variant="outlined" severity="error" sx={{ mt: 1.5 }}>
      <Typography sx={{ fontWeight: 600 }}>✗ {t("coins.genesisBad")}</Typography>
      <Typography sx={{ color: "text.secondary", fontFamily: C.mono, fontSize: 12, mt: 0.75, wordBreak: "break-all" }}>
        {v.msg}
      </Typography>
    </Alert>
  );
}
