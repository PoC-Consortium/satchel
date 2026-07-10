import { useEffect, useRef, useState } from "react";
import {
  Alert,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
  TextField,
} from "@mui/material";
import { invoke } from "@tauri-apps/api/core";
import { useApp } from "../AppContext";
import { useT } from "../i18n";
import { inTauri, rpc } from "../api/tauri";
import { isActive } from "../format";
import type { Offer } from "../api/types";

// Gate window-close on pactd state, for fund safety (UI_REQUIREMENTS "Gate app
// exit"). Premise: pactd is the Corkboard's client and manages ALONE — headless,
// it keeps polling the relay, servicing takes and completing swaps. So "keep
// pactd running" is a real, safe option everywhere, and C6 makes it true: when
// the user keeps it running we DETACH the managed pactd (skip the `stop` RPC) and
// re-adopt it next launch, rather than killing it mid-timelock.
//
// The 4-state matrix (managed mode), branched on active swaps + our open offers:
//   • nothing active           → quiet exit (stop pactd gracefully, then close).
//   • offers, no live swap      → Withdraw & exit / Keep running / Cancel.
//   • live swap, no offers      → Keep running (rec.) / Cancel / Force-quit
//                                 (typed confirm + loud fund-loss warning).
//   • live swap + offers        → Keep running + withdraw offers (rec.) /
//                                 Keep running + leave offers / Cancel /
//                                 Force-quit (typed confirm).
//
// Every terminal action funnels through the Rust `quit_app { keepRunning, withdraw }`
// command, which detaches-or-stops per `keepRunning` (and is a safe no-op stop in
// external/adopt mode). "Withdraw" = `boardrevoke` each of our open offers here
// (we hold the offer + identity context) BEFORE quitting.
type Pending =
  | null
  | { kind: "live"; count: number; offers: Offer[] }
  | { kind: "offers"; offers: Offer[] };

