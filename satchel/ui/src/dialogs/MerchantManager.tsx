import { useState } from "react";
import {
  Alert,
  Box,
  Button,
  Chip,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
  Paper,
  Stack,
  Tooltip,
  Typography,
} from "@mui/material";
import LockOutlinedIcon from "@mui/icons-material/LockOutlined";
import { errMsg, selectMerchant } from "../api/tauri";
import { useApp } from "../AppContext";
import { useT } from "../i18n";
import Identicon from "../components/Identicon";
import DialogLanguageMenu from "../components/DialogLanguageMenu";
import { shortId } from "../identity";
import { C } from "../theme";
import type { Merchant } from "../api/types";

// UI-5: phoenix wallet-selector parity. Pick a merchant by CLICKING its row,
// then act via buttons — Create new (left) · Import (left/secondary) · Load
// Merchant (right, primary). Per-row loaded/active state + a lock chip only for
// encrypted+locked merchants. Model (C10): ONE merchant loaded at a time, so
// "Load" = switch active; pactd gates the switch if the current merchant has a
// live swap (surfaced as the thrown error).
export default function MerchantManager({
  onClose,
  onNewMerchant,
  firstRun,
}: {
  /** Omitted on first run — the welcome screen can't be dismissed. */
  onClose?: () => void;
  onNewMerchant: (mode: "create" | "import") => void;
  firstRun?: boolean;
}) {
  const { merchants, activeId, boot, log } = useApp();
  const t = useT();
  const [switching, setSwitching] = useState(false);
  const [selected, setSelected] = useState<string | null>(activeId);
  const [error, setError] = useState<string | null>(null);

  async function loadMerchant() {
    if (!selected) return;
    setError(null);
    setSwitching(true);
    try {
      await selectMerchant(selected);
      await boot();
      log(t("log.switchedMerchant", { id: selected }));
      onClose?.();
    } catch (e) {
      // Most commonly the fund-safety gate: pactd refuses to switch away from a
      // merchant with a live swap. Surface the thrown message in-dialog.
      setSwitching(false);
      setError(errMsg(e));
      log(t("log.loadMerchantError", { err: errMsg(e) }));
    }
  }

  if (switching) {
    return (
      <Dialog open maxWidth="sm" fullWidth disableEscapeKeyDown>
        <DialogTitle>{t("merchants.switching")}</DialogTitle>
        <DialogContent>
          <DialogContentText>{t("merchants.switchingBody")}</DialogContentText>
        </DialogContent>
      </Dialog>
    );
  }

  const selectedIsActive = selected != null && selected === activeId;

  return (
    <Dialog
      open
      onClose={firstRun ? undefined : onClose}
      maxWidth="sm"
      fullWidth
      disableEscapeKeyDown={firstRun}
      slotProps={{ paper: { sx: { position: "relative" } } }}
    >
      {/* First-run only: a language switcher in the welcome dialog's corner, so
          a new user can pick their language before stepping through setup (the
          header's picker is behind this dialog's backdrop). */}
      {firstRun && <DialogLanguageMenu />}
      <DialogTitle>{firstRun ? t("merchants.welcomeTitle") : t("merchants.title")}</DialogTitle>
      <DialogContent>
        <DialogContentText sx={{ mb: 2 }}>
          {firstRun ? t("merchants.welcomeIntro") : t("merchants.intro")}
        </DialogContentText>
        {firstRun && (
          <Alert severity="warning" variant="outlined" sx={{ mb: 2, fontSize: 12.5 }}>
            <Typography sx={{ fontWeight: 600, fontSize: 13 }}>{t("disclaimer.title")}</Typography>
            <Typography sx={{ fontSize: 12.5 }}>{t("disclaimer.body")}</Typography>
          </Alert>
        )}
        {merchants.length === 0 ? (
          <Typography sx={{ color: "text.secondary", fontSize: 13 }}>{t("merchants.none")}</Typography>
        ) : (
          <Stack spacing={1}>
            {merchants.map((m) => (
              <MerchantRow
                key={m.id}
                merchant={m}
                active={m.id === activeId}
                selected={m.id === selected}
                onSelect={() => {
                  setSelected(m.id);
                  setError(null);
                }}
              />
            ))}
          </Stack>
        )}
        {error && (
          <Typography sx={{ color: "error.main", fontSize: 13, mt: 1.5, whiteSpace: "pre-wrap" }}>
            {error}
          </Typography>
        )}
      </DialogContent>
      <DialogActions sx={{ px: 3, pb: 2, justifyContent: "space-between" }}>
        {/* Left cluster: create / import — distinct paths into the wizard
            (it names the merchant, then provisions the matching seed flow). */}
        <Box sx={{ display: "flex", gap: 1 }}>
          <Button variant="outlined" color="inherit" onClick={() => onNewMerchant("create")}>
            + {t("merchants.createNew")}
          </Button>
          <Button variant="text" color="inherit" onClick={() => onNewMerchant("import")}>
            {t("merchants.import")}
          </Button>
        </Box>
        {/* Right cluster: close + load the selected merchant. On first run there's
            nothing to close or load yet (Load greyed until a merchant exists). */}
        <Box sx={{ display: "flex", gap: 1 }}>
          {!firstRun && (
            <Button color="inherit" onClick={onClose}>
              {t("merchants.close")}
            </Button>
          )}
          <Button
            variant="contained"
            disabled={!selected || selectedIsActive}
            onClick={() => void loadMerchant()}
          >
            {t("merchants.load")}
          </Button>
        </Box>
      </DialogActions>
    </Dialog>
  );
}

