import { Box } from "@mui/material";
import { glyph } from "../format";
import { COIN_ICON } from "../assets/coins";
import { C } from "../theme";

// The square coin badge, shared by the Coins + Wallet cards. Icon resolution:
// a bundled logo asset (COIN_ICON, for the built-ins) → a `iconUrl` (a data:
// URL from a coins.toml template's icon file, for added coins) → the generated
// text glyph (₿ / ◈ / first letter). Gold-bordered when the coin is configured.
export default function CoinGlyph({
  coin,
  configured,
  iconUrl,
}: {
  coin: { id: string; symbol?: string };
  configured?: boolean;
  /** A data: URL for a coin added via coins.toml (no bundled asset). */
  iconUrl?: string | null;
}) {
  const icon = COIN_ICON[coin.id] ?? iconUrl ?? undefined;
  return (
    <Box
      sx={{
        width: 42,
        height: 42,
        flex: "none",
        borderRadius: "11px",
        display: "grid",
        placeItems: "center",
        fontWeight: 700,
        fontSize: 16,
        bgcolor: C.glyphBg,
        border: `1px solid ${configured ? C.accent : C.line}`,
        color: "primary.main",
        boxShadow: configured ? "inset 0 0 0 1px rgba(217,167,67,.18)" : "none",
      }}
    >
      {icon ? (
        <Box
          component="img"
          src={icon}
          alt={coin.symbol || coin.id}
          sx={{ width: 26, height: 26, display: "block" }}
        />
      ) : (
        glyph(coin)
      )}
    </Box>
  );
}
