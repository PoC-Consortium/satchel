//! BTC/USD reference rate for the cash annotations.
//!
//! Crier only QUOTES a reference — it never computes with it beyond display.
//! Sources: CoinGecko, falling back to Coinbase spot. If both fail long
//! enough that the rate goes stale (`cash.max_age_secs`), the annotations
//! silently disappear rather than showing an outdated number.

use std::sync::{Arc, RwLock};
use std::time::Duration;

use anyhow::{Context, Result};

use crate::config::{unix_now, CashCfg};

#[derive(Clone, Default)]
pub struct CashRate {
    /// `(usd_per_btc, fetched_at)` of the last successful fetch.
    inner: Arc<RwLock<Option<(f64, u64)>>>,
    max_age_secs: u64,
}

impl CashRate {
    pub fn new(cfg: &CashCfg) -> CashRate {
        CashRate {
            inner: Arc::default(),
            max_age_secs: cfg.max_age_secs,
        }
    }

    /// The rate, if fresh enough to show.
    pub fn fresh(&self) -> Option<f64> {
        let guard = self.inner.read().ok()?;
        let (rate, at) = (*guard)?;
        (unix_now().saturating_sub(at) <= self.max_age_secs).then_some(rate)
    }

    /// `(rate, age_secs)` regardless of freshness — for /status.
    pub fn last(&self) -> Option<(f64, u64)> {
        let guard = self.inner.read().ok()?;
        let (rate, at) = (*guard)?;
        Some((rate, unix_now().saturating_sub(at)))
    }

    pub async fn run(self, refresh_secs: u64) {
        let client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("crier/0.1 (pact orderbook announcer)")
            .build()
        {
            Ok(c) => c,
            Err(err) => {
                tracing::error!("cash: http client: {err:#} — USD annotations disabled");
                return;
            }
        };
        loop {
            match fetch_usd_per_btc(&client).await {
                Ok(rate) => {
                    if let Ok(mut guard) = self.inner.write() {
                        *guard = Some((rate, unix_now()));
                    }
                    tracing::debug!("cash: 1 BTC = {rate} USD");
                }
                Err(err) => tracing::warn!("cash: rate fetch failed: {err:#}"),
            }
            tokio::time::sleep(Duration::from_secs(refresh_secs)).await;
        }
    }
}

async fn fetch_usd_per_btc(client: &reqwest::Client) -> Result<f64> {
    match coingecko(client).await {
        Ok(rate) => Ok(rate),
        Err(primary) => coinbase(client)
            .await
            .with_context(|| format!("coingecko failed first: {primary:#}")),
    }
}

async fn coingecko(client: &reqwest::Client) -> Result<f64> {
    let value: serde_json::Value = client
        .get("https://api.coingecko.com/api/v3/simple/price?ids=bitcoin&vs_currencies=usd")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    value
        .pointer("/bitcoin/usd")
        .and_then(|v| v.as_f64())
        .filter(|r| r.is_finite() && *r > 0.0)
        .context("coingecko: no bitcoin.usd in response")
}

async fn coinbase(client: &reqwest::Client) -> Result<f64> {
    let value: serde_json::Value = client
        .get("https://api.coinbase.com/v2/prices/BTC-USD/spot")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    value
        .pointer("/data/amount")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|r| r.is_finite() && *r > 0.0)
        .context("coinbase: no data.amount in response")
}
