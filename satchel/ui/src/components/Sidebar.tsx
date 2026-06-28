import {
  Box,
  Drawer,
  IconButton,
  List,
  ListItemButton,
  ListItemIcon,
  ListItemText,
  TextField,
  Tooltip,
  Typography,
  useMediaQuery,
} from "@mui/material";
import { useTheme } from "@mui/material/styles";
import PushPinOutlinedIcon from "@mui/icons-material/PushPinOutlined";
import EditOutlinedIcon from "@mui/icons-material/EditOutlined";
import CheckIcon from "@mui/icons-material/Check";
import CloseIcon from "@mui/icons-material/Close";
import CampaignOutlinedIcon from "@mui/icons-material/CampaignOutlined";
import NoteAddOutlinedIcon from "@mui/icons-material/NoteAddOutlined";
import ListAltOutlinedIcon from "@mui/icons-material/ListAltOutlined";
import MoveToInboxOutlinedIcon from "@mui/icons-material/MoveToInboxOutlined";
import SwapHorizIcon from "@mui/icons-material/SwapHoriz";
import SensorsOutlinedIcon from "@mui/icons-material/SensorsOutlined";
import AccountBalanceWalletOutlinedIcon from "@mui/icons-material/AccountBalanceWalletOutlined";
import SettingsOutlinedIcon from "@mui/icons-material/SettingsOutlined";
import ChevronLeftIcon from "@mui/icons-material/ChevronLeft";
import ArrowUpwardIcon from "@mui/icons-material/ArrowUpward";
import { useState, type ReactNode } from "react";
import StorefrontIcon from "@mui/icons-material/Storefront";
import { useT } from "../i18n";
import { useApp } from "../AppContext";
import { errMsg, renameMerchant } from "../api/tauri";
import { useUpdate } from "../update";
import { useDialogs } from "../ui/dialogs";
import Identicon from "./Identicon";
import { shortId } from "../identity";
import { C } from "../theme";
import logoUrl from "../assets/satchel-logo.svg";

// Primary nav (UI_REQUIREMENTS): two venue groups — PUBLIC (the noticeboard +
// posting to it) and PRIVATE (bilateral off-market slips) — then Swaps (the
// ledger) and Wallets (balances) top-level below them. Coins is configuration →
// it lives under Settings, not here.
export type Route =
  | "board"
  | "post-offer"
  | "private-create"
  | "private-slips"
  | "private-receive"
  | "swaps"
  | "relays"
  | "wallets"
  | "settings";

const WIDTH = 224;

interface NavDef {
  route: Route;
  labelKey: string;
  icon: ReactNode;
}
// PUBLIC group: browse the board, or post a public listing to it.
const PUBLIC_ITEMS: NavDef[] = [
  { route: "board", labelKey: "nav.corkboard", icon: <PushPinOutlinedIcon /> },
  { route: "post-offer", labelKey: "nav.postOffer", icon: <CampaignOutlinedIcon /> },
];
// PRIVATE group (off-market slips): the two actions first (create / take), then
// the review-and-cancel list last.
const PRIVATE_ITEMS: NavDef[] = [
  { route: "private-create", labelKey: "nav.privateCreate", icon: <NoteAddOutlinedIcon /> },
  { route: "private-receive", labelKey: "nav.privateReceive", icon: <MoveToInboxOutlinedIcon /> },
  { route: "private-slips", labelKey: "nav.privateSlips", icon: <ListAltOutlinedIcon /> },
];
// Top-level items below the venue groups.
const ACTIVITY: NavDef[] = [
  { route: "swaps", labelKey: "nav.swaps", icon: <SwapHorizIcon /> },
  { route: "relays", labelKey: "nav.relays", icon: <SensorsOutlinedIcon /> },
  { route: "wallets", labelKey: "nav.wallets", icon: <AccountBalanceWalletOutlinedIcon /> },
];

