import { useCallback, useEffect, useMemo, useState } from "react";
import { Alert, Box, Button, Card, CardContent, Skeleton, Stack, Tooltip, Typography } from "@mui/material";
import { useApp, type Bal } from "../AppContext";
import { useNavigate } from "../ui/nav";
import { useT } from "../i18n";
import { EmptyState } from "../components/StatusViews";
import CoinGlyph from "../components/CoinGlyph";
import { ActivityDialog, ReceiveDialog, SendDialog } from "../dialogs/WalletActions";
import { C } from "../theme";
import type { CoinInfo } from "../api/types";

// Every configured coin's card offers Send / Receive (user decision
// 2026-07-04: RPC-backed wallets get them too — the engine's
// getnewaddress/sendtoaddress always worked against the node wallet).
// Activity stays Electrum-only: listtransactions reads the bdk tx graph;
// an RPC coin's node wallet has its own tooling for history.
//
// Cards seed from AppContext.coins and balances from the app-wide cache, so
// the page renders instantly on every visit — stale, never blank (#91). The
// mount-time refreshes update statuses and numbers in place; only a coin
// that has never reported a balance shows a skeleton slot.
export default function WalletScreen() {
  const { coins, refreshCoins, balances, refreshBalances, connOk } = useApp();
  const navigate = useNavigate();
  const t = useT();

  const configured = useMemo(() => coins.filter((c) => c.configured), [coins]);
  // Keyed as a joined string so the global 10s coins poll (fresh array, same
  // ids) doesn't refire the balance fetch on every tick.
  const configuredIds = useMemo(() => configured.map((c) => c.id).join(","), [configured]);

  const reload = useCallback(async () => {
    await Promise.all([
      refreshCoins(),
      configuredIds ? refreshBalances(configuredIds.split(",")) : Promise.resolve(),
    ]);
  }, [refreshCoins, refreshBalances, configuredIds]);

  useEffect(() => {
    void reload();
  }, [reload]);

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

      {!connOk ? (
        <EmptyState title={t("wallets.notConnected")}>{t("wallets.notConnectedBody")}</EmptyState>
      ) : configured.length === 0 ? (
        <EmptyState
          title={t("wallets.noCoins")}
          action={
            <Button variant="contained" onClick={() => navigate("settings", "coins")}>
              {t("wallets.goToCoins")}
            </Button>
          }
        >
          {t("wallets.noCoinsBody")}
        </EmptyState>
      ) : (
        <Box sx={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(300px, 1fr))", gap: 1.875 }}>
          {configured.map((c) => (
            <WalletCard key={c.id} c={c} bal={balances[c.id]} onChanged={reload} />
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
  // Degraded, not dead (issue #98/#100): the coin still works off healthy
  // servers, but the wallet-home server is down (balance may be stale,
  // sends fall over to views) or a minority of the fleet is down.
  const walletDown = c.wallet_server_state === "down";
  const degraded = c.status === "ok" && (walletDown || (c.servers_down ?? 0) > 0);
  const degradedTip = walletDown
    ? t("wallets.degradedWalletTip")
    : t("wallets.degradedViewsTip", { down: c.servers_down ?? 0, total: c.servers_total ?? 0 });
  return (
    <Card variant="outlined">
      <CardContent sx={{ display: "flex", alignItems: "center", gap: 1.6 }}>
        <CoinGlyph coin={c} configured />
        <Box sx={{ minWidth: 0 }}>
          <Typography sx={{ fontSize: 15, fontWeight: 600 }}>
            {c.display_name}
            {degraded && (
              <Tooltip title={degradedTip}>
                <Box
                  component="span"
                  sx={{
                    ml: 0.75,
                    px: 0.6,
                    py: 0.1,
                    fontSize: 10,
                    fontWeight: 600,
                    letterSpacing: "0.05em",
                    textTransform: "uppercase",
                    color: "warning.main",
                    border: "1px solid",
                    borderColor: "warning.main",
                    borderRadius: 0.75,
                    verticalAlign: "middle",
                  }}
                >
                  {t("wallets.degraded")}
                </Box>
              </Tooltip>
            )}
          </Typography>
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
          {bal ? (
            <Tooltip title={bal.error ?? ""} disableHoverListener={!bal.error}>
              <Typography
                sx={{
                  fontFamily: C.mono,
                  fontSize: 22,
                  fontWeight: 600,
                  fontVariantNumeric: "tabular-nums",
                  color: bal.error ? "error.main" : "text.primary",
                }}
              >
                {bal.text}
              </Typography>
            </Tooltip>
          ) : (
            // First-ever load only: no cached balance for this coin yet. Sized
            // to the balance line so the card doesn't reflow when it lands.
            <Skeleton variant="text" sx={{ fontSize: 22, width: 96, ml: "auto" }} />
          )}
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
