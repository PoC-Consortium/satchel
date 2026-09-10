//! Swap-leg state reconstruction from chain ground truth
//! (docs/design/STATE_RECONSTRUCTION.md).
//!
//! Every classification here is a PURE, idempotent function of the chain:
//! it answers "what happened to this leg" identically whether the swap
//! completed a minute or a month ago, so a follower/rescuer that missed
//! events (dormant observer, restart, DB restore) converges on the next
//! evaluation — there is no history-catchup vs live-monitor mode switch to
//! desync. Live triggers (ticks, subscriptions) only decide WHEN to
//! re-evaluate, never what is true.
//!
//! The classifier needs a script-history-capable backend ([`ChainBackend::
//! spk_history`], Electrum) — the live-UTXO reads (`find_funding`/
//! `get_txout`) structurally cannot see an output that is already spent,
//! which is every completed swap. History-less backends (Core RPC, tier L)
//! return `Ok(None)` and callers degrade to live reads + the timelock
//! age-out.
//!
//! All backend data is untrusted (spec §10): funding outputs are matched
//! byte-for-byte against the locally derived scriptPubKey AND the agreed
//! amount; spend classification rests on witness content that cannot be
//! fabricated (a v1 redeem must carry a preimage hashing to `H`; a v2
//! refund must reveal the exact tapleaf we can rebuild) — AND, before any
//! protocol shape is judged, the spending input's signature is verified
//! against the funding output it claims to spend ([`witness_authentic`]):
//! a history provider can invent a transaction, but not a signature by the
//! swap keys over it (security review 2026-09-09 #7). A garbage witness
//! lands in [`SpendKind::Unknown`], which never drives a terminal decision.

use anyhow::{bail, Context, Result};
use bitcoin::hashes::{sha256, Hash};
use bitcoin::script::Instruction;
use bitcoin::secp256k1::{Message, PublicKey, Secp256k1, XOnlyPublicKey};
use bitcoin::sighash::{Prevouts, SighashCache};
use bitcoin::taproot::{ControlBlock, TapLeafHash};
use bitcoin::{Amount, OutPoint, Script, ScriptBuf, Transaction, TxOut};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::chain::ChainBackend;
use crate::htlc::extract_preimage;
use crate::params::Network;

/// How a swap-leg funding output was spent, judged from the spending
/// input's witness alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpendKind {
    /// The claim path: v1 hash-branch (preimage verified against `H`),
    /// v2 key-path (single Schnorr sig — only the MuSig2 aggregate can).
    Redeem,
    /// The timeout path: v1 CLTV branch, v2 refund tapleaf (byte-equal to
    /// the locally rebuilt leaf script).
    Refund,
    /// Neither shape — anomalous. Never treated as a terminal signal.
    Unknown,
}

/// One leg's spent-state, with everything a caller needs to persist
/// pointers, judge finality, or fast-forward a record.
#[derive(Debug, Clone)]
pub struct SpentLeg {
    pub outpoint: OutPoint,
    /// Funding block height (0 = still unconfirmed — possible when the
    /// spend is a same-mempool chain).
    pub funding_height: u64,
    pub spend_txid: String,
    /// Spend block height (0 = unconfirmed).
    pub spend_height: u64,
    pub spend_confs: u64,
    pub kind: SpendKind,
    /// Full hex of the spending tx — lets a takeover adopt the spend as its
    /// own `final_tx` so the existing confirmation nurses converge on it.
    pub spend_tx_hex: String,
}

/// The complete classification of one swap leg, front-to-back.
#[derive(Debug, Clone)]
pub enum LegClass {
    /// No output paying `(spk, amount)` has ever appeared.
    Unfunded,
    /// The funding output exists and is unspent.
    Funded {
        outpoint: OutPoint,
        /// Funding block height (0 = mempool).
        height: u64,
        confs: u64,
    },
    /// The funding output existed and is spent — the historical fact no
    /// live-UTXO read can see.
    Spent(SpentLeg),
    /// The funding is PROVEN (wallet evidence, #171) and the output is gone
    /// from the UTXO set — an unambiguous spend whose spending tx we cannot
    /// retrieve (tier L, spent by the counterparty). Enough to refuse a
    /// re-fund and to resolve a follower via the tip-drift buffer; never
    /// enough for a depth-verified terminal.
    Vanished {
        outpoint: OutPoint,
        /// Funding block height (0 = unknown/unconfirmed at recording).
        funding_height: u64,
    },
}

/// Classify a v1 (P2WSH HTLC) spend from its witness. The spk match already
/// proves the spend runs OUR witness script (consensus checks the last item
/// against the P2WSH hash), so only the branch needs judging:
/// redeem = `[sig, pubkey, s, 0x01, script]` (a 32-byte item hashing to `H`
/// — position-independent and hash-verified, `crate::htlc::extract_preimage`);
/// refund = `[sig, pubkey, <>, script]` (empty OP_ELSE selector).
pub fn classify_v1_spend(witness: &[Vec<u8>], hash_h: &[u8; 32]) -> SpendKind {
    if extract_preimage(witness, hash_h).is_some() {
        return SpendKind::Redeem;
    }
    if witness.len() == 4 && witness[2].is_empty() {
        return SpendKind::Refund;
    }
    SpendKind::Unknown
}

/// Classify a v2 (Taproot) spend from its witness. The leg has exactly ONE
/// tapleaf (`crate::taproot::TaprootLeg`), so consensus admits two shapes:
/// a key-path spend (single 64/65-byte Schnorr sig — only the 2-of-2 MuSig2
/// aggregate can produce it, i.e. the cooperative redeem) or our CLTV
/// refund leaf (`[sig, leaf_script, control_block]`, leaf byte-equal to the
/// locally rebuilt `refund_script()`). Anything else — including a
/// fabricated "signature" a lying server could invent — is `Unknown`.
pub fn classify_v2_spend(witness: &[Vec<u8>], refund_script: &Script) -> SpendKind {
    match witness {
        [sig] if sig.len() == 64 || sig.len() == 65 => SpendKind::Redeem,
        [_sig, script, _ctrl] if script.as_slice() == refund_script.as_bytes() => SpendKind::Refund,
        _ => SpendKind::Unknown,
    }
}

