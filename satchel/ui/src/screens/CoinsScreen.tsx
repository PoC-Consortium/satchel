import { useCallback, useEffect, useState } from "react";
import { Alert, Box, Button, Card, CardContent, Chip, Tooltip, Typography } from "@mui/material";
import { useApp } from "../AppContext";
import { useConfirm } from "../ui/ConfirmProvider";
import { useT } from "../i18n";
import { errMsg, getCoinIcon, listCoinConfig, listCoinTemplates, removeCoin, rpc } from "../api/tauri";
import CoinGlyph from "../components/CoinGlyph";
import NetworkStamp from "../components/NetworkStamp";
import CoinSetup from "../dialogs/CoinSetup";
import { commas, isMainnet } from "../format";
import { C } from "../theme";
import type { CoinConn, CoinInfo, NetConnDefaults, Pair } from "../api/types";

export default function CoinsScreen() {
  const { network, coins: liveCoins, refreshCoins, setConn, setSymbol, log } = useApp();
  const confirm = useConfirm();
  const t = useT();

  const [coins, setCoins] = useState<CoinInfo[] | null>(liveCoins.length ? liveCoins : null);
  const [savedConns, setSavedConns] = useState<Record<string, CoinConn>>({});
  const [templates, setTemplates] = useState<Record<string, NetConnDefaults>>({});
  const [icons, setIcons] = useState<Record<string, string>>({});
  const [pairs, setPairs] = useState<Pair[]>([]);
  const [notConnected, setNotConnected] = useState(false);
  const [setupCoin, setSetupCoin] = useState<CoinInfo | null>(null);

  const load = useCallback(async () => {
    try {
      const cfg = await listCoinConfig();
      const map: Record<string, CoinConn> = {};
      cfg.coins.forEach((c) => (map[c.coin_id] = c));
      setSavedConns(map);
    } catch {
      /* no config yet */
    }

    let info: { coins: CoinInfo[] };
    try {
      info = await rpc<{ coins: CoinInfo[] }>("listcoins");
      setConn(true);
      setNotConnected(false);
    } catch {
      setConn(false);
      setNotConnected(true);
      setCoins(null);
      setPairs([]);
      return;
    }
    info.coins.forEach((c) => setSymbol(c.id, c.symbol));
    setCoins(info.coins);

    try {
      setPairs((await rpc<{ pairs: Pair[] }>("listpairs")).pairs);
    } catch {
      /* keep prior */
    }
  }, [setConn, setSymbol]);

  useEffect(() => {
    void load();
  }, [load]);

  // Coin templates: connection defaults that pre-fill the setup form, plus icon
  // data-URLs for any coin without a bundled glyph (file-added coins).
  useEffect(() => {
    let alive = true;
    (async () => {
      try {
        const r = await listCoinTemplates();
        if (!alive) return;
        const map: Record<string, NetConnDefaults> = {};
        r.coins.forEach((c) => (map[c.coin_id] = c.defaults));
        setTemplates(map);
        for (const c of r.coins.filter((x) => x.has_icon)) {
          getCoinIcon(c.coin_id)
            .then((url) => url && alive && setIcons((m) => ({ ...m, [c.coin_id]: url })))
            .catch(() => {});
        }
      } catch {
        /* templates optional — coins still render with bare defaults */
      }
    })();
    return () => {
      alive = false;
    };
  }, []);

  async function remove(coinId: string) {
    const ok = await confirm({
      title: t("coins.disconnectTitle", { coin: coinId.toUpperCase() }),
      body: t("coins.disconnectBody"),
      confirmLabel: t("coins.remove"),
      danger: true,
    });
    if (!ok) return;
    try {
      await removeCoin(coinId);
      log(t("log.coinDisconnected", { coin: coinId }));
      await load();
      void refreshCoins();
    } catch (e) {
      log(t("log.removeCoinError", { err: errMsg(e) }));
    }
  }

  const onSaved = async () => {
    await load();
    void refreshCoins();
  };

  return (
    <>
      <Alert
        icon={false}
        severity="warning"
        variant="outlined"
        sx={{ color: "primary.main", borderColor: "divider", fontSize: 13, mb: 2, "& .MuiAlert-message": { py: 0.5 } }}
      >
        {t("coins.intro")}
      </Alert>

      {/* The coin config validates against this client's one network. */}
      {network && (
        <Box sx={{ display: "flex", alignItems: "center", gap: 1, mb: 2 }}>
          <Typography sx={{ color: "text.secondary", fontSize: 13 }}>
            {t("coins.networkBadge", {
              network: isMainnet(network) ? t("network.mainnet") : network.toUpperCase(),
            })}
          </Typography>
          {!isMainnet(network) && <NetworkStamp network={network} />}
        </Box>
      )}

      {notConnected ? (
        <Typography sx={{ color: "text.secondary", fontSize: 13 }}>{t("coins.needMerchant")}</Typography>
      ) : (
        <Box
          sx={{
            display: "grid",
            gridTemplateColumns: "repeat(auto-fill, minmax(320px, 1fr))",
            gap: 1.75,
            mb: 3,
          }}
        >
          {(coins ?? []).map((c) => (
            <CoinCard
              key={c.id}
              c={c}
              saved={savedConns[c.id]}
              iconUrl={icons[c.id]}
              onSetup={() => setSetupCoin(c)}
              onRemove={() => void remove(c.id)}
            />
          ))}
        </Box>
      )}

      <Typography sx={{ fontSize: 12, textTransform: "uppercase", letterSpacing: "0.08em", color: "text.secondary" }}>
        {t("coins.pairsTitle")}
      </Typography>
      <Typography sx={{ color: "text.secondary", fontSize: 12, mt: 0.5 }}>{t("coins.pairsHint")}</Typography>
      <Box sx={{ display: "flex", flexDirection: "column", gap: 1, mt: 1.25 }}>
        {pairs.length === 0 ? (
          <Typography sx={{ color: "text.secondary", fontSize: 13 }}>{t("coins.noPairs")}</Typography>
        ) : (
          pairs.map((p, i) => <PairRow key={i} p={p} coins={coins ?? []} />)
        )}
      </Box>

      {setupCoin && (
        <CoinSetup
          coin={setupCoin}
          saved={savedConns[setupCoin.id]}
          template={templates[setupCoin.id]}
          onClose={() => setSetupCoin(null)}
          onSaved={onSaved}
        />
      )}
    </>
  );
}

