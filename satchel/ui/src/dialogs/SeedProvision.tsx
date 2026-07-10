import { Alert, Dialog } from "@mui/material";
import SeedForm from "./SeedForm";
import { useT } from "../i18n";

// Resume gate: a merchant is active but its seed isn't provisioned yet
// (info.seed_exists === false). Same SeedForm as the wizard's last step.
//
// `reimport` (#133) is the keyring-recovery variant of the same gate: the seed
// EXISTS but this machine's OS-keystore key can no longer decrypt it (data dir
// moved / keychain reset). Import-only — re-entering the recovery phrase is
// the one seed overwrite the engine permits in this state, and it re-provisions
// the wrap under a fresh machine key.
export default function SeedProvision({
  label,
  reimport,
  onDone,
  onLater,
}: {
  label: string;
  reimport?: boolean;
  onDone: () => void | Promise<void>;
  onLater?: () => void;
}) {
  const t = useT();
  return (
    <Dialog open maxWidth="sm" fullWidth>
      {reimport && (
        <Alert severity="warning" sx={{ mx: 3, mt: 2, borderRadius: 1.5 }}>
          {t("seed.reimportBanner")}
        </Alert>
      )}
      <SeedForm
        label={label}
        mode={reimport ? "import" : undefined}
        onDone={onDone}
        onLater={onLater}
      />
    </Dialog>
  );
}
