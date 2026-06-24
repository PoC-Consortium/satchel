import { useRef, useState } from "react";
import {
  Box,
  Button,
  IconButton,
  ListItemIcon,
  ListItemText,
  Menu,
  MenuItem,
  Tooltip,
} from "@mui/material";
import LanguageOutlinedIcon from "@mui/icons-material/LanguageOutlined";
import CheckIcon from "@mui/icons-material/Check";
import { LANGUAGES, useI18n, useT } from "../i18n";

// phoenix-pocx breakpoint: the toolbar shows the language NAME on wide screens
// and collapses to a bare globe icon below 1280px. We mirror that exactly.
const WIDE = "@media (min-width:1280px)";

// The language picker. Two presentations share one menu:
//  - default (header toolbar): responsive — current language's nativeName as
//    text on ≥1280px, a globe icon below (phoenix parity).
//  - iconOnly (onboarding dialog corner): always a compact globe icon.
// The dropdown is height-capped and scrolls — there are 26 languages.
//
// `menuZIndex` lifts the dropdown above a dialog backdrop (MUI modal = 1300)
// when the control lives inside an onboarding dialog.
export default function LanguageMenu({
  iconOnly = false,
  menuZIndex,
}: {
  iconOnly?: boolean;
  menuZIndex?: number;
}) {
  const t = useT();
  const { lang, setLang } = useI18n();
  const ref = useRef<HTMLButtonElement>(null);
  const [open, setOpen] = useState(false);
  const current = LANGUAGES.find((l) => l.code === lang) ?? LANGUAGES[0];

  return (
    <>
      <Tooltip title={t("header.language")}>
        {iconOnly ? (
          <IconButton ref={ref} onClick={() => setOpen(true)} sx={{ color: "text.secondary" }}>
            <LanguageOutlinedIcon />
          </IconButton>
        ) : (
          <Button
            ref={ref}
            onClick={() => setOpen(true)}
            sx={{
              color: "text.secondary",
              textTransform: "none",
              fontWeight: 500,
              minWidth: 0,
              px: 1,
              [WIDE]: { px: 1.25 },
            }}
          >
            <Box component="span" sx={{ display: "none", [WIDE]: { display: "inline" } }}>
              {current.nativeName}
            </Box>
            <LanguageOutlinedIcon
              fontSize="small"
              sx={{ display: "inline-flex", [WIDE]: { display: "none" } }}
            />
          </Button>
        )}
      </Tooltip>
      <Menu
        anchorEl={ref.current}
        open={open}
        onClose={() => setOpen(false)}
        anchorOrigin={{ vertical: "bottom", horizontal: "right" }}
        transformOrigin={{ vertical: "top", horizontal: "right" }}
        sx={menuZIndex ? { zIndex: menuZIndex } : undefined}
        // Cap the height so the 26-language list scrolls instead of running off
        // the screen (≈ 9 rows + scrollbar).
        slotProps={{ paper: { sx: { maxHeight: 340 } } }}
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
