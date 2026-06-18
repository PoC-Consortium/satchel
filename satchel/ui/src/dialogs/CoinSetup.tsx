import { useMemo, useState } from "react";
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
import { composeCoinUrl, errMsg, rpc, saveCoin } from "../api/tauri";
import { useApp } from "../AppContext";
import { useT } from "../i18n";
import { commas } from "../format";
import { C } from "../theme";
import type { CoinConn, CoinConnInput, CoinInfo, NetConnDefaults } from "../api/types";

type Verdict =
  | null
  | { kind: "checking" }
  | { kind: "ok"; tip_height?: number; genesis_hash?: string }
  | { kind: "bad"; msg: string };

type Auth = "cookie" | "userpass";

// Coin setup: structured RPC connection (host / port / auth / datadir or
// user-pass / wallet), pre-filled from the coin's template → any saved config →
// network defaults. Validate composes the exact backend URL Satchel will save
// and runs the node's genesis-hash check; nothing persists until that passes,
// so funds can never be pointed at the wrong chain. Editing any field
// invalidates a prior check.
export default function CoinSetup({
  coin,
  saved,
  template,
  onClose,
  onSaved,
}: {
  coin: CoinInfo;
  saved: CoinConn | undefined;
  /** Connection defaults from the coin's coins.toml template (current network). */
  template?: NetConnDefaults;
  onClose: () => void;
  onSaved: () => void | Promise<void>;
}) {
  const { log } = useApp();
  const t = useT();

  // Prefill: a saved structured field wins, else the template, else a default.
  const pick = <T,>(s: T | null | undefined, tpl: T | undefined, def: T): T =>
    s ?? tpl ?? def;

  const [host, setHost] = useState(pick(saved?.rpc_host, template?.rpc_host, "127.0.0.1"));
  const [port, setPort] = useState(String(saved?.rpc_port ?? template?.rpc_port ?? ""));
  const [auth, setAuth] = useState<Auth>(
    (pick(saved?.auth_method, template?.auth_method, "cookie") as Auth) === "userpass"
      ? "userpass"
      : "cookie",
  );
  const [user, setUser] = useState(saved?.rpc_user ?? "");
  const [password, setPassword] = useState(saved?.rpc_password ?? "");
  const [datadir, setDatadir] = useState(pick(saved?.datadir, template?.datadir, ""));
  // The cookie sub-path comes from the template/default, not an edited field.
  const cookieSub = pick(saved?.cookie_subpath, template?.cookie_subpath, "");
  const [wallet, setWallet] = useState(pick(saved?.wallet, template?.wallet, ""));
  const [confs, setConfs] = useState(
    saved?.confirmations != null ? String(saved.confirmations) : "",
  );

  const [validated, setValidated] = useState(false);
  const [verdict, setVerdict] = useState<Verdict>(null);
  const [err, setErr] = useState("");
  const [busy, setBusy] = useState(false);

  // Any edit invalidates a prior validation (you can't validate one node and
  // save another). Wrap each setter so the form can't drift from its check.
  function edited<T>(setter: (v: T) => void) {
    return (v: T) => {
      setter(v);
      setValidated(false);
      setVerdict(null);
    };
  }

  const portNum = parseInt(port.trim(), 10);
  const connInput = useMemo<CoinConnInput>(
    () => ({
      rpc_host: host.trim() || "127.0.0.1",
      rpc_port: Number.isFinite(portNum) ? portNum : undefined,
      auth_method: auth,
      rpc_user: auth === "userpass" ? user.trim() : undefined,
      rpc_password: auth === "userpass" ? password : undefined,
      datadir: auth === "cookie" ? datadir.trim() : undefined,
      cookie_subpath: auth === "cookie" && cookieSub.trim() ? cookieSub.trim() : undefined,
      wallet: wallet.trim() || undefined,
    }),
    [host, portNum, auth, user, password, datadir, cookieSub, wallet],
  );

  async function validate() {
    if (!Number.isFinite(portNum) || portNum <= 0) {
      setErr(t("coins.needPort"));
      return;
    }
    setErr("");
    setBusy(true);
    setVerdict({ kind: "checking" });
    try {
      // Compose the exact URL (cookie read in Rust), then genesis-check it.
      const url = await composeCoinUrl(coin.id, connInput);
      const r = await rpc<{ tip_height?: number; genesis_hash?: string }>("validatecoin", [
        coin.id,
        url,
      ]);
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
      setErr(t("coins.validateFirst"));
      return;
    }
    setErr(t("coins.savingReconnecting"));
    setBusy(true);
    try {
      const parsed = parseInt(confs.trim(), 10);
      const confValue = Number.isFinite(parsed) && parsed >= 1 ? parsed : null;
      await saveCoin(coin.id, connInput, confValue);
      log(t("coins.connected", { coin: coin.id }));
      onClose();
      await onSaved();
    } catch (e) {
      setErr(errMsg(e));
      setBusy(false);
    }
  }

  const cookiePath = cookieSub.trim() || template?.cookie_subpath || ".cookie";

  return (
    <Dialog open onClose={busy ? undefined : onClose} maxWidth="sm" fullWidth>
      <DialogTitle>{t("coins.setupTitle", { coin: coin.display_name })}</DialogTitle>
      <DialogContent>
        <DialogContentText sx={{ mb: 2 }}>
          {t("coins.setupIntro", { sym: coin.symbol })}
        </DialogContentText>

        <Stack direction="row" spacing={1.5}>
          <TextField
            label={t("coins.rpcHostLabel")}
            value={host}
            onChange={(e) => edited(setHost)(e.target.value)}
            sx={{ flex: 2 }}
            slotProps={{ htmlInput: { style: { fontFamily: C.mono } } }}
          />
          <TextField
            label={t("coins.rpcPortLabel")}
            type="number"
            value={port}
            onChange={(e) => edited(setPort)(e.target.value)}
            placeholder={template?.rpc_port ? String(template.rpc_port) : ""}
            sx={{ flex: 1 }}
            slotProps={{ htmlInput: { min: 1, step: 1, style: { fontFamily: C.mono } } }}
          />
        </Stack>

        <Typography
          sx={{ fontSize: 12, textTransform: "uppercase", letterSpacing: "0.08em", color: "text.secondary", mt: 2 }}
        >
          {t("coins.authMethodLabel")}
        </Typography>
        <Stack direction="row" spacing={1.5} sx={{ mt: 1 }}>
          <ChoiceCard
            title={t("coins.authCookie")}
            desc={t("coins.authCookieDesc")}
            selected={auth === "cookie"}
            onClick={() => edited(setAuth)("cookie")}
          />
          <ChoiceCard
            title={t("coins.authUserPass")}
            desc={t("coins.authUserPassDesc")}
            selected={auth === "userpass"}
            onClick={() => edited(setAuth)("userpass")}
          />
        </Stack>

        {auth === "cookie" ? (
          <>
            <TextField
              label={t("coins.datadirLabel")}
              value={datadir}
              onChange={(e) => edited(setDatadir)(e.target.value)}
              placeholder={template?.datadir}
              fullWidth
              sx={{ mt: 2 }}
              slotProps={{ htmlInput: { style: { fontFamily: C.mono } } }}
            />
            <Typography sx={{ color: "text.secondary", fontSize: 12, mt: 1 }}>
              {t("coins.cookiePathNote", { path: cookiePath })}
            </Typography>
          </>
        ) : (
          <Stack direction="row" spacing={1.5} sx={{ mt: 2 }}>
            <TextField
              label={t("coins.rpcUserLabel")}
              value={user}
              onChange={(e) => edited(setUser)(e.target.value)}
              fullWidth
            />
            <TextField
              label={t("coins.rpcPasswordLabel")}
              type="password"
              value={password}
              onChange={(e) => edited(setPassword)(e.target.value)}
              fullWidth
            />
          </Stack>
        )}

        <TextField
          label={t("coins.walletLabel")}
          value={wallet}
          onChange={(e) => edited(setWallet)(e.target.value)}
          placeholder={t("coins.walletPlaceholder")}
          fullWidth
          sx={{ mt: 2 }}
          slotProps={{ htmlInput: { style: { fontFamily: C.mono } } }}
        />

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
