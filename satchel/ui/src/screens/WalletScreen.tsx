import { useCallback, useEffect, useState } from "react";
import { Alert, Box, Button, Card, CardContent, Link, TextField, Tooltip, Typography } from "@mui/material";
import { useApp } from "../AppContext";
import { useConfirm } from "../ui/ConfirmProvider";
import { useNavigate } from "../ui/nav";
import { useT } from "../i18n";
import { errMsg, rpc } from "../api/tauri";
import { EmptyState } from "../components/StatusViews";
import CoinGlyph from "../components/CoinGlyph";
import { canonicalAmount, decimalSeparator, fmtBare, parseAmount, sanitizeAmountInput } from "../format";
import { C } from "../theme";
import type { CoinInfo } from "../api/types";

interface Bal {
  text: string;
  error?: string;
}

export default function WalletScreen() {
  const { setConn, setSymbol, symOf, log } = useApp();
  const confirm = useConfirm();
  const navigate = useNavigate();
  const t = useT();

  const [coins, setCoins] = useState<CoinInfo[] | null>(null);
  const [notConnected, setNotConnected] = useState(false);
  const [balances, setBalances] = useState<Record<string, Bal>>({});
  const [addrs, setAddrs] = useState<Record<string, string>>({});

  const load = useCallback(async () => {
    let configured: CoinInfo[];
    try {
      configured = (await rpc<{ coins: CoinInfo[] }>("listcoins")).coins.filter((c) => c.configured);
      setConn(true);
      setNotConnected(false);
    } catch {
      setNotConnected(true);
      setCoins(null);
      return;
    }
    configured.forEach((c) => setSymbol(c.id, c.symbol));
    setCoins(configured);

    // Balances load after the cards render (each independent — one failing
    // doesn't blank the others).
    for (const c of configured) {
      try {
        const r = await rpc<{ balance_sat: number }>("getbalance", [c.id]);
        setBalances((b) => ({ ...b, [c.id]: { text: fmtBare(r.balance_sat) } }));
      } catch (e) {
        setBalances((b) => ({ ...b, [c.id]: { text: "—", error: errMsg(e) } }));
      }
    }
  }, [setConn, setSymbol]);

  useEffect(() => {
    void load();
  }, [load]);

  async function receive(id: string) {
    try {
      const r = await rpc<{ address: string }>("getnewaddress", [id]);
      setAddrs((a) => ({ ...a, [id]: r.address }));
      log(`fresh ${id} address: ${r.address}`);
    } catch (e) {
      log("receive: " + errMsg(e));
    }
  }

  async function send(id: string, to: string, amountInput: string) {
    // Normalize the locale-entered amount to canonical dot-decimal for the wire.
    const amount = canonicalAmount(amountInput);
    if (!to || !(parseAmount(amountInput) > 0)) {
      log("send: enter an address and a valid amount");
      return;
    }
    const ok = await confirm({
      title: t("wallets.sendTitle", { amount, sym: symOf(id) }),
      body: t("wallets.sendConfirmBody", { to }),
      confirmLabel: t("wallets.send"),
      danger: true,
    });
    if (!ok) return;
    try {
      const r = await rpc<{ txid: string }>("sendtoaddress", [id, to, amount]);
      log(`${id} send: ${r.txid}`);
      await load();
    } catch (e) {
      log("send: " + errMsg(e));
    }
  }

  return (
    <>
      <Alert
        icon={false}
        severity="warning"
        variant="outlined"
        sx={{ color: "primary.main", borderColor: "divider", fontSize: 13, mb: 2, "& .MuiAlert-message": { py: 0.5 } }}
      >
        {t("wallets.intro")}{" "}
        <Box component="span" sx={{ display: "block", mt: 0.75, color: "text.secondary" }}>
          {t("wallets.hotSeedNudge")}
        </Box>
      </Alert>

      {notConnected ? (
        <EmptyState title={t("wallets.notConnected")}>{t("wallets.notConnectedBody")}</EmptyState>
      ) : coins && coins.length === 0 ? (
        <EmptyState
          title={t("wallets.noCoins")}
          action={
            <Button variant="contained" onClick={() => navigate("settings")}>
              {t("wallets.goToCoins")}
            </Button>
          }
        >
          {t("wallets.noCoinsBody")}
        </EmptyState>
      ) : (
        <Box sx={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(380px, 1fr))", gap: 1.875 }}>
          {(coins ?? []).map((c) => (
            <WalletCard
              key={c.id}
              c={c}
              bal={balances[c.id]}
              addr={addrs[c.id]}
              onReceive={() => void receive(c.id)}
              onSend={(to, amt) => void send(c.id, to, amt)}
            />
          ))}
        </Box>
      )}
    </>
  );
}

