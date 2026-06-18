import { useCallback, useEffect, useMemo, useState } from "react";
import {
  Box,
  Button,
  Chip,
  Dialog,
  DialogContent,
  DialogTitle,
  LinearProgress,
  Stack,
  Tooltip,
  Typography,
} from "@mui/material";
import { useApp } from "../AppContext";
import { useT } from "../i18n";
import { getCoinIcon, listCoinTemplates } from "../api/tauri";
import CoinGlyph from "../components/CoinGlyph";
import CoinSetup from "./CoinSetup";
import { C } from "../theme";
import type { CoinInfo, CoinTemplate, NetConnDefaults } from "../api/types";

// At least this many live coins are required before trading is unlocked — a swap
// needs two chains, so the first-run gate insists on two working nodes.
const MIN_COINS = 2;

// First-run coin gate: the app cannot reach the trading UI until ≥2 coins are
// configured AND their nodes are live. Lists every coin the engine supports
// (from listcoins) joined with the connection templates (coins.toml); picking
// one opens the structured CoinSetup pre-filled from its template. A template
// whose coin the engine doesn't know is shown as unsupported.
export default function CoinWizard({ onDone }: { onDone: () => void | Promise<void> }) {
  const { coins, refreshCoins } = useApp();
  const t = useT();
  const [templates, setTemplates] = useState<CoinTemplate[]>([]);
  const [icons, setIcons] = useState<Record<string, string>>({});
  const [setupCoin, setSetupCoin] = useState<{ coin: CoinInfo; template?: NetConnDefaults } | null>(null);

  const loadTemplates = useCallback(async () => {
    try {
      const r = await listCoinTemplates();
      setTemplates(r.coins);
      // Fetch icons for templated coins (data: URLs); ignore failures.
      for (const c of r.coins.filter((x) => x.has_icon)) {
        getCoinIcon(c.coin_id)
          .then((url) => url && setIcons((m) => ({ ...m, [c.coin_id]: url })))
          .catch(() => {});
      }
    } catch {
      /* no templates — the engine coins still render with bare defaults */
    }
  }, []);

  useEffect(() => {
    void loadTemplates();
    void refreshCoins();
  }, [loadTemplates, refreshCoins]);

  const tplOf = useCallback(
    (id: string) => templates.find((x) => x.coin_id === id),
    [templates],
  );

  const liveCount = useMemo(
    () => coins.filter((c) => c.configured && c.status === "ok").length,
    [coins],
  );

  // Engine-supported coins (from listcoins) + any template coin the engine
  // doesn't know (shown disabled as "unsupported by this engine").
  const supportedIds = new Set(coins.map((c) => c.id));
  const unsupported = templates.filter((tpl) => !supportedIds.has(tpl.coin_id));

  const onSaved = async () => {
    await refreshCoins();
  };

  return (
    <Dialog open maxWidth="sm" fullWidth>
      <DialogTitle>{t("coinWizard.title")}</DialogTitle>
      <DialogContent>
        <Typography sx={{ color: "text.secondary", fontSize: 13, mb: 2 }}>
          {t("coinWizard.intro")}
        </Typography>

        <Box sx={{ mb: 2 }}>
          <Box sx={{ display: "flex", alignItems: "center", gap: 1, mb: 0.5 }}>
            <Typography sx={{ fontSize: 12, color: "text.secondary" }}>
              {t("coinWizard.progress", { count: Math.min(liveCount, MIN_COINS), min: MIN_COINS })}
            </Typography>
          </Box>
          <LinearProgress
            variant="determinate"
            value={Math.min(100, (liveCount / MIN_COINS) * 100)}
            sx={{ height: 6, borderRadius: 3 }}
          />
        </Box>

        <Stack spacing={1}>
          {coins.map((c) => (
            <CoinRow
              key={c.id}
              coin={c}
              iconUrl={icons[c.id]}
              onSetup={() => setSetupCoin({ coin: c, template: tplOf(c.id)?.defaults })}
            />
          ))}
          {unsupported.map((tpl) => (
            <UnsupportedRow key={tpl.coin_id} tpl={tpl} iconUrl={icons[tpl.coin_id]} />
          ))}
        </Stack>
      </DialogContent>
      <Box sx={{ display: "flex", px: 3, pb: 2, pt: 1 }}>
        <Box sx={{ flex: 1 }} />
        <Button
          variant="contained"
          disabled={liveCount < MIN_COINS}
          onClick={() => void onDone()}
        >
          {t("coinWizard.continue")}
        </Button>
      </Box>

      {setupCoin && (
        <CoinSetup
          coin={setupCoin.coin}
          saved={undefined}
          template={setupCoin.template}
          onClose={() => setSetupCoin(null)}
          onSaved={onSaved}
        />
      )}
    </Dialog>
  );
}

