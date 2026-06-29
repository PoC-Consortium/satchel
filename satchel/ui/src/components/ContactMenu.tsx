import { useState, type ReactNode } from "react";
import {
  Divider,
  ListItemIcon,
  ListItemText,
  Menu,
  MenuItem,
} from "@mui/material";
import PersonAddAlt1OutlinedIcon from "@mui/icons-material/PersonAddAlt1Outlined";
import EditOutlinedIcon from "@mui/icons-material/EditOutlined";
import VerifiedUserOutlinedIcon from "@mui/icons-material/VerifiedUserOutlined";
import PersonOutlineIcon from "@mui/icons-material/PersonOutline";
import BlockIcon from "@mui/icons-material/Block";
import ContentCopyOutlinedIcon from "@mui/icons-material/ContentCopyOutlined";
import LaunchOutlinedIcon from "@mui/icons-material/LaunchOutlined";
import CheckIcon from "@mui/icons-material/Check";
import { useContacts } from "../contacts";
import { useApp } from "../AppContext";
import { useT } from "../i18n";
import { useNavigate } from "../ui/nav";
import { C } from "../theme";
import ContactEditDialog from "./ContactEditDialog";
import type { ContactStatus } from "../api/types";

// The three standings, as data (not inline JSX) so the i18n-key strings live in
// plain code — the no-literal-string lint only exempts literals passed to t().
const STATUS_OPTIONS: { s: ContactStatus; labelKey: string; icon: ReactNode }[] = [
  {
    s: "trusted",
    labelKey: "contacts.menuMarkTrusted",
    icon: <VerifiedUserOutlinedIcon fontSize="small" sx={{ color: C.good }} />,
  },
  { s: "neutral", labelKey: "contacts.menuMarkNeutral", icon: <PersonOutlineIcon fontSize="small" /> },
  {
    s: "blocked",
    labelKey: "contacts.menuMarkBlocked",
    icon: <BlockIcon fontSize="small" sx={{ color: C.bad }} />,
  },
];

// The click menu on a counterparty's identicon/tag (CounterpartyTag). Add/edit a
// nickname, set the trusted/neutral/blocked standing, copy the full key, or jump
// to the Contacts tab. Rendered (closed) alongside every interactive tag, so the
// edit dialog it owns survives the menu closing.
export default function ContactMenu({
  id,
  anchorEl,
  open,
  onClose,
}: {
  id: string;
  anchorEl: HTMLElement | null;
  open: boolean;
  onClose: () => void;
}) {
  const t = useT();
  const { get, setStatus } = useContacts();
  const { showToast } = useApp();
  const navigate = useNavigate();
  const [editOpen, setEditOpen] = useState(false);

  const contact = get(id);
  const status: ContactStatus = contact?.status ?? "neutral";

  const copyKey = async () => {
    try {
      await navigator.clipboard.writeText(id);
      showToast(t("contacts.keyCopied"));
    } catch {
      /* clipboard blocked — the full key is in the tag tooltip */
    }
    onClose();
  };

  const mark = (s: ContactStatus) => {
    setStatus(id, s);
    onClose();
  };

  return (
    <>
      <Menu
        anchorEl={anchorEl}
        open={open}
        onClose={onClose}
        anchorOrigin={{ vertical: "bottom", horizontal: "left" }}
        transformOrigin={{ vertical: "top", horizontal: "left" }}
      >
        <MenuItem
          onClick={() => {
            setEditOpen(true);
            onClose();
          }}
        >
          <ListItemIcon>
            {contact ? (
              <EditOutlinedIcon fontSize="small" />
            ) : (
              <PersonAddAlt1OutlinedIcon fontSize="small" />
            )}
          </ListItemIcon>
          <ListItemText>{contact ? t("contacts.menuEdit") : t("contacts.menuAdd")}</ListItemText>
        </MenuItem>
        <Divider />
        {STATUS_OPTIONS.map((opt) => (
          <MenuItem key={opt.s} selected={status === opt.s} onClick={() => mark(opt.s)}>
            <ListItemIcon>
              {status === opt.s ? <CheckIcon fontSize="small" /> : opt.icon}
            </ListItemIcon>
            <ListItemText>{t(opt.labelKey)}</ListItemText>
          </MenuItem>
        ))}
        <Divider />
        <MenuItem onClick={copyKey}>
          <ListItemIcon>
            <ContentCopyOutlinedIcon fontSize="small" />
          </ListItemIcon>
          <ListItemText>{t("contacts.menuCopyKey")}</ListItemText>
        </MenuItem>
        <MenuItem
          onClick={() => {
            navigate("contacts");
            onClose();
          }}
        >
          <ListItemIcon>
            <LaunchOutlinedIcon fontSize="small" />
          </ListItemIcon>
          <ListItemText>{t("contacts.menuOpen")}</ListItemText>
        </MenuItem>
      </Menu>
      <ContactEditDialog id={id} open={editOpen} onClose={() => setEditOpen(false)} />
    </>
  );
}
