import { Badge, Box, Tooltip } from "@mui/material";
import HubIcon from "@mui/icons-material/Hub";
import SensorsIcon from "@mui/icons-material/Sensors";
import SwapHorizIcon from "@mui/icons-material/SwapHoriz";
import { useApp } from "../AppContext";
import { useT } from "../i18n";
import { commas, glyph, isActive } from "../format";
import { COIN_ICON } from "../assets/coins";
import type { CoinInfo, RelayStatus } from "../api/types";

// The phoenix content-toolbar status row: small monochrome indicators, greyed
// when inactive / colored when active, each with a tooltip. Signals: pactd
// reachability, Nostr relay connectivity (when configured), per-coin node
// health, and in-flight swaps.
export default function StatusIndicators({ onLiveSwaps }: { onLiveSwaps: () => void }) {
  const { connOk, coins, swaps, relays } = useApp();
  const t = useT();

  const configured = coins.filter((c) => c.configured);
  const liveCount = swaps.filter(isActive).length;

  return (
    <Box sx={{ display: "flex", alignItems: "center", gap: 1.25 }}>
      {/* pactd connection */}
      <Tooltip title={connOk ? t("header.pactConnected") : t("header.pactUnreachable")}>
        <Box sx={{ display: "inline-flex" }}>
          <HubIcon
            sx={{ color: connOk ? "success.main" : "error.main", opacity: connOk ? 1 : 0.85 }}
          />
        </Box>
      </Tooltip>

      {/* Nostr relay connectivity (only when the transport is configured) */}
      {relays.length > 0 && <RelayHealth relays={relays} />}

      {/* per-coin node health — a glyph per configured coin */}
      {configured.length > 0 && (
        <Box sx={{ display: "flex", alignItems: "center", gap: 0.5 }}>
          {configured.map((c) => (
            <CoinHealth key={c.id} c={c} />
          ))}
        </Box>
      )}

      {/* live swaps */}
      <Tooltip
        title={
          liveCount === 0
            ? t("header.liveSwapsNone")
            : liveCount === 1
              ? t("header.liveSwapsOne")
              : t("header.liveSwapsMany", { count: liveCount })
        }
      >
        <Box
          onClick={liveCount > 0 ? onLiveSwaps : undefined}
          sx={{
            display: "inline-flex",
            cursor: liveCount > 0 ? "pointer" : "default",
          }}
        >
          <Badge badgeContent={liveCount} color="primary" overlap="circular">
            <SwapHorizIcon
              sx={{
                color: liveCount > 0 ? "primary.main" : "text.disabled",
                ...(liveCount > 0 && {
                  animation: "satchelPulse 1.8s ease-in-out infinite",
                  "@keyframes satchelPulse": { "50%": { opacity: 0.45 } },
                }),
              }}
            />
          </Badge>
        </Box>
      </Tooltip>
    </Box>
  );
}

// Nostr relay connectivity: green when ≥1 relay is connected (the board can
// reach the network), amber when configured but none are up. Tooltip reports
// the up/total count. Hidden entirely when no relays are configured.
function RelayHealth({ relays }: { relays: RelayStatus[] }) {
  const t = useT();
  const up = relays.filter((r) => r.connected).length;
  const ok = up > 0;
  const title = ok
    ? t("header.relaysOk", { up, total: relays.length })
    : t("header.relaysDown", { total: relays.length });
  return (
    <Tooltip title={title}>
      <Box sx={{ display: "inline-flex" }}>
        <SensorsIcon sx={{ color: ok ? "success.main" : "warning.main", opacity: ok ? 1 : 0.8 }} />
      </Box>
    </Tooltip>
  );
}

function CoinHealth({ c }: { c: CoinInfo }) {
  const t = useT();
  const { coinIcons } = useApp();
  const ok = c.status === "ok";
  const err = !!c.status && c.status !== "ok" && c.status !== "unconfigured";
  const color = ok ? "success.main" : err ? "error.main" : "text.disabled";
  const title = ok
    ? t("header.coinOk", { name: c.display_name, tip: commas(c.tip_height) })
    : err
      ? t("header.coinError", { name: c.display_name, status: c.status ?? "error" })
      : t("header.coinUnconfigured", { name: c.display_name });
  // Same logo the Coins/Wallet cards use (CoinGlyph), so the header matches;
  // health is carried by the border colour + dimming. Built-ins use the bundled
  // asset; file-coins (e.g. ltc) use the shared fetched data URL; anything still
  // without an icon falls back to the generated text glyph.
  const icon = COIN_ICON[c.id] ?? coinIcons[c.id] ?? undefined;

  return (
    <Tooltip title={title}>
      <Box
        sx={{
          width: 24,
          height: 24,
          borderRadius: "6px",
          display: "grid",
          placeItems: "center",
          fontSize: 13,
          fontWeight: 700,
          border: "1px solid",
          borderColor: color,
          color,
          opacity: ok ? 1 : 0.55,
        }}
      >
        {icon ? (
          <Box component="img" src={icon} alt={c.symbol || c.id} sx={{ width: 15, height: 15, display: "block" }} />
        ) : (
          glyph(c)
        )}
      </Box>
    </Tooltip>
  );
}
