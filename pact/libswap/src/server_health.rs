//! Passive per-server health for Electrum backends — issue #98, Phase 0.
//!
//! One [`ServerHealth`] cell per `(coin_id, url)`, handed out by a
//! process-wide registry so every holder of a connection to the same server
//! feeds the same cell: the engine's pooled [`crate::chain::ElectrumBackend`]
//! AND the wallet sync worker's private one (they deliberately keep separate
//! sockets — one caller domain each, issue #87 — but they are views of the
//! same server, so their health must be one fact, not two).
//!
//! Health is **observed, never probed**: every real request already
//! succeeds or fails and takes a measurable time, and that is the only
//! input. Nothing here dials a server, and nothing here *blocks* one —
//! Phase 0 records only. (Phase 1's `ServerSet` turns the recorded state
//! into skip/promote routing decisions; the `serverstatus` RPC and the
//! Network page render it.)
//!
//! Failure counting is deduplicated **per connection generation**: several
//! engine calls can share one broken socket (the connection is behind an
//! `RwLock` and cloned around), and when it dies they all fail together —
//! that is ONE server incident, not N, or a single hiccup would leap the
//! backoff straight to its cap.

use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use std::sync::Arc;

/// First `Down` window after a fresh failure streak starts. Short — the
/// field incident that motivated all of this was a server that recovered
/// within milliseconds; a blip must cost seconds, not minutes.
const BACKOFF_BASE: Duration = Duration::from_secs(5);

/// Ceiling for the `Down` window however long the streak gets — a dead
/// server is re-eligible for a (cheap, promotion-time) recovery attempt at
/// least this often.
const BACKOFF_CAP: Duration = Duration::from_secs(300);

/// EWMA smoothing for request latency: `ewma += (sample - ewma) / 8`.
const EWMA_SHIFT: u32 = 3;

/// Health as routing sees it: can this server be handed a request right
/// now? (`Down` only means "inside its backoff window" — after `until`
/// passes the server is eligible again and the next real use decides.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthState {
    /// Never used this run — standby servers live here until promoted.
    Untested,
    /// Last request succeeded.
    Healthy,
    /// In a backoff window after a failure streak.
    Down { until: Instant },
}

/// One server's health cell. All writes are passive side effects of real
/// traffic; reads are a cheap mutex-guarded copy.
pub struct ServerHealth {
    coin_id: String,
    url: String,
    inner: Mutex<Inner>,
}

struct Inner {
    state: HealthState,
    /// Consecutive failed incidents (drives the backoff exponent).
    streak: u32,
    /// Connection generation of the last counted transport failure —
    /// failures on the same generation are the same incident (dedup).
    failed_generation: u64,
    latency_ewma_micros: Option<u64>,
    last_ok: Option<Instant>,
    last_error: Option<(String, Instant)>,
    requests: u64,
    failures: u64,
}

/// Serializable snapshot for the `serverstatus` RPC / Network page.
/// Durations are seconds-ago so the reader needs no clock agreement.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HealthSnapshot {
    pub coin_id: String,
    pub url: String,
    /// `"untested" | "healthy" | "down"`.
    pub state: String,
    /// When `down`: seconds until the backoff window expires (0 = now).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_in_secs: Option<u64>,
    /// Smoothed request latency, milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_ok_secs_ago: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error_secs_ago: Option<u64>,
    pub requests: u64,
    pub failures: u64,
}

impl ServerHealth {
    fn new(coin_id: &str, url: &str) -> Self {
        Self {
            coin_id: coin_id.to_string(),
            url: url.to_string(),
            inner: Mutex::new(Inner {
                state: HealthState::Untested,
                streak: 0,
                failed_generation: 0,
                latency_ewma_micros: None,
                last_ok: None,
                last_error: None,
                requests: 0,
                failures: 0,
            }),
        }
    }

