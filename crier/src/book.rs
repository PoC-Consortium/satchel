//! The reconstructed public orderbook. Pure data structure — no I/O — so the
//! replace/tombstone/expiry semantics are fully unit-testable.
//!
//! Semantics mirror the engine's consumer side (`nostr_offer_cache` +
//! `nostr_revoked:` tombstones in libswap/pactd):
//! - key `(maker, swap_id)`, freshest `event_created_at` wins (NIP-33),
//! - a verified NIP-09 revocation tombstones the key: the offer is dropped
//!   and lingering relay copies can never resurrect it,
//! - offers age out at the NIP-40 rolling TTL or their final expiry.

use std::collections::HashMap;

use crate::config::Pair;
use crate::offer::BookOffer;

/// Spam bounds: a relay flood cannot grow the book without bound.
const MAX_OFFERS: usize = 10_000;
const MAX_PER_MAKER: usize = 100;
/// Tombstones outlive any lingering relay copy of the offer (relay TTL is
/// 30 min, final offer TTLs are typically 24h).
const TOMBSTONE_TTL_SECS: u64 = 48 * 3600;

/// Exact price: quote sats per base sat, stored reduced (gcd) so equal prices
/// group into one level regardless of offer size. Same idea as the UI's
/// `priceKey` in `satchel/ui/src/format.ts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ratio {
    pub num: u64,
    pub den: u64,
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a.max(1)
}

impl Ratio {
    pub fn new(num: u64, den: u64) -> Ratio {
        let g = gcd(num, den);
        Ratio {
            num: num / g,
            den: den / g,
        }
    }
    pub fn to_f64(self) -> f64 {
        self.num as f64 / self.den as f64
    }
}

impl Ord for Ratio {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Cross-multiply in u128: exact, no overflow for u64 inputs.
        (self.num as u128 * other.den as u128).cmp(&(other.num as u128 * self.den as u128))
    }
}
impl PartialOrd for Ratio {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// One price level: exact unit price + summed base size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Level {
    pub price: Ratio,
    pub size_base_sats: u64,
}

/// A pair's rendered-side book, best price first on both sides.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ladder {
    pub base: String,
    pub quote: String,
    pub asks: Vec<Level>,
    pub bids: Vec<Level>,
}

/// Top-of-book signature: what the announcer diffs. Size changes below the
/// configured threshold are ignored by `TopSig::changed`, so refresh churn
/// and dust adjustments stay silent.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SideTop {
    pub num: u64,
    pub den: u64,
    pub size_base_sats: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TopSig {
    pub ask: Option<SideTop>,
    pub bid: Option<SideTop>,
}

fn side_changed(prev: &Option<SideTop>, new: &Option<SideTop>, min_size_delta_pct: u64) -> bool {
    match (prev, new) {
        (None, None) => false,
        (Some(_), None) | (None, Some(_)) => true,
        (Some(p), Some(n)) => {
            if (p.num, p.den) != (n.num, n.den) {
                return true;
            }
            let delta = p.size_base_sats.abs_diff(n.size_base_sats) as u128 * 100;
            delta >= p.size_base_sats.max(1) as u128 * min_size_delta_pct as u128
        }
    }
}

impl TopSig {
    pub fn changed(&self, new: &TopSig, min_size_delta_pct: u64) -> bool {
        side_changed(&self.ask, &new.ask, min_size_delta_pct)
            || side_changed(&self.bid, &new.bid, min_size_delta_pct)
    }
}

type Key = (String, String); // (maker, swap_id)

#[derive(Default)]
pub struct Book {
    offers: HashMap<Key, BookOffer>,
    per_maker: HashMap<String, usize>,
    tombstones: HashMap<Key, u64>,
    /// Bumped on every effective mutation (dry-run printer + /status).
    pub rev: u64,
    /// Unix time of the last completed ingest poll (0 = never).
    pub last_poll: u64,
}

impl Book {
    pub fn new() -> Book {
        Book::default()
    }

    pub fn len(&self) -> usize {
        self.offers.len()
    }

    /// NIP-33 upsert. Returns true if the book changed.
    pub fn upsert(&mut self, offer: BookOffer) -> bool {
        let key = (offer.maker.clone(), offer.swap_id.clone());
        if self.tombstones.contains_key(&key) {
            return false;
        }
        if let Some(existing) = self.offers.get(&key) {
            if existing.event_created_at >= offer.event_created_at {
                return false; // replaceable: freshest event wins, ties keep first
            }
        } else {
            if self.offers.len() >= MAX_OFFERS {
                tracing::warn!(
                    "book full ({MAX_OFFERS}) — dropping offer {}",
                    offer.swap_id
                );
                return false;
            }
            let count = self.per_maker.entry(offer.maker.clone()).or_insert(0);
            if *count >= MAX_PER_MAKER {
                tracing::warn!("maker {} at cap ({MAX_PER_MAKER}) — dropping", offer.maker);
                return false;
            }
            *count += 1;
        }
        self.offers.insert(key, offer);
        self.rev += 1;
        true
    }