function WalletCard({
  c,
  bal,
  addr,
  onReceive,
  onSend,
}: {
  c: CoinInfo;
  bal: Bal | undefined;
  addr: string | undefined;
  onReceive: () => void;
  onSend: (to: string, amount: string) => void;
}) {
  const t = useT();
  const [to, setTo] = useState("");
  const [amount, setAmount] = useState("");

  return (
    <Card variant="outlined">
      <CardContent sx={{ display: "flex", flexDirection: "column", gap: 1.75 }}>
        <Box sx={{ display: "flex", alignItems: "center", gap: 1.6 }}>
          <CoinGlyph coin={c} configured />
          <Box>
            <Typography sx={{ fontSize: 15, fontWeight: 600 }}>{c.display_name}</Typography>
            <Typography sx={{ color: "text.secondary", fontFamily: C.mono, fontSize: 12 }}>
              {c.symbol}
            </Typography>
          </Box>
          <Box sx={{ ml: "auto", textAlign: "right" }}>
            <Tooltip title={bal?.error ?? ""} disableHoverListener={!bal?.error}>
              <Typography
                sx={{
                  fontFamily: C.mono,
                  fontSize: 22,
                  fontWeight: 600,
                  fontVariantNumeric: "tabular-nums",
                  color: bal?.error ? "error.main" : "text.primary",
                }}
              >
                {bal?.text ?? "…"}
              </Typography>
            </Tooltip>
            <Typography sx={{ fontSize: 10.5, color: "text.secondary", letterSpacing: "0.06em", textTransform: "uppercase" }}>
              {t("wallets.balanceLabel", { symbol: c.symbol })}
            </Typography>
          </Box>
        </Box>

        <Box sx={{ display: "flex", gap: 1.25, alignItems: "center", minHeight: 30 }}>
          <Button size="small" variant="outlined" color="inherit" onClick={onReceive}>
            {t("wallets.receive")}
          </Button>
          {addr && (
            <Link
              sx={{ fontFamily: C.mono, fontSize: 12, color: "primary.main", wordBreak: "break-all", userSelect: "all" }}
              underline="none"
              component="span"
            >
              {addr}
            </Link>
          )}
        </Box>

        <Box
          component="form"
          onSubmit={(e) => {
            e.preventDefault();
            onSend(to.trim(), amount.trim());
          }}
          sx={{ display: "flex", gap: 1, flexWrap: "wrap", alignItems: "flex-end", borderTop: `1px solid ${C.line}`, pt: 1.5 }}
        >
          <TextField
            label={t("wallets.sendTo")}
            size="small"
            placeholder={`${c.symbol} address`}
            value={to}
            onChange={(e) => setTo(e.target.value)}
            sx={{ flex: 1, minWidth: 180 }}
          />
          <TextField
            label={t("wallets.amount")}
            size="small"
            placeholder={`0${decimalSeparator()}0`}
            value={amount}
            onChange={(e) => setAmount(sanitizeAmountInput(e.target.value))}
            inputMode="decimal"
            autoComplete="off"
            sx={{ width: 120 }}
          />
          <Button type="submit" variant="contained">
            {t("wallets.send")}
          </Button>
        </Box>
      </CardContent>
    </Card>
  );
}
