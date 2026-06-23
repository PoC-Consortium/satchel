// BIP39 English wordlist for seed-entry autocomplete + validation (the same
// list pactd's bip39 crate uses). Sourced from @scure/bip39 so we don't ship a
// hand-copied 2048-word array.
import { validateMnemonic } from "@scure/bip39";
import { wordlist } from "@scure/bip39/wordlists/english.js";

export const BIP39_WORDS = wordlist as readonly string[];

const wordSet = new Set(BIP39_WORDS);

/** Is `w` a valid BIP39 word (case/space-insensitive)? */
export function isBip39Word(w: string): boolean {
  return wordSet.has(w.trim().toLowerCase());
}

/**
 * Does `phrase` form a complete, checksum-valid BIP39 mnemonic? This is the
 * same gate pactd applies on `importseed` (bip39::Mnemonic::parse_normalized),
 * mirrored client-side so the import wizard can block "Continue" until the
 * words actually check out — not just until something is typed. Tolerates extra
 * whitespace/casing and never throws (a malformed phrase is simply invalid).
 */
export function isValidMnemonic(phrase: string): boolean {
  const norm = phrase.trim().toLowerCase().split(/\s+/).filter(Boolean).join(" ");
  if (!norm) return false;
  try {
    return validateMnemonic(norm, wordlist);
  } catch {
    return false;
  }
}
