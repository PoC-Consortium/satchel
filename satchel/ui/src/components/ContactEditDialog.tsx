import { useEffect, useState } from "react";
import {
  Box,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  TextField,
  Typography,
} from "@mui/material";
import Identicon from "./Identicon";
import { shortId } from "../identity";
import { useContacts } from "../contacts";
import { useT } from "../i18n";
import { C } from "../theme";

// Add/edit one contact's nick + freeform notes. Identity is fixed (the hex
// pubkey) and shown read-only as the identicon + fingerprint — a reminder that
// the nick is just a label on top of the real, spoof-proof identity.
export default function ContactEditDialog({
  id,
  open,
  onClose,
}: {
  id: string;
  open: boolean;
  onClose: () => void;
}) {
  const t = useT();
  const { get, upsert } = useContacts();
  const existing = get(id);
  const [nick, setNick] = useState("");
  const [note, setNote] = useState("");

  // Seed the fields from the current contact each time the dialog opens (or the
  // target identity changes) — not on every `existing` change, so typing isn't
  // clobbered by the write-through round-trip.
  useEffect(() => {
    if (open) {
      setNick(existing?.nick ?? "");
      setNote(existing?.note ?? "");
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, id]);

  const save = () => {
    upsert(id, { nick: nick.trim(), note: note.trim() || undefined });
    onClose();
  };

  return (
    <Dialog open={open} onClose={onClose} maxWidth="xs" fullWidth>
      <DialogTitle>{existing ? t("contacts.editTitle") : t("contacts.addTitle")}</DialogTitle>
      <DialogContent>
        <Box sx={{ display: "flex", flexDirection: "column", gap: 2, pt: 0.5 }}>
          <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
            <Identicon id={id} size={26} />
            <Typography sx={{ fontFamily: C.mono, fontSize: 12, color: "text.secondary" }} noWrap>
              {shortId(id)}
            </Typography>
          </Box>
          <TextField
            label={t("contacts.nickLabel")}
            placeholder={t("contacts.nickPlaceholder")}
            value={nick}
            autoFocus
            size="small"
            slotProps={{ htmlInput: { maxLength: 40 } }}
            onChange={(e) => setNick(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") save();
            }}
          />
          <TextField
            label={t("contacts.noteLabel")}
            placeholder={t("contacts.notePlaceholder")}
            value={note}
            multiline
            minRows={3}
            maxRows={8}
            size="small"
            slotProps={{ htmlInput: { maxLength: 500 } }}
            onChange={(e) => setNote(e.target.value)}
          />
        </Box>
      </DialogContent>
      <DialogActions sx={{ px: 3, pb: 2 }}>
        <Button onClick={onClose} color="inherit">
          {t("contacts.cancel")}
        </Button>
        <Button onClick={save} variant="contained">
          {t("contacts.save")}
        </Button>
      </DialogActions>
    </Dialog>
  );
}
