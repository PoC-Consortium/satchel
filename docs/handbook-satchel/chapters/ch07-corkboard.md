# The Corkboard

The **Corkboard** is where trades live in public. Makers pin offers to it, and
you browse those offers and take the one you like. It's the screen Satchel opens
on, and it's the heart of the app.

One thing to understand up front: the Corkboard is a *noticeboard*, not an
exchange. It never matches buyers with sellers, and it never holds your coins. It
simply displays everyone's offers in a tidy, familiar layout so you can find the
price you want and take it yourself.

![The Corkboard order-book ladder, BTCX/BTC.](images/processed/ch07-corkboard.png){width=90%}

## The controls along the top

- **Pair selector** — chooses which two coins you're looking at, shown as
  `SYM ↔ SYM` (for example **BTCX ↔ BTC**). Only pairs whose coins you've set up
  appear here. Your last choice is remembered.
- **All / Mine** — **All** shows every offer on the board; **Mine** narrows it to
  just the offers you posted, so you can find and withdraw your own.
- **Denomination toggle** — switches the display unit for the quote coin, so you
  can read prices in whichever unit you think in.
- **Board selector** — if you've connected more than one noticeboard, this picks
  which one you're viewing. (You post to all of them at once, but you browse one
  at a time.)

## Reading the order book

The book has two columns:

- **Bids** are on the **left** and shown in **green**. These are makers who want
  the base coin and are paying the quote coin for it.
- **Asks** are on the **right** and shown in **red**. These are makers selling the
  base coin for the quote coin.

Each row is a *price level*. Offers at the exact same rate are grouped together
onto one level, so a single row can represent several offers. The **depth bar**
behind the row shows how much coin is on offer at that price — a longer bar means
more is available there. Within each column the best price sits closest to the
centre divider: bids run high-to-low downward, asks run low-to-high.

To keep things readable, Satchel shows the top eight levels per side. If there's
more, a **Show more** toggle reveals the rest.

### The spread banner

Above the two columns is a small banner describing the gap between the best bid
and the best ask:

- A **spread percentage** and a **mid price** when there are offers on both sides.
- **One-sided** when there are offers on only one side.
- A **crossed** chip when the top bid is at or above the top ask.

That last one can look alarming if you're used to an exchange, so here's the key
point: **the board never matches trades**. When offers overlap, they simply sit
there — nothing fires automatically. You can take either side at the price shown.
Crossed just means two makers happen to have posted overlapping prices.

> **Note** — On an exchange, a crossed book would instantly execute. On the
> Corkboard nothing executes on its own — *you* choose an offer and take it. The
> ladder is only a way to *read* the board.

## The detail pane: seeing the offers at a level

Clicking a price level fills the **detail pane** below the book with the
individual offers sitting at that rate, biggest first. (Before you pick a level,
the pane prompts you to "pick a price level above.")

Each offer row shows:

- **The counterparty** — a tag for who posted it, or "your offer" if it's yours.
  An offer you've *just* posted shows a dimmed, italic **"posting…"** badge (and a
  hollow dot in the ladder) for the brief moment between when you post it and when
  a relay echoes it back; once confirmed it switches to the normal "your offer"
  tag and goes solid. See *Posting an offer* below.
- **Amounts from your perspective** — what you would give and what you would
  receive if you took it. You don't have to do the maker's math in reverse;
  Satchel flips it for you.
- **The protocol chip** — either **Standard (HTLC)** (the classic swap, muted
  style) or **Private (Taproot)** (the newer private swap, highlighted style).
  Both are safe; the private one simply looks like an ordinary payment on-chain.
  See the safety chapter for the difference.
- **The safety-refund times** — shown as `t2h / t1h` (for example `12h / 24h`).
  These are the auto-refund deadlines that protect you if a swap stalls; the
  taker's side unlocks first, the maker's a little later, so nobody gets stuck.
- **A freshness dot** — how recently the offer was seen, so you can favour live
  ones.

![The detail pane: individual offers at one price level, with protocol chips and refund times.](images/processed/ch07-detail.png){width=80%}

## Taking an offer

When you've found an offer you like, click **Take offer** on its row. Satchel
shows a confirmation dialog summarising the trade — amounts, protocol, refund
times, and the network-fee preview — before anything happens. The chapter on
taking an offer walks through what comes next.

The **Take offer** button is disabled in two cases:

- **It's your own offer** — you can't take yourself. (Use **Withdraw** instead, see
  below.)
- **A node is down** — if one of the two coins for this pair isn't reachable,
  Satchel blocks the take with a note to start the node or check **Settings →
  Coins**. This is a friendly guard; the engine would refuse it anyway.

## Your own offers on the board

When you post an offer, it shows up on the board **instantly** — you don't wait
for the network. While it's still travelling out to a relay it wears a dimmed,
italic **"posting…"** badge (a hollow dot in the ladder), which simply means
*"posted from this device and already advertising; waiting to be confirmed back
from a relay."* The moment a relay echoes it, the badge becomes the ordinary
**"your offer"** tag and the dot fills in — it's now fully live.

> **Note** — Your offers also **survive a restart.** When you close Satchel
> cleanly it quietly stops advertising your offers (so the board doesn't show
> stale listings while you're away), and the moment you start it again it
> re-advertises every offer that's still within its valid-for window — before the
> first new board refresh. You won't lose offers just by closing the app. Only an
> explicit **withdraw** removes one for good.

## Withdrawing your own offer

For an offer you posted, the row shows **Withdraw** instead of **Take offer**.
Click it to pull the listing immediately. Because an offer never locks any funds,
withdrawing is instant and costs nothing — it just removes the advert.

## Empty and error states

The Corkboard tells you clearly when there's nothing to show:

- **No Corkboard connected** — you haven't pointed Satchel at a noticeboard yet.
  A **Configure in Settings** link takes you there.
- **No offers you can take right now** — the board is reachable but has no offers
  for a pair you've set up. They'll appear as soon as a maker posts one, and you
  can always post your own.

The Corkboard only shows offers for pairs whose coins you've connected — offers
for other pairs simply don't appear. To trade a pair you don't yet see, set up
both of its coins in **Settings → Coins**.
