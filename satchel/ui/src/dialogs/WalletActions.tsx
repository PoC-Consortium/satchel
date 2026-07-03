import { useCallback, useEffect, useState } from "react";
import {
  Alert,
  Box,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
  IconButton,
  InputAdornment,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableRow,
  TextField,
  Tooltip,
  Typography,
} from "@mui/material";
import ContentCopyIcon from "@mui/icons-material/ContentCopy";
import { errMsg, rpc } from "../api/tauri";
import { useApp } from "../AppContext";
import { useT } from "../i18n";
import { canonicalAmount, fmtBare, parseAmount, sanitizeAmountInput } from "../format";
import { C } from "../theme";
import type { CoinInfo, WalletTx } from "../api/types";

// Send / Receive / Activity for a NODELESS coin (epic #58): the coin's wallet
// is the bdk one derived from the Pact seed, so Satchel is the only UI it has —
// unlike Core-backed coins, whose node wallet stays the operator's own tool
// (the Wallets screen stays read-only for those).

export function ReceiveDialog({ coin, onClose }: { coin: CoinInfo; onClose: () => void }) {
  const t = useT();
  const [address, setAddress] = useState<string | null>(null);
  const [err, setErr] = useState("");
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    void (async () => {
      try {
        const r = await rpc<{ address: string }>("getnewaddress", [coin.id]);
        setAddress(r.address);
      } catch (e) {
        setErr(errMsg(e));
      }
    })();
  }, [coin.id]);

  return (
    <Dialog open onClose={onClose} maxWidth="sm" fullWidth>
      <DialogTitle>{t("wallets.receiveTitle", { sym: coin.symbol })}</DialogTitle>
      <DialogContent>
        <DialogContentText sx={{ mb: 2 }}>{t("wallets.receiveIntro")}</DialogContentText>
        {err ? (
          <Alert icon={false} variant="outlined" severity="error">
            {err}
          </Alert>
        ) : (
          <Box
            sx={{
              display: "flex",
              alignItems: "center",
              gap: 1,
              border: 1,
              borderColor: "divider",
              borderRadius: 1,
              px: 1.5,
              py: 1.25,
            }}
          >
            <Typography sx={{ fontFamily: C.mono, fontSize: 13.5, wordBreak: "break-all", flex: 1 }}>
              {address ?? "…"}
            </Typography>
            <Tooltip title={copied ? t("wallets.copied") : t("wallets.copy")}>
              <IconButton
                size="small"
                disabled={!address}
                onClick={() => {
                  if (!address) return;
                  void navigator.clipboard.writeText(address);
                  setCopied(true);
                }}
              >
                <ContentCopyIcon fontSize="inherit" />
              </IconButton>
            </Tooltip>
          </Box>
        )}
        <Typography sx={{ color: "text.secondary", fontSize: 12, mt: 1.5 }}>
          {t("wallets.receiveFreshNote")}
        </Typography>
      </DialogContent>
      <DialogActions sx={{ px: 3, pb: 2 }}>
        <Button variant="contained" onClick={onClose}>
          {t("wallets.close")}
        </Button>
      </DialogActions>
    </Dialog>
  );
}

export function SendDialog({
  coin,
  balanceSat,
  onClose,
  onSent,
}: {
  coin: CoinInfo;
  balanceSat: number | undefined;
  onClose: () => void;
  onSent: () => void | Promise<void>;
}) {
  const { log } = useApp();
  const t = useT();
  const [address, setAddress] = useState("");
  const [amount, setAmount] = useState("");
  const [err, setErr] = useState("");
  const [busy, setBusy] = useState(false);

  const amountSat = Math.round(parseAmount(amount) * 1e8);
  const overspend = balanceSat != null && amountSat > balanceSat;

  async function send() {
    if (!address.trim()) {
      setErr(t("wallets.sendNeedAddress"));
      return;
    }
    if (!(amountSat > 0)) {
      setErr(t("wallets.sendNeedAmount"));
      return;
    }
    setErr("");
    setBusy(true);
    try {
      const r = await rpc<{ txid: string }>("sendtoaddress", [
        coin.id,
        address.trim(),
        canonicalAmount(amount),
      ]);
      log(t("wallets.sendBroadcast", { txid: r.txid.slice(0, 16), sym: coin.symbol }));
      onClose();
      await onSent();
    } catch (e) {
      setErr(errMsg(e));
      setBusy(false);
    }
  }

  return (
    <Dialog open onClose={busy ? undefined : onClose} maxWidth="sm" fullWidth>
      <DialogTitle>{t("wallets.sendTitle", { sym: coin.symbol })}</DialogTitle>
      <DialogContent>
        <DialogContentText sx={{ mb: 2 }}>
          {t("wallets.sendIntro", {
            balance: balanceSat != null ? fmtBare(balanceSat) : "…",
            sym: coin.symbol,
          })}
        </DialogContentText>
        <TextField
          label={t("wallets.sendAddressLabel", { sym: coin.symbol })}
          value={address}
          onChange={(e) => setAddress(e.target.value)}
          fullWidth
          slotProps={{ htmlInput: { style: { fontFamily: C.mono } } }}
        />
        <TextField
          label={t("wallets.sendAmountLabel")}
          value={amount}
          onChange={(e) => setAmount(sanitizeAmountInput(e.target.value))}
          error={overspend}
          helperText={overspend ? t("wallets.sendOverBalance") : " "}
          fullWidth
          sx={{ mt: 2 }}
          slotProps={{
            htmlInput: { inputMode: "decimal", style: { fontFamily: C.mono } },
            input: {
              endAdornment: <InputAdornment position="end">{coin.symbol}</InputAdornment>,
            },
          }}
        />
        <Typography sx={{ color: "text.secondary", fontSize: 12, mt: 0.5 }}>
          {t("wallets.sendFeeNote")}
        </Typography>
        {err && <Typography sx={{ color: "error.main", fontSize: 13, mt: 1.25 }}>{err}</Typography>}
      </DialogContent>
      <DialogActions sx={{ px: 3, pb: 2 }}>
        <Button color="inherit" onClick={onClose} disabled={busy} sx={{ mr: "auto" }}>
          {t("common.cancel")}
        </Button>
        <Button variant="contained" onClick={() => void send()} disabled={busy || overspend}>
          {t("wallets.sendConfirm")}
        </Button>
      </DialogActions>
    </Dialog>
  );
}

