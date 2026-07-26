// Revive-offers dialog (revoke-on-open lifecycle). On boot the engine firmly
// de-lists every offer that was still `live` — boot-time state is unknowable
// (clean close / crash / a foreign withdrawal while offline), so it never
// guesses and re-advertises. Those rows land in state `revoked_on_open`, and
// this dialog — once per merchant per session, when the merchant is ready —
// offers them back: Revive re-posts the SAME terms as a FRESH offer (new id,
// full ttl — which also heals stale viewer-side tombstones by construction)
// and settles the old row via `offerdismiss`; Dismiss settles it without
// re-posting.
//
// The dialog RESOLVES every row it lists: whatever isn't revived is dismissed
// — per-offer, or in bulk when the dialog closes — so nothing stays in
// `revoked_on_open` once it ends. (If the app dies mid-dialog, the leftovers
// simply reappear on the next boot.) Rows whose pair involves a coin that is
// no longer configured are auto-dismissed up front with a log line — they
// cannot be revived on a nonexistent pair.
//
// Timing: pactd retires on its first scheduler tick after the merchant (and
// seed) are ready, which can land up to a tick AFTER the UI reaches "ready" —
// so we poll `listmyoffers` briefly instead of sampling once, and latch the
// once-per-session flag when the probe actually finds parked rows.

import { useEffect, useState } from "react";
import {
  Box,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
  Typography,
} from "@mui/material";
import { useApp } from "../AppContext";
import { useT } from "../i18n";
import { errMsg, rpc } from "../api/tauri";
import { ago, fmtBare } from "../format";
import { C } from "../theme";
import ProtocolChip from "./ProtocolChip";
import type { Offer } from "../api/types";

// One row from pactd `listmyoffers` (a BARE array). `offer` is the stored
// signed envelope, Offer-shaped — its body carries the original terms.
interface MyOfferRow {
  offer_id: string;
  offer: Offer;
  state: string;
  created: number;
}

// How long to keep looking for boot-retired offers after the merchant is
// ready: pactd's retire pass runs on its scheduler tick (default 30s), so a
// few polls bridge the gap without polling forever in sessions with nothing
// to revive.
const POLL_MS = 5000;
const MAX_POLLS = 24; // ~2 minutes

// Once per merchant per app session — module-level, like the board snapshot.
const shownFor = new Set<string>();

