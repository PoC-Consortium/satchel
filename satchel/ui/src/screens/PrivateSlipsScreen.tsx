// Private ▸ My slips. The maker's outstanding private offers (listprivateoffers)
// with their pair, amounts, expiry countdown, and a Cancel action
// (cancelprivateoffer) to stop honoring a slip before its ttl lapses. Polls so a
// slip that expires (or is taken) updates without a manual refresh.

import { useCallback, useEffect, useState } from "react";
import { Box, Button, Card, CardContent, Chip, Tooltip, Typography } from "@mui/material";
import { cancelPrivateOffer, errMsg, listPrivateOffers } from "../api/tauri";
import { useApp } from "../AppContext";
import { useNavigate } from "../ui/nav";
import { useT } from "../i18n";
import { fmtBare, until } from "../format";
import { C } from "../theme";
import { EmptyState } from "../components/StatusViews";
import type { PrivateOffer } from "../api/types";

export default function PrivateSlipsScreen() {
  const { symOf, log } = useApp();
  const navigate = useNavigate();
  const t = useT();
  const [offers, setOffers] = useState<PrivateOffer[]>([]);
  const [loaded, setLoaded] = useState(false);

  const refresh = useCallback(async () => {
    try {
      setOffers((await listPrivateOffers()).offers || []);
    } catch {
      /* no merchant / pactd down — leave the list as-is */
    }
    setLoaded(true);
  }, []);

  useEffect(() => {
    void refresh();
    const id = setInterval(() => void refresh(), 12000);
    return () => clearInterval(id);
  }, [refresh]);

  async function cancel(offerId: string) {
    try {
      await cancelPrivateOffer(offerId);
      log(`cancelled private offer ${offerId}`);
    } catch (e) {
      log("cancel: " + errMsg(e));
    }
    void refresh();
  }

  if (!loaded) return null;

  return (
    <Box sx={{ maxWidth: 620, mx: "auto" }}>
      <Typography variant="h2" sx={{ fontSize: 20, fontWeight: 600, mb: 0.5 }}>
        {t("makeOffer.myPrivateTitle")}
      </Typography>
      <Typography sx={{ color: "text.secondary", fontSize: 13, mb: 2.5 }}>{t("private.slipsIntro")}</Typography>

      {offers.length === 0 ? (
        <EmptyState
          title={t("makeOffer.myPrivateEmpty")}
          action={
            <Button variant="contained" onClick={() => navigate("private-create")}>
              {t("nav.privateCreate")}
            </Button>
          }
        >
          {t("private.slipsEmptyBody")}
        </EmptyState>
      ) : (
        <Box sx={{ display: "flex", flexDirection: "column", gap: 1 }}>
          {offers.map((o) => (
            <Card key={o.offer_id} variant="outlined">
              <CardContent
                sx={{ display: "flex", alignItems: "center", gap: 1.5, py: 1.25, "&:last-child": { pb: 1.25 } }}
              >
                <Box sx={{ minWidth: 0, flex: 1 }}>
                  <Typography sx={{ fontFamily: C.mono, fontSize: 13 }}>
                    {fmtBare(o.give_amount)} {symOf(o.give_asset)}
                    <Box component="span" sx={{ color: "text.secondary" }}>
                      {" → "}
                    </Box>
                    {fmtBare(o.get_amount)} {symOf(o.get_asset)}
                  </Typography>
                  <Typography sx={{ fontSize: 11, color: "text.secondary" }}>
                    {o.expired
                      ? t("makeOffer.privateExpired")
                      : t("makeOffer.privateExpires", { when: until(o.expiry) })}
                  </Typography>
                </Box>
                {o.expired && (
                  <Chip size="small" variant="outlined" label={t("makeOffer.privateExpired")} sx={{ height: 20 }} />
                )}
                <Tooltip title={t("makeOffer.cancelTip")}>
                  <Button size="small" variant="outlined" color="inherit" onClick={() => void cancel(o.offer_id)}>
                    {t("makeOffer.cancel")}
                  </Button>
                </Tooltip>
              </CardContent>
            </Card>
          ))}
        </Box>
      )}
    </Box>
  );
}