export default function ExitGate() {
  const { swaps, identity } = useApp();
  const t = useT();
  const [pending, setPending] = useState<Pending>(null);
  const [confirmText, setConfirmText] = useState("");
  const [busy, setBusy] = useState(false);

  // Keep the latest swaps/identity reachable from the (long-lived) close handler.
  const swapsRef = useRef(swaps);
  swapsRef.current = swaps;
  const idRef = useRef(identity);
  idRef.current = identity;
  // A quiet exit already tearing down — ignore further X presses meanwhile.
  const closingRef = useRef(false);

  useEffect(() => {
    if (!inTauri()) return;
    let unlisten: (() => void) | undefined;
    let disposed = false;

    (async () => {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      const win = getCurrentWindow();
      unlisten = await win.onCloseRequested(async (event) => {
        // PREVENT FIRST, synchronously — Tauri holds the close request until
        // this handler resolves, so an awaited RPC before preventDefault left
        // the X press dead for as long as the engine lock was busy with slow
        // chain calls (observed live: ~40s, or seemingly forever). Answer the
        // event immediately, decide what to do asynchronously after.
        event.preventDefault();
        if (closingRef.current) return; // teardown already in flight
        // Our own open offers, if any — best-effort AND time-boxed: no board,
        // an unreachable relay, or a busy engine must not stall the exit
        // decision (worst case we skip the revoke shortcut; posted offers
        // expire via their TTL anyway).
        let mine: Offer[] = [];
        try {
          const list = await Promise.race([
            rpc<{ offers?: Offer[] }>("boardlistoffers"),
            new Promise<{ offers?: Offer[] }>((_, reject) =>
              setTimeout(() => reject(new Error("exit-gate offers lookup timed out")), 2000),
            ),
          ]);
          const me = idRef.current;
          mine = (list.offers || []).filter((o) => o.from === me && !o.revoked);
        } catch {
          /* no board / unreachable / busy — nothing to revoke */
        }

        const live = swapsRef.current.filter(isActive);
        if (live.length > 0) {
          // Live swap dominates (timelocks): never auto-stop pactd.
          setConfirmText("");
          setPending({ kind: "live", count: live.length, offers: mine });
          return;
        }
        if (mine.length > 0) {
          // Offers but no live swap: ask before leaving them posted / stopping.
          setPending({ kind: "offers", offers: mine });
          return;
        }
        // Nothing active → quiet exit: stop pactd gracefully, then close.
        closingRef.current = true;
        try {
          await quit(false, false);
        } finally {
          closingRef.current = false; // quit failed/aborted — X must work again
        }
      });
      if (disposed) unlisten?.();
    })();

    return () => {
      disposed = true;
      unlisten?.();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  /** Revoke each of our open offers (best-effort), then quit per `keepRunning`. */
  async function withdrawThen(offers: Offer[], keepRunning: boolean) {
    setBusy(true);
    for (const o of offers) {
      try {
        await rpc("boardrevoke", [o.swap_id]);
      } catch {
        /* best effort — proceed regardless */
      }
    }
    await quit(keepRunning, true);
  }

  /** Terminal hand-off to Rust: detach-or-stop the managed pactd, then exit. */
  async function quit(keepRunning: boolean, withdraw: boolean) {
    setBusy(true);
    try {
      await invoke("quit_app", { keepRunning, withdraw });
    } catch {
      // If the command fails, fall back to closing the window so the user is
      // never trapped (managed pactd may then be stopped by the Exit handler).
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().destroy();
    }
  }

  if (!pending) return null;

  // ---- live swap (with or without offers) ---------------------------------
  if (pending.kind === "live") {
    const armed = confirmText.trim().toUpperCase() === t("exit.confirmWord").toUpperCase();
    const hasOffers = pending.offers.length > 0;
    return (
      <Dialog open maxWidth="sm" fullWidth onClose={() => !busy && setPending(null)}>
        <DialogTitle>{t("exit.liveTitle")}</DialogTitle>
        <DialogContent>
          <Alert severity="warning" variant="outlined" sx={{ mb: 2 }}>
            {t(pending.count === 1 ? "exit.liveBodyOne" : "exit.liveBodyMany", {
              count: pending.count,
            })}
          </Alert>
          <DialogContentText sx={{ mb: 2 }}>{t("exit.keepRunningExplain")}</DialogContentText>
          <DialogContentText sx={{ mb: 1 }}>
            <b>{t("exit.forceQuitWarn")}</b> {t("exit.typeToConfirm", { word: t("exit.confirmWord") })}
          </DialogContentText>
          <TextField
            value={confirmText}
            onChange={(e) => setConfirmText(e.target.value)}
            placeholder={t("exit.confirmWord")}
            size="small"
            fullWidth
            autoFocus
          />
        </DialogContent>
        <DialogActions sx={{ px: 3, pb: 2, flexWrap: "wrap", gap: 1 }}>
          <Button onClick={() => setPending(null)} sx={{ mr: "auto" }} disabled={busy}>
            {t("common.cancel")}
          </Button>
          {hasOffers ? (
            <>
              <Button
                variant="contained"
                disabled={busy}
                onClick={() => void withdrawThen(pending.offers, true)}
              >
                {t("exit.keepWithdraw")}
              </Button>
              <Button color="inherit" disabled={busy} onClick={() => void quit(true, false)}>
                {t("exit.keepLeaveOffers")}
              </Button>
            </>
          ) : (
            <Button variant="contained" disabled={busy} onClick={() => void quit(true, false)}>
              {t("exit.keepRunning")}
            </Button>
          )}
          <Button color="error" disabled={!armed || busy} onClick={() => void quit(false, false)}>
            {t("exit.forceQuit")}
          </Button>
        </DialogActions>
      </Dialog>
    );
  }

  // ---- open offers, no live swap ------------------------------------------
  return (
    <Dialog open maxWidth="sm" fullWidth onClose={() => !busy && setPending(null)}>
      <DialogTitle>{t("exit.offersTitle")}</DialogTitle>
      <DialogContent>
        <DialogContentText>
          {t(pending.offers.length === 1 ? "exit.offersBodyOne" : "exit.offersBodyMany", {
            count: pending.offers.length,
          })}
        </DialogContentText>
      </DialogContent>
      <DialogActions sx={{ px: 3, pb: 2, flexWrap: "wrap", gap: 1 }}>
        <Button onClick={() => setPending(null)} sx={{ mr: "auto" }} disabled={busy}>
          {t("common.cancel")}
        </Button>
        <Button color="inherit" disabled={busy} onClick={() => void quit(true, false)}>
          {t("exit.keepRunning")}
        </Button>
        <Button
          variant="contained"
          disabled={busy}
          onClick={() => void withdrawThen(pending.offers, false)}
        >
          {t("exit.withdrawExit")}
        </Button>
      </DialogActions>
    </Dialog>
  );
}