/// Is input `index` of `tx` a signature-valid spend of a funding output
/// `(prev_spk, prev_value_sat)`? This is the trust boundary between a
/// history provider's claims and our terminal decisions: the provider can
/// hand us any transaction bytes, but it cannot produce a signature by the
/// swap keys over them, so a spend whose witness does not verify is not
/// evidence of anything.
///
/// - **P2WSH (v1 HTLC):** the last witness item must hash to the output's
///   program (it IS our witness script), the witness must open with
///   `[sig, pubkey, …]`, that pubkey must be one the script pushes (the
///   only keys it ever CHECKSIGs), and the ECDSA signature must verify over
///   the BIP143 sighash for this input.
/// - **P2TR key path (v2 cooperative redeem):** the single Schnorr
///   signature must verify against the OUTPUT key over the BIP341 key-path
///   sighash. Only the MuSig2 aggregate can produce it.
/// - **P2TR script path (v2 refund):** the control block must commit the
///   revealed leaf to the output key, and the signature must verify against
///   a key the leaf pushes over the script-path sighash.
///
/// BIP341 sighashes commit to EVERY prevout of the spending tx; only this
/// one is known here, so a Taproot spend with more than one input is
/// unverifiable and reads as not authentic (our redeems/refunds are always
/// single-input). Output types this crate never funds are passed through.
pub fn witness_authentic(
    prev_spk: &Script,
    prev_value_sat: u64,
    tx: &Transaction,
    index: usize,
) -> bool {
    let Some(input) = tx.input.get(index) else {
        return false;
    };
    let witness: Vec<&[u8]> = input.witness.iter().collect();
    let secp = Secp256k1::verification_only();
    if prev_spk.is_p2wsh() {
        // Program = SHA256(witness script) — consensus requires the last
        // item to BE that script.
        let Some(script_bytes) = witness.last() else {
            return false;
        };
        if sha256::Hash::hash(script_bytes).as_byte_array() != &prev_spk.as_bytes()[2..] {
            return false;
        }
        let script = Script::from_bytes(script_bytes);
        let (Some(sig_bytes), Some(pk_bytes)) = (witness.first(), witness.get(1)) else {
            return false;
        };
        let Ok(pubkey) = PublicKey::from_slice(pk_bytes) else {
            return false;
        };
        // The script names its CHECKSIG keys either raw or, as our HTLC
        // does (`OP_DUP OP_HASH160 <hash160(key)> OP_EQUALVERIFY OP_CHECKSIG`),
        // by hash160. A key it never names cannot satisfy it.
        let pk_hash = bitcoin::hashes::hash160::Hash::hash(pk_bytes);
        if !script_pushes(script, pk_bytes) && !script_pushes(script, pk_hash.as_byte_array()) {
            return false;
        }
        let Ok(sig) = bitcoin::ecdsa::Signature::from_slice(sig_bytes) else {
            return false;
        };
        let Ok(sighash) = SighashCache::new(tx).p2wsh_signature_hash(
            index,
            script,
            Amount::from_sat(prev_value_sat),
            sig.sighash_type,
        ) else {
            return false;
        };
        return secp
            .verify_ecdsa(
                &Message::from_digest(sighash.to_byte_array()),
                &sig.signature,
                &pubkey,
            )
            .is_ok();
    }
    if prev_spk.is_p2tr() {
        let Ok(output_key) = XOnlyPublicKey::from_slice(&prev_spk.as_bytes()[2..]) else {
            return false;
        };
        if tx.input.len() != 1 || index != 0 {
            return false; // sighash needs every prevout; we only know ours
        }
        let prevout = TxOut {
            value: Amount::from_sat(prev_value_sat),
            script_pubkey: prev_spk.to_owned(),
        };
        let prevouts = Prevouts::All(std::slice::from_ref(&prevout));
        match witness.as_slice() {
            [sig_bytes] => {
                let Ok(sig) = bitcoin::taproot::Signature::from_slice(sig_bytes) else {
                    return false;
                };
                let Ok(sighash) = SighashCache::new(tx).taproot_key_spend_signature_hash(
                    0,
                    &prevouts,
                    sig.sighash_type,
                ) else {
                    return false;
                };
                secp.verify_schnorr(
                    &sig.signature,
                    &Message::from_digest(sighash.to_byte_array()),
                    &output_key,
                )
                .is_ok()
            }
            [sig_bytes, script_bytes, control_bytes] => {
                let Ok(control) = ControlBlock::decode(control_bytes) else {
                    return false;
                };
                let script = Script::from_bytes(script_bytes);
                if !control.verify_taproot_commitment(&secp, output_key, script) {
                    return false;
                }
                let Ok(sig) = bitcoin::taproot::Signature::from_slice(sig_bytes) else {
                    return false;
                };
                let leaf_hash = TapLeafHash::from_script(script, control.leaf_version);
                let Ok(sighash) = SighashCache::new(tx).taproot_script_spend_signature_hash(
                    0,
                    &prevouts,
                    leaf_hash,
                    sig.sighash_type,
                ) else {
                    return false;
                };
                let msg = Message::from_digest(sighash.to_byte_array());
                // The leaf's CHECKSIG key is one of its 32-byte pushes.
                script.instructions().any(|ins| match ins {
                    Ok(Instruction::PushBytes(push)) if push.len() == 32 => {
                        XOnlyPublicKey::from_slice(push.as_bytes())
                            .map(|key| secp.verify_schnorr(&sig.signature, &msg, &key).is_ok())
                            .unwrap_or(false)
                    }
                    _ => false,
                })
            }
            _ => false,
        }
    } else {
        true // not a swap-leg output type this crate builds — no opinion
    }
}

/// Does `script` push exactly `bytes` anywhere? (Which keys a script can
/// CHECKSIG is the set of keys it pushes.)
fn script_pushes(script: &Script, bytes: &[u8]) -> bool {
    script
        .instructions()
        .any(|ins| matches!(ins, Ok(Instruction::PushBytes(push)) if push.as_bytes() == bytes))
}

/// Defensive cap on how many history entries a leg classification will
/// fetch. A swap leg's script is unique to the swap, so its real history is
/// a handful of transactions; anything larger is address spam and reads as
/// inconclusive rather than an unbounded fetch loop.
const MAX_HISTORY_TXS: usize = 24;

