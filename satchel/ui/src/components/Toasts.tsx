import { Snackbar } from "@mui/material";
import { useApp } from "../AppContext";

// A transient echo of the latest log line — so an action's outcome (or an
// error) is visible without watching the log panel.
export default function Toasts() {
  const { toast, clearToast } = useApp();
  return (
    <Snackbar
      open={!!toast}
      autoHideDuration={4000}
      onClose={clearToast}
      message={toast ?? ""}
      anchorOrigin={{ vertical: "bottom", horizontal: "right" }}
    />
  );
}
