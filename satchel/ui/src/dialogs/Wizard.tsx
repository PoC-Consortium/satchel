import { useState } from "react";
import {
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
  TextField,
  Typography,
} from "@mui/material";
import SeedForm from "./SeedForm";
import { createMerchant, errMsg } from "../api/tauri";
import { useApp } from "../AppContext";
import { useT } from "../i18n";

// New-merchant wizard. The create-vs-import choice is made upstream (the merchant
// manager's welcome), so this is just: name the merchant -> provision its seed
// (SeedForm runs the chosen create/import sub-flow). Reached on first run (after
// the empty welcome) and from "Create/Import" in the merchant manager.
type Step = "name" | "seed";

export default function Wizard({
  mode,
  firstRun,
  onClose,
  onDone,
}: {
  mode: "create" | "import";
  firstRun: boolean;
  onClose: () => void;
  onDone: () => void | Promise<void>;
}) {
  const { log } = useApp();
  const t = useT();
  const [step, setStep] = useState<Step>("name");
  const [label, setLabel] = useState("");
  const [createdLabel, setCreatedLabel] = useState(t("merchants.thisMerchant"));
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState("");

  async function createAndContinue() {
    setBusy(true);
    setErr("");
    try {
      const m = await createMerchant(label.trim());
      log(t("log.merchantCreated", { id: m.id }));
      setCreatedLabel(m.label);
      setStep("seed");
    } catch (e) {
      setErr(errMsg(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog open maxWidth="sm" fullWidth disableEscapeKeyDown={firstRun} onClose={firstRun ? undefined : onClose}>
      {step === "name" && (
        <>
          <DialogTitle>
            {mode === "import" ? t("merchants.importMerchant") : t("merchants.newMerchant")}
          </DialogTitle>
          <DialogContent>
            <DialogContentText sx={{ mb: 2 }}>
              {firstRun ? t("merchants.introFirst") : t("merchants.introNew")}
            </DialogContentText>
            <TextField
              label={t("merchants.nameLabel")}
              placeholder={t("merchants.namePlaceholder")}
              value={label}
              onChange={(e) => setLabel(e.target.value)}
              autoFocus
              fullWidth
              onKeyDown={(e) => {
                if (e.key === "Enter" && !busy) void createAndContinue();
              }}
            />
            {err && <Typography sx={{ color: "error.main", fontSize: 13, mt: 1.25 }}>{err}</Typography>}
          </DialogContent>
          <DialogActions sx={{ px: 3, pb: 2 }}>
            <Button color="inherit" onClick={onClose} sx={{ mr: "auto" }}>
              {firstRun ? t("wizard.back") : t("common.cancel")}
            </Button>
            <Button variant="contained" disabled={busy} onClick={() => void createAndContinue()}>
              {t("wizard.continue")}
            </Button>
          </DialogActions>
        </>
      )}

      {step === "seed" && (
        <SeedForm
          mode={mode}
          label={createdLabel}
          onDone={onDone}
          onBack={() => setStep("name")}
        />
      )}
    </Dialog>
  );
}
