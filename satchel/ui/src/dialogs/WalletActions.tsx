import { useCallback, useEffect, useMemo, useState } from "react";
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
  ToggleButton,
  ToggleButtonGroup,
  Tooltip,
  Typography,
} from "@mui/material";
import ContentCopyIcon from "@mui/icons-material/ContentCopy";
import { errMsg, rpc } from "../api/tauri";
import { useApp } from "../AppContext";
import { useT } from "../i18n";
import type { Translate } from "../i18n";
import {
  canonicalAmount,
  fmtBare,
  fmtFee,
  isActive,
  parseAmount,
  sanitizeAmountInput,
} from "../format";
import { C } from "../theme";
import type { CoinInfo, SendFeeEstimates, WalletTx } from "../api/types";

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
        <DialogContentText sx={{ mb: 2 }}>
          {coin.nodeless ? t("wallets.receiveIntro") : t("wallets.receiveIntroRpc")}
        </DialogContentText>
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

// ---- fee choice (phoenix-parity presets + custom fallback) -----------------

/** Display-only size of a typical 1-in/2-out send, for previewing the fee a
 *  chosen rate implies (phoenix assumes the same); the wallet computes the
 *  real fee from the actual tx when it builds it. */
const ASSUMED_SEND_VSIZE = 170;

type FeePresetKey = "slow" | "normal" | "fast";
type FeeSelKey = FeePresetKey | "custom";

/** Phoenix-parity block targets behind the presets. */
const FEE_PRESET_BLOCKS: Record<FeePresetKey, number> = { slow: 144, normal: 6, fast: 1 };
const FEE_PRESET_ORDER: FeePresetKey[] = ["slow", "normal", "fast"];

interface FeeChoice {
  est: SendFeeEstimates | null;
  sel: FeeSelKey;
  setSel: (k: FeeSelKey) => void;
  customRate: string;
  setCustomRate: (v: string) => void;
  /** Effective sat/vB of the current choice; null while loading or while the
   *  custom field is empty/zero. */
  rate: number | null;
  /** sendtoaddress fee args: [conf_target, fee_rate] — a preset re-estimates
   *  server-side at broadcast time, a custom rate is passed verbatim. */
  rpcFeeArgs: [number | null, number | null];
}

/** Load the fee preview and manage the preset/custom selection, phoenix
 *  defaulting included: Normal when the market has estimates, otherwise fall
 *  back to a custom rate at the coin's floor (estimate-less presets stay
 *  disabled in the selector). */
function useFeeChoice(coinId: string): FeeChoice {
  const [est, setEst] = useState<SendFeeEstimates | null>(null);
  const [sel, setSel] = useState<FeeSelKey>("normal");
  const [customRate, setCustomRate] = useState("");

  useEffect(() => {
    void (async () => {
      try {
        const r = await rpc<SendFeeEstimates>("estimatesendfee", [coinId]);
        setEst(r);
        const preset = FEE_PRESET_ORDER.filter((k) => r[k] != null);
        if (preset.includes("normal")) setSel("normal");
        else if (preset.length > 0) setSel(preset[preset.length - 1]);
        else {
          setSel("custom");
          setCustomRate(String(r.min_sat_per_vb));
        }
      } catch {
        // No preview reachable — degrade like an estimate-less market: custom
        // rate at 1 (the backend still floors to the coin minimum at send).
        setEst({ min_sat_per_vb: 1 });
        setSel("custom");
        setCustomRate("1");
      }
    })();
  }, [coinId]);

  const customSat = Math.floor(Number(customRate));
  const rate = sel === "custom" ? (customSat > 0 ? customSat : null) : (est?.[sel] ?? null);
  const rpcFeeArgs: [number | null, number | null] =
    sel === "custom" ? [null, rate] : [FEE_PRESET_BLOCKS[sel], null];
  return { est, sel, setSel, customRate, setCustomRate, rate, rpcFeeArgs };
}