    /// A request on connection `generation` completed successfully in
    /// `latency`. Clears any failure streak — the server is back.
    pub(crate) fn record_success(&self, latency: Duration) {
        let mut inner = self.lock();
        inner.state = HealthState::Healthy;
        inner.streak = 0;
        inner.requests += 1;
        inner.last_ok = Some(Instant::now());
        let sample = latency.as_micros().min(u128::from(u64::MAX)) as u64;
        inner.latency_ewma_micros = Some(match inner.latency_ewma_micros {
            None => sample,
            // ewma += (sample - ewma) / 8, in signed space to move both ways.
            Some(ewma) => {
                (ewma as i64 + ((sample as i64) - (ewma as i64)) / (1 << EWMA_SHIFT)).max(0) as u64
            }
        });
    }

    /// A transport failure on connection `generation`. Counted as a new
    /// incident (streak++, longer backoff) only for a generation that has
    /// not failed before; repeat failures of the same broken socket only
    /// bump the raw counter.
    pub(crate) fn record_failure(&self, generation: u64, error: &str) {
        let mut inner = self.lock();
        inner.failures += 1;
        inner.requests += 1;
        inner.last_error = Some((error.to_string(), Instant::now()));
        if generation != 0 && generation == inner.failed_generation {
            return; // same broken socket, same incident
        }
        inner.failed_generation = generation;
        inner.streak = inner.streak.saturating_add(1);
        inner.state = HealthState::Down {
            until: Instant::now() + backoff(inner.streak),
        };
    }

    /// A dial (TCP/TLS connect or handshake) failure — there is no
    /// generation yet. Dials are serialized by the connection slot's write
    /// lock, so every one is its own incident.
    pub(crate) fn record_connect_failure(&self, error: &str) {
        self.record_failure(0, error);
    }

    /// Is the server outside any backoff window right now? (Phase 1 routing
    /// uses this to skip `Down` servers without paying a connect timeout.)
    pub fn available(&self) -> bool {
        match self.lock().state {
            HealthState::Untested | HealthState::Healthy => true,
            HealthState::Down { until } => Instant::now() >= until,
        }
    }

    /// Current state (with the `Down` window resolved against now).
    pub fn state(&self) -> HealthState {
        self.lock().state
    }

    pub fn snapshot(&self) -> HealthSnapshot {
        let inner = self.lock();
        let now = Instant::now();
        let (state, retry_in_secs) = match inner.state {
            HealthState::Untested => ("untested", None),
            HealthState::Healthy => ("healthy", None),
            HealthState::Down { until } => {
                ("down", Some(until.saturating_duration_since(now).as_secs()))
            }
        };
        HealthSnapshot {
            coin_id: self.coin_id.clone(),
            url: self.url.clone(),
            state: state.to_string(),
            retry_in_secs,
            latency_ms: inner
                .latency_ewma_micros
                .map(|us| (us as f64 / 100.0).round() / 10.0),
            last_ok_secs_ago: inner.last_ok.map(|t| now.duration_since(t).as_secs()),
            last_error: inner.last_error.as_ref().map(|(e, _)| e.clone()),
            last_error_secs_ago: inner
                .last_error
                .as_ref()
                .map(|(_, t)| now.duration_since(*t).as_secs()),
            requests: inner.requests,
            failures: inner.failures,
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().expect("server health poisoned")
    }
}

/// `Down` window for the `streak`-th consecutive incident:
/// 5s, 10s, 20s, … capped at [`BACKOFF_CAP`].
fn backoff(streak: u32) -> Duration {
    BACKOFF_BASE
        .saturating_mul(2u32.saturating_pow(streak.saturating_sub(1).min(16)))
        .min(BACKOFF_CAP)
}

// ---- registry ---------------------------------------------------------------

static REGISTRY: OnceLock<Mutex<BTreeMap<(String, String), Arc<ServerHealth>>>> = OnceLock::new();

fn registry() -> &'static Mutex<BTreeMap<(String, String), Arc<ServerHealth>>> {
    REGISTRY.get_or_init(|| Mutex::new(BTreeMap::new()))
}

/// The (one) health cell for this server, created on first sight. Every
/// connection holder to the same `(coin_id, url)` gets the same cell.
pub fn server_health(coin_id: &str, url: &str) -> Arc<ServerHealth> {
    let mut reg = registry().lock().expect("health registry poisoned");
    reg.entry((coin_id.to_string(), url.to_string()))
        .or_insert_with(|| Arc::new(ServerHealth::new(coin_id, url)))
        .clone()
}

