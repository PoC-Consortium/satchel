//! One parameterized fee-bump policy, shared by every bump site. The *strategy*
//! per transaction type is intrinsic (you cannot RBF a v2 cooperative redeem) and
//! is not a user choice; the numeric *parameters* are. Defaults reproduce the
//! historical hardcoded consts exactly, so behaviour is unchanged until overridden.
//!
//! This module is deliberately self-contained: it does not reach into
//! [`crate::engine`]. The policy ceiling lives here as [`MAX_FEERATE_CEILING`]; the
//! protocol-level redeem-feerate bound (`engine::MAX_REDEEM_FEERATE`) is a separate
//! concept that happens to equal the same number.

use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};

/// Hard ceiling for [`FeeBumpPolicy::max_feerate_sat_vb`] (sat/vB). Equal to the
/// backend estimator's own sanity clamp (`chain.rs` `MAX_SAT_PER_VB`), so the knob
/// can only *tighten* the system-wide ceiling, never raise it past what the
/// estimator can report. The operator may set any value in `1..=MAX_FEERATE_CEILING`.
pub const MAX_FEERATE_CEILING: u64 = 500;

/// Value-at-risk cap (percent of the amount being claimed/recovered) applied to
/// every bump target — a bump's absolute fee never exceeds this share of the
/// leg value. Eclair's rule: "it wouldn't make sense to pay more in fees than
/// the amount we're trying to claim on-chain." Hardcoded (not a policy knob) on
/// purpose: it is a safety invariant, not a tuning parameter — the market term
/// in [`FeeBumpPolicy::target_feerate`] is what keeps real fees low; this is the
/// last-resort insanity backstop. 100 = cap at the full claim (never blocks a
/// legitimate confirmation; still stops paying more than the claim is worth).
pub const FEE_CAP_PCT: u64 = 100;

/// Upper bound on the RBF escalation step (percent), a pure sanity guard.
const MAX_STEP_PCT: u64 = 1000;

/// Upper bound on the funding-reservation and committed-redeem multipliers, a
/// pure sanity guard. Well above any sensible value (the redeem feerate then
/// clamps to `MAX_FEERATE_CEILING` anyway), but bounded so a fat-fingered RPC
/// value can't overflow the `rate × mult` products at the use sites.
const MAX_MULT: u64 = 1000;

/// The unified, configurable fee-bump policy. `Copy` (small, all-scalar) so bump
/// sites take it by value freely.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct FeeBumpPolicy {
    /// Absolute ceiling (sat/vB) applied to every local bump path, including the
    /// RBF escalators (which were uncapped before this policy existed). Bounded by
    /// [`MAX_FEERATE_CEILING`]. Default 500.
    pub max_feerate_sat_vb: u64,
    /// **Deprecated / inert.** No longer read by any fee path — every spend and
    /// bump is market-derived ([`Self::target_feerate`]); the old flat 1000-sat
    /// floor was removed (it overrode the market price on quiet mempools).
    /// Retained as a field only so policies persisted before its removal still
    /// deserialize under `deny_unknown_fields`. Not exposed by `get/setfeepolicy`.
    pub min_fee_sat: u64,
    pub funding: FundingPolicy,
    pub redeem: RedeemPolicy,
    pub refund: RefundPolicy,
}

/// Funding / lock bump parameters (the nurse: RBF on v1, CPFP-via-change on v2).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct FundingPolicy {
    /// The funds gate (`Engine::ensure_can_fund`) reserves
    /// `reservation_mult × live estimate` of balance as bump headroom. The nurse
    /// then chases current market, bounded by this reservation (`× old_feerate`)
    /// and by [`FeeBumpPolicy::max_feerate_sat_vb`]. Default 3 (was
    /// `FUNDING_BUMP_FEERATE_MULT`).
    pub reservation_mult: u64,
}

/// Claim / redeem bump parameters.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct RedeemPolicy {
    /// v2: multiplier on the live feerate baked into the adaptor signature at
    /// funding time. Fixed per swap at funding → applies to NEW swaps only.
    /// Default 1 (commit at market; the CPFP child chases market up if it rises
    /// while the redeem is pending). Raise it for an unattended floor — a higher
    /// commit confirms on its own even if the scheduler never runs to CPFP-bump.
    pub committed_mult: u64,
    /// v1: percent the fee escalates per scheduler tick. Default 50.
    pub step_pct: u64,
}

/// Refund bump parameters (RBF escalation, both protocols).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct RefundPolicy {
    /// Percent the fee escalates per scheduler tick. Default 50.
    pub step_pct: u64,
}

impl Default for FeeBumpPolicy {
    fn default() -> Self {
        Self {
            max_feerate_sat_vb: MAX_FEERATE_CEILING,
            min_fee_sat: crate::swap::MIN_SPEND_FEE_SAT,
            funding: FundingPolicy::default(),
            redeem: RedeemPolicy::default(),
            refund: RefundPolicy::default(),
        }
    }
}

impl Default for FundingPolicy {
    fn default() -> Self {
        Self {
            reservation_mult: 3,
        }
    }
}

impl Default for RedeemPolicy {
    fn default() -> Self {
        Self {
            committed_mult: 1,
            step_pct: 50,
        }
    }
}

impl Default for RefundPolicy {
    fn default() -> Self {
        Self { step_pct: 50 }
    }
}