function MerchantRow({
  merchant,
  active,
  selected,
  onSelect,
}: {
  merchant: Merchant;
  active: boolean;
  selected: boolean;
  onSelect: () => void;
}) {
  const t = useT();
  const m = merchant;
  // Lock chip ONLY for an encrypted merchant whose seed is still locked.
  const showLock = !!m.encrypted && !!m.locked;
  return (
    <Paper
      variant="outlined"
      onClick={onSelect}
      sx={{
        display: "flex",
        alignItems: "center",
        gap: 1.5,
        p: 1.25,
        cursor: "pointer",
        borderColor: selected ? "primary.main" : "divider",
        bgcolor: selected ? "action.selected" : "transparent",
        "&:hover": { borderColor: "primary.main" },
      }}
    >
      <Identicon id={m.identity} size={32} />
      <Box sx={{ flex: 1, minWidth: 0 }}>
        <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
          <Typography sx={{ fontWeight: 600 }} noWrap>
            {m.label}
          </Typography>
          {/* Single loaded/active badge — "active" and the old "in use" were the
              same boolean shown twice. */}
          {active && (
            <Chip
              size="small"
              label={t("merchants.loaded")}
              color="primary"
              variant="outlined"
              sx={{ height: 20 }}
            />
          )}
          {showLock && (
            <Tooltip title={t("merchants.lockedTip")}>
              <Chip
                size="small"
                icon={<LockOutlinedIcon sx={{ fontSize: 13 }} />}
                label={t("merchants.locked")}
                variant="outlined"
                sx={{ height: 20 }}
              />
            </Tooltip>
          )}
        </Box>
        <Box sx={{ display: "flex", alignItems: "center", gap: 1, mt: 0.4 }}>
          {/* Raw data-dir id demoted to a small copyable detail. */}
          <Tooltip title={`${t("merchants.idLabel")}: ${m.id}${m.identity ? ` · ${m.identity}` : ""}`}>
            <Typography
              sx={{ color: "text.secondary", fontSize: 11, fontFamily: C.mono, userSelect: "all" }}
              onClick={(e) => e.stopPropagation()}
            >
              {m.identity ? shortId(m.identity) : m.id}
            </Typography>
          </Tooltip>
        </Box>
      </Box>
    </Paper>
  );
}