// A credentials-free summary of a saved connection: structured host:port + auth
// kind when present, else the host:port parsed out of a legacy chain_data URL.
function connSummary(saved: CoinConn): string {
  if (saved.rpc_host && saved.rpc_port) {
    const auth = saved.auth_method === "userpass" ? "user/pass" : "cookie";
    return `${saved.rpc_host}:${saved.rpc_port} · ${auth}`;
  }
  const first = saved.chain_data.split(",")[0] ?? "";
  const at = first.split("@");
  return at.length > 1 ? `${first.split("://")[0]}://${at[at.length - 1]}` : first;
}

function StatusPill({ c }: { c: CoinInfo }) {
  const t = useT();
  if (!c.configured)
    return <Chip size="small" variant="outlined" label={t("coins.notSetUp")} sx={{ height: 24, color: "text.secondary" }} />;
  if (c.status === "ok")
    return (
      <Chip
        size="small"
        variant="outlined"
        label={t("coins.connectedTip", { tip: commas(c.tip_height) })}
        sx={{ height: 24, color: C.good, borderColor: C.goodTintBorder, bgcolor: C.goodTintBg }}
      />
    );
  return (
    <Tooltip title={c.status ?? t("coins.errorShort")}>
      <Chip
        size="small"
        variant="outlined"
        label={t("coins.connError")}
        sx={{ height: 24, color: C.bad, borderColor: C.badTintBorder, bgcolor: C.badTintBg, cursor: "help" }}
      />
    </Tooltip>
  );
}

function Cap({ label, on }: { label: string; on?: boolean }) {
  return (
    <Box
      component="span"
      sx={{
        fontSize: 10.5,
        letterSpacing: "0.07em",
        textTransform: "uppercase",
        px: 1,
        py: 0.25,
        borderRadius: 0.75,
        border: "1px solid",
        borderColor: on ? "text.disabled" : "divider",
        color: on ? "text.primary" : "text.secondary",
        bgcolor: on ? C.raised : "transparent",
      }}
    >
      {label}
    </Box>
  );
}

