// Private ▸ Receive a slip. The taker entry for a private (off-market) offer
// (PRIVATE_OFFERS.md §6): a friend sends a `pactoffer1:` slip over their own
// chat; paste it here. On submit we decode it locally (for the confirm card),
// route through the SAME take-confirmation dialog the Corkboard uses, then call
// `takeoffer` — pactd re-decodes and verifies the signature authoritatively.
// From there the swap is indistinguishable from a board take.

import { useMemo, useState } from "react";
import { Box, Button, Card, CardContent, TextField, Typography } from "@mui/material";
import { errMsg, takeOffer } from "../api/tauri";
import { useApp } from "../AppContext";
import { useT } from "../i18n";
import { useTakeConfirm } from "../hooks/useTakeConfirm";
import { decodeSlipForDisplay, looksLikeSlip } from "../format-slip";

export default function PrivateReceiveScreen() {
  const { log, coins, symOf } = useApp();
  const t = useT();
  const confirmTake = useTakeConfirm();
  const [slip, setSlip] = useState("");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [done, setDone] = useState(false);

  const valid = looksLikeSlip(slip);
  // Decode for DISPLAY only (no signature check) — lets anyone inspect a slip's
  // terms even before (or without) taking it.
  const preview = useMemo(() => (valid ? decodeSlipForDisplay(slip) : null), [valid, slip]);
  // Per-action gate (#119): both legs of the pasted slip must be connected + live
  // to take it (the engine enforces too; this is the friendly up-front block).
  const coinLive = (id?: string) => !!coins.find((c) => c.id === id && c.status === "ok");
  const pairReady = !!preview && coinLive(preview.body.give_asset) && coinLive(preview.body.get_asset);

  async function submit() {
    setErr(null);
    setDone(false);
    const decoded = decodeSlipForDisplay(slip);
    if (!decoded) {
      setErr(t("takeSlip.invalid"));
      return;
    }
    // Same confirmation card as a board take (amounts, maker-funds-first, cost).
    // The slip carries the maker id, so the summary can name the counterparty.
    const ok = await confirmTake(decoded.body, { from: decoded.from });
    if (!ok) return;
    setBusy(true);
    try {
      // pactd is the authority: it re-decodes, verifies the BIP340 signature,
      // checks expiry + pair support, then relays the take to the maker.
      await takeOffer(slip);
      log(t("log.tookPrivateOffer", { id: decoded.swap_id }));
      setSlip("");
      setDone(true);
      setBusy(false);
    } catch (e) {
      setErr(errMsg(e));
      setBusy(false);
    }
  }

  return (
    <Box sx={{ maxWidth: 560, mx: "auto", textAlign: "center" }}>
      <Typography variant="h2" sx={{ fontSize: 20, fontWeight: 600, mb: 0.5 }}>
        {t("private.receiveTitle")}
      </Typography>
      <Typography sx={{ color: "text.secondary", fontSize: 13, mb: 2.5 }}>{t("takeSlip.intro")}</Typography>

      <Card variant="outlined" sx={{ textAlign: "left" }}>
        <CardContent sx={{ display: "flex", flexDirection: "column", gap: 2 }}>
          <TextField
            autoFocus
            fullWidth
            multiline
            minRows={3}
            value={slip}
            onChange={(e) => {
              setSlip(e.target.value);
              setDone(false);
            }}
            placeholder={t("takeSlip.placeholder")}
            slotProps={{ htmlInput: { style: { fontFamily: "monospace", fontSize: 12 } } }}
          />
          {/* Display-only decode of a pasted slip — its terms, before (and even
              without) taking it. The signature is verified authoritatively by
              the engine only on take. */}
          {preview && (
            <Box sx={{ fontSize: 13, bgcolor: "action.hover", borderRadius: 1, px: 1.25, py: 1 }}>
              <Typography sx={{ fontSize: 12, color: "text.secondary", mb: 0.25 }}>
                {t("takeSlip.previewLabel")}
              </Typography>
              <Typography sx={{ fontFamily: "monospace", fontSize: 13 }}>
                {preview.body.get_amount} {symOf(preview.body.get_asset)} → {preview.body.give_amount}{" "}
                {symOf(preview.body.give_asset)}
              </Typography>
            </Box>
          )}
          {err && (
            <Typography sx={{ color: "error.main", fontSize: 13, whiteSpace: "pre-wrap" }}>{err}</Typography>
          )}
          {done && <Typography sx={{ color: "primary.main", fontSize: 13 }}>{t("private.received")}</Typography>}
          {preview && !pairReady && (
            <Typography sx={{ color: "text.secondary", fontSize: 13 }}>{t("setup.takeNeedsCoins")}</Typography>
          )}
          <Box sx={{ display: "flex", justifyContent: "flex-end" }}>
            <Button variant="contained" disabled={!valid || busy || !pairReady} onClick={() => void submit()}>
              {t("takeSlip.take")}
            </Button>
          </Box>
        </CardContent>
      </Card>
    </Box>
  );
}