/// Reconstruct one swap leg's state from the chain, front-to-back.
///
/// `Ok(None)` = the backend has no script history (tier L) — the caller
/// falls back to live reads + the timelock age-out. Any other
/// inconclusiveness (unfetchable history tx, oversized history) is an
/// `Err`, which callers treat as "leave the record untouched, retry later".
///
/// Cost: one `spk_history` round-trip, plus one `fetch_tx` per history
/// entry — a handful, and only paid when the caller has no cached
/// classification (see the follow evaluator's spend cache).
pub fn classify_leg(
    backend: &dyn ChainBackend,
    spk: &ScriptBuf,
    amount_sat: u64,
    classify_spend: &dyn Fn(&[Vec<u8>]) -> SpendKind,
) -> Result<Option<LegClass>> {
    let Some(entries) = backend.spk_history(spk)? else {
        return Ok(None); // tier L — no script index on any view
    };
    if entries.is_empty() {
        return Ok(Some(LegClass::Unfunded));
    }
    if entries.len() > MAX_HISTORY_TXS {
        bail!(
            "script history has {} entries — not a plausible swap leg, refusing to classify",
            entries.len()
        );
    }
    let tip = backend.tip_height()?;
    let confs_of = |height: u64| -> u64 {
        if height > 0 && tip >= height {
            tip - height + 1
        } else {
            0
        }
    };

    // Fetch every history tx once. A missing tx is inconclusive — the
    // entry came from the same backend set, so absence is a transient gap,
    // not evidence.
    let mut txs = Vec::with_capacity(entries.len());
    for (txid, height) in &entries {
        let tx = backend
            .fetch_tx(txid)?
            .with_context(|| format!("history tx {txid} not retrievable — inconclusive"))?;
        // Electrum reports mempool entries as height 0 / -1.
        let height = u64::try_from(*height).unwrap_or(0);
        txs.push((txid.clone(), height, tx));
    }

    // Funding candidates: outputs byte-matching the derived spk AND the
    // agreed amount (a wrong-amount payment to the same script is a
    // mis-funding, ignored exactly like the live `find_funding` path).
    let mut candidates: Vec<(OutPoint, u64)> = Vec::new(); // (outpoint, funding height)
    for (txid, height, tx) in &txs {
        for (vout, out) in tx.output.iter().enumerate() {
            if out.script_pubkey == *spk && out.value.to_sat() == amount_sat {
                candidates.push((
                    OutPoint {
                        txid: bitcoin::Txid::from_str(txid)?,
                        vout: vout as u32,
                    },
                    *height,
                ));
            }
        }
    }
    if candidates.is_empty() {
        return Ok(Some(LegClass::Unfunded));
    }

    // Spend lookup per candidate; prefer a SPENT candidate (a completed
    // swap must classify terminal even if a stray duplicate funding
    // lingers unspent).
    /// The history tx (and which of its inputs) that spends a candidate.
    struct SpendHit<'a> {
        txid: &'a String,
        height: u64,
        tx: &'a bitcoin::Transaction,
        index: usize,
        witness: Vec<Vec<u8>>,
    }
    let find_spend = |op: &OutPoint| -> Option<SpendHit<'_>> {
        for (txid, height, tx) in &txs {
            for (index, input) in tx.input.iter().enumerate() {
                if input.previous_output == *op {
                    let witness: Vec<Vec<u8>> =
                        input.witness.iter().map(|item| item.to_vec()).collect();
                    return Some(SpendHit {
                        txid,
                        height: *height,
                        tx,
                        index,
                        witness,
                    });
                }
            }
        }
        None
    };
    for (op, funding_height) in &candidates {
        if let Some(hit) = find_spend(op) {
            // Trust boundary: a spend the swap keys did not sign is not
            // evidence of a spend at all (lying/fabricating provider).
            let kind = if witness_authentic(spk, amount_sat, hit.tx, hit.index) {
                classify_spend(&hit.witness)
            } else {
                SpendKind::Unknown
            };
            return Ok(Some(LegClass::Spent(SpentLeg {
                outpoint: *op,
                funding_height: *funding_height,
                spend_txid: hit.txid.clone(),
                spend_height: hit.height,
                spend_confs: confs_of(hit.height),
                kind,
                spend_tx_hex: bitcoin::consensus::encode::serialize_hex(hit.tx),
            })));
        }
    }
    let (op, height) = candidates[0];
    Ok(Some(LegClass::Funded {
        outpoint: op,
        height,
        confs: confs_of(height),
    }))
}

/// Tier-L spend recovery — the block-scan fallback.
///
/// [`classify_leg`] needs a script index; without one, an already-spent leg
/// is unknowable through history. But when the funding OUTPOINT is already
/// known (a recorded pointer, a relay snapshot, wallet evidence), the ONE
/// missing fact is its spending transaction — and that a bare node can still
/// dig out of the mempool/blocks ([`ChainBackend::find_spend_tx`], the same
/// lookup the live driver uses to extract a counterparty's reveal). Verified
/// like everything here: the returned tx is only accepted if one of its
/// inputs spends `outpoint` byte-exactly, and the spend KIND is judged from
/// the witness against locally rebuilt scripts. `Ok(None)` = no spend
/// visible (still unspent, scan floor too high, or the backend cannot scan)
/// — inconclusive, exactly as before this fallback existed.
pub fn classify_spent_by_scan(
    backend: &dyn ChainBackend,
    outpoint: &OutPoint,
    watch_spk: &ScriptBuf,
    funding_value_sat: u64,
    funding_height: u64,
    scan_floor: u64,
    classify_spend: &dyn Fn(&[Vec<u8>]) -> SpendKind,
) -> Result<Option<SpentLeg>> {
    let Some((tx, spend_height)) = backend.find_spend_tx(outpoint, watch_spk, scan_floor)? else {
        return Ok(None);
    };
    let Some(index) = tx.input.iter().position(|i| i.previous_output == *outpoint) else {
        return Ok(None); // a hit that doesn't spend our outpoint is not evidence
    };
    let witness: Vec<Vec<u8>> = tx.input[index]
        .witness
        .iter()
        .map(|item| item.to_vec())
        .collect();
    // Same trust boundary as `classify_leg`: an unsigned/mis-signed spend
    // is not evidence (the scan result may come from an untrusted view).
    let kind = if witness_authentic(watch_spk, funding_value_sat, &tx, index) {
        classify_spend(&witness)
    } else {
        SpendKind::Unknown
    };
    let tip = backend.tip_height()?;
    let spend_confs = if spend_height > 0 && tip >= spend_height {
        tip - spend_height + 1
    } else {
        0
    };
    Ok(Some(SpentLeg {
        outpoint: *outpoint,
        funding_height,
        spend_txid: tx.compute_txid().to_string(),
        spend_height,
        spend_confs,
        kind,
        spend_tx_hex: bitcoin::consensus::encode::serialize_hex(&tx),
    }))
}

/// Safety margin past a swap's LAST timelock before a followed record with
/// no visible funds may be aged out (docs/design/STATE_RECONSTRUCTION.md §4.2):
/// generous enough that no rational continuation exists, and at least the
/// finality budget so a reorg cannot un-pass it. `0` on regtest, matching
/// the `action_margins` house style (tests jump clocks).
pub fn age_out_margin_secs(network: Network, needed_confs: u32, target_spacing_secs: u32) -> u64 {
    if network == Network::Regtest {
        return 0;
    }
    86_400u64.max(6 * u64::from(needed_confs.max(1)) * u64::from(target_spacing_secs))
}

/// What the NODE WALLET's own history can prove about a swap leg (#171).
/// POSITIVE-ONLY: the backup-session contract shares the wallet across a
/// merchant's machines, so this sees every transaction the merchant SIDE
/// made — but never the counterparty's. Absence proves nothing.
#[derive(Debug, Clone)]
pub enum WalletEvidence {
    /// The leg's funding output was spent, and the wallet holds the spending
    /// tx (our claim/refund — it pays the wallet), fully classified.
    Spent(SpentLeg),
    /// The wallet FUNDED this leg (our own send) — the pointer survives the
    /// output being spent, unlike any live-UTXO read. Liveness is NOT
    /// implied: the caller must ask the chain (`get_txout`) whether the
    /// output still exists; a vanished pointer is an unambiguous spend
    /// (depth unknowable without the spending tx).
    FundingPointer { outpoint: OutPoint, height: u64 },
}

