import { useRef, useState } from "react";
import { IconButton, ListItemIcon, ListItemText, Menu, MenuItem, Tooltip } from "@mui/material";
import LanguageOutlinedIcon from "@mui/icons-material/LanguageOutlined";
import CheckIcon from "@mui/icons-material/Check";
import { LANGUAGES, useI18n, useT } from "../i18n";

// The language picker (a globe button + native-name menu), shared by the header
// toolbar and the first-run floating switcher. Lives in one place so both
// surfaces offer the full shipped LANGUAGES list and stay in lockstep.
//
// `menuZIndex` lifts the dropdown above onboarding dialogs (MUI modal = 1300):
// during first-run the header sits behind a dialog backdrop, so the floating
// instance renders the menu on top of it.
export default function LanguageMenu({
  color = "text.secondary",
  menuZIndex,
}: {
  color?: string;
  menuZIndex?: number;
}) {
  const t = useT();
  const { lang, setLang } = useI18n();
  const ref = useRef<HTMLButtonElement>(null);
  const [open, setOpen] = useState(false);

  return (
    <>
      <Tooltip title={t("header.language")}>
        <IconButton ref={ref} onClick={() => setOpen(true)} sx={{ color }}>
          <LanguageOutlinedIcon />
        </IconButton>
      </Tooltip>
      <Menu
        anchorEl={ref.current}
        open={open}
        onClose={() => setOpen(false)}
        anchorOrigin={{ vertical: "bottom", horizontal: "right" }}
        transformOrigin={{ vertical: "top", horizontal: "right" }}
        sx={menuZIndex ? { zIndex: menuZIndex } : undefined}
      >
        {LANGUAGES.map((l) => (
          <MenuItem
            key={l.code}
            selected={lang === l.code}
            onClick={() => {
              setLang(l.code);
              setOpen(false);
            }}
          >
            <ListItemIcon>
              {lang === l.code ? <CheckIcon fontSize="small" color="primary" /> : null}
            </ListItemIcon>
            <ListItemText>{l.nativeName}</ListItemText>
          </MenuItem>
        ))}
      </Menu>
    </>
  );
}