export default function Sidebar({
  route,
  onNavigate,
  open,
  onToggle,
}: {
  route: Route;
  onNavigate: (r: Route) => void;
  open: boolean;
  onToggle: () => void;
}) {
  const theme = useTheme();
  const narrow = useMediaQuery(theme.breakpoints.down("sm"));
  const t = useT();
  const { activeMerchant, activeId, identity, ready, watchOnly, boot, log } = useApp();
  const { version, showBadge, openDialog } = useUpdate();
  const { openMerchants } = useDialogs();

  const merchantLabel = activeMerchant?.label ?? (activeId ? activeId : t("header.noMerchant"));
  const merchantId = activeMerchant?.identity ?? identity ?? null;

  // Inline rename of the active merchant (UI-5): click the name to edit it in
  // place, no dialog. Only the label changes — id/identity/seed are immutable —
  // so it's safe even mid-swap. Renaming targets the active merchant only (the
  // one the sidebar shows); switching/other merchants stay in the manager.
  const renameId = activeMerchant?.id ?? null;
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState("");
  const [saving, setSaving] = useState(false);

  const startEdit = () => {
    setDraft(activeMerchant?.label ?? "");
    setEditing(true);
  };

  const saveEdit = async () => {
    const name = draft.trim();
    if (!renameId || !name || name === activeMerchant?.label) {
      setEditing(false);
      return;
    }
    setSaving(true);
    try {
      await renameMerchant(renameId, name);
      await boot();
      log(t("log.renamedMerchant", { name }));
      setEditing(false);
    } catch (e) {
      log(t("log.renameMerchantError", { err: errMsg(e) }));
    } finally {
      setSaving(false);
    }
  };

  const go = (r: Route) => {
    onNavigate(r);
    if (narrow) onToggle();
  };

  // Watch-only hides the two posting actions (you can't post or create slips);
  // browsing, taking a slip (display-only), Swaps/Wallets/Settings all stay.
  const publicItems = watchOnly ? PUBLIC_ITEMS.filter((n) => n.route !== "post-offer") : PUBLIC_ITEMS;
  const privateItems = watchOnly
    ? PRIVATE_ITEMS.filter((n) => n.route !== "private-create")
    : PRIVATE_ITEMS;

  const body = (
    <Box sx={{ display: "flex", flexDirection: "column", height: "100%", width: WIDTH }}>
      {/* Drawer header: branding + version (phoenix-style). */}
      <Box
        sx={{
          display: "flex",
          alignItems: "center",
          gap: 1.25,
          height: 64,
          px: 2,
          borderBottom: `1px solid ${C.line}`,
        }}
      >
        <Box
          component="img"
          src={logoUrl}
          alt={t("app.name")}
          sx={{ width: 52, height: 52, flex: "none", display: "block", transform: "translateY(-3px)" }}
        />
        {/* UI-8: bigger logo (46px) + the Montserrat wordmark Phoenix uses
            (20px/700), version smaller (10px). */}
        <Box sx={{ minWidth: 0, flex: 1, display: "flex", flexDirection: "column", justifyContent: "center" }}>
          <Typography
            sx={{
              fontFamily: '"Montserrat", system-ui, sans-serif',
              fontWeight: 700,
              fontSize: 20,
              lineHeight: 1.1,
              color: "primary.main",
            }}
          >
            {t("app.name")}
          </Typography>
          {/* Version + update badge (phoenix AppUpdateService pattern): polls
              GitHub releases; an up-arrow shows when newer. Click → dialog. */}
          <Tooltip title={showBadge ? t("update.badgeTooltip") : t("update.versionTooltip")}>
            <Box
              onClick={openDialog}
              sx={{ display: "inline-flex", alignItems: "center", gap: 0.4, cursor: "pointer" }}
            >
              <Typography sx={{ fontSize: 10, lineHeight: 1.2, color: showBadge ? "primary.main" : "text.secondary" }}>
                v{version}
              </Typography>
              {showBadge && <ArrowUpwardIcon sx={{ fontSize: 13, color: "primary.main" }} />}
            </Box>
          </Tooltip>
        </Box>
        <IconButton size="small" onClick={onToggle} aria-label={t("header.collapseMenu")} sx={{ color: "text.secondary" }}>
          <ChevronLeftIcon fontSize="small" />
        </IconButton>
      </Box>

      {/* Active-merchant area (phoenix wallet-info analog): identicon + name +
          short identity, sitting between the logo header and the nav. Click
          opens the merchant manager. */}
      <Box
        onClick={openMerchants}
        sx={{
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          gap: 0.5,
          px: 2,
          py: 1.5,
          cursor: "pointer",
          borderBottom: `1px solid ${C.line}`,
          "&:hover": { bgcolor: "action.hover" },
        }}
      >
        {/* Active merchant's identicon (centered), click to manage/switch. */}
        {merchantId ? (
          <Identicon id={merchantId} size={34} />
        ) : (
          <StorefrontIcon sx={{ color: "text.secondary", fontSize: 28 }} />
        )}
        <Box sx={{ minWidth: 0, maxWidth: "100%", width: "100%", textAlign: "center" }}>
          {editing ? (
            // Inline edit box: stop the click bubbling so the surrounding box's
            // openMerchants doesn't fire. Enter saves, Esc cancels.
            <Box
              onClick={(e) => e.stopPropagation()}
              sx={{ display: "flex", alignItems: "center", gap: 0.25 }}
            >
              <TextField
                value={draft}
                autoFocus
                size="small"
                disabled={saving}
                placeholder={t("merchants.namePlaceholder")}
                onChange={(e) => setDraft(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") void saveEdit();
                  else if (e.key === "Escape") setEditing(false);
                }}
                slotProps={{ htmlInput: { maxLength: 40, style: { textAlign: "center", fontWeight: 600, fontSize: 13, padding: "4px 6px" } } }}
                sx={{ flex: 1, minWidth: 0 }}
              />
              <IconButton size="small" disabled={saving} aria-label={t("common.save")} onClick={() => void saveEdit()}>
                <CheckIcon sx={{ fontSize: 16 }} />
              </IconButton>
              <IconButton size="small" disabled={saving} aria-label={t("common.cancel")} onClick={() => setEditing(false)}>
                <CloseIcon sx={{ fontSize: 16 }} />
              </IconButton>
            </Box>
          ) : (
            <Box sx={{ display: "inline-flex", alignItems: "center", gap: 0.25, maxWidth: "100%", "&:hover .renamePencil": { opacity: 1 } }}>
              <Typography
                noWrap
                onClick={renameId ? (e) => { e.stopPropagation(); startEdit(); } : undefined}
                sx={{ fontWeight: 600, fontSize: 13, cursor: renameId ? "text" : "inherit" }}
              >
                {merchantLabel}
              </Typography>
              {renameId && (
                <Tooltip title={t("merchants.rename")}>
                  <IconButton
                    size="small"
                    className="renamePencil"
                    aria-label={t("merchants.rename")}
                    onClick={(e) => { e.stopPropagation(); startEdit(); }}
                    sx={{ p: 0.25, opacity: 0, transition: "opacity 0.15s", color: "text.disabled", "&:hover": { color: "primary.main" } }}
                  >
                    <EditOutlinedIcon sx={{ fontSize: 14 }} />
                  </IconButton>
                </Tooltip>
              )}
            </Box>
          )}
          <Typography noWrap sx={{ fontSize: 11, color: "text.secondary", fontFamily: C.mono }}>
            {ready && merchantId ? shortId(merchantId) : t("header.noMerchant")}
          </Typography>
        </Box>
      </Box>

      {/* Primary nav: the PUBLIC + PRIVATE venue groups, then Swaps/Wallets. */}
      <List sx={{ px: 1, pt: 1, flex: 1 }}>
        <SectionLabel>{t("nav.public")}</SectionLabel>
        {publicItems.map((n) => (
          <NavRow key={n.route} label={t(n.labelKey)} icon={n.icon} active={route === n.route} nested onClick={() => go(n.route)} />
        ))}

        <SectionLabel>{t("nav.private")}</SectionLabel>
        {privateItems.map((n) => (
          <NavRow key={n.route} label={t(n.labelKey)} icon={n.icon} active={route === n.route} nested onClick={() => go(n.route)} />
        ))}

        <Box sx={{ height: 8 }} />
        {ACTIVITY.map((n) => (
          <NavRow key={n.route} label={t(n.labelKey)} icon={n.icon} active={route === n.route} onClick={() => go(n.route)} />
        ))}
      </List>

      {/* Footer: Settings (Coins, theme, version, network live in here). */}
      <Box sx={{ px: 1, pb: 1, borderTop: `1px solid ${C.line}`, pt: 1 }}>
        <NavRow
          label={t("nav.settings")}
          icon={<SettingsOutlinedIcon />}
          active={route === "settings"}
          onClick={() => go("settings")}
        />
      </Box>
    </Box>
  );

  // Narrow windows: overlay drawer (phoenix `over` mode). Desktop: in-flow,
  // width-collapsible rail (phoenix `side` mode) so content reflows.
  if (narrow) {
    return (
      <Drawer variant="temporary" open={open} onClose={onToggle} ModalProps={{ keepMounted: true }}>
        {body}
      </Drawer>
    );
  }

  return (
    <Box
      component="nav"
      sx={{
        width: open ? WIDTH : 0,
        flexShrink: 0,
        overflow: "hidden",
        transition: theme.transitions.create("width", { duration: theme.transitions.duration.shorter }),
        bgcolor: "background.paper",
        borderRight: open ? `1px solid ${C.line}` : "none",
      }}
    >
      {body}
    </Box>
  );
}

