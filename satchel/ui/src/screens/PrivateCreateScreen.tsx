// Private ▸ Create slip. Builds a signed offer that is NEVER posted to a board
// (makeprivateoffer) and shows the resulting `pactoffer1:` slip to copy and hand
// to a friend over their own chat. The friend takes it on Private ▸ Receive.
// Reuses the shared OfferForm; this screen only owns the slip output.

import { useCallback, useState } from "react";
import { Box, Button, Card, CardContent, IconButton, Tooltip, Typography } from "@mui/material";
import AddIcon from "@mui/icons-material/Add";
import ContentCopyIcon from "@mui/icons-material/ContentCopy";
import OfferForm from "../components/OfferForm";
import { errMsg, makePrivateOffer } from "../api/tauri";
import { useApp } from "../AppContext";
import { useNavigate } from "../ui/nav";
import { useT } from "../i18n";
import { C } from "../theme";
import { EmptyState } from "../components/StatusViews";

export default function PrivateCreateScreen() {
  const { log, watchOnly } = useApp();
  const navigate = useNavigate();
  const t = useT();
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [slip, setSlip] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  const submit = useCallback(
    async (
      give: string,
      want: string,
      t1: number,
      t2: number,
      protocol?: string,
      ttlSecs?: number,
    ) => {
      setBusy(true);
      setErr(null);
      try {
        const r = await makePrivateOffer(give, want, t1, t2, protocol, ttlSecs);
        setSlip(r.slip);
        log(t("log.createdSlip"));
      } catch (e) {
        setErr(errMsg(e));
      }
      setBusy(false);
    },
    [log],
  );

  async function copySlip() {
    if (!slip) return;
    try {
      await navigator.clipboard.writeText(slip);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      /* clipboard blocked — the box is selectable as a fallback */
    }
  }

  if (watchOnly) {
    return (
      <EmptyState title={t("watchOnly.postBlockedTitle")}>{t("watchOnly.postBlockedBody")}</EmptyState>
    );
  }

  return (
    <Box sx={{ maxWidth: 460, mx: "auto", textAlign: "center" }}>
      <Typography variant="h2" sx={{ fontSize: 20, fontWeight: 600, mb: 0.5 }}>
        {t("private.createTitle")}
      </Typography>
      <Typography sx={{ color: "text.secondary", fontSize: 13, mb: 2.5 }}>
        {t("private.createIntro")}
      </Typography>

      <Card variant="outlined" sx={{ textAlign: "left" }}>
        <CardContent>
          <OfferForm
            submitLabel={t("makeOffer.makeSlip")}
            submitIcon={<AddIcon />}
            confirmTitle={t("makeOffer.reviewSlipTitle")}
            busy={busy}
            error={err}
            onSubmit={submit}
          />
        </CardContent>
      </Card>

      {/* Slip output: copy box + one-line explainer (PRIVATE_OFFERS.md §6). The
          slip's ttl defaults to ~24h regardless of the swap timelock preset. */}
      {slip && (
        <Card variant="outlined" sx={{ mt: 2.5, textAlign: "left" }}>
          <CardContent sx={{ display: "flex", flexDirection: "column", gap: 1.5 }}>
            <Typography sx={{ fontSize: 13, fontWeight: 600 }}>{t("makeOffer.slipTitle")}</Typography>
            <Box sx={{ display: "flex", alignItems: "flex-start", gap: 1 }}>
              <Box
                sx={{
                  flex: 1,
                  fontFamily: C.mono,
                  fontSize: 12,
                  wordBreak: "break-all",
                  bgcolor: "background.default",
                  border: `1px solid ${C.line}`,
                  borderRadius: 1.5,
                  px: 1.25,
                  py: 1,
                  userSelect: "all",
                }}
              >
                {slip}
              </Box>
              <Tooltip title={copied ? t("makeOffer.copied") : t("makeOffer.copy")}>
                <IconButton size="small" onClick={() => void copySlip()} aria-label={t("makeOffer.copy")}>
                  <ContentCopyIcon fontSize="small" />
                </IconButton>
              </Tooltip>
            </Box>
            <Typography sx={{ fontSize: 12, color: "text.secondary" }}>
              {t("makeOffer.slipExplainer", { ttl: "~24h" })}
            </Typography>
            <Box sx={{ display: "flex", justifyContent: "space-between" }}>
              <Button size="small" color="inherit" onClick={() => navigate("private-slips")}>
                {t("nav.privateSlips")}
              </Button>
              <Button size="small" variant="outlined" color="inherit" onClick={() => setSlip(null)}>
                {t("makeOffer.makeAnother")}
              </Button>
            </Box>
          </CardContent>
        </Card>
      )}
    </Box>
  );
}
