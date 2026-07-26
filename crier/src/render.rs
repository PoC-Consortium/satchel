//! Message rendering — the user-reviewed formats from PLAN.md §4.
//!
//! Order convention: `<size> BTCX @ <price>`, bare numbers, unit stated once
//! per message (embed footer for announcements, header for /book). Prices are
//! UNIT prices: what 1 base coin costs, in the quote coin's display unit
//! (mBTC by default). All prices within one message share a fixed decimal
//! count so a ladder's price column aligns (`0.670` next to `0.676`, never
//! `0.67`). Optional USD reference annotations from the cash rate.

use crate::book::{Ladder, Ratio, SideTop, TopSig};
use crate::config::{BtcUnit, CoinInfo, Config, Pair};

/// Everything needed to turn exact ratios into display strings for one pair.
pub struct RenderCtx {
    pub base: CoinInfo,
    pub quote: CoinInfo,
    pub quote_is_btc: bool,
    pub btc_unit: BtcUnit,
    /// USD per 1 BTC reference rate, if fresh (annotations dropped when None).
    pub usd_per_btc: Option<f64>,
}

impl RenderCtx {
    pub fn for_pair(cfg: &Config, pair: &Pair, usd_per_btc: Option<f64>) -> RenderCtx {
        let (base, quote) = pair.orient();
        let quote_is_btc = quote == "btc";
        RenderCtx {
            base: cfg.coin(&base).clone(),
            quote: cfg.coin(&quote).clone(),
            quote_is_btc,
            btc_unit: cfg.render.btc_unit,
            usd_per_btc: usd_per_btc.filter(|_| quote_is_btc),
        }
    }

    pub fn pair_label(&self) -> String {
        format!("{}/{}", self.base.symbol, self.quote.symbol)
    }

    /// "mBTC/BTCX" — the once-per-message unit legend.
    pub fn unit_label(&self) -> String {
        let quote_unit = if self.quote_is_btc {
            self.btc_unit.label().to_string()
        } else {
            self.quote.symbol.clone()
        };
        format!("{quote_unit}/{}", self.base.symbol)
    }

    pub fn legend(&self) -> String {
        let mut s = format!("prices in {}", self.unit_label());
        if let Some(rate) = self.usd_per_btc {
            s.push_str(&format!(" · 1 BTC ≈ ${} ref", fmt_usd(rate)));
        }
        s
    }

    /// Exact ratio (quote sats per base sat) → display unit price.
    fn display_price(&self, price: Ratio) -> f64 {
        let decimals_shift = 10f64.powi(self.base.decimals as i32 - self.quote.decimals as i32);
        let unit_scale = if self.quote_is_btc {
            self.btc_unit.scale()
        } else {
            1.0
        };
        price.to_f64() * decimals_shift * unit_scale
    }

    /// USD value of 1 base coin at `price`, if a fresh rate is available.
    fn usd_price(&self, price: Ratio) -> Option<f64> {
        let rate = self.usd_per_btc?;
        let shift = 10f64.powi(self.base.decimals as i32 - self.quote.decimals as i32);
        Some(price.to_f64() * shift * rate)
    }

    /// The shared decimal count for a message's prices: the max natural
    /// precision among them, so every price aligns without losing digits.
    fn price_decimals<I: IntoIterator<Item = Ratio>>(&self, prices: I) -> usize {
        prices
            .into_iter()
            .map(|p| decimals_for(self.display_price(p)))
            .max()
            .unwrap_or(0)
    }

    fn price_str(&self, price: Ratio, decimals: usize) -> String {
        fmt_fixed(self.display_price(price), decimals)
    }

    /// Standalone price with natural (trimmed) decimals — /top lines.
    pub fn price_str_natural(&self, price: Ratio) -> String {
        fmt_price(self.display_price(price))
    }

    fn price_with_usd(&self, price: Ratio, decimals: usize) -> String {
        match self.usd_price(price) {
            Some(usd) => format!("{} (${})", self.price_str(price, decimals), fmt_usd(usd)),
            None => self.price_str(price, decimals),
        }
    }

    pub fn size_str(&self, size_base_sats: u64) -> String {
        fmt_amount(size_base_sats as f64 / 10f64.powi(self.base.decimals as i32))
    }

    /// `(spread, pct, mid)` display strings.
    fn spread_parts(&self, ask: Ratio, bid: Ratio) -> (String, String, String) {
        let (a, b) = (self.display_price(ask), self.display_price(bid));
        let spread = a - b;
        let mid = (a + b) / 2.0;
        let pct = if mid > 0.0 { spread / mid * 100.0 } else { 0.0 };
        (fmt_price(spread), fmt_pct(pct), fmt_price(mid))
    }
}

