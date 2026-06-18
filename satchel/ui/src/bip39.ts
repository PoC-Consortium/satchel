// BIP39 English wordlist for seed-entry autocomplete + validation (the same
// list pactd's bip39 crate uses). Sourced from @scure/bip39 so we don't ship a
// hand-copied 2048-word array.
import { wordlist } from "@scure/bip39/wordlists/english.js";

export const BIP39_WORDS = wordlist as readonly string[];

const wordSet = new Set(BIP39_WORDS);

/** Is `w` a valid BIP39 word (case/space-insensitive)? */
export function isBip39Word(w: string): boolean {
  return wordSet.has(w.trim().toLowerCase());
}