impl FeeBumpPolicy {
    /// Reject a nonsensical policy. Called once where the policy enters the engine
    /// (`set_fee_bump`), never on the hot tick path.
    pub fn validated(self) -> Result<Self> {
        ensure!(self.min_fee_sat >= 1, "min_fee_sat must be >= 1");
        ensure!(
            (1..=MAX_FEERATE_CEILING).contains(&self.max_feerate_sat_vb),
            "max_feerate_sat_vb must be 1..={MAX_FEERATE_CEILING} sat/vB (the estimator's hard ceiling)"
        );
        ensure!(
            (1..=MAX_MULT).contains(&self.funding.reservation_mult),
            "funding.reservation_mult must be 1..={MAX_MULT}"
        );
        ensure!(
            (1..=MAX_MULT).contains(&self.redeem.committed_mult),
            "redeem.committed_mult must be 1..={MAX_MULT}"
        );
        ensure!(
            (1..=MAX_STEP_PCT).contains(&self.redeem.step_pct),
            "redeem.step_pct must be 1..={MAX_STEP_PCT}"
        );
        ensure!(
            (1..=MAX_STEP_PCT).contains(&self.refund.step_pct),
            "refund.step_pct must be 1..={MAX_STEP_PCT}"
        );
        Ok(self)
    }

    /// The unified bump **target feerate** (sat/vB) for every nurse — the shape
    /// the funding nurses already used, now applied everywhere. Track the live
    /// market, capped by the value being claimed (`fee_cap_pct`) and the absolute
    /// ceiling (`max_feerate_sat_vb`). Never below 1.
    ///
    /// This replaces the market-blind geometric [`Self::escalate`]: it can never
    /// bid 159 sat/vB into a 1 sat/vB market the way `escalate` did, because the
    /// market term bounds it from above by construction. `value_at_risk` is the
    /// leg amount (claim/recover); `vsize` the spend's virtual size.
    pub fn target_feerate(&self, market_sat_vb: u64, value_at_risk: u64, vsize: u64) -> u64 {
        let value_cap = value_at_risk.saturating_mul(FEE_CAP_PCT) / 100 / vsize.max(1);
        market_sat_vb
            .min(value_cap)
            .min(self.max_feerate_sat_vb)
            .max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A representative spend vsize (≈ v1 redeem) for the fee-policy tests.
    const VSIZE: u64 = 155;

    #[test]
    fn defaults_match_historical_consts() {
        let p = FeeBumpPolicy::default();
        assert_eq!(p.max_feerate_sat_vb, 500);
        assert_eq!(p.min_fee_sat, crate::swap::MIN_SPEND_FEE_SAT);
        assert_eq!(p.funding.reservation_mult, 3);
        // committed_mult lowered 2 → 1: commit at market, CPFP nurse bumps up.
        assert_eq!(p.redeem.committed_mult, 1);
        assert_eq!(p.redeem.step_pct, 50);
        assert_eq!(p.refund.step_pct, 50);
        // Default policy is valid.
        assert!(p.validated().is_ok());
    }

    #[test]
    fn target_feerate_tracks_market_and_caps() {
        let p = FeeBumpPolicy::default(); // ceiling 500, hardcoded FEE_CAP_PCT=100
                                          // Quiet market: target follows the market, never escalates past it.
        assert_eq!(p.target_feerate(1, 206_250, VSIZE), 1);
        assert_eq!(p.target_feerate(8, 206_250, VSIZE), 8);
        // Value-at-risk cap (100% of claim) bites for a small claim: 30_000 sat /
        // 155 vB ≈ 193 sat/vB, so a hot market is clamped to the claim, not 500.
        assert_eq!(p.target_feerate(400, 30_000, VSIZE), 30_000 / VSIZE);
        // Absolute ceiling still binds when the claim is large.
        assert_eq!(p.target_feerate(900, 10_000_000, VSIZE), 500);
    }

    #[test]
    fn validated_rejects_bad_values() {
        let over = FeeBumpPolicy {
            max_feerate_sat_vb: MAX_FEERATE_CEILING + 1,
            ..Default::default()
        };
        assert!(over.validated().is_err());

        let zero = FeeBumpPolicy {
            max_feerate_sat_vb: 0,
            ..Default::default()
        };
        assert!(zero.validated().is_err());

        let mut p = FeeBumpPolicy::default();
        p.funding.reservation_mult = 0;
        assert!(p.validated().is_err());

        let mut p = FeeBumpPolicy::default();
        p.redeem.committed_mult = 0;
        assert!(p.validated().is_err());

        let mut p = FeeBumpPolicy::default();
        p.redeem.step_pct = 0;
        assert!(p.validated().is_err());

        // Multipliers are bounded above so a fat-fingered value can't overflow
        // `rate × mult` at the use sites.
        let mut p = FeeBumpPolicy::default();
        p.redeem.committed_mult = MAX_MULT + 1;
        assert!(p.validated().is_err());

        let mut p = FeeBumpPolicy::default();
        p.funding.reservation_mult = MAX_MULT + 1;
        assert!(p.validated().is_err());
    }

    #[test]
    fn serde_fills_missing_fields_with_defaults() {
        // An empty object → all defaults (serde(default) on every field/struct).
        let p: FeeBumpPolicy = serde_json::from_str("{}").unwrap();
        assert_eq!(p, FeeBumpPolicy::default());

        // A partial object overrides only what it names.
        let p: FeeBumpPolicy = serde_json::from_str(r#"{"max_feerate_sat_vb": 200}"#).unwrap();
        assert_eq!(p.max_feerate_sat_vb, 200);
        assert_eq!(p.funding.reservation_mult, 3); // untouched default

        // round-trip
        let s = serde_json::to_string(&p).unwrap();
        let back: FeeBumpPolicy = serde_json::from_str(&s).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn serde_rejects_unknown_fields() {
        assert!(serde_json::from_str::<FeeBumpPolicy>(r#"{"bogus": 1}"#).is_err());
    }
}
