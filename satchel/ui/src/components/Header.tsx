import { useRef, useState } from "react";
import {
  Box,
  Chip,
  Divider,
  IconButton,
  ListItemIcon,
  ListItemText,
  Menu,
  MenuItem,
  Tooltip,
} from "@mui/material";
import MenuIcon from "@mui/icons-material/Menu";
import ArrowDropDownIcon from "@mui/icons-material/ArrowDropDown";
import StorefrontIcon from "@mui/icons-material/Storefront";
import SettingsOutlinedIcon from "@mui/icons-material/SettingsOutlined";
import ManageAccountsOutlinedIcon from "@mui/icons-material/ManageAccountsOutlined";
import LockOutlinedIcon from "@mui/icons-material/LockOutlined";
import { errMsg, selectMerchant } from "../api/tauri";
import { useApp } from "../AppContext";
import { useDialogs } from "../ui/dialogs";
import { useT } from "../i18n";
import CashrateWidget from "./CashrateWidget";
import LanguageMenu from "./LanguageMenu";
import NetworkStamp from "./NetworkStamp";
import WatchOnlyStamp from "./WatchOnlyStamp";
import StatusIndicators from "./StatusIndicators";
import Identicon from "./Identicon";

// The content-area toolbar (phoenix pattern): context + actions, NOT branding
// (branding moved into the sidenav header). Left: menu toggle (when collapsed)
// + status indicators. Right: network stamp, active-merchant DROPDOWN, settings.
export default function Header({
  navOpen,
  onMenuToggle,
  onSettings,
  onLiveSwaps,
}: {
  navOpen: boolean;
  onMenuToggle: () => void;
  onSettings: () => void;
  onLiveSwaps: () => void;
}) {
  const { activeMerchant, activeId, merchants, identity, network, boot, log } = useApp();
  const { openMerchants } = useDialogs();
  const t = useT();

  // UI-6: the merchant chip opens a phoenix-style wallet menu. Anchor to the
  // chip root (not the clicked sub-element) so the chip arrow + body open the
  // same menu in the same place.
  const chipRef = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);
  const openMenu = () => setOpen(true);
  const closeMenu = () => setOpen(false);

  const label = activeMerchant?.label ?? (activeId ? activeId : t("header.noMerchant"));
  const id = activeMerchant?.identity ?? identity ?? null;

  async function switchTo(mid: string) {
    closeMenu();
    if (mid === activeId) return;
    try {
      await selectMerchant(mid);
      await boot();
      log(t("log.switchedMerchant", { id: mid }));
    } catch (e) {
      // Fund-safety gate (live swap on the current merchant) surfaces here.
      log(t("log.switchMerchantError", { err: errMsg(e) }));
    }
  }

  return (
    <Box
      component="header"
      sx={{
        position: "sticky",
        top: 0,
        zIndex: (th) => th.zIndex.appBar,
        display: "flex",
        alignItems: "center",
        gap: 1.5,
        minHeight: 64,
        px: 2,
        bgcolor: "background.paper",
        borderBottom: 1,
        borderColor: "divider",
      }}
    >
      {!navOpen && (
        <Tooltip title={t("header.openMenu")}>
          <IconButton onClick={onMenuToggle} sx={{ color: "text.secondary" }}>
            <MenuIcon />
          </IconButton>
        </Tooltip>
      )}

      <StatusIndicators onLiveSwaps={onLiveSwaps} />

      {/* Stamps, absolutely centered in the bar (phoenix banner style). The
          watch-only stamp sits on top; the network stamp below it (each renders
          nothing when inapplicable, so this is empty on mainnet + non-watch-only). */}
      <Box
        sx={{
          position: "absolute",
          left: "50%",
          top: "50%",
          transform: "translate(-50%, -50%)",
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          gap: 0.5,
          pointerEvents: "none",
          "& > *": { pointerEvents: "auto" },
        }}
      >
        <WatchOnlyStamp />
        <NetworkStamp network={network} />
      </Box>

      <Box sx={{ ml: "auto", display: "flex", alignItems: "center", gap: 1.5 }}>
        {/* Cashrate chip (issue #56) — context chips grouped left of the
            merchant chip; action icons keep the far-right corner. */}
        <CashrateWidget />

        <Tooltip title={t("header.activeMerchant")}>
          <Chip
            ref={chipRef}
            onClick={openMenu}
            deleteIcon={<ArrowDropDownIcon />}
            onDelete={openMenu}
            variant="outlined"
            avatar={id ? <Identicon id={id} size={22} /> : undefined}
            icon={id ? undefined : <StorefrontIcon />}
            label={
              <Box component="span" sx={{ fontWeight: 600 }}>
                {label}
              </Box>
            }
            sx={{
              bgcolor: "background.default",
              borderColor: "divider",
              "&:hover": { borderColor: "primary.main" },
              "& .MuiChip-avatar": { width: 22, height: 22 },
            }}
          />
        </Tooltip>

        {/* UI-6: phoenix-style merchant dropdown — Manage Merchants… first, a
            divider, then the merchant list to switch between. */}
        <Menu
          anchorEl={chipRef.current}
          open={open}
          onClose={closeMenu}
          anchorOrigin={{ vertical: "bottom", horizontal: "right" }}
          transformOrigin={{ vertical: "top", horizontal: "right" }}
          slotProps={{ paper: { sx: { minWidth: 240, mt: 0.5 } } }}
        >
          <MenuItem
            onClick={() => {
              closeMenu();
              openMerchants();
            }}
          >
            <ListItemIcon>
              <ManageAccountsOutlinedIcon fontSize="small" />
            </ListItemIcon>
            <ListItemText>{t("header.manageMerchants")}</ListItemText>
          </MenuItem>
          {merchants.length > 0 && <Divider />}
          {merchants.map((m) => {
            const isActive = m.id === activeId;
            const showLock = !!m.encrypted && !!m.locked;
            return (
              <MenuItem key={m.id} selected={isActive} onClick={() => void switchTo(m.id)}>
                {/* No check on the active row — the `selected` highlight is
                    enough; show each merchant's identicon instead. */}
                <ListItemIcon>
                  <Identicon id={m.identity} size={20} />
                </ListItemIcon>
                <ListItemText
                  primary={m.label}
                  slotProps={{ primary: { noWrap: true, fontWeight: isActive ? 600 : 500 } }}
                />
                {showLock && (
                  <Tooltip title={t("merchants.lockedTip")}>
                    <LockOutlinedIcon fontSize="small" sx={{ ml: 1, color: "text.secondary" }} />
                  </Tooltip>
                )}
              </MenuItem>
            );
          })}
        </Menu>

        <Tooltip title={t("header.settings")}>
          <IconButton onClick={onSettings} sx={{ color: "text.secondary" }}>
            <SettingsOutlinedIcon />
          </IconButton>
        </Tooltip>

        {/* UI-8: language selector (phoenix parity), next to settings. */}
        <LanguageMenu />
      </Box>
    </Box>
  );
}