// Group heading (phoenix-style): a quiet uppercase label above a set of rows.
function SectionLabel({ children }: { children: ReactNode }) {
  return (
    <Typography
      sx={{
        px: 2,
        pt: 1.5,
        pb: 0.5,
        fontSize: 10.5,
        fontWeight: 600,
        textTransform: "uppercase",
        letterSpacing: "0.08em",
        color: "text.disabled",
      }}
    >
      {children}
    </Typography>
  );
}

function NavRow({
  label,
  icon,
  active,
  nested,
  onClick,
}: {
  label: string;
  icon: ReactNode;
  active: boolean;
  nested?: boolean;
  onClick: () => void;
}) {
  return (
    <ListItemButton
      selected={active}
      onClick={onClick}
      sx={{
        borderRadius: 1.5,
        mb: 0.5,
        pl: nested ? 2.5 : 1.5,
        "&.Mui-selected": { bgcolor: "action.selected" },
        "&.Mui-selected .MuiListItemIcon-root": { color: "primary.main" },
      }}
    >
      <ListItemIcon sx={{ minWidth: 34, color: active ? "primary.main" : "text.secondary" }}>
        {icon}
      </ListItemIcon>
      <ListItemText
        primary={label}
        slotProps={{
          primary: {
            fontWeight: active ? 600 : 500,
            color: active ? "text.primary" : "text.secondary",
            noWrap: true,
          },
        }}
      />
    </ListItemButton>
  );
}
