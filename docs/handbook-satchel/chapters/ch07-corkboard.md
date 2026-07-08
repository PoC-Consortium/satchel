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

- **Pair selector** — chooses which two coins you're looking at, labelled
  **base/quote** (for example **BTCX/BTC**) in the same orientation as the order
  book it drives, so the selector and the book always read the same way round.
  Normally only pairs whose coins you've set up appear here — flip on
  **All pairs** (below) to browse the rest. Your last choice is remembered.
- **All / Mine** — **All** shows every offer on the board; **Mine** narrows it to
  just the offers you posted, so you can find and withdraw your own.
- **All pairs** — a toggle that widens the pair selector to **every pair actually
  on the board**, including coins you haven't set up, so you can window-shop the
  whole market. Offers on a pair you haven't connected are **view-only** — the
  **Take offer** button stays disabled until you set the coin up in **Settings →
  Coins**. Your choice is remembered. (The toggle appears once you have configured
  pairs to narrow the view against; with **no** coins set up the board already
  shows every pair, so there's nothing to widen.)
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

- A **spread percentage** and a **mid price** when there are offers on both sides
  (with the mid's **~Cash** equivalent beside it, if you've set a Cashrate — see
  below).
- **One-sided** when there are offers on only one side.
- A **crossed** chip when the top bid is at or above the top ask.

That last one can look alarming if you're used to an exchange, so here's the key
point: **the board never matches trades**. When offers overlap, they simply sit
there — nothing fires automatically. You can take either side at the price shown.
Crossed just means two makers happen to have posted overlapping prices.

> **Note** — On an exchange, a crossed book would instantly execute. On the
> Corkboard nothing executes on its own — *you* choose an offer and take it. The
> ladder is only a way to *read* the board.

## Prices in your own money: the Cashrate

Coin-per-coin prices are exact, but they're not how most people *think*. The
**Cashrate** lets you read the board in your own money — without Satchel ever
touching a price feed.

Click the **Cashrate chip** in the header (just left of the merchant chip) and a
popover opens with an on/off toggle and a rate field. Enter what you call one
unit of the **quote coin** in whatever money you think in — EUR, USD, RMB,
anything. The chip always binds to the quote coin of the pair you're looking at,
and each coin's rate is **remembered separately**, so switching from BTCX/BTC to
BTCX/LTC recalls the LTC rate you set last time. On screens with no coin context
(Swaps, Wallets, Settings…) the chip greys out until you're back on the board or
an offer form.

![The Cashrate chip and its popover: the on/off toggle and the rate field for the current quote coin.](images/processed/ch07-cashrate.png){width=70%}

Once a rate is set, muted **~Cash** figures appear beside the real numbers:

- **In the ladder** — an extra unit-price column on each side, hugging the
  centre divider, showing each price level in your money. (The columns disappear
  entirely when the Cashrate is off.)
- **In the spread banner** — the cash equivalent next to the mid price.
- **On offer rows** — one figure per offer next to the amounts. An offer's two
  legs are worth the same at its own price, so a single figure values the whole
  trade.
- **In the offer form and the take-offer confirmation** — beside the "You give"
  / "You get" lines, so you sanity-check a trade in familiar terms (see the next
  two chapters).

A few things worth knowing about how ~Cash is designed:

- **It's deliberately currency-neutral.** Figures render as `~` plus a bare
  number, always with two decimals — never a `$` or a currency name. The money
  is whatever *you* meant when you typed the rate.
- **It's display-only.** Satchel never fetches a price and makes no external
  calls for this. That's by design, not a gap: it keeps your browsing private,
  and BTCX isn't listed anywhere a feed could price it anyway. Your rate is your
  reference, not a market price.
- **It's off by default.** Until you flip the toggle on, no ~Cash figure appears
  anywhere.

> **Tip** — Treat ~Cash as a sanity check, not a quote. It tells you what a
> trade is worth *at the rate you believe in* — a quick way to spot an offer
> that's priced far from your own view of the market.

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
  Satchel flips it for you. (With a Cashrate set, a muted **~Cash** figure
  beside the amounts values the trade in your own money.)
- **The protocol chip** — either **Standard (HTLC)** (the classic swap, muted
  style) or **Private (Taproot)** (the newer private swap, highlighted style).
  Both are safe; the private one simply looks like an ordinary payment on-chain.
  See the safety chapter for the difference.
- **The safety-refund times** — shown as `t2h / t1h` (for example `12h / 24h`).
  These are the auto-refund deadlines that protect you if a swap stalls; the
  taker's side unlocks first, the maker's a little later, so nobody gets stuck.
- **A freshness dot** — how recently the offer was seen, so you can favour live
  ones.
- **A copyable offer id** — a short, muted id with a copy button, so you can
  reference a specific offer when chatting with a counterparty.

![The detail pane: individual offers at one price level, with protocol chips and refund times.](images/processed/ch07-detail.png){width=80%}

## Knowing the maker: contacts on the board

If you've saved a maker as a contact, the board recognises them: their **nickname**
and a small **status dot** for their standing show on the offer's counterparty tag,
so a familiar trader stands out from a stranger at a glance. And once you've blocked
anyone, a **Hide blocked offers** toggle appears above the board — turn it on to
drop blocked makers' offers off the ladder entirely. See the chapter *"Contacts"*
for how to add a maker and what standing does (and, importantly, doesn't) do.

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

Out of the box, the Corkboard only shows offers for pairs whose coins you've
connected — offers for other pairs simply don't appear. To *see* them anyway,
flip on the **All pairs** toggle in the toolbar and browse the whole board; to
*trade* a pair, you still need both of its coins set up in **Settings → Coins**.