function CoinRow({
  coin,
  iconUrl,
  onSetup,
}: {
  coin: CoinInfo;
  iconUrl?: string;
  onSetup: () => void;
}) {
  const t = useT();
  const live = coin.configured && coin.status === "ok";
  return (
    <Box
      sx={{
        display: "flex",
        alignItems: "center",
        gap: 1.5,
        p: 1.25,
        border: "1px solid",
        borderColor: live ? C.goodTintBorder : "divider",
        borderRadius: 1.5,
      }}
    >
      <CoinGlyph coin={coin} configured={coin.configured} iconUrl={iconUrl} />
      <Box sx={{ minWidth: 0 }}>
        <Typography sx={{ fontSize: 14, fontWeight: 600 }}>{coin.display_name}</Typography>
        <Typography sx={{ color: "text.secondary", fontFamily: C.mono, fontSize: 12 }}>
          {coin.symbol}
        </Typography>
      </Box>
      <Box sx={{ ml: "auto", display: "flex", alignItems: "center", gap: 1 }}>
        {live ? (
          <Chip
            size="small"
            variant="outlined"
            label={t("coinWizard.live")}
            sx={{ height: 24, color: C.good, borderColor: C.goodTintBorder, bgcolor: C.goodTintBg }}
          />
        ) : coin.configured && coin.status && coin.status !== "ok" ? (
          <Tooltip title={coin.status}>
            <Chip
              size="small"
              variant="outlined"
              label={t("coinWizard.nodeDown")}
              sx={{ height: 24, color: C.bad, borderColor: C.badTintBorder, bgcolor: C.badTintBg, cursor: "help" }}
            />
          </Tooltip>
        ) : null}
        <Button size="small" variant={live ? "outlined" : "contained"} color={live ? "inherit" : "primary"} onClick={onSetup}>
          {coin.configured ? t("coins.editConnection") : t("coins.setUp")}
        </Button>
      </Box>
    </Box>
  );
}

function UnsupportedRow({ tpl, iconUrl }: { tpl: CoinTemplate; iconUrl?: string }) {
  const t = useT();
  return (
    <Tooltip title={t("coins.unsupportedByEngineTip")}>
      <Box
        sx={{
          display: "flex",
          alignItems: "center",
          gap: 1.5,
          p: 1.25,
          border: "1px solid",
          borderColor: "divider",
          borderRadius: 1.5,
          opacity: 0.5,
          cursor: "help",
        }}
      >
        <CoinGlyph coin={{ id: tpl.coin_id, symbol: tpl.symbol }} iconUrl={iconUrl} />
        <Box sx={{ minWidth: 0 }}>
          <Typography sx={{ fontSize: 14, fontWeight: 600 }}>{tpl.display_name}</Typography>
          <Typography sx={{ color: "text.secondary", fontFamily: C.mono, fontSize: 12 }}>
            {tpl.symbol}
          </Typography>
        </Box>
        <Chip
          size="small"
          variant="outlined"
          label={t("coins.unsupportedByEngine")}
          sx={{ height: 24, ml: "auto", color: "text.secondary" }}
        />
      </Box>
    </Tooltip>
  );
}
