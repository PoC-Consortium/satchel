// Public ▸ Post an offer. Lists a signed offer on the noticeboard
// (boardpostoffer) — the public counterpart of Private ▸ Create slip. Reuses the
// shared OfferForm; on success it jumps to the Corkboard where the new notice
// shows up. Posting locks nothing; an offer is just a withdrawable advert.

import { useCallback, useState } from "react";
import { Box, Button, Card, CardContent, Typography } from "@mui/material";
import AddIcon from "@mui/icons-material/Add";
import OfferForm from "../components/OfferForm";
import { errMsg, rpc } from "../api/tauri";
import { useApp } from "../AppContext";
import { useNavigate } from "../ui/nav";
import { useT } from "../i18n";
import { EmptyState } from "../components/StatusViews";

export default function PostOfferScreen() {
  const { log, coins } = useApp();
  const navigate = useNavigate();
  const t = useT();
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

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
        // protocol (param 4) + ttl_secs (param 5) are both optional; a null at 4
        // lets us set the ttl without forcing a protocol (opt_str ignores null).
        const params = [give, want, t1, t2, protocol ?? null, ttlSecs ?? null];
        const r = await rpc<{ offer_id: string }>("boardpostoffer", params);
        log(t("log.postedOffer", { id: r.offer_id }));
        navigate("board");
      } catch (e) {
        setErr(errMsg(e));
        setBusy(false);
      }
    },
    [log, navigate],
  );

  // Per-action gate (#119): posting needs two connected coins to form a pair.
  // Instead of a hard app-wide wall, this screen shows a soft nudge to set them
  // up (the engine also refuses authoritatively). Gates on CONFIGURED, not live,
  // so a momentarily-down node doesn't hide the form.
  if (coins.filter((c) => c.configured).length < 2) {
    return (
      <EmptyState
        title={t("setup.tradeTitle")}
        action={
          <Button variant="contained" onClick={() => navigate("settings", "coins")}>
            {t("setup.tradeCta")}
          </Button>
        }
      >
        {t("setup.tradeBody")}
      </EmptyState>
    );
  }

  return (
    <Box sx={{ maxWidth: 460, mx: "auto", textAlign: "center" }}>
      <Typography variant="h2" sx={{ fontSize: 20, fontWeight: 600, mb: 0.5 }}>
        {t("makeOffer.title")}
      </Typography>
      <Typography sx={{ color: "text.secondary", fontSize: 13, mb: 2.5 }}>{t("makeOffer.intro")}</Typography>

      <Card variant="outlined" sx={{ textAlign: "left" }}>
        <CardContent>
          <OfferForm
            submitLabel={t("makeOffer.post")}
            submitIcon={<AddIcon />}
            confirmTitle={t("makeOffer.reviewOfferTitle")}
            busy={busy}
            error={err}
            onSubmit={submit}
          />
        </CardContent>
      </Card>
    </Box>
  );
}
