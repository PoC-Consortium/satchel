// Public ▸ Post an offer. Lists a signed offer on the noticeboard
// (boardpostoffer) — the public counterpart of Private ▸ Create slip. Reuses the
// shared OfferForm; on success it jumps to the Corkboard where the new notice
// shows up. Posting locks nothing; an offer is just a withdrawable advert.

import { useCallback, useState } from "react";
import { Box, Card, CardContent, Typography } from "@mui/material";
import AddIcon from "@mui/icons-material/Add";
import OfferForm from "../components/OfferForm";
import { errMsg, rpc } from "../api/tauri";
import { useApp } from "../AppContext";
import { useNavigate } from "../ui/nav";
import { useT } from "../i18n";

export default function PostOfferScreen() {
  const { log } = useApp();
  const navigate = useNavigate();
  const t = useT();
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  const submit = useCallback(
    async (give: string, want: string, t1: number, t2: number, protocol?: string) => {
      setBusy(true);
      setErr(null);
      try {
        const params = protocol ? [give, want, t1, t2, protocol] : [give, want, t1, t2];
        const r = await rpc<{ offer_id: string }>("boardpostoffer", params);
        log(`posted offer ${r.offer_id} — withdraw any time; nothing is locked`);
        navigate("board");
      } catch (e) {
        setErr(errMsg(e));
        setBusy(false);
      }
    },
    [log, navigate],
  );

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
