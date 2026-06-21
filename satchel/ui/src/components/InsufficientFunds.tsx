import { Alert } from "@mui/material";
import { useApp } from "../AppContext";
import { useT } from "../i18n";
import { fmtBare } from "../format";
import type { LockFunds } from "../api/tauri";

// The funds gate shown on the make/take decision screens: when the core wallet
// can't cover the leg the user would LOCK (amount + funding fee), this red note
// explains the shortfall and the confirm button is disabled (the engine refuses
// it too — this just surfaces it before the click). Renders nothing when funds
// are sufficient or the check couldn't run.
export default function InsufficientFunds({ check }: { check: LockFunds | null }) {
  const { symOf } = useApp();
  const t = useT();
  if (!check || check.ok) return null;
  const sym = symOf(check.coin);
  return (
    <Alert severity="error" sx={{ py: 0.5, fontSize: 12.5 }}>
      {t("funds.insufficient", {
        sym,
        need: fmtBare(check.needSat),
        have: fmtBare(check.haveSat),
      })}
    </Alert>
  );
}
