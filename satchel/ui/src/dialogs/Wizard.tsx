import { useState } from "react";
import {
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
  TextField,
} from "@mui/material";
import SeedForm from "./SeedForm";
import { useT } from "../i18n";

// New-merchant wizard. The create-vs-import choice is made upstream (the merchant
// manager's welcome), so this is just: name the merchant -> provision its seed
// (SeedForm runs the chosen create/import sub-flow). Reached on first run (after
// the empty welcome) and from "Create/Import" in the merchant manager.
//
// NOTHING is created until the final commit (#209): the name step only keeps
// the label in state, and SeedForm's terminal action creates the merchant AND
// imports its seed together. Cancelling anywhere before that leaves no trace —
// no ghost merchant, and the previously active merchant keeps the active slot.
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
  const t = useT();
  const [step, setStep] = useState<Step>("name");
  const [label, setLabel] = useState("");

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
                if (e.key === "Enter") setStep("seed");
              }}
            />
          </DialogContent>
          <DialogActions sx={{ px: 3, pb: 2 }}>
            <Button color="inherit" onClick={onClose} sx={{ mr: "auto" }}>
              {firstRun ? t("wizard.back") : t("common.cancel")}
            </Button>
            <Button variant="contained" onClick={() => setStep("seed")}>
              {t("wizard.continue")}
            </Button>
          </DialogActions>
        </>
      )}

      {step === "seed" && (
        <SeedForm
          mode={mode}
          label={label.trim() || t("merchants.thisMerchant")}
          createLabel={label.trim()}
          onDone={onDone}
          onBack={() => setStep("name")}
        />
      )}
    </Dialog>
  );
}
