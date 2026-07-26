import type { Swap } from "../api/types";
import { asset } from "../format";
import { tr } from "../i18n";

/** The notification-worthy event kinds (issue #55) — 1:1 with the NotifyPrefs
 *  sub-toggles (Settings → Notifications). */
export type NotifyEvent = "swap_started" | "locks" | "completed" | "failed" | "reorg";

/** Where a swap sits on the notification ladder (issue #55). `rank` orders the
 *  milestones so a poll that skips steps fires only the latest one; `event`
 *  classifies it under a Settings toggle (null = a step notifications stay
 *  quiet on):
 *    0 nothing committed · 1 swap underway · 2 one leg locked ·
 *    3 both legs locked · 4 finalizing (claim burying — silent, automated) ·
 *    5 terminal (completed / refunded / aborted).
 *
 *  This ladder lives HERE, directly above narrate(), because it is the same
 *  (state, role, progress.watching) checkpoint switch — any honest-split tweak
 *  to narrate() below (like PR #64's) must move the matching branch here too,
 *  or a notification fires at the wrong milestone with a body that contradicts
 *  its title. */
export function milestone(s: Swap): { rank: number; event: NotifyEvent | null } {
  const w = s.progress?.watching;
  const maker = s.role === "initiator";
  switch (s.state) {
    case "initiating":
    case "created":
      return { rank: 0, event: null };
    case "accepted":
      // v2 maker broadcasts its lock on accept — once the progress line tracks
      // it, "one leg is locked" is the honest milestone (same split as below).
      // `funding` (wallet mid-lock, U-3) counts the same as `our_lock`,
      // matching narrate's progress-first override.
      if (maker && (w === "our_lock" || w === "awaiting_lock" || w === "funding")) {
        return { rank: 2, event: "locks" };
      }
      return { rank: 1, event: "swap_started" };
    case "signed":
      if (maker) return { rank: w === "their_lock" ? 3 : 2, event: "locks" };
      return w === "our_lock" || w === "awaiting_claim" || w === "funding"
        ? { rank: 3, event: "locks" }
        : { rank: 1, event: "swap_started" };
    case "funded_a":
      return { rank: maker && w === "their_lock" ? 3 : 2, event: "locks" };
    case "funded_b":
      return { rank: 3, event: "locks" };
    case "redeemed_b":
      // Deliberately silent (event null) for BOTH roles, so no role split is
      // needed here even though narrate's redeemed_b params are role-mapped.
      return { rank: 4, event: null };
    case "completed":
      // Finalizing (isFinalizing's definition) stays silent; the terminal
      // "coins are in your wallet" notification lands when the claim buries.
      return w === "settlement" ? { rank: 4, event: null } : { rank: 5, event: "completed" };
    case "refunded":
    case "aborted":
      return { rank: 5, event: "failed" };
    default:
      return { rank: 0, event: null };
  }
}

// Plain-language story per (role, state) — ported VERBATIM from index.html's
// narrate() into the i18n bundle (narrate.*). This copy is load-bearing UX (the
// honest, no-jargon framing of who is exposed when), so it must not drift.
// narrate() is a pure helper (no React context), so it translates via the tr()
// module mirror rather than the useT() hook.
export function narrate(s: Swap): string {
  const a = asset(s.chain_a).toUpperCase();
  const b = asset(s.chain_b).toUpperCase();
  const t1 = new Date(s.t1 * 1000).toLocaleTimeString();
  const t2 = new Date(s.t2 * 1000).toLocaleTimeString();
  const maker = s.role === "initiator";
  const v = { a, b, t1, t2 };
  // Progress-first overrides (state-matrix audit U-3): the engine emits these
  // `watching` values only pre-lock, where the chain-derived state would still
  // narrate "you can cancel freely" — override with the honest in-flight story.
  // `funding` = THIS machine's wallet is sending the lock right now;
  // `awaiting_our_lock` exists only on followed rows — our side locks next,
  // but another of this merchant's machines drives the send.
  const wNow = s.progress?.watching;
  if (wNow === "funding") {
    return tr("narrate.locking", { coin: maker ? a : b });
  }
  if (wNow === "awaiting_our_lock") {
    return tr("narrate.awaitingOurLock", { coin: maker ? a : b });
  }
  switch (s.state) {
    case "initiating":
      return tr("narrate.initiating");
    case "created":
      return tr("narrate.created");
    // v2 (Taproot/MuSig2 adaptor) handshake. Funding + the claim run
    // automatically from "signed"; the timelock refund is the safety net.
    // The v2 maker broadcasts its lock the moment `accept` arrives but stays in
    // this state through the nonce/sig round-trips — once the progress line
    // tracks that lock, "you can still cancel freely" / "nothing is locked yet"
    // would be false: the honest story is the funded-maker one.
    case "accepted": {
      const w = s.progress?.watching;
      if (maker && (w === "our_lock" || w === "awaiting_lock")) {
        return tr("narrate.fundedAMaker", v);
      }
      return tr(maker ? "narrate.acceptedMaker" : "narrate.acceptedTaker", v);
    }
    // v2 "signed" is a single state spanning the whole execution phase, so a
    // flat story freezes there while only the progress bar moves. Sub-divide it
    // by the progress sub-phase the tick already computes, so it steps through
    // checkpoints like v1's funded_a/funded_b. Reuses existing keys (no new copy):
    //   maker: waiting for the taker to lock B (signedMaker) → both locked,
    //          claiming their B (fundedBMaker).
    //   taker: waiting on the maker's A, about to lock B (signedTaker) → both
    //          locked, awaiting their claim (fundedBTaker).
    case "signed": {
      const w = s.progress?.watching;
      if (maker) {
        return tr(w === "their_lock" ? "narrate.fundedBMaker" : "narrate.signedMaker", v);
      }
      return tr(
        w === "our_lock" || w === "awaiting_claim" ? "narrate.fundedBTaker" : "narrate.signedTaker",
        v,
      );
    }
    // The v1 maker stays in funded_a until leg B is n_b-deep, so once the
    // progress line tracks the taker's lock ("their lock confirming") the
    // honest story is the both-locked one — the same watching-based split
    // v2's `signed` case does above.
    case "funded_a":
      if (maker && s.progress?.watching === "their_lock") {
        return tr("narrate.fundedBMaker", v);
      }
      return tr(maker ? "narrate.fundedAMaker" : "narrate.fundedATaker", v);
    case "funded_b":
      return tr(maker ? "narrate.fundedBMaker" : "narrate.fundedBTaker", v);
    // "Finalizing": the claim is broadcast but still burying — not done yet.
    // The maker is here at `redeemed_b`; a DRIVEN taker skips it (signed →
    // completed atomically), but a followed/taken-over taker record can be
    // ratcheted here by the chain watcher — so claimed/locked must follow the
    // role: a taker at `redeemed_b` locked coin B and claims coin A. Same
    // wording for both roles: claimed-coin = {got}, locked-coin = {gave}.
    case "redeemed_b":
      return tr("narrate.finalizing", {
        got: maker ? b : a,
        gave: maker ? a : b,
      });
    case "completed":
      return s.progress?.watching === "settlement"
        ? tr("narrate.finalizing", { got: maker ? b : a, gave: maker ? a : b })
        : tr("narrate.completed", { coin: maker ? b : a });
    case "refunded":
      return tr("narrate.refunded", { coin: maker ? a : b });
    case "aborted":
      return tr("narrate.aborted");
    default:
      return "";
  }
}
