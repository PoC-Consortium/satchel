// Counterparty / merchant identity rendering. Every friendly form here is
// DETERMINISTIC from the BIP340 pubkey — never a self-chosen name (which would
// be spoofable). See UI_REQUIREMENTS "Counterparty identity rendering".

import { tr } from "./i18n";

/** FNV-1a over the hex string → a stable 32-bit unsigned int. */
export function hashId(hex: string): number {
  let h = 0x811c9dc5;
  const s = (hex || "").toLowerCase();
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 0x01000193);
  }
  return h >>> 0;
}

/** Truncated fingerprint, grouped for readability: `ab12 cd34`. Stays bound to
 *  the key (it's a prefix of the pubkey), so it can't be impersonated. */
export function shortId(hex: string | null | undefined): string {
  if (!hex) return tr("counterparty.unknownShort");
  const h = hex.toLowerCase();
  return `${h.slice(0, 4)} ${h.slice(4, 8)}`;
}

/** A stable hue for an identity, so the identicon + any accent agree. */
export function idHue(hex: string | null | undefined): number {
  return hex ? hashId(hex) % 360 : 0;
}
