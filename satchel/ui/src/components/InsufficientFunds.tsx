import { useEffect, useState } from "react";
import { Alert, Typography } from "@mui/material";
import { useApp } from "../AppContext";
import { useConfirmDisable } from "../ui/ConfirmProvider";
import { useT } from "../i18n";
import { fmtBare } from "../format";
import { assessLockFunds, type LockFunds } from "../api/tauri";

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

// The streaming variant of the funds gate, rendered INSIDE a confirm body so the
// review dialog opens instantly and the chain-touching check (balance + fee
// estimate) resolves in place, like FeePreview. While the check is pending — or
// when it can't be assessed — the confirm button stays ENABLED (the engine
// refuses authoritatively; this gate is the friendly warning, not the
// enforcement); it disables the button only when the check resolves not-ok.
export function LockFundsGate({
  lockCoin,
  otherCoin,
  amountSat,
}: {
  lockCoin: string;
  otherCoin: string;
  amountSat: number;
}) {
  const t = useT();
  const setConfirmDisabled = useConfirmDisable();
  // undefined = still checking; null = couldn't be assessed (never blocks).
  const [funds, setFunds] = useState<LockFunds | null | undefined>(undefined);

  useEffect(() => {
    let alive = true;
    setFunds(undefined);
    setConfirmDisabled(false);
    void assessLockFunds(lockCoin, otherCoin, amountSat).then((f) => {
      if (!alive) return;
      setFunds(f);
      setConfirmDisabled(f ? !f.ok : false);
    });
    return () => {
      alive = false;
    };
  }, [lockCoin, otherCoin, amountSat, setConfirmDisabled]);

  if (funds === undefined) {
    return (
      <Typography sx={{ fontSize: 12, color: "text.secondary", fontStyle: "italic" }}>
        {t("funds.checking")}
      </Typography>
    );
  }
  return <InsufficientFunds check={funds} />;
}
