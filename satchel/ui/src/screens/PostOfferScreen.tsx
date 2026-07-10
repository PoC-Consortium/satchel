// Public ▸ Post an offer. Lists a signed offer on the noticeboard
// (boardpostoffer) — the public counterpart of Private ▸ Create slip. Reuses the
// shared OfferForm; confirming jumps straight to the Corkboard while the post
// finishes in flight (the new notice shows up there on success; a failure lands
// in the activity log). Posting locks nothing; an offer is a withdrawable advert.

import { useCallback } from "react";
import { Box, Button, Card, CardContent, Typography } from "@mui/material";
import AddIcon from "@mui/icons-material/Add";
import OfferForm from "../components/OfferForm";
import { errMsg, rpc } from "../api/tauri";
import { useApp } from "../AppContext";
import { useNavigate } from "../ui/nav";
import { useT } from "../i18n";
import { EmptyState } from "../components/StatusViews";

export default function PostOfferScreen() {
  const { log, coins, coinsLoaded } = useApp();
  const navigate = useNavigate();
  const t = useT();

  const submit = useCallback(
    async (
      give: string,
      want: string,
      t1: number,
      t2: number,
      protocol?: string,
      ttlSecs?: number,
    ) => {
      // Confirmed — jump straight to the Corkboard and let the post finish in
      // flight (no second wait on the busy form). The board shows the notice
      // only on success; the outcome lands in the activity log either way.
      navigate("board");
      try {
        // protocol (param 4) + ttl_secs (param 5) are both optional; a null at 4
        // lets us set the ttl without forcing a protocol (opt_str ignores null).
        const params = [give, want, t1, t2, protocol ?? null, ttlSecs ?? null];
        const r = await rpc<{ offer_id: string }>("boardpostoffer", params);
        log(t("log.postedOffer", { id: r.offer_id }));
      } catch (e) {
        log(t("log.postOfferError", { err: errMsg(e) }));
      }
    },
    [log, navigate, t],
  );

  // #139: don't decide anything before coins have loaded once — the context
  // starts with an empty array, and gating on it would flash the setup
  // nudge at every first navigation even with coins fully configured.
  if (!coinsLoaded) {
    return null;
  }

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
            busy={false}
            error={null}
            onSubmit={submit}
          />
        </CardContent>
      </Card>
    </Box>
  );
}
