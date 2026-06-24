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
    case "signed":
      return tr(maker ? "narrate.signedMaker" : "narrate.signedTaker", v);
    case "funded_a":
      return tr(maker ? "narrate.fundedAMaker" : "narrate.fundedATaker", v);
    case "funded_b":
      return tr(maker ? "narrate.fundedBMaker" : "narrate.fundedBTaker", v);
    case "redeemed_b":
      return tr("narrate.redeemedB", v);
    case "completed":
      return tr("narrate.completed", { coin: maker ? b : a });
    case "refunded":
      return tr("narrate.refunded", { coin: maker ? a : b });
    case "aborted":
      return tr("narrate.aborted");
    default:
      return "";
  }
}