function CoinCard({
  c,
  saved,
  iconUrl,
  onSetup,
  onRemove,
}: {
  c: CoinInfo;
  saved: CoinConn | undefined;
  iconUrl?: string;
  onSetup: () => void;
  onRemove: () => void;
}) {
  const t = useT();
  const caps = c.capabilities || {};
  return (
    <Card variant="outlined" sx={{ borderColor: c.configured ? "text.disabled" : "divider" }}>
      <CardContent sx={{ display: "flex", flexDirection: "column", gap: 1.5 }}>
        <Box sx={{ display: "flex", alignItems: "center", gap: 1.6 }}>
          <CoinGlyph coin={c} configured={c.configured} iconUrl={iconUrl} />
          <Box>
            <Typography sx={{ fontSize: 15, fontWeight: 600 }}>{c.display_name}</Typography>
            <Typography sx={{ color: "text.secondary", fontFamily: C.mono, fontSize: 12 }}>
              {c.symbol}
            </Typography>
          </Box>
          <Box sx={{ ml: "auto" }}>
            <StatusPill c={c} />
          </Box>
        </Box>
        <Box sx={{ display: "flex", gap: 0.75, flexWrap: "wrap" }}>
          <Cap label="CLTV" on={caps.cltv} />
          <Cap label="SegWit" on={caps.segwit_v0} />
          <Cap label="Taproot" on={caps.taproot} />
        </Box>
        {saved && (
          <Typography sx={{ fontSize: 12, color: "text.secondary", fontFamily: C.mono, wordBreak: "break-all" }}>
            {connSummary(saved)}
          </Typography>
        )}
        <Box sx={{ display: "flex", gap: 1, alignItems: "center" }}>
          <Button size="small" variant="contained" onClick={onSetup}>
            {c.configured ? t("coins.editConnection") : t("coins.setUp")}
          </Button>
          {c.configured && (
            <>
              <Box sx={{ flex: 1 }} />
              <Tooltip title={t("coins.disconnectTip")}>
                <Button size="small" variant="outlined" color="inherit" onClick={onRemove}>
                  {t("coins.remove")}
                </Button>
              </Tooltip>
            </>
          )}
        </Box>
      </CardContent>
    </Card>
  );
}

function PairRow({ p, coins }: { p: Pair; coins: CoinInfo[] }) {
  const t = useT();
  const info = (id: string) => coins.find((c) => c.id === id);
  const symFor = (id: string) => info(id)?.symbol || id.toUpperCase();
  const A = symFor(p.coin_a);
  const B = symFor(p.coin_b);

  let state: string;
  if (p.available) state = t("coins.ready");
  else if (!p.both_configured) {
    const missing = [p.coin_a, p.coin_b].filter((id) => !info(id)?.configured).map(symFor);
    state = t("coins.connectMissing", { coins: missing.join(" & ") });
  } else state = t("coins.notBuildable");

  return (
    <Card
      variant="outlined"
      sx={{ display: "flex", alignItems: "center", gap: 1.5, px: 1.75, py: 1.25, borderColor: p.available ? C.goodTintBorder : "divider" }}
    >
      <Typography sx={{ fontWeight: 600 }}>
        {A} ↔ {B}
      </Typography>
      <Box sx={{ display: "flex", gap: 0.75 }}>
        {(p.protocols || []).map((pr) => (
          <Box
            key={pr}
            component="span"
            title={pr === "adaptor" ? t("coins.protoPrivateTip") : t("coins.protoHtlcTip")}
            sx={{
              fontSize: 11,
              textTransform: "uppercase",
              letterSpacing: "0.06em",
              px: 1,
              py: 0.25,
              borderRadius: 0.75,
              border: "1px solid",
              borderColor: p.available && pr === p.selectable ? "primary.main" : "divider",
              color: p.available && pr === p.selectable ? "primary.main" : "text.secondary",
            }}
          >
            {pr === "adaptor" ? t("coins.protoPrivate") : pr}
          </Box>
        ))}
      </Box>
      <Typography sx={{ ml: "auto", fontSize: 12, color: p.available ? C.good : "text.secondary" }}>
        {state}
      </Typography>
    </Card>
  );
}
