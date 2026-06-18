import { Box, Tooltip } from "@mui/material";
import { useT } from "../i18n";
import { isMainnet } from "../format";

// The phoenix "worn stamp": a loud, unmistakable marker that this client is on
// a non-mainnet network (i.e. NOT real funds). Mainnet shows nothing — the
// absence of a stamp is the signal that funds are real. Per the constraint
// "non-mainnet must be visually unmistakable".
const COLORS: Record<string, string> = {
  testnet: "#d23",
  signet: "#9b59b6",
  regtest: "#2196f3",
};

export default function NetworkStamp({ network }: { network: string | null | undefined }) {
  const t = useT();
  if (!network || isMainnet(network)) return null;

  const color = COLORS[network] ?? "#d23";
  const labelKey = `network.${network}`;
  const label = t(labelKey) === labelKey ? network.toUpperCase() : t(labelKey);

  return (
    <Tooltip title={t("network.notRealFunds", { network: label })}>
      <Box
        sx={{
          display: "inline-block",
          px: 0.9,
          py: 0.15,
          color,
          border: `2px double ${color}`,
          borderRadius: "6px",
          fontFamily: '"Courier New", Courier, monospace',
          fontWeight: 700,
          fontSize: 12,
          letterSpacing: "0.06em",
          textTransform: "uppercase",
          opacity: 0.92,
          userSelect: "none",
          cursor: "help",
        }}
      >
        {label}
      </Box>
    </Tooltip>
  );
}
