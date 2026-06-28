import { Chip, Tooltip } from "@mui/material";
import { useT } from "../i18n";

// The swap-type badge: v2 adaptor is primary-accented "Private (Taproot)", v1
// HTLC is a muted "Standard (HTLC)" chip. Mirrors the Corkboard offer rows so
// the Swaps ledger and active-swaps dock label BOTH protocols, not just v2.
export default function ProtocolChip({
  protocol,
  height = 20,
}: {
  /** Absent ⇒ v1 (pact-htlc-v1); "pact-htlc-v2" = adaptor. */
  protocol?: string;
  height?: number;
}) {
  const t = useT();
  const isV2 = protocol === "pact-htlc-v2";
  return (
    <Tooltip title={isV2 ? t("coins.protoPrivateTip") : t("coins.protoHtlcTip")}>
      <Chip
        size="small"
        variant="outlined"
        label={isV2 ? t("coins.protoPrivate") : t("makeOffer.protoStandard")}
        sx={{
          height,
          cursor: "help",
          ...(isV2
            ? { color: "primary.main", borderColor: "primary.main" }
            : { color: "text.secondary", borderColor: "divider" }),
        }}
      />
    </Tooltip>
  );
}
