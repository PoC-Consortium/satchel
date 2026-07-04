import { useCallback, useEffect, useState } from "react";
import { Alert, Box, Button, Card, CardContent, Stack, Tooltip, Typography } from "@mui/material";
import { useApp } from "../AppContext";
import { useNavigate } from "../ui/nav";
import { useT } from "../i18n";
import { errMsg, rpc } from "../api/tauri";
import { EmptyState } from "../components/StatusViews";
import CoinGlyph from "../components/CoinGlyph";
import { ActivityDialog, ReceiveDialog, SendDialog } from "../dialogs/WalletActions";
import { fmtBare } from "../format";
import { C } from "../theme";
import type { CoinInfo } from "../api/types";

interface Bal {
  text: string;
  sat?: number;
  error?: string;
}

// Every configured coin's card offers Send / Receive (user decision
// 2026-07-04: RPC-backed wallets get them too — the engine's
// getnewaddress/sendtoaddress always worked against the node wallet).
// Activity stays Electrum-only: listtransactions reads the bdk tx graph;
// an RPC coin's node wallet has its own tooling for history.
export default function WalletScreen() {
  const { setConn, setSymbol, watchOnly } = useApp();
  const navigate = useNavigate();
  const t = useT();

  const [coins, setCoins] = useState<CoinInfo[] | null>(null);
  const [notConnected, setNotConnected] = useState(false);
  const [balances, setBalances] = useState<Record<string, Bal>>({});

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
        setBalances((b) => ({
          ...b,
          [c.id]: { text: fmtBare(r.balance_sat), sat: r.balance_sat },
        }));
      } catch (e) {
        setBalances((b) => ({ ...b, [c.id]: { text: "—", error: errMsg(e) } }));
      }
    }
  }, [setConn, setSymbol]);

  useEffect(() => {
    void load();
  }, [load]);

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
      ) : coins && coins.length === 0 && watchOnly ? (
        <EmptyState title={t("wallets.watchOnlyTitle")}>{t("wallets.watchOnlyBody")}</EmptyState>
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
        <Box sx={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(300px, 1fr))", gap: 1.875 }}>
          {(coins ?? []).map((c) => (
            <WalletCard key={c.id} c={c} bal={balances[c.id]} onChanged={load} />
          ))}
        </Box>
      )}
    </>
  );
}

function WalletCard({
  c,
  bal,
  onChanged,
}: {
  c: CoinInfo;
  bal: Bal | undefined;
  onChanged: () => void | Promise<void>;
}) {
  const t = useT();
  const [dialog, setDialog] = useState<null | "receive" | "send" | "activity">(null);
  return (
    <Card variant="outlined">
      <CardContent sx={{ display: "flex", alignItems: "center", gap: 1.6 }}>
        <CoinGlyph coin={c} configured />
        <Box sx={{ minWidth: 0 }}>
          <Typography sx={{ fontSize: 15, fontWeight: 600 }}>{c.display_name}</Typography>
          <Typography sx={{ color: "text.secondary", fontFamily: C.mono, fontSize: 12 }}>{c.symbol}</Typography>
          {c.nodeless ? (
            <Tooltip title={t("wallets.pactSeedHint")}>
              <Typography sx={{ color: "success.main", fontFamily: C.mono, fontSize: 11 }}>
                {t("wallets.pactSeed")}
              </Typography>
            </Tooltip>
          ) : c.wallet ? (
            <Tooltip title={t("wallets.walletScopedHint")}>
              <Typography
                sx={{
                  color: "text.secondary",
                  fontFamily: C.mono,
                  fontSize: 11,
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                  whiteSpace: "nowrap",
                }}
              >
                {t("wallets.walletName", { wallet: c.wallet })}
              </Typography>
            </Tooltip>
          ) : (
            <Tooltip title={t("wallets.walletDefaultHint")}>
              <Typography sx={{ color: "warning.main", fontSize: 11 }}>
                {t("wallets.walletDefault")}
              </Typography>
            </Tooltip>
          )}
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
      </CardContent>
      <CardContent sx={{ pt: 0, pb: "12px !important" }}>
        <Stack direction="row" spacing={1}>
          <Button size="small" variant="outlined" onClick={() => setDialog("receive")}>
            {t("wallets.receive")}
          </Button>
          <Button size="small" variant="outlined" onClick={() => setDialog("send")}>
            {t("wallets.send")}
          </Button>
          {c.nodeless && (
            <Button size="small" color="inherit" onClick={() => setDialog("activity")}>
              {t("wallets.activity")}
            </Button>
          )}
        </Stack>
      </CardContent>
      {dialog === "receive" && <ReceiveDialog coin={c} onClose={() => setDialog(null)} />}
      {dialog === "send" && (
        <SendDialog
          coin={c}
          balanceSat={bal?.sat}
          onClose={() => setDialog(null)}
          onSent={onChanged}
        />
      )}
      {dialog === "activity" && <ActivityDialog coin={c} onClose={() => setDialog(null)} />}
    </Card>
  );
}
