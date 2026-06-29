import { useRef, useState } from "react";
import { Box, Tooltip, Typography } from "@mui/material";
import Identicon from "./Identicon";
import { shortId } from "../identity";
import { useT } from "../i18n";
import { C } from "../theme";
import { useContacts } from "../contacts";
import ContactMenu from "./ContactMenu";
import type { ContactStatus } from "../api/types";

// Status dot colour on the identicon corner: trusted = green, blocked = red,
// neutral = no dot. Purely a local, personal cue (see contacts.tsx).
const STATUS_COLOR: Record<ContactStatus, string | null> = {
  trusted: C.good,
  blocked: C.bad,
  neutral: null,
};

// How the other side of an offer/swap is identified: a derived identicon + a
// truncated fingerprint of the BIP340 pubkey. Both are deterministic from the
// key, so a maker is recognisable across offers and CANNOT be impersonated by a
// chosen name. A local contact nickname, when set, is shown ALONGSIDE the
// fingerprint (never instead of it). Clicking opens the contact menu.
export default function CounterpartyTag({
  id,
  size = 22,
  you,
  interactive = true,
}: {
  id: string | null | undefined;
  size?: number;
  you?: boolean;
  /** When false, render a plain non-clickable tag (e.g. inside the contact
   *  editor, or on the take-confirm summary). Self ("you") is never clickable. */
  interactive?: boolean;
}) {
  const t = useT();
  const { get } = useContacts();
  const ref = useRef<HTMLDivElement>(null);
  const [menuOpen, setMenuOpen] = useState(false);

  const contact = id ? get(id) : undefined;
  const canMenu = interactive && !you && !!id;
  const statusColor = contact ? STATUS_COLOR[contact.status] : null;
  const nick = contact?.nick?.trim();

  return (
    <>
      <Tooltip title={id || t("counterparty.unknown")}>
        <Box
          ref={ref}
          onClick={
            canMenu
              ? (e) => {
                  e.stopPropagation();
                  setMenuOpen(true);
                }
              : undefined
          }
          sx={{
            display: "inline-flex",
            alignItems: "center",
            gap: 0.75,
            minWidth: 0,
            borderRadius: 1,
            cursor: canMenu ? "pointer" : "inherit",
            ...(canMenu ? { px: 0.25, mx: -0.25, "&:hover": { bgcolor: "action.hover" } } : null),
          }}
        >
          <Box sx={{ position: "relative", display: "inline-flex", flex: "none" }}>
            <Identicon id={id} size={size} />
            {statusColor && (
              <Box
                sx={{
                  position: "absolute",
                  right: -2,
                  bottom: -2,
                  width: 9,
                  height: 9,
                  borderRadius: "50%",
                  bgcolor: statusColor,
                  border: "2px solid var(--mui-palette-background-paper)",
                }}
              />
            )}
          </Box>
          {nick ? (
            <Box sx={{ display: "inline-flex", alignItems: "baseline", gap: 0.6, minWidth: 0 }}>
              <Typography sx={{ fontSize: 12.5, fontWeight: 600, color: "text.primary" }} noWrap>
                {nick}
              </Typography>
              <Typography sx={{ fontFamily: C.mono, fontSize: 11, color: "text.disabled" }} noWrap>
                {shortId(id)}
              </Typography>
            </Box>
          ) : (
            <Typography sx={{ fontFamily: C.mono, fontSize: 12, color: "text.secondary" }} noWrap>
              {shortId(id)}
              {you ? ` (${t("counterparty.youShort")})` : ""}
            </Typography>
          )}
        </Box>
      </Tooltip>
      {canMenu && id && (
        <ContactMenu
          id={id}
          anchorEl={ref.current}
          open={menuOpen}
          onClose={() => setMenuOpen(false)}
        />
      )}
    </>
  );
}