    /// Verified NIP-09 revocation: drop + tombstone `(maker, swap_id)`.
    pub fn revoke(&mut self, maker: &str, swap_id: &str, now: u64) -> bool {
        let key = (maker.to_string(), swap_id.to_string());
        let removed = self.remove(&key);
        let fresh = self.tombstones.insert(key, now).is_none();
        if removed || fresh {
            self.rev += 1;
        }
        removed || fresh
    }

    fn remove(&mut self, key: &Key) -> bool {
        if self.offers.remove(key).is_some() {
            if let Some(c) = self.per_maker.get_mut(&key.0) {
                *c = c.saturating_sub(1);
                if *c == 0 {
                    self.per_maker.remove(&key.0);
                }
            }
            true
        } else {
            false
        }
    }

    /// Age out expired offers and stale tombstones. Returns true on change.
    pub fn sweep(&mut self, now: u64) -> bool {
        let dead: Vec<Key> = self
            .offers
            .iter()
            .filter(|(_, o)| o.expired(now))
            .map(|(k, _)| k.clone())
            .collect();
        let changed = !dead.is_empty();
        for key in dead {
            self.remove(&key);
        }
        self.tombstones
            .retain(|_, at| now.saturating_sub(*at) < TOMBSTONE_TTL_SECS);
        if changed {
            self.rev += 1;
        }
        changed
    }

    /// All pairs currently on the board (for `/top` — commands are not
    /// limited by the announce whitelist).
    pub fn pairs(&self) -> Vec<Pair> {
        let mut seen: Vec<Pair> = Vec::new();
        for offer in self.offers.values() {
            let pair = Pair::from_assets(&offer.give_asset, &offer.get_asset);
            if !seen.contains(&pair) {
                seen.push(pair);
            }
        }
        seen.sort_by(|x, y| (&x.a, &x.b).cmp(&(&y.a, &y.b)));
        seen
    }

    /// Build the display ladder for a pair: unit prices (quote per base),
    /// levels grouped by exact reduced ratio, best price first, `depth`
    /// levels per side.
    pub fn ladder(&self, pair: &Pair, depth: usize) -> Ladder {
        let (base, quote) = pair.orient();
        let mut asks: HashMap<Ratio, u64> = HashMap::new();
        let mut bids: HashMap<Ratio, u64> = HashMap::new();
        for offer in self.offers.values() {
            if offer.give_asset == base && offer.get_asset == quote {
                // Maker sells base at get/give quote-per-base.
                let price = Ratio::new(offer.get_amount, offer.give_amount);
                *asks.entry(price).or_insert(0) += offer.give_amount;
            } else if offer.give_asset == quote && offer.get_asset == base {
                // Maker buys base, paying give/get quote-per-base.
                let price = Ratio::new(offer.give_amount, offer.get_amount);
                *bids.entry(price).or_insert(0) += offer.get_amount;
            }
        }
        let mut asks: Vec<Level> = asks
            .into_iter()
            .map(|(price, size_base_sats)| Level {
                price,
                size_base_sats,
            })
            .collect();
        let mut bids: Vec<Level> = bids
            .into_iter()
            .map(|(price, size_base_sats)| Level {
                price,
                size_base_sats,
            })
            .collect();
        asks.sort_by_key(|x| x.price); // lowest ask = best
        bids.sort_by_key(|x| std::cmp::Reverse(x.price)); // highest bid = best
        asks.truncate(depth);
        bids.truncate(depth);
        Ladder {
            base,
            quote,
            asks,
            bids,
        }
    }