// ---- number formatting ----

fn group_thousands(int_part: &str) -> String {
    let mut out = String::new();
    let bytes = int_part.as_bytes();
    for (i, ch) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i).is_multiple_of(3) {
            out.push(' ');
        }
        out.push(*ch as char);
    }
    out
}

fn trim_zeros(s: &str) -> String {
    if !s.contains('.') {
        return s.to_string();
    }
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Natural decimal count for ~3-4 significant digits of `v`.
fn decimals_for(v: f64) -> usize {
    if !v.is_finite() || v <= 0.0 {
        return 0;
    }
    let exponent = v.abs().log10().floor() as i32;
    if exponent >= 0 {
        (3 - exponent).clamp(0, 8) as usize
    } else {
        (2 - exponent).clamp(0, 8) as usize
    }
}

/// Fixed decimals, thousands grouped, NO zero-trimming (column alignment).
fn fmt_fixed(v: f64, decimals: usize) -> String {
    if !v.is_finite() {
        return "0".to_string();
    }
    let s = format!("{v:.decimals$}");
    match s.split_once('.') {
        Some((int, frac)) => format!("{}.{frac}", group_thousands(int)),
        None => group_thousands(&s),
    }
}

/// Standalone price (spread/mid): natural decimals, trailing zeros trimmed.
pub fn fmt_price(v: f64) -> String {
    if !v.is_finite() || v <= 0.0 {
        return "0".to_string();
    }
    let trimmed = trim_zeros(&format!("{:.*}", decimals_for(v), v));
    match trimmed.split_once('.') {
        Some((int, frac)) => format!("{}.{frac}", group_thousands(int)),
        None => group_thousands(&trimmed),
    }
}

/// Sizes: up to 4 decimals, trimmed, grouped.
pub fn fmt_amount(v: f64) -> String {
    if !v.is_finite() || v <= 0.0 {
        return "0".to_string();
    }
    let trimmed = trim_zeros(&format!("{v:.4}"));
    match trimmed.split_once('.') {
        Some((int, frac)) => format!("{}.{frac}", group_thousands(int)),
        None => group_thousands(&trimmed),
    }
}

/// USD: 2 decimals (Cashrate convention), 4 below a cent, grouped.
pub fn fmt_usd(v: f64) -> String {
    if !v.is_finite() || v <= 0.0 {
        return "0".to_string();
    }
    let decimals = if v < 0.01 { 4 } else { 2 };
    let s = format!("{v:.decimals$}");
    match s.split_once('.') {
        Some((int, frac)) => format!("{}.{frac}", group_thousands(int)),
        None => group_thousands(&s),
    }
}

fn fmt_pct(v: f64) -> String {
    trim_zeros(&format!("{v:.1}"))
}

// ---- /book ladder ----

/// The `/book` message: `**BTCX/BTC** — prices in mBTC/BTCX` header + code
/// block. Asks rendered worst→best downward so the best price hugs the
/// spread banner; bids best→worst below it.
pub fn render_book(ladder: &Ladder, ctx: &RenderCtx) -> String {
    let decimals = ctx.price_decimals(
        ladder
            .asks
            .iter()
            .chain(ladder.bids.iter())
            .map(|lv| lv.price),
    );
    let mut rows: Vec<(&str, String, String)> = Vec::new(); // (side, size, price)
    for lv in ladder.asks.iter().rev() {
        rows.push((
            "ASK",
            ctx.size_str(lv.size_base_sats),
            ctx.price_with_usd(lv.price, decimals),
        ));
    }
    let spread_at = rows.len();
    for lv in &ladder.bids {
        rows.push((
            "BID",
            ctx.size_str(lv.size_base_sats),
            ctx.price_with_usd(lv.price, decimals),
        ));
    }
    let size_w = rows.iter().map(|r| r.1.len()).max().unwrap_or(0);

    let mut body = String::new();
    if rows.is_empty() {
        body.push_str("  (empty book)\n");
    }
    for (i, (side, size, price)) in rows.iter().enumerate() {
        if i == spread_at {
            push_spread(&mut body, ladder, ctx);
        }
        body.push_str(&format!(
            "  {side}  {size:>size_w$} {} @ {price}\n",
            ctx.base.symbol
        ));
    }
    if spread_at == rows.len() && !rows.is_empty() {
        push_spread(&mut body, ladder, ctx);
    }
    format!(
        "**{}** — {}\n```\n{body}```",
        ctx.pair_label(),
        ctx.legend()
    )
}

fn push_spread(body: &mut String, ladder: &Ladder, ctx: &RenderCtx) {
    match (ladder.asks.first(), ladder.bids.first()) {
        (Some(a), Some(b)) => {
            let (s, pct, mid) = ctx.spread_parts(a.price, b.price);
            body.push_str(&format!("  ───  spread {s} ({pct} %) · mid {mid}  ───\n"));
        }
        (Some(_), None) => body.push_str("  ───  (no bids)  ───\n"),
        (None, Some(_)) => body.push_str("  ───  (no asks)  ───\n"),
        (None, None) => {}
    }
}

// ---- announcements ----

/// A rendered announcement: embed title / body / footer (also printed as
/// plain text by --dry-run).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Announcement {
    pub title: String,
    pub body: String,
    pub footer: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SideChange {
    Same,
    New,
    Gone,
    Improved,
    BackedOff,
    Size,
}

/// `better_is_lower`: true for asks (a lower ask improves the book for
/// takers), false for bids.
fn classify(prev: &Option<SideTop>, new: &Option<SideTop>, better_is_lower: bool) -> SideChange {
    match (prev, new) {
        (None, None) => SideChange::Same,
        (None, Some(_)) => SideChange::New,
        (Some(_), None) => SideChange::Gone,
        (Some(p), Some(n)) => {
            let (pp, np) = (Ratio::new(p.num, p.den), Ratio::new(n.num, n.den));
            if pp == np {
                if p.size_base_sats != n.size_base_sats {
                    SideChange::Size
                } else {
                    SideChange::Same
                }
            } else if (np < pp) == better_is_lower {
                SideChange::Improved
            } else {
                SideChange::BackedOff
            }
        }
    }
}

fn side_headline(side: &str, change: SideChange) -> Option<String> {
    match change {
        SideChange::Same => None,
        SideChange::New => Some(format!("new best {side}")),
        SideChange::Gone => Some(format!("best {side} gone")),
        SideChange::Improved => Some(format!("best {side} improved")),
        SideChange::BackedOff => Some(format!("best {side} backed off")),
        SideChange::Size => Some(format!("best {side} size changed")),
    }
}

pub fn render_announcement(prev: &TopSig, new: &TopSig, ctx: &RenderCtx) -> Announcement {
    let ask_change = classify(&prev.ask, &new.ask, true);
    let bid_change = classify(&prev.bid, &new.bid, false);
    let headline = match (
        side_headline("ask", ask_change),
        side_headline("bid", bid_change),
    ) {
        (Some(_), Some(_)) => "top of book moved".to_string(),
        (Some(h), None) | (None, Some(h)) => h,
        (None, None) => "book update".to_string(),
    };

    let shown = [&new.ask, &new.bid, &prev.ask, &prev.bid]
        .into_iter()
        .flatten()
        .map(|s| Ratio::new(s.num, s.den));
    let decimals = ctx.price_decimals(shown);

    let mut lines = Vec::new();
    lines.push(side_line(
        ctx, "Ask", &new.ask, &prev.ask, ask_change, decimals,
    ));
    lines.push(side_line(
        ctx, "Bid", &new.bid, &prev.bid, bid_change, decimals,
    ));
    if let (Some(a), Some(b)) = (&new.ask, &new.bid) {
        let (s, pct, mid) = ctx.spread_parts(Ratio::new(a.num, a.den), Ratio::new(b.num, b.den));
        lines.push(format!("**Spread** `{s}` ({pct} %) · mid `{mid}`"));
    }

    Announcement {
        title: format!("{} — {headline}", ctx.pair_label()),
        body: lines.join("\n"),
        footer: ctx.legend(),
    }
}

fn side_line(
    ctx: &RenderCtx,
    label: &str,
    side: &Option<SideTop>,
    prev: &Option<SideTop>,
    change: SideChange,
    decimals: usize,
) -> String {
    let Some(top) = side else {
        return format!("**{label}** *none*");
    };
    let price = Ratio::new(top.num, top.den);
    let mut line = format!(
        "**{label}** `{} {} @ {}`",
        ctx.size_str(top.size_base_sats),
        ctx.base.symbol,
        ctx.price_with_usd(price, decimals)
    );
    if matches!(change, SideChange::Improved | SideChange::BackedOff) {
        if let Some(p) = prev {
            line.push_str(&format!(
                " *(was {})*",
                ctx.price_str(Ratio::new(p.num, p.den), decimals)
            ));
        }
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::book::Book;
    use crate::offer::BookOffer;
    use std::path::Path;

    const BTCX: u64 = 100_000_000;

    fn cfg() -> Config {
        Config::load(Path::new("Z:/definitely/not/here/crier.toml")).unwrap()
    }

    fn ctx(usd: Option<f64>) -> RenderCtx {
        RenderCtx::for_pair(&cfg(), &Pair::parse("btc/btcx").unwrap(), usd)
    }

    fn offer(maker: &str, swap_id: &str, give: (&str, u64), get: (&str, u64)) -> BookOffer {
        BookOffer {
            maker: maker.into(),
            swap_id: swap_id.into(),
            give_asset: give.0.into(),
            give_amount: give.1,
            get_asset: get.0.into(),
            get_amount: get.1,
            created: 100,
            ttl_secs: 86400,
            event_created_at: 100,
            relay_expiration: None,
        }
    }

    #[test]
    fn number_formats() {
        assert_eq!(fmt_price(0.676), "0.676");
        assert_eq!(fmt_price(0.006), "0.006");
        assert_eq!(fmt_price(0.00003), "0.00003");
        assert_eq!(fmt_price(1480.0), "1 480");
        assert_eq!(fmt_price(14.8), "14.8");
        assert_eq!(fmt_amount(400.0), "400");
        assert_eq!(fmt_amount(1500.25), "1 500.25");
        assert_eq!(fmt_usd(118432.1), "118 432.10");
        assert_eq!(fmt_usd(0.79), "0.79");
    }

    #[test]
    fn book_golden() {
        let mut book = Book::new();
        book.upsert(offer(
            "m1",
            "a1",
            ("btcx", 120 * BTCX),
            ("btc", 120 * 69_100),
        ));
        book.upsert(offer("m2", "a2", ("btcx", 50 * BTCX), ("btc", 50 * 68_300)));
        book.upsert(offer("m3", "a3", ("btcx", 37 * BTCX), ("btc", 37 * 67_600)));
        book.upsert(offer("m4", "b1", ("btc", 26 * 67_000), ("btcx", 26 * BTCX)));
        book.upsert(offer(
            "m5",
            "b2",
            ("btc", 400 * 65_500),
            ("btcx", 400 * BTCX),
        ));
        let ladder = book.ladder(&Pair::parse("btc/btcx").unwrap(), 8);
        let text = render_book(&ladder, &ctx(None));
        let expected = "**BTCX/BTC** — prices in mBTC/BTCX\n\
```\n  \
ASK  120 BTCX @ 0.691\n  \
ASK   50 BTCX @ 0.683\n  \
ASK   37 BTCX @ 0.676\n  \
───  spread 0.006 (0.9 %) · mid 0.673  ───\n  \
BID   26 BTCX @ 0.670\n  \
BID  400 BTCX @ 0.655\n\
```";
        assert_eq!(text, expected);
    }

    #[test]
    fn announcement_golden_with_usd() {
        let prev = TopSig {
            ask: Some(SideTop {
                num: 69_100,
                den: BTCX,
                size_base_sats: 37 * BTCX,
            }),
            bid: Some(SideTop {
                num: 67_000,
                den: BTCX,
                size_base_sats: 26 * BTCX,
            }),
        };
        let new = TopSig {
            ask: Some(SideTop {
                num: 67_600,
                den: BTCX,
                size_base_sats: 37 * BTCX,
            }),
            ..prev.clone()
        };
        let a = render_announcement(&prev, &new, &ctx(Some(118_000.0)));
        assert_eq!(a.title, "BTCX/BTC — best ask improved");
        assert_eq!(
            a.body,
            "**Ask** `37 BTCX @ 0.676 ($79.77)` *(was 0.691)*\n\
             **Bid** `26 BTCX @ 0.670 ($79.06)`\n\
             **Spread** `0.006` (0.9 %) · mid `0.673`"
        );
        assert_eq!(a.footer, "prices in mBTC/BTCX · 1 BTC ≈ $118 000.00 ref");
    }

    #[test]
    fn side_vanishing_headline() {
        let prev = TopSig {
            ask: Some(SideTop {
                num: 67_600,
                den: BTCX,
                size_base_sats: BTCX,
            }),
            bid: None,
        };
        let new = TopSig {
            ask: None,
            bid: None,
        };
        let a = render_announcement(&prev, &new, &ctx(None));
        assert_eq!(a.title, "BTCX/BTC — best ask gone");
        assert!(a.body.contains("**Ask** *none*"));
    }
}
