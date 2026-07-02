import type { Swap } from "../api/types";
import { asset } from "../format";
import { tr } from "../i18n";

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
  switch (s.state) {
    case "initiating":
      return tr("narrate.initiating");
    case "created":
      return tr("narrate.created");
    case "accepted":
      return tr(maker ? "narrate.acceptedMaker" : "narrate.acceptedTaker", v);
    // v2 (Taproot/MuSig2 adaptor) handshake states. Funding + the claim run
    // automatically from "signed"; the timelock refund is the safety net.
    case "nonces_exchanged":
      return tr("narrate.noncesExchanged");
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
    case "funded_a":
      return tr(maker ? "narrate.fundedAMaker" : "narrate.fundedATaker", v);
    case "funded_b":
      return tr(maker ? "narrate.fundedBMaker" : "narrate.fundedBTaker", v);
    // "Finalizing": the claim is broadcast but still burying — not done yet.
    // The maker is here at `redeemed_b`; the taker reaches it at `completed`
    // while its settlement bar is still counting (see isFinalizing). Same wording
    // for both roles: claimed-coin = {got}, locked-coin = {gave}.
    case "redeemed_b":
      return tr("narrate.finalizing", { got: b, gave: a });
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
