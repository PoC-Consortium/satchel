import type { Swap } from "../api/types";
import { asset } from "../format";

// Plain-language story per (role, state). Ported VERBATIM from index.html's
// narrate() — this copy is load-bearing UX (the honest, no-jargon framing of
// who is exposed when), so it must not drift.
export function narrate(s: Swap): string {
  const A = asset(s.chain_a).toUpperCase();
  const B = asset(s.chain_b).toUpperCase();
  const t1 = new Date(s.t1 * 1000).toLocaleTimeString();
  const t2 = new Date(s.t2 * 1000).toLocaleTimeString();
  const maker = s.role === "initiator";
  const map: Record<string, string> = {
    initiating:
      "Take sent — waiting for the maker to start the swap. Nothing is locked yet; it cancels on its own if they don't respond.",
    created: "Offer sent — waiting for the other side to agree. Nothing is committed.",
    accepted: maker
      ? `Terms agreed. Next: lock your ${A}. Until you fund, you can still cancel freely.`
      : `Terms agreed. The other side locks their ${A} first — you never send first.`,
    // v2 (Taproot/MuSig2 adaptor) handshake states. Funding + the claim run
    // automatically from "signed"; the timelock refund is the safety net.
    nonces_exchanged: "Setting up the private swap — exchanging signing material. Nothing is locked yet.",
    signed: maker
      ? `Both sides signed. Your daemon locks the ${A}, then claims the ${B} automatically. If anything stalls, your ${A} returns at ${t1}.`
      : `Both sides signed. Your daemon locks the ${B} and claims the ${A} the moment the other side moves. Safety net: refund at ${t2}.`,
    funded_a: maker
      ? `Your ${A} is locked. Waiting for the other side to lock their ${B}. If they never do, your ${A} returns automatically at ${t1}.`
      : `Their ${A} is locked and verified. Next: lock your ${B}. Safety net: automatic refund at ${t2} if anything stalls.`,
    funded_b: maker
      ? `Both locked. Your daemon claims the ${B} as soon as it is safely confirmed.`
      : `Both locked. Your daemon will claim the ${A} the moment the other side takes their ${B}.`,
    redeemed_b: `You claimed the ${B} — waiting for it to confirm. Your locked ${A} stays protected until this is final.`,
    completed: `Swap complete — the ${maker ? B : A} is in your wallet.`,
    refunded: `The swap did not complete, so your ${maker ? A : B} came back automatically. Nothing lost but fees.`,
    aborted: "Cancelled before any money moved.",
  };
  return map[s.state] || "";
}
