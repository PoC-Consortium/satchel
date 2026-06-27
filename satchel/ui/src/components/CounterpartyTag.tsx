import { Box, Tooltip, Typography } from "@mui/material";
import Identicon from "./Identicon";
import { shortId } from "../identity";
import { useT } from "../i18n";
import { C } from "../theme";

// How the other side of an offer is identified: a derived identicon + a
// truncated fingerprint of the BIP340 pubkey. Both are deterministic from the
// key, so a maker is recognisable across offers and cannot be impersonated by a
// chosen name. Full key in the tooltip.
export default function CounterpartyTag({
  id,
  size = 22,
  you,
}: {
  id: string | null | undefined;
  size?: number;
  you?: boolean;
}) {
  const t = useT();
  return (
    <Tooltip title={id || t("counterparty.unknown")}>
      <Box sx={{ display: "inline-flex", alignItems: "center", gap: 0.75, minWidth: 0 }}>
        <Identicon id={id} size={size} />
        <Typography sx={{ fontFamily: C.mono, fontSize: 12, color: "text.secondary" }} noWrap>
          {shortId(id)}
          {you ? ` (${t("counterparty.youShort")})` : ""}
        </Typography>
      </Box>
    </Tooltip>
  );
}