function FeeSelector({ t, fee, busy }: { t: Translate; fee: FeeChoice; busy: boolean }) {
  const { est, sel, setSel, customRate, setCustomRate } = fee;
  const noEstimates = est != null && FEE_PRESET_ORDER.every((k) => est[k] == null);
  return (
    <Box sx={{ mt: 2 }}>
      <Typography sx={{ fontSize: 12, color: "text.secondary", mb: 0.75 }}>
        {t("wallets.feeLabel")}
      </Typography>
      <ToggleButtonGroup
        exclusive
        fullWidth
        size="small"
        value={sel}
        onChange={(_, v) => v && setSel(v as FeeSelKey)}
        disabled={busy}
      >
        {FEE_PRESET_ORDER.map((k) => (
          <ToggleButton key={k} value={k} disabled={est == null || est[k] == null}>
            <Box>
              <Typography sx={{ fontSize: 12.5, fontWeight: 600, lineHeight: 1.3 }}>
                {t(`wallets.fee_${k}`)}
              </Typography>
              <Typography sx={{ fontSize: 11, color: "text.secondary", fontFamily: C.mono }}>
                {est == null
                  ? "…"
                  : est[k] != null
                    ? t("wallets.feeRate", { rate: est[k] })
                    : t("wallets.feeNoEstimate")}
              </Typography>
            </Box>
          </ToggleButton>
        ))}
        <ToggleButton value="custom">
          <Box>
            <Typography sx={{ fontSize: 12.5, fontWeight: 600, lineHeight: 1.3 }}>
              {t("wallets.fee_custom")}
            </Typography>
            <Typography sx={{ fontSize: 11, color: "text.secondary", fontFamily: C.mono }}>
              {sel === "custom" && fee.rate != null
                ? t("wallets.feeRate", { rate: fee.rate })
                : "—"}
            </Typography>
          </Box>
        </ToggleButton>
      </ToggleButtonGroup>
      {noEstimates && (
        <Typography sx={{ fontSize: 12, color: "warning.main", mt: 0.75 }}>
          {t("wallets.feeNoEstimatesNote")}
        </Typography>
      )}
      {sel === "custom" && (
        <TextField
          label={t("wallets.feeCustomLabel")}
          value={customRate}
          onChange={(e) => setCustomRate(e.target.value.replace(/[^0-9]/g, ""))}
          helperText={t("wallets.feeCustomMin", { min: est?.min_sat_per_vb ?? 1 })}
          fullWidth
          size="small"
          sx={{ mt: 1.5 }}
          disabled={busy}
          slotProps={{ htmlInput: { inputMode: "numeric", style: { fontFamily: C.mono } } }}
        />
      )}
    </Box>
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
  // Send-everything (phoenix parity): amount becomes the WHOLE balance and
  // the fee comes out of it (the backend sweeps — no user-computed
  // balance-minus-fee guesswork). Typing in the amount field switches back
  // to a normal send.
  const [sendAll, setSendAll] = useState(false);
  const [err, setErr] = useState("");
  const [busy, setBusy] = useState(false);
  const [confirming, setConfirming] = useState(false);
  const fee = useFeeChoice(coin.id);

  const amountSat = sendAll ? (balanceSat ?? 0) : Math.round(parseAmount(amount) * 1e8);
  // The preview fee, at the assumed typical size — display + overspend guard
  // only; the wallet computes the real fee when it builds the tx.
  const feeSat = fee.rate != null ? fee.rate * ASSUMED_SEND_VSIZE : null;
  const overspend =
    !sendAll && balanceSat != null && amountSat + (feeSat ?? 0) > balanceSat;

  function review() {
    if (!address.trim()) {
      setErr(t("wallets.sendNeedAddress"));
      return;
    }
    if (!(amountSat > 0)) {
      setErr(t("wallets.sendNeedAmount"));
      return;
    }
    if (fee.rate == null) {
      setErr(t("wallets.sendNeedFee"));
      return;
    }
    setErr("");
    setConfirming(true);
  }

  async function send() {
    setErr("");
    setBusy(true);
    try {
      const [confTarget, feeRate] = fee.rpcFeeArgs;
      const r = await rpc<{ txid: string }>("sendtoaddress", [
        coin.id,
        address.trim(),
        sendAll ? "all" : canonicalAmount(amount),
        confTarget,
        feeRate,
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
      <DialogTitle>
        {confirming ? t("wallets.sendConfirmTitle", { sym: coin.symbol }) : t("wallets.sendTitle", { sym: coin.symbol })}
      </DialogTitle>
      {confirming ? (
        <DialogContent>
          <ConfirmRow label={t("wallets.sendConfirmRecipient")} value={address.trim()} mono />
          <ConfirmRow
            label={t("wallets.sendConfirmAmount")}
            value={
              sendAll
                ? `~${fmtBare(Math.max(0, amountSat - (feeSat ?? 0)))} ${coin.symbol}`
                : `${fmtBare(amountSat)} ${coin.symbol}`
            }
            mono
          />
          <ConfirmRow
            label={t("wallets.sendConfirmFee")}
            value={
              feeSat != null
                ? t("wallets.sendConfirmFeeValue", {
                    fee: fmtBare(feeSat),
                    sym: coin.symbol,
                    rate: fee.rate ?? 0,
                  })
                : "…"
            }
            mono
          />
          <ConfirmRow
            label={t("wallets.sendConfirmTotal")}
            value={
              sendAll
                ? `${fmtBare(amountSat)} ${coin.symbol}`
                : `~${fmtBare(amountSat + (feeSat ?? 0))} ${coin.symbol}`
            }
            mono
          />
          {sendAll && (
            <Typography sx={{ color: "text.secondary", fontSize: 12, mt: 1 }}>
              {t("wallets.sendAllNote")}
            </Typography>
          )}
          <Alert icon={false} variant="outlined" severity="warning" sx={{ mt: 2 }}>
            {t("wallets.sendIrreversible")}
          </Alert>
          {err && <Typography sx={{ color: "error.main", fontSize: 13, mt: 1.25 }}>{err}</Typography>}
        </DialogContent>
      ) : (
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
            value={sendAll && balanceSat != null ? fmtFee(balanceSat) : amount}
            onChange={(e) => {
              setSendAll(false);
              setAmount(sanitizeAmountInput(e.target.value));
            }}
            error={overspend}
            helperText={
              sendAll ? t("wallets.sendAllNote") : overspend ? t("wallets.sendOverBalance") : " "
            }
            fullWidth
            sx={{ mt: 2 }}
            slotProps={{
              htmlInput: { inputMode: "decimal", style: { fontFamily: C.mono } },
              input: {
                endAdornment: (
                  <InputAdornment position="end">
                    <Button
                      size="small"
                      color={sendAll ? "primary" : "inherit"}
                      disabled={busy || !balanceSat}
                      onClick={() => setSendAll(true)}
                      sx={{ minWidth: 0, mr: 0.5 }}
                    >
                      {t("wallets.sendMax")}
                    </Button>
                    {coin.symbol}
                  </InputAdornment>
                ),
              },
            }}
          />
          <FeeSelector t={t} fee={fee} busy={busy} />
          <Typography sx={{ color: "text.secondary", fontSize: 12, mt: 1 }}>
            {feeSat != null
              ? t("wallets.sendFeePreview", { fee: fmtBare(feeSat), sym: coin.symbol })
              : " "}
          </Typography>
          {err && <Typography sx={{ color: "error.main", fontSize: 13, mt: 1.25 }}>{err}</Typography>}
        </DialogContent>
      )}
      <DialogActions sx={{ px: 3, pb: 2 }}>
        <Button color="inherit" onClick={onClose} disabled={busy} sx={{ mr: "auto" }}>
          {t("common.cancel")}
        </Button>
        {confirming ? (
          <>
            <Button color="inherit" onClick={() => setConfirming(false)} disabled={busy}>
              {t("wallets.sendBack")}
            </Button>
            <Button variant="contained" onClick={() => void send()} disabled={busy}>
              {t("wallets.sendConfirm")}
            </Button>
          </>
        ) : (
          <Button variant="contained" onClick={review} disabled={busy || overspend}>
            {t("wallets.sendReview")}
          </Button>
        )}
      </DialogActions>
    </Dialog>
  );
}

/** One label/value line of the send confirmation summary. */
function ConfirmRow({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return (
    <Box sx={{ display: "flex", gap: 2, py: 0.5, alignItems: "baseline" }}>
      <Typography sx={{ fontSize: 12.5, color: "text.secondary", minWidth: 130 }}>{label}</Typography>
      <Typography
        sx={{
          fontSize: 13.5,
          fontFamily: mono ? C.mono : undefined,
          wordBreak: "break-all",
          textAlign: "right",
          flex: 1,
        }}
      >
        {value}
      </Typography>
    </Box>
  );
}

/** Effective feerate a pending send pays now (ceil, like the backends round). */
function paidRate(tx: WalletTx): number | null {
  return tx.fee_sat != null && tx.vsize > 0 ? Math.ceil(tx.fee_sat / tx.vsize) : null;
}

/** A row the user can RBF-bump: our own broadcast, still unconfirmed, with a
 *  known fee. A timestamp-less row is a built-but-unbroadcast v2 funding
 *  reservation — nothing on the network to replace yet. */
function canBump(tx: WalletTx): boolean {
  return (
    tx.direction === "sent" && tx.confirmations === 0 && tx.timestamp != null && paidRate(tx) != null
  );
}

function BumpFeeDialog({
  coin,
  tx,
  onClose,
  onBumped,
}: {
  coin: CoinInfo;
  tx: WalletTx;
  onClose: () => void;
  onBumped: () => void | Promise<void>;
}) {
  const { log } = useApp();
  const t = useT();
  const [err, setErr] = useState("");
  const [busy, setBusy] = useState(false);
  const fee = useFeeChoice(coin.id);

  const oldRate = paidRate(tx) ?? 0;
  // BIP125 rule 4: the replacement must beat the old rate; strictly-higher is
  // the useful UI floor (bdk enforces the real incremental-relay margin).
  const tooLow = fee.rate != null && fee.rate <= oldRate;

  async function bump() {
    if (fee.rate == null) {
      setErr(t("wallets.sendNeedFee"));
      return;
    }
    setErr("");
    setBusy(true);
    try {
      const r = await rpc<{ txid: string }>("bumpfee", [coin.id, tx.txid, fee.rate]);
      log(t("wallets.bumpBroadcast", { txid: r.txid.slice(0, 16), sym: coin.symbol }));
      onClose();
      await onBumped();
    } catch (e) {
      setErr(errMsg(e));
      setBusy(false);
    }
  }

  return (
    <Dialog open onClose={busy ? undefined : onClose} maxWidth="sm" fullWidth>
      <DialogTitle>{t("wallets.bumpTitle", { sym: coin.symbol })}</DialogTitle>
      <DialogContent>
        <DialogContentText sx={{ mb: 1 }}>
          {t("wallets.bumpIntro", { rate: oldRate })}
        </DialogContentText>
        <Typography
          sx={{ fontFamily: C.mono, fontSize: 11.5, color: "text.secondary", wordBreak: "break-all" }}
        >
          {tx.txid}
        </Typography>
        <FeeSelector t={t} fee={fee} busy={busy} />
        {tooLow && (
          <Typography sx={{ color: "warning.main", fontSize: 12.5, mt: 1 }}>
            {t("wallets.bumpNeedHigher", { rate: oldRate })}
          </Typography>
        )}
        {err && <Typography sx={{ color: "error.main", fontSize: 13, mt: 1.25 }}>{err}</Typography>}
      </DialogContent>
      <DialogActions sx={{ px: 3, pb: 2 }}>
        <Button color="inherit" onClick={onClose} disabled={busy} sx={{ mr: "auto" }}>
          {t("common.cancel")}
        </Button>
        <Button
          variant="contained"
          onClick={() => void bump()}
          disabled={busy || fee.rate == null || tooLow}
        >
          {t("wallets.bumpConfirm")}
        </Button>
      </DialogActions>
    </Dialog>
  );
}

export function ActivityDialog({ coin, onClose }: { coin: CoinInfo; onClose: () => void }) {
  const t = useT();
  const { swaps } = useApp();
  const [txs, setTxs] = useState<WalletTx[] | null>(null);
  const [bumping, setBumping] = useState<WalletTx | null>(null);
  const [err, setErr] = useState("");

  // A live swap's funding is the nurse's to bump (v1 RBF / v2 CPFP — a
  // hand-RBF of a v2 funding would invalidate its pre-signed redeems), so
  // those rows don't offer Bump. The engine refuses too; this hides the lever.
  const swapFunding = useMemo(() => {
    const set = new Set<string>();
    for (const s of swaps) {
      if (!isActive(s)) continue;
      if (s.fund_a_txid) set.add(s.fund_a_txid);
      if (s.fund_b_txid) set.add(s.fund_b_txid);
    }
    return set;
  }, [swaps]);

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
                <TableCell />
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
                  <TableCell align="right" sx={{ whiteSpace: "nowrap" }}>
                    {canBump(tx) && !swapFunding.has(tx.txid) && (
                      <Tooltip title={t("wallets.bumpHint")}>
                        <Button size="small" color="inherit" onClick={() => setBumping(tx)}>
                          {t("wallets.bump")}
                        </Button>
                      </Tooltip>
                    )}
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
      {bumping && (
        <BumpFeeDialog
          coin={coin}
          tx={bumping}
          onClose={() => setBumping(null)}
          onBumped={load}
        />
      )}
    </Dialog>
  );
}