export default function ReviveOffersDialog() {
  const { phase, identity, symOf, log, pokeBoard, coins, coinsLoaded } = useApp();
  const t = useT();
  const [rows, setRows] = useState<MyOfferRow[]>([]);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    // coinsLoaded gates the dead-pair filter below: an empty not-yet-loaded
    // coin list would read as "every pair is dead" and dismiss everything.
    if (phase !== "ready" || !identity || !coinsLoaded || shownFor.has(identity)) return;
    const configured = new Set(coins.filter((c) => c.configured).map((c) => c.id));
    let stopped = false;
    let polls = 0;
    const probe = async () => {
      if (stopped || shownFor.has(identity)) return;
      try {
        const mine = (await rpc<MyOfferRow[]>("listmyoffers")) || [];
        const parked = mine.filter((m) => m.state === "revoked_on_open" && !!m.offer?.body);
        if (parked.length > 0 && !stopped) {
          shownFor.add(identity); // latch once the parked rows are being handled
          // Dead-pair filter: a parked offer on a coin that is no longer
          // configured cannot be revived — settle it right away, never show it.
          const revivable: MyOfferRow[] = [];
          for (const m of parked) {
            const b = m.offer.body;
            if (configured.has(b.give_asset) && configured.has(b.get_asset)) {
              revivable.push(m);
              continue;
            }
            try {
              await rpc("offerdismiss", [m.offer_id]);
              log(t("log.offerDismissedDeadPair", { id: m.offer_id }));
            } catch (e) {
              log(t("log.dismissError", { id: m.offer_id, err: errMsg(e) }));
            }
          }
          if (!stopped) setRows(revivable);
          return;
        }
      } catch {
        /* older pactd / no merchant context — keep trying within the window */
      }
      polls += 1;
      if (polls < MAX_POLLS && !stopped) setTimeout(() => void probe(), POLL_MS);
    };
    void probe();
    return () => {
      stopped = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [phase, identity, coinsLoaded]);

  /** Re-post the stored terms as a fresh offer, then settle the old row. */
  async function reviveOne(m: MyOfferRow): Promise<boolean> {
    const b = m.offer.body;
    try {
      // Same terms, FRESH post: a new id and the full original ttl (pactd
      // stamps `created` now). protocol/ttl at 4/5 are optional — null keeps
      // the engine defaults, mirroring PostOfferScreen.
      const params = [
        `${b.give_asset}:${fmtBare(b.give_amount)}`,
        `${b.get_asset}:${fmtBare(b.get_amount)}`,
        b.t1_secs,
        b.t2_secs,
        b.protocol ?? null,
        b.ttl_secs ?? null,
      ];
      const r = await rpc<{ offer_id: string }>("boardpostoffer", params);
      log(t("log.offerRevived", { old: m.offer_id, id: r.offer_id }));
      pokeBoard(); // surface the fresh listing on the Corkboard now
      try {
        await rpc("offerdismiss", [m.offer_id]); // settle the retired row
      } catch (e) {
        log(t("log.dismissError", { id: m.offer_id, err: errMsg(e) }));
      }
      return true;
    } catch (e) {
      log(t("log.reviveError", { id: m.offer_id, err: errMsg(e) }));
      return false;
    }
  }

  /** Settle the retired row as plain `revoked` without re-posting. */
  async function dismissOne(m: MyOfferRow): Promise<boolean> {
    try {
      await rpc("offerdismiss", [m.offer_id]);
      log(t("log.offerDismissed", { id: m.offer_id }));
      return true;
    } catch (e) {
      log(t("log.dismissError", { id: m.offer_id, err: errMsg(e) }));
      return false;
    }
  }

  /** Run an action over rows, dropping the handled ones (dialog closes empty). */
  async function act(targets: MyOfferRow[], action: (m: MyOfferRow) => Promise<boolean>) {
    setBusy(true);
    try {
      const done = new Set<string>();
      for (const m of targets) {
        if (await action(m)) done.add(m.offer_id);
      }
      setRows((rs) => rs.filter((r) => !done.has(r.offer_id)));
    } finally {
      setBusy(false);
    }
  }

  /** Closing resolves the remainder: whatever wasn't revived is dismissed, so
   *  nothing stays `revoked_on_open` once the dialog ends. Best-effort — a
   *  failed dismiss just reappears on the next boot — and the dialog always
   *  closes. */
  async function closeAll() {
    setBusy(true);
    try {
      for (const m of rows) {
        await dismissOne(m);
      }
    } finally {
      setRows([]);
      setBusy(false);
    }
  }

  if (rows.length === 0) return null;

  return (
    <Dialog open maxWidth="sm" fullWidth onClose={() => !busy && void closeAll()}>
      <DialogTitle>{t("revive.title")}</DialogTitle>
      <DialogContent>
        <DialogContentText sx={{ mb: 2 }}>{t("revive.intro")}</DialogContentText>
        <Box sx={{ display: "flex", flexDirection: "column", gap: 1 }}>
          {rows.map((m) => {
            const b = m.offer.body;
            return (
              <Box
                key={m.offer_id}
                sx={{
                  display: "flex",
                  alignItems: "center",
                  gap: 1.5,
                  border: 1,
                  borderColor: "divider",
                  borderRadius: 1,
                  px: 1.5,
                  py: 1,
                }}
              >
                <Box sx={{ minWidth: 0, flex: 1 }}>
                  <Typography sx={{ fontFamily: C.mono, fontSize: 13 }}>
                    {fmtBare(b.give_amount)} {symOf(b.give_asset)}
                    <Box component="span" sx={{ color: "text.secondary" }}>
                      {" → "}
                    </Box>
                    {fmtBare(b.get_amount)} {symOf(b.get_asset)}
                  </Typography>
                  <Typography sx={{ fontSize: 11, color: "text.secondary" }}>
                    {t("format.posted", { age: ago(m.created || b.created || 0) })}
                    {" · "}
                    {t("revive.timelocks", {
                      t1: Math.round(b.t1_secs / 3600),
                      t2: Math.round(b.t2_secs / 3600),
                    })}
                  </Typography>
                </Box>
                <ProtocolChip protocol={b.protocol} />
                <Button
                  size="small"
                  variant="outlined"
                  disabled={busy}
                  onClick={() => void act([m], reviveOne)}
                >
                  {t("revive.revive")}
                </Button>
                <Button
                  size="small"
                  color="inherit"
                  disabled={busy}
                  onClick={() => void act([m], dismissOne)}
                >
                  {t("revive.dismiss")}
                </Button>
              </Box>
            );
          })}
        </Box>
      </DialogContent>
      <DialogActions sx={{ px: 3, pb: 2, flexWrap: "wrap", gap: 1 }}>
        <Button onClick={() => void closeAll()} sx={{ mr: "auto" }} disabled={busy}>
          {t("revive.close")}
        </Button>
        <Button variant="contained" disabled={busy} onClick={() => void act([...rows], reviveOne)}>
          {t("revive.reviveAll")}
        </Button>
      </DialogActions>
    </Dialog>
  );
}