/// Extract wallet evidence for one leg from the wallet's decoded
/// transactions ([`crate::chain::ChainBackend::wallet_txs_since`]).
///
/// Three positive shapes, strongest first:
/// - a wallet tx SPENDS a funding we can also see the wallet make → full
///   [`SpentLeg`] (kind from the witness, both heights known);
/// - a wallet tx is OUR CLAIM of a counterparty-funded leg (`claim_probe`
///   matches an input — v1: the revealed witness script byte-equals ours;
///   v2: the refund leaf byte-equals, or a key-path spend sweeping to the
///   record's negotiated sweep address) → [`SpentLeg`] with the funding
///   outpoint recovered from the claim's input (funding height unknown);
/// - a wallet tx FUNDS the leg (output pays `(spk, amount)`) with no
///   wallet-visible spend → [`WalletEvidence::FundingPointer`].
pub fn classify_leg_wallet(
    wallet_txs: &[(bitcoin::Transaction, u64)],
    tip: u64,
    spk: &ScriptBuf,
    amount_sat: u64,
    classify_spend: &dyn Fn(&[Vec<u8>]) -> SpendKind,
    claim_probe: &dyn Fn(&bitcoin::Transaction, usize) -> bool,
) -> Option<WalletEvidence> {
    let confs_of = |height: u64| -> u64 {
        if height > 0 && tip >= height {
            tip - height + 1
        } else {
            0
        }
    };
    // Fundings the wallet itself made.
    let mut fundings: Vec<(OutPoint, u64)> = Vec::new();
    for (tx, height) in wallet_txs {
        for (vout, out) in tx.output.iter().enumerate() {
            if out.script_pubkey == *spk && out.value.to_sat() == amount_sat {
                fundings.push((
                    OutPoint {
                        txid: tx.compute_txid(),
                        vout: vout as u32,
                    },
                    *height,
                ));
            }
        }
    }
    let spent_leg = |outpoint: OutPoint,
                     funding_height: u64,
                     tx: &bitcoin::Transaction,
                     height: u64,
                     witness: Vec<Vec<u8>>| {
        WalletEvidence::Spent(SpentLeg {
            outpoint,
            funding_height,
            spend_txid: tx.compute_txid().to_string(),
            spend_height: height,
            spend_confs: confs_of(height),
            kind: classify_spend(&witness),
            spend_tx_hex: bitcoin::consensus::encode::serialize_hex(tx),
        })
    };
    // Wallet-visible spends of those fundings (our own refunds, and claims
    // of legs we also funded — not a real v1/v2 shape, but cheap to cover).
    for (tx, height) in wallet_txs {
        for input in &tx.input {
            if let Some((op, fh)) = fundings.iter().find(|(op, _)| input.previous_output == *op) {
                let witness: Vec<Vec<u8>> = input.witness.iter().map(|i| i.to_vec()).collect();
                return Some(spent_leg(*op, *fh, tx, *height, witness));
            }
        }
    }
    // Our claim of a COUNTERPARTY-funded leg: the claim pays our wallet, so
    // it is a wallet tx; the probe identifies which input spends OUR leg,
    // and its prevout IS the funding outpoint we never saw live.
    for (tx, height) in wallet_txs {
        for (idx, input) in tx.input.iter().enumerate() {
            if claim_probe(tx, idx) {
                let witness: Vec<Vec<u8>> = input.witness.iter().map(|i| i.to_vec()).collect();
                return Some(spent_leg(input.previous_output, 0, tx, *height, witness));
            }
        }
    }
    fundings
        .into_iter()
        .next()
        .map(|(outpoint, height)| WalletEvidence::FundingPointer { outpoint, height })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::{ChainParams, BTC_REGTEST};
    use crate::taproot::TaprootLeg;
    use anyhow::Result;
    use bitcoin::absolute::LockTime;
    use bitcoin::hashes::{sha256, Hash};
    use bitcoin::secp256k1::{Keypair, Secp256k1};
    use bitcoin::transaction::Version;
    use bitcoin::{Amount, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Txid, Witness};

    fn hash_of(preimage: &[u8; 32]) -> [u8; 32] {
        sha256::Hash::hash(preimage).to_byte_array()
    }

    // ---- authentic (really signed) spend fixtures ----------------------
    //
    // `witness_authentic` verifies real signatures, so the classification
    // tests below build real ones: a v1 HTLC redeemed/refunded with the
    // keys the script names, and a v2 leg key-path-redeemed with the
    // (tweaked) internal key / script-path-refunded with the refund key.

    struct V1Fixture {
        htlc: crate::htlc::Htlc,
        redeem: Keypair,
        refund: Keypair,
        preimage: [u8; 32],
    }

    fn v1_fixture() -> (V1Fixture, Secp256k1<bitcoin::secp256k1::All>) {
        let secp = Secp256k1::new();
        let redeem = Keypair::from_seckey_slice(&secp, &[0x11; 32]).unwrap();
        let refund = Keypair::from_seckey_slice(&secp, &[0x22; 32]).unwrap();
        let preimage = [9u8; 32];
        let htlc = crate::htlc::Htlc::new(
            hash_of(&preimage),
            bitcoin::secp256k1::PublicKey::from_keypair(&redeem),
            bitcoin::secp256k1::PublicKey::from_keypair(&refund),
            1_780_000_000,
        )
        .unwrap();
        (
            V1Fixture {
                htlc,
                redeem,
                refund,
                preimage,
            },
            secp,
        )
    }

    /// Sign input 0 of `tx` (spending `funding`'s P2WSH output) with `key`
    /// and lay down the v1 witness `[sig, pubkey, <branch items…>, script]`.
    fn sign_v1(
        secp: &Secp256k1<bitcoin::secp256k1::All>,
        fx: &V1Fixture,
        key: &Keypair,
        value: u64,
        branch: &[Vec<u8>],
        mut tx: Transaction,
    ) -> Transaction {
        use bitcoin::sighash::EcdsaSighashType;
        let script = fx.htlc.witness_script();
        let sighash = SighashCache::new(&tx)
            .p2wsh_signature_hash(0, &script, Amount::from_sat(value), EcdsaSighashType::All)
            .unwrap();
        let sig = secp.sign_ecdsa(
            &Message::from_digest(sighash.to_byte_array()),
            &key.secret_key(),
        );
        let mut sig_bytes = sig.serialize_der().to_vec();
        sig_bytes.push(EcdsaSighashType::All as u8);
        let mut items = vec![
            sig_bytes,
            bitcoin::secp256k1::PublicKey::from_keypair(key)
                .serialize()
                .to_vec(),
        ];
        items.extend_from_slice(branch);
        items.push(script.to_bytes());
        let mut w = Witness::new();
        for item in items {
            w.push(item);
        }
        tx.input[0].witness = w;
        tx
    }

    /// Key-path (cooperative) spend of a v2 leg, signed by the tweaked
    /// internal key — what only the MuSig2 aggregate can do for real.
    fn keypath_spend(
        secp: &Secp256k1<bitcoin::secp256k1::All>,
        leg: &TaprootLeg,
        internal: &Keypair,
        funding: &Transaction,
        value: u64,
    ) -> Transaction {
        use bitcoin::key::TapTweak;
        use bitcoin::sighash::TapSighashType;
        let mut tx = spend_tx(funding, &[]);
        let spend_info = leg.spend_info(secp).unwrap();
        let prevout = leg.funding_txout(secp, value).unwrap();
        let sighash = SighashCache::new(&tx)
            .taproot_key_spend_signature_hash(
                0,
                &Prevouts::All(&[prevout]),
                TapSighashType::Default,
            )
            .unwrap();
        let tweaked = internal.tap_tweak(secp, spend_info.merkle_root());
        let sig = secp.sign_schnorr(
            &Message::from_digest(sighash.to_byte_array()),
            &tweaked.to_keypair(),
        );
        crate::taproot::attach_keypath_signature(&mut tx, sig);
        tx
    }

    #[test]
    fn witness_authentic_v1_accepts_real_signatures_and_rejects_forgeries() {
        let (fx, secp) = v1_fixture();
        let spk = fx.htlc.script_pubkey();
        let value = 100_000;
        let f = funding_tx(&spk, value);
        // Real redeem by the redeem key: authentic, and classified Redeem.
        let redeem = sign_v1(
            &secp,
            &fx,
            &fx.redeem,
            value,
            &[fx.preimage.to_vec(), vec![1]],
            spend_tx(&f, &[]),
        );
        assert!(witness_authentic(&spk, value, &redeem, 0));
        let w: Vec<Vec<u8>> = redeem.input[0].witness.iter().map(<[u8]>::to_vec).collect();
        assert_eq!(classify_v1_spend(&w, &fx.htlc.hash_h), SpendKind::Redeem);
        // Real refund by the refund key.
        let refund = sign_v1(&secp, &fx, &fx.refund, value, &[vec![]], spend_tx(&f, &[]));
        assert!(witness_authentic(&spk, value, &refund, 0));
        // Wrong amount → sighash differs → not authentic.
        assert!(!witness_authentic(&spk, value + 1, &refund, 0));
        // A stranger's key (not pushed by the script) signing the refund
        // shape — exactly the forgery a lying provider could produce.
        let stranger = Keypair::from_seckey_slice(&secp, &[0x33; 32]).unwrap();
        let forged = sign_v1(&secp, &fx, &stranger, value, &[vec![]], spend_tx(&f, &[]));
        assert!(!witness_authentic(&spk, value, &forged, 0));
        // Garbage signature bytes with the right key and script.
        let mut garbage = refund.clone();
        let mut w = Witness::new();
        w.push(vec![0x30u8; 71]);
        for item in refund.input[0].witness.iter().skip(1) {
            w.push(item);
        }
        garbage.input[0].witness = w;
        assert!(!witness_authentic(&spk, value, &garbage, 0));
        // Wrong witness script (does not hash to the program).
        let mut wrong_script = refund.clone();
        let mut w = Witness::new();
        for item in refund.input[0].witness.iter().take(3) {
            w.push(item);
        }
        w.push(vec![0xAA; 40]);
        wrong_script.input[0].witness = w;
        assert!(!witness_authentic(&spk, value, &wrong_script, 0));
    }

    #[test]
    fn witness_authentic_v2_accepts_real_keypath_and_refund_and_rejects_forgeries() {
        let (leg, secp) = sample_leg();
        let internal = Keypair::from_seckey_slice(&secp, &[0x24; 32]).unwrap();
        let refund_key = Keypair::from_seckey_slice(&secp, &[0x42; 32]).unwrap();
        let spk = leg.script_pubkey(&secp).unwrap();
        let value = 100_000;
        let f = funding_tx(&spk, value);
        // Real key-path spend by the tweaked internal key.
        let redeem = keypath_spend(&secp, &leg, &internal, &f, value);
        assert!(witness_authentic(&spk, value, &redeem, 0));
        let w: Vec<Vec<u8>> = redeem.input[0].witness.iter().map(<[u8]>::to_vec).collect();
        assert_eq!(
            classify_v2_spend(&w, &leg.refund_script()),
            SpendKind::Redeem
        );
        // The fabricated 64 zero bytes a lying server could invent.
        let forged = spend_tx(&f, &[vec![0u8; 64]]);
        assert!(!witness_authentic(&spk, value, &forged, 0));
        // Key-path signed by the WRONG key (the refund key, untweaked).
        let wrong = keypath_spend(&secp, &leg, &refund_key, &f, value);
        assert!(!witness_authentic(&spk, value, &wrong, 0));
        // Real script-path refund via the production builder.
        let op = bitcoin::OutPoint {
            txid: f.compute_txid(),
            vout: 0,
        };
        let dest = ScriptBuf::new_p2wsh(&ScriptBuf::from(vec![0x51u8]).wscript_hash());
        let refund =
            crate::taproot::build_refund_tx(&secp, &leg, op, value, dest.clone(), 500, &refund_key)
                .unwrap();
        assert!(witness_authentic(&spk, value, &refund, 0));
        let w: Vec<Vec<u8>> = refund.input[0].witness.iter().map(<[u8]>::to_vec).collect();
        assert_eq!(
            classify_v2_spend(&w, &leg.refund_script()),
            SpendKind::Refund
        );
        // Same leaf + control block, garbage signature.
        let mut forged_refund = refund.clone();
        let mut w = Witness::new();
        w.push(vec![0u8; 64]);
        for item in refund.input[0].witness.iter().skip(1) {
            w.push(item);
        }
        forged_refund.input[0].witness = w;
        assert!(!witness_authentic(&spk, value, &forged_refund, 0));
        // A leaf the control block does not commit to.
        let mut alien_leaf = refund.clone();
        let mut w = Witness::new();
        w.push(refund.input[0].witness.iter().next().unwrap());
        w.push(vec![0x51u8]);
        w.push(refund.input[0].witness.iter().nth(2).unwrap());
        alien_leaf.input[0].witness = w;
        assert!(!witness_authentic(&spk, value, &alien_leaf, 0));
    }

    #[test]
    fn v1_witness_classification() {
        let s = [7u8; 32];
        let h = hash_of(&s);
        let script = vec![0xAAu8; 40];
        // Redeem: [sig, pubkey, s, 0x01, script] — preimage verifies.
        let redeem = vec![
            vec![0x30; 71],
            vec![0x02; 33],
            s.to_vec(),
            vec![1],
            script.clone(),
        ];
        assert_eq!(classify_v1_spend(&redeem, &h), SpendKind::Redeem);
        // Refund: [sig, pubkey, <>, script] — empty OP_ELSE selector.
        let refund = vec![vec![0x30; 71], vec![0x02; 33], vec![], script.clone()];
        assert_eq!(classify_v1_spend(&refund, &h), SpendKind::Refund);
        // A 32-byte item that does NOT hash to H is not a redeem; the
        // 4-item shape with a non-empty selector is not a refund either.
        let bogus = vec![vec![0x30; 71], vec![0x02; 33], vec![0x55; 32], script];
        assert_eq!(classify_v1_spend(&bogus, &h), SpendKind::Unknown);
        assert_eq!(classify_v1_spend(&[vec![0u8; 64]], &h), SpendKind::Unknown);
    }

    fn sample_leg() -> (TaprootLeg, Secp256k1<bitcoin::secp256k1::All>) {
        let secp = Secp256k1::new();
        let internal = Keypair::from_seckey_slice(&secp, &[0x24; 32])
            .unwrap()
            .x_only_public_key()
            .0;
        let refund = Keypair::from_seckey_slice(&secp, &[0x42; 32])
            .unwrap()
            .x_only_public_key()
            .0;
        (
            TaprootLeg::new(internal, refund, 1_780_000_000).unwrap(),
            secp,
        )
    }

    #[test]
    fn v2_witness_classification() {
        let (leg, _secp) = sample_leg();
        let leaf = leg.refund_script();
        // Key-path spend: exactly one 64-byte sig (SIGHASH_DEFAULT)…
        assert_eq!(
            classify_v2_spend(&[vec![0u8; 64]], &leaf),
            SpendKind::Redeem
        );
        // …or 65 with an explicit sighash byte.
        assert_eq!(
            classify_v2_spend(&[vec![0u8; 65]], &leaf),
            SpendKind::Redeem
        );
        // Script-path refund: [sig, leaf, control] with OUR leaf bytes.
        let refund = vec![vec![0u8; 64], leaf.as_bytes().to_vec(), vec![0xC0; 33]];
        assert_eq!(classify_v2_spend(&refund, &leaf), SpendKind::Refund);
        // A different leaf script is not our refund.
        let alien = vec![vec![0u8; 64], vec![0x51], vec![0xC0; 33]];
        assert_eq!(classify_v2_spend(&alien, &leaf), SpendKind::Unknown);
        // A fabricated "sig" of the wrong size is nothing.
        assert_eq!(
            classify_v2_spend(&[vec![0u8; 63]], &leaf),
            SpendKind::Unknown
        );
    }

    // ---- classify_leg over a canned-history mock backend -------------------

    struct MockBackend {
        history: Option<Vec<(String, i64)>>,
        txs: Vec<Transaction>,
        tip: u64,
        /// What `find_spend_tx` (the tier-L block scan) reports.
        spend: Option<(Transaction, u64)>,
    }

    impl ChainBackend for MockBackend {
        fn params(&self) -> &ChainParams {
            &BTC_REGTEST
        }
        fn verify_chain(&self) -> Result<()> {
            Ok(())
        }
        fn broadcast(&self, _tx: &Transaction) -> Result<Txid> {
            anyhow::bail!("mock")
        }
        fn get_txout(
            &self,
            _outpoint: &bitcoin::OutPoint,
            _expected_spk: &ScriptBuf,
        ) -> Result<Option<crate::chain::TxOutInfo>> {
            anyhow::bail!("mock")
        }
        fn find_funding(
            &self,
            _spk: &ScriptBuf,
        ) -> Result<Option<(bitcoin::OutPoint, crate::chain::TxOutInfo)>> {
            anyhow::bail!("mock")
        }
        fn find_vout(&self, _txid: &str, _spk_hex: &str) -> Result<u32> {
            anyhow::bail!("mock")
        }
        fn find_spend_witness(
            &self,
            _outpoint: &bitcoin::OutPoint,
            _watch_spk: &ScriptBuf,
            _from_height: u64,
        ) -> Result<Option<Vec<Vec<u8>>>> {
            anyhow::bail!("mock")
        }
        fn find_spend_tx(
            &self,
            _outpoint: &bitcoin::OutPoint,
            _watch_spk: &ScriptBuf,
            _from_height: u64,
        ) -> Result<Option<(Transaction, u64)>> {
            Ok(self.spend.clone())
        }
        fn spk_history(&self, _spk: &ScriptBuf) -> Result<Option<Vec<(String, i64)>>> {
            Ok(self.history.clone())
        }
        fn fetch_tx(&self, txid: &str) -> Result<Option<Transaction>> {
            let want = Txid::from_str(txid)?;
            Ok(self.txs.iter().find(|t| t.compute_txid() == want).cloned())
        }
        fn tip_height(&self) -> Result<u64> {
            Ok(self.tip)
        }
        fn tip_median_time(&self) -> Result<u64> {
            anyhow::bail!("mock")
        }
        fn tx_confirmations(&self, _txid: &str, _spk: Option<&ScriptBuf>) -> Result<u64> {
            anyhow::bail!("mock")
        }
        fn fee_rate_for(&self, _conf_target: u16, _conservative: bool) -> Result<u64> {
            anyhow::bail!("mock")
        }
        fn wallet_new_address(&self) -> Result<String> {
            anyhow::bail!("mock")
        }
        fn wallet_balance(&self) -> Result<u64> {
            anyhow::bail!("mock")
        }
        fn wallet_send(
            &self,
            _address: &str,
            _amount_sat: u64,
            _fee: crate::chain::SendFee,
        ) -> Result<String> {
            anyhow::bail!("mock")
        }
    }

    fn spk() -> ScriptBuf {
        ScriptBuf::new_p2wsh(&ScriptBuf::from(vec![0x51u8]).wscript_hash())
    }

    fn funding_tx(spk: &ScriptBuf, amount: u64) -> Transaction {
        Transaction {
            version: Version::TWO,
            lock_time: LockTime::ZERO,
            input: vec![TxIn {
                previous_output: bitcoin::OutPoint {
                    txid: Txid::from_str(&"33".repeat(32)).unwrap(),
                    vout: 0,
                },
                script_sig: ScriptBuf::new(),
                sequence: Sequence::MAX,
                witness: Witness::default(),
            }],
            output: vec![TxOut {
                value: Amount::from_sat(amount),
                script_pubkey: spk.clone(),
            }],
        }
    }

    fn spend_tx(funding: &Transaction, witness_items: &[Vec<u8>]) -> Transaction {
        let mut w = Witness::new();
        for item in witness_items {
            w.push(item.clone());
        }
        Transaction {
            version: Version::TWO,
            lock_time: LockTime::ZERO,
            input: vec![TxIn {
                previous_output: bitcoin::OutPoint {
                    txid: funding.compute_txid(),
                    vout: 0,
                },
                script_sig: ScriptBuf::new(),
                sequence: Sequence::MAX,
                witness: w,
            }],
            output: vec![TxOut {
                value: Amount::from_sat(90_000),
                script_pubkey: ScriptBuf::new(),
            }],
        }
    }

    fn kind_always(kind: SpendKind) -> impl Fn(&[Vec<u8>]) -> SpendKind {
        move |_| kind
    }

    #[test]
    fn leg_unfunded_and_tier_l() {
        let spk = spk();
        let none = MockBackend {
            history: None,
            txs: vec![],
            tip: 100,
            spend: None,
        };
        assert!(
            classify_leg(&none, &spk, 100_000, &kind_always(SpendKind::Unknown))
                .unwrap()
                .is_none(),
            "history-less backend reads as tier L"
        );
        let empty = MockBackend {
            history: Some(vec![]),
            txs: vec![],
            tip: 100,
            spend: None,
        };
        assert!(matches!(
            classify_leg(&empty, &spk, 100_000, &kind_always(SpendKind::Unknown))
                .unwrap()
                .unwrap(),
            LegClass::Unfunded
        ));
    }

    #[test]
    fn leg_funded_live() {
        let spk = spk();
        let f = funding_tx(&spk, 100_000);
        let backend = MockBackend {
            history: Some(vec![(f.compute_txid().to_string(), 90)]),
            txs: vec![f.clone()],
            tip: 100,
            spend: None,
        };
        match classify_leg(&backend, &spk, 100_000, &kind_always(SpendKind::Unknown))
            .unwrap()
            .unwrap()
        {
            LegClass::Funded {
                outpoint,
                height,
                confs,
            } => {
                assert_eq!(outpoint.txid, f.compute_txid());
                assert_eq!(height, 90);
                assert_eq!(confs, 11);
            }
            other => panic!("expected Funded, got {other:?}"),
        }
    }

    #[test]
    fn leg_wrong_amount_is_unfunded() {
        let spk = spk();
        let f = funding_tx(&spk, 55_555); // pays the spk, but not the agreed amount
        let backend = MockBackend {
            history: Some(vec![(f.compute_txid().to_string(), 90)]),
            txs: vec![f],
            tip: 100,
            spend: None,
        };
        assert!(matches!(
            classify_leg(&backend, &spk, 100_000, &kind_always(SpendKind::Unknown))
                .unwrap()
                .unwrap(),
            LegClass::Unfunded
        ));
    }

    #[test]
    fn leg_spent_classifies_front_to_back() {
        // The field bug's shape: funding AND spend are both history — a
        // live-UTXO read sees nothing, the classifier sees the whole story.
        let (fx, secp) = v1_fixture();
        let h = fx.htlc.hash_h;
        let spk = fx.htlc.script_pubkey();
        let f = funding_tx(&spk, 100_000);
        let sp = sign_v1(
            &secp,
            &fx,
            &fx.redeem,
            100_000,
            &[fx.preimage.to_vec(), vec![1]],
            spend_tx(&f, &[]),
        );
        let backend = MockBackend {
            history: Some(vec![
                (f.compute_txid().to_string(), 90),
                (sp.compute_txid().to_string(), 95),
            ]),
            txs: vec![f.clone(), sp.clone()],
            tip: 100,
            spend: None,
        };
        let classify = |w: &[Vec<u8>]| classify_v1_spend(w, &h);
        match classify_leg(&backend, &spk, 100_000, &classify)
            .unwrap()
            .unwrap()
        {
            LegClass::Spent(leg) => {
                assert_eq!(leg.outpoint.txid, f.compute_txid());
                assert_eq!(leg.spend_txid, sp.compute_txid().to_string());
                assert_eq!(leg.spend_height, 95);
                assert_eq!(leg.spend_confs, 6);
                assert_eq!(leg.kind, SpendKind::Redeem);
                assert_eq!(leg.funding_height, 90);
            }
            other => panic!("expected Spent, got {other:?}"),
        }
        // The same history with a FORGED spend (a refund-shaped witness
        // signed by a key the script never names) is a spend of Unknown
        // kind — never a terminal signal (security review 2026-09-09 #7).
        let stranger = Keypair::from_seckey_slice(&secp, &[0x33; 32]).unwrap();
        let forged = sign_v1(&secp, &fx, &stranger, 100_000, &[vec![]], spend_tx(&f, &[]));
        let backend = MockBackend {
            history: Some(vec![
                (f.compute_txid().to_string(), 90),
                (forged.compute_txid().to_string(), 95),
            ]),
            txs: vec![f.clone(), forged.clone()],
            tip: 100,
            spend: None,
        };
        match classify_leg(&backend, &spk, 100_000, &classify)
            .unwrap()
            .unwrap()
        {
            LegClass::Spent(leg) => assert_eq!(leg.kind, SpendKind::Unknown),
            other => panic!("expected Spent(Unknown), got {other:?}"),
        }
    }

    #[test]
    fn leg_mempool_spend_has_zero_confs() {
        let spk = spk();
        let f = funding_tx(&spk, 100_000);
        let sp = spend_tx(&f, &[vec![0u8; 64]]);
        let backend = MockBackend {
            history: Some(vec![
                (f.compute_txid().to_string(), 90),
                (sp.compute_txid().to_string(), -1), // unconfirmed-parents marker
            ]),
            txs: vec![f, sp],
            tip: 100,
            spend: None,
        };
        match classify_leg(&backend, &spk, 100_000, &kind_always(SpendKind::Redeem))
            .unwrap()
            .unwrap()
        {
            LegClass::Spent(leg) => {
                assert_eq!(leg.spend_height, 0);
                assert_eq!(leg.spend_confs, 0);
            }
            other => panic!("expected Spent, got {other:?}"),
        }
    }

    #[test]
    fn missing_history_tx_is_an_error_not_evidence() {
        let spk = spk();
        let f = funding_tx(&spk, 100_000);
        let backend = MockBackend {
            history: Some(vec![(f.compute_txid().to_string(), 90)]),
            txs: vec![], // the referenced tx is not retrievable
            tip: 100,
            spend: None,
        };
        assert!(classify_leg(&backend, &spk, 100_000, &kind_always(SpendKind::Unknown)).is_err());
    }

    // ---- classify_spent_by_scan (tier-L block-scan fallback) ---------------

    #[test]
    fn scan_recovers_spend_of_known_outpoint() {
        let spk = spk();
        let f = funding_tx(&spk, 100_000);
        let sp = spend_tx(&f, &[vec![0u8; 64]]); // v2 key-path shape
        let op = bitcoin::OutPoint {
            txid: f.compute_txid(),
            vout: 0,
        };
        let backend = MockBackend {
            history: None, // tier L — classify_leg would give up here
            txs: vec![],
            tip: 100,
            spend: Some((sp.clone(), 95)),
        };
        let leg = classify_spent_by_scan(
            &backend,
            &op,
            &spk,
            100_000,
            0,
            80,
            &kind_always(SpendKind::Redeem),
        )
        .unwrap()
        .expect("spend recovered from the block scan");
        assert_eq!(leg.spend_txid, sp.compute_txid().to_string());
        assert_eq!(leg.spend_height, 95);
        assert_eq!(leg.spend_confs, 6);
        // The pointer facts are recovered, but a witness the swap keys did
        // not sign never classifies (the protocol classifier is not even
        // consulted): the scan's view may be lying.
        assert_eq!(leg.kind, SpendKind::Unknown);
        assert_eq!(
            leg.spend_tx_hex,
            bitcoin::consensus::encode::serialize_hex(&sp)
        );
    }

    #[test]
    fn scan_mempool_spend_has_zero_confs() {
        let spk = spk();
        let f = funding_tx(&spk, 100_000);
        let sp = spend_tx(&f, &[vec![0u8; 64]]);
        let op = bitcoin::OutPoint {
            txid: f.compute_txid(),
            vout: 0,
        };
        let backend = MockBackend {
            history: None,
            txs: vec![],
            tip: 100,
            spend: Some((sp, 0)), // still in the mempool
        };
        let leg = classify_spent_by_scan(
            &backend,
            &op,
            &spk,
            100_000,
            0,
            80,
            &kind_always(SpendKind::Redeem),
        )
        .unwrap()
        .unwrap();
        assert_eq!(leg.spend_confs, 0, "unconfirmed spend must never read deep");
    }

    #[test]
    fn scan_rejects_tx_not_spending_the_outpoint() {
        // A lying/buggy view returning a tx that doesn't spend the watched
        // outpoint is not evidence.
        let spk = spk();
        let f = funding_tx(&spk, 100_000);
        let other = funding_tx(&spk, 50_000);
        let sp = spend_tx(&other, &[vec![0u8; 64]]);
        let op = bitcoin::OutPoint {
            txid: f.compute_txid(),
            vout: 0,
        };
        let backend = MockBackend {
            history: None,
            txs: vec![],
            tip: 100,
            spend: Some((sp, 95)),
        };
        assert!(classify_spent_by_scan(
            &backend,
            &op,
            &spk,
            100_000,
            0,
            80,
            &kind_always(SpendKind::Redeem)
        )
        .unwrap()
        .is_none());
    }

    #[test]
    fn scan_none_when_backend_cannot_scan() {
        let spk = spk();
        let op = bitcoin::OutPoint {
            txid: Txid::from_str(&"44".repeat(32)).unwrap(),
            vout: 0,
        };
        let backend = MockBackend {
            history: None,
            txs: vec![],
            tip: 100,
            spend: None,
        };
        assert!(classify_spent_by_scan(
            &backend,
            &op,
            &spk,
            100_000,
            0,
            0,
            &kind_always(SpendKind::Redeem)
        )
        .unwrap()
        .is_none());
    }

    #[test]
    fn age_out_margin_shape() {
        assert_eq!(age_out_margin_secs(Network::Regtest, 6, 600), 0);
        // Mainnet floor is a day…
        assert_eq!(age_out_margin_secs(Network::Mainnet, 1, 600), 86_400);
        // …and scales with the finality budget on slow/deep configs.
        assert_eq!(age_out_margin_secs(Network::Mainnet, 30, 600), 108_000);
    }

    // ---- wallet-assisted evidence (#171) -----------------------------------

    fn no_probe(_tx: &Transaction, _idx: usize) -> bool {
        false
    }

    #[test]
    fn wallet_funding_only_yields_pointer() {
        let spk = spk();
        let f = funding_tx(&spk, 100_000);
        let txs = vec![(f.clone(), 90u64)];
        match classify_leg_wallet(
            &txs,
            100,
            &spk,
            100_000,
            &kind_always(SpendKind::Unknown),
            &no_probe,
        ) {
            Some(WalletEvidence::FundingPointer { outpoint, height }) => {
                assert_eq!(outpoint.txid, f.compute_txid());
                assert_eq!(height, 90);
            }
            other => panic!("expected FundingPointer, got {other:?}"),
        }
        // Wrong amount → the wallet proves nothing about THIS leg.
        assert!(classify_leg_wallet(
            &txs,
            100,
            &spk,
            55_555,
            &kind_always(SpendKind::Unknown),
            &no_probe
        )
        .is_none());
    }

    #[test]
    fn wallet_funding_plus_spend_is_fully_classified() {
        let spk = spk();
        let f = funding_tx(&spk, 100_000);
        let sp = spend_tx(
            &f,
            &[vec![0x30; 71], vec![0x02; 33], vec![], vec![0xAA; 40]],
        );
        let txs = vec![(f.clone(), 90u64), (sp.clone(), 95u64)];
        match classify_leg_wallet(
            &txs,
            100,
            &spk,
            100_000,
            &kind_always(SpendKind::Refund),
            &no_probe,
        ) {
            Some(WalletEvidence::Spent(leg)) => {
                assert_eq!(leg.outpoint.txid, f.compute_txid());
                assert_eq!(leg.spend_txid, sp.compute_txid().to_string());
                assert_eq!(leg.funding_height, 90);
                assert_eq!(leg.spend_confs, 6);
                assert_eq!(leg.kind, SpendKind::Refund);
            }
            other => panic!("expected Spent, got {other:?}"),
        }
    }

    #[test]
    fn wallet_claim_of_counterparty_funding_recovers_the_outpoint() {
        // The user's ghost shape: the counterparty funded the leg (invisible
        // to the wallet), OUR claim swept it to the wallet — the claim's
        // input IS the funding outpoint we never saw.
        let spk = spk();
        let s = [9u8; 32];
        let h = hash_of(&s);
        let ws = vec![0xAB; 40]; // the leg's witness script bytes (probe target)
        let foreign_funding = funding_tx(&spk, 100_000); // NOT in the wallet set
        let claim = spend_tx(
            &foreign_funding,
            &[
                vec![0x30; 71],
                vec![0x02; 33],
                s.to_vec(),
                vec![1],
                ws.clone(),
            ],
        );
        let txs = vec![(claim.clone(), 95u64)];
        let classify = |w: &[Vec<u8>]| classify_v1_spend(w, &h);
        let probe = |tx: &Transaction, idx: usize| {
            tx.input[idx]
                .witness
                .last()
                .map(|w| w == ws.as_slice())
                .unwrap_or(false)
        };
        match classify_leg_wallet(&txs, 100, &spk, 100_000, &classify, &probe) {
            Some(WalletEvidence::Spent(leg)) => {
                assert_eq!(leg.outpoint.txid, foreign_funding.compute_txid());
                assert_eq!(leg.funding_height, 0, "funding height unknowable");
                assert_eq!(leg.kind, SpendKind::Redeem, "preimage verified");
                assert_eq!(leg.spend_confs, 6);
            }
            other => panic!("expected Spent via claim probe, got {other:?}"),
        }
        // Without the probe the wallet proves nothing (positive-only).
        assert!(classify_leg_wallet(&txs, 100, &spk, 100_000, &classify, &no_probe).is_none());
    }
}
