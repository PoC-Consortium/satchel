import { Dialog } from "@mui/material";
import SeedForm from "./SeedForm";

// Resume gate: a merchant is active but its seed isn't provisioned yet
// (info.seed_exists === false). Same SeedForm as the wizard's last step.
export default function SeedProvision({
  label,
  onDone,
  onLater,
}: {
  label: string;
  onDone: () => void | Promise<void>;
  onLater: () => void;
}) {
  return (
    <Dialog open maxWidth="sm" fullWidth>
      <SeedForm label={label} onDone={onDone} onLater={onLater} />
    </Dialog>
  );
}