export function ActivityDialog({ coin, onClose }: { coin: CoinInfo; onClose: () => void }) {
  const t = useT();
  const [txs, setTxs] = useState<WalletTx[] | null>(null);
  const [err, setErr] = useState("");

  const load = useCallback(async () => {
    try {
      const r = await rpc<{ transactions: WalletTx[] }>("listtransactions", [coin.id]);
      setTxs(r.transactions);
      setErr("");
    } catch (e) {
      setErr(errMsg(e));
    }
  }, [coin.id]);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <Dialog open onClose={onClose} maxWidth="md" fullWidth>
      <DialogTitle>{t("wallets.activityTitle", { sym: coin.symbol })}</DialogTitle>
      <DialogContent>
        {err && (
          <Alert icon={false} variant="outlined" severity="error" sx={{ mb: 1.5 }}>
            {err}
          </Alert>
        )}
        {txs && txs.length === 0 ? (
          <Typography sx={{ color: "text.secondary", fontSize: 14, py: 2 }}>
            {t("wallets.activityEmpty")}
          </Typography>
        ) : (
          <Table size="small">
            <TableHead>
              <TableRow>
                <TableCell>{t("wallets.activityWhen")}</TableCell>
                <TableCell>{t("wallets.activityDirection")}</TableCell>
                <TableCell align="right">{t("wallets.activityAmount", { sym: coin.symbol })}</TableCell>
                <TableCell align="right">{t("wallets.activityFee")}</TableCell>
                <TableCell align="right">{t("wallets.activityConfs")}</TableCell>
                <TableCell>{t("wallets.activityTxid")}</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {(txs ?? []).map((tx) => (
                <TableRow key={tx.txid}>
                  <TableCell sx={{ whiteSpace: "nowrap" }}>
                    {tx.timestamp
                      ? new Date(tx.timestamp * 1000).toLocaleString()
                      : t("wallets.activityPending")}
                  </TableCell>
                  <TableCell
                    sx={{ color: tx.direction === "sent" ? "warning.main" : "success.main" }}
                  >
                    {tx.direction === "sent" ? t("wallets.activitySent") : t("wallets.activityReceived")}
                  </TableCell>
                  <TableCell align="right" sx={{ fontFamily: C.mono }}>
                    {tx.direction === "sent" ? "−" : "+"}
                    {fmtBare(tx.amount_sat)}
                  </TableCell>
                  <TableCell align="right" sx={{ fontFamily: C.mono, color: "text.secondary" }}>
                    {tx.fee_sat != null ? fmtBare(tx.fee_sat) : "—"}
                  </TableCell>
                  <TableCell align="right" sx={{ fontFamily: C.mono }}>
                    {tx.confirmations}
                  </TableCell>
                  <TableCell
                    sx={{
                      fontFamily: C.mono,
                      fontSize: 11.5,
                      maxWidth: 140,
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                    }}
                  >
                    <Tooltip title={tx.txid}>
                      <span>{tx.txid}</span>
                    </Tooltip>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        )}
      </DialogContent>
      <DialogActions sx={{ px: 3, pb: 2 }}>
        <Button color="inherit" onClick={() => void load()} sx={{ mr: "auto" }}>
          {t("wallets.refresh")}
        </Button>
        <Button variant="contained" onClick={onClose}>
          {t("wallets.close")}
        </Button>
      </DialogActions>
    </Dialog>
  );
}