/// Snapshots for a coin's configured server list, in list order. Servers
/// never seen by the registry (never used this run) report as `untested`.
pub fn coin_snapshots(coin_id: &str, urls: &[&str]) -> Vec<HealthSnapshot> {
    urls.iter()
        .map(|url| server_health(coin_id, url).snapshot())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_doubles_from_base_and_caps() {
        assert_eq!(backoff(1), BACKOFF_BASE);
        assert_eq!(backoff(2), BACKOFF_BASE * 2);
        assert_eq!(backoff(4), BACKOFF_BASE * 8);
        assert_eq!(backoff(10), BACKOFF_CAP);
        assert_eq!(backoff(60), BACKOFF_CAP); // no overflow far out
    }

    #[test]
    fn success_failure_transitions() {
        let h = ServerHealth::new("btc", "tcp://a:1");
        assert_eq!(h.state(), HealthState::Untested);
        assert!(h.available(), "untested must be routable (standby dial)");

        h.record_success(Duration::from_millis(20));
        assert_eq!(h.state(), HealthState::Healthy);

        h.record_failure(1, "io: broken pipe");
        assert!(matches!(h.state(), HealthState::Down { .. }));
        assert!(!h.available(), "fresh Down window blocks routing");

        // Recovery clears the streak: after success, the next failure gets
        // the BASE window again, not a continued exponent.
        h.record_success(Duration::from_millis(20));
        assert_eq!(h.state(), HealthState::Healthy);
        let snap = h.snapshot();
        assert_eq!(snap.state, "healthy");
        assert_eq!(snap.requests, 3);
        assert_eq!(snap.failures, 1);
        assert_eq!(snap.last_error.as_deref(), Some("io: broken pipe"));
    }

    #[test]
    fn failures_dedupe_per_generation() {
        let h = ServerHealth::new("btc", "tcp://a:1");
        // Three engine calls all failing on the same broken socket (gen 7):
        // one incident — the streak (and so the backoff) advances once.
        h.record_failure(7, "io");
        h.record_failure(7, "io");
        h.record_failure(7, "io");
        assert_eq!(h.lock().streak, 1);
        assert_eq!(h.snapshot().failures, 3, "raw counter still counts all");

        // The reconnect's NEW socket failing too is a new incident.
        h.record_failure(8, "io");
        assert_eq!(h.lock().streak, 2);

        // Dial failures (no generation) each count — they are serialized.
        h.record_connect_failure("connect timeout");
        h.record_connect_failure("connect timeout");
        assert_eq!(h.lock().streak, 4);
    }

    #[test]
    fn latency_ewma_converges_toward_samples() {
        let h = ServerHealth::new("btc", "tcp://a:1");
        h.record_success(Duration::from_millis(100));
        assert_eq!(h.snapshot().latency_ms, Some(100.0));
        // A run of faster samples pulls the average down monotonically.
        let mut last = 100.0;
        for _ in 0..20 {
            h.record_success(Duration::from_millis(10));
            let now = h.snapshot().latency_ms.unwrap();
            assert!(now <= last, "ewma must move toward the samples");
            last = now;
        }
        assert!(last < 30.0, "ewma converged near the new level, got {last}");
    }

    #[test]
    fn registry_hands_out_one_cell_per_coin_url() {
        let a1 = server_health("test-reg", "tcp://x:1");
        let a2 = server_health("test-reg", "tcp://x:1");
        assert!(Arc::ptr_eq(&a1, &a2), "same (coin,url) → same cell");
        let b = server_health("test-reg", "tcp://y:1");
        assert!(!Arc::ptr_eq(&a1, &b));
        // Same url under another coin is a different server relationship.
        let c = server_health("test-reg-2", "tcp://x:1");
        assert!(!Arc::ptr_eq(&a1, &c));

        a1.record_success(Duration::from_millis(5));
        let snaps = coin_snapshots("test-reg", &["tcp://x:1", "tcp://never-used:1"]);
        assert_eq!(snaps[0].state, "healthy");
        assert_eq!(snaps[1].state, "untested");
    }
}
