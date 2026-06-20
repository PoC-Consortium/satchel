// Read-only slip helpers for the UI. The slip is the off-market artifact
// (spec/protocol.md §10): `pactoffer1:<base64url(canonical_json(offer envelope))>`.
//
// pactd is the ONLY authority — `takeoffer` re-decodes AND verifies the BIP340
// signature in Rust before anything happens. This module decodes the slip
// purely to render the take-confirmation card (amounts, pair) up front; it does
// NOT verify the signature and its output is never trusted for the swap itself.

import type { Offer, OfferBody } from "./api/types";

const SLIP_PREFIX = "pactoffer1:";

/** Quick shape check used to enable the "Review offer" button. */
export function looksLikeSlip(s: string): boolean {
  return s.trim().startsWith(SLIP_PREFIX);
}

/** Decode unpadded base64url to a byte array (mirrors pact-proto::slip). */
function decodeB64Url(input: string): Uint8Array {
  let b64 = input.replace(/-/g, "+").replace(/_/g, "/");
  while (b64.length % 4 !== 0) b64 += "=";
  const bin = atob(b64);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}

/** Decode a slip to its offer envelope for DISPLAY only. Returns null if the
 *  prefix/base64/JSON is malformed (so the caller can show a friendly error);
 *  authoritative verification lives in pactd `takeoffer`. */
export function decodeSlipForDisplay(slip: string): Offer | null {
  const trimmed = slip.trim();
  if (!trimmed.startsWith(SLIP_PREFIX)) return null;
  try {
    const bytes = decodeB64Url(trimmed.slice(SLIP_PREFIX.length));
    const json = new TextDecoder().decode(bytes);
    const env = JSON.parse(json) as { type?: string; swap_id?: string; from?: string; body?: OfferBody };
    if (env.type !== "offer" || !env.body || !env.swap_id || !env.from) return null;
    return { swap_id: env.swap_id, from: env.from, body: env.body };
  } catch {
    return null;
  }
}