    pub fn top_sig(&self, pair: &Pair) -> TopSig {
        let ladder = self.ladder(pair, 1);
        let side = |lv: Option<&Level>| {
            lv.map(|l| SideTop {
                num: l.price.num,
                den: l.price.den,
                size_base_sats: l.size_base_sats,
            })
        };
        TopSig {
            ask: side(ladder.asks.first()),
            bid: side(ladder.bids.first()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offer(
        maker: &str,
        swap_id: &str,
        give: (&str, u64),
        get: (&str, u64),
        event_created_at: u64,
    ) -> BookOffer {
        BookOffer {
            maker: maker.into(),
            swap_id: swap_id.into(),
            give_asset: give.0.into(),
            give_amount: give.1,
            get_asset: get.0.into(),
            get_amount: get.1,
            created: event_created_at,
            ttl_secs: 86400,
            event_created_at,
            relay_expiration: Some(event_created_at + 1800),
        }
    }

    const BTCX: u64 = 100_000_000; // 1 BTCX in sats

    fn pair() -> Pair {
        Pair::parse("btc/btcx").unwrap()
    }

    #[test]
    fn replaceable_semantics_freshest_wins() {
        let mut book = Book::new();
        assert!(book.upsert(offer("m1", "s1", ("btcx", BTCX), ("btc", 70_000), 100)));
        // Older duplicate ignored; newer replaces.
        assert!(!book.upsert(offer("m1", "s1", ("btcx", BTCX), ("btc", 60_000), 99)));
        assert!(book.upsert(offer("m1", "s1", ("btcx", BTCX), ("btc", 65_000), 101)));
        let ladder = book.ladder(&pair(), 8);
        assert_eq!(ladder.asks.len(), 1);
        assert_eq!(ladder.asks[0].price, Ratio::new(65_000, BTCX));
    }

    #[test]
    fn tombstone_wins_over_lingering_copies() {
        let mut book = Book::new();
        book.upsert(offer("m1", "s1", ("btcx", BTCX), ("btc", 70_000), 100));
        assert!(book.revoke("m1", "s1", 200));
        // A lingering relay copy (even fresher) can never resurrect it.
        assert!(!book.upsert(offer("m1", "s1", ("btcx", BTCX), ("btc", 70_000), 300)));
        assert_eq!(book.len(), 0);
        // Foreign revocation of an unknown offer never touches m2's entry.
        book.upsert(offer("m2", "s2", ("btcx", BTCX), ("btc", 70_000), 100));
        book.revoke("m3", "s2", 200);
        assert_eq!(book.len(), 1);
    }

    #[test]
    fn sweep_drops_relay_ttl_lapsed_offers() {
        let mut book = Book::new();
        book.upsert(offer("m1", "s1", ("btcx", BTCX), ("btc", 70_000), 1000));
        assert!(!book.sweep(1000 + 1799));
        assert!(book.sweep(1000 + 1801)); // past NIP-40 rolling TTL
        assert_eq!(book.len(), 0);
    }

    #[test]
    fn ladder_groups_levels_and_sorts_best_first() {
        let mut book = Book::new();
        // Two asks at the same unit price (different sizes) → one level.
        book.upsert(offer("m1", "a1", ("btcx", BTCX), ("btc", 67_600), 100));
        book.upsert(offer("m2", "a2", ("btcx", 2 * BTCX), ("btc", 135_200), 100));
        book.upsert(offer("m3", "a3", ("btcx", BTCX), ("btc", 69_100), 100));
        // A bid: maker gives btc, wants btcx.
        book.upsert(offer("m4", "b1", ("btc", 67_000), ("btcx", BTCX), 100));
        let ladder = book.ladder(&pair(), 8);
        assert_eq!(
            (ladder.base.as_str(), ladder.quote.as_str()),
            ("btcx", "btc")
        );
        assert_eq!(ladder.asks.len(), 2);
        assert_eq!(ladder.asks[0].price, Ratio::new(67_600, BTCX));
        assert_eq!(ladder.asks[0].size_base_sats, 3 * BTCX);
        assert_eq!(ladder.bids.len(), 1);
        assert_eq!(ladder.bids[0].price, Ratio::new(67_000, BTCX));
    }

    #[test]
    fn top_sig_change_detection_with_size_threshold() {
        let mut book = Book::new();
        book.upsert(offer("m1", "a1", ("btcx", BTCX), ("btc", 67_600), 100));
        let sig1 = book.top_sig(&pair());
        // Refresh (same terms, newer event) → no change.
        book.upsert(offer("m1", "a1", ("btcx", BTCX), ("btc", 67_600), 200));
        assert!(!sig1.changed(&book.top_sig(&pair()), 10));
        // +5% size at same price → below 10% threshold, silent.
        book.upsert(offer(
            "m2",
            "a2",
            ("btcx", BTCX / 20),
            ("btc", 67_600 / 20),
            100,
        ));
        assert!(!sig1.changed(&book.top_sig(&pair()), 10));
        // Price move → change.
        book.upsert(offer("m3", "a3", ("btcx", BTCX), ("btc", 67_000), 100));
        assert!(sig1.changed(&book.top_sig(&pair()), 10));
    }

    #[test]
    fn per_maker_cap_holds() {
        let mut book = Book::new();
        for i in 0..200u64 {
            book.upsert(offer(
                "spammer",
                &format!("s{i}"),
                ("btcx", BTCX),
                ("btc", 70_000 + i),
                100,
            ));
        }
        assert_eq!(book.len(), 100);
    }
}
