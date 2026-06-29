# Making an Offer

Instead of taking someone else's offer, you can post your own and let others come
to you. Click **Post an offer** in the left navigation to open the offer form.

Posting an offer locks nothing. It's just a signed advert pinned to the board. A
swap only starts when someone takes your offer and both sides fund their legs —
and you can withdraw the offer any time before that. So there's no risk in
posting; take your time getting the terms right.

![The Post an offer form: pair, direction, amount, price, swap type, and safety timelock.](images/processed/ch08-offer-form.png){width=70%}

## Filling in the form

The form reads top to bottom the way you'd describe a trade out loud: *which pair,
which way, how much, at what price.*

### Pair

First choose the **Pair** you want to trade — a single canonical pairing such as
**BTCX / BTC**. The dropdown lists every pair your connected coins can form, and
it defaults to whatever pair you were last looking at on the Corkboard. If it's
empty, you simply haven't connected enough coins yet — the form tells you to
connect at least two under **Settings → Coins**.

### Sell / Buy

A two-button **direction toggle** sets which way you're trading the pair's *base*
coin: **Sell {base}** means you give the base coin, **Buy {base}** means you
receive it. (If you've used an earlier version: the old flip button is gone —
this toggle replaces it, and it's clearer about what's happening.)

### Amount

The **Amount** is always entered in the **base coin**, with the base symbol shown
at the end of the field. Just beneath it Satchel shows your **live balance** in
that coin (pulled straight from your own node), so you can see what you have to
work with.

### Price

The **Price** is always quoted **quote-coin per base-coin**, and — this is the
nice part — it **doesn't change when you flip Sell to Buy.** The price of the pair
is the price of the pair; only the direction of the trade changes. A small label
spells out the unit ("{unit} per {base}") and shows the raw rate as a hint.

Next to the price is a **denomination dropdown** — **BTC / mBTC / µBTC / sat** —
so you can quote in whatever unit you think in. This is the **same** denomination
setting the Corkboard uses, so the two always agree and never drift apart.

### Give / get summary

From your amount and price, Satchel shows a plain **"You give …" / "You get …"**
summary so there's no ambiguity about what you're actually offering before you go
any further.

### Swap type

If the pair you chose supports more than one kind of swap, a **Swap type**
dropdown appears so you can pick:

- **Standard (HTLC)** — the classic atomic swap.
- **Private (Taproot)** — a newer style that looks like an ordinary payment
  on-chain.

If the pair only supports one type, Satchel just shows it as a line of text —
there's nothing to choose. Both are equally safe; the safety chapter explains the
difference. **Private (Taproot)** is now selectable on **mainnet** too, not just
on the test networks, whenever both coins support Taproot.

### Safety timelock

The **Safety timelock** is the auto-refund window: if a swap stalls, this is how
long before your funds automatically come back to you. Rather than ask you for
raw block times, Satchel offers three presets:

| Preset | Refund times (t2 / t1) | What it means |
|---|---|---|
| **Short** | ~6h / ~12h | Funds auto-refund fastest if a trade stalls — but the smallest safety margin. |
| **Medium** (default) | ~12h / ~24h | A balanced window. Recommended for most trades. |
| **Long** | ~18h / ~36h | The widest safety margin; auto-refund takes longest if a trade stalls. |

The two numbers are the two sides' refund deadlines (the taker's leg unlocks at
the shorter one, yours at the longer). A **shorter** timelock gets your money back
faster if something goes wrong, but leaves a thinner margin for the swap to
complete normally. A **longer** one is the safest choice but means a stalled swap
takes longer to unwind. When in doubt, leave it on **Medium**.

> **Tip** — The timelock only matters if a swap *stalls*. A normal swap completes
> in minutes; the timelock is purely the safety net.

### Valid for (minutes)

**Valid for** sets how long the offer stays listed, in minutes (default 60).
While Satchel is open, the engine keeps the listing fresh automatically; once the
window passes, the offer expires. Closing the app also withdraws it.

## Reviewing and confirming

When the form is complete, click **Post offer**. Satchel shows a **review dialog**
before anything goes out, summarising:

- what you give and what you receive,
- the swap type,
- the safety-refund window (`t2h / t1h`),
- how long the offer is valid, and
- a **network-cost preview** so you know roughly what the on-chain fees will be.

Check it over, then confirm.

### The funds check

Before it lets you confirm, the review dialog quietly checks that your wallet can
actually fund your side — and it checks for the **amount plus the on-chain funding
fee**, not just the bare amount. If you'd come up short, it shows a red **"Not
enough … (amount + funding fee)"** alert telling you roughly what you need versus
what you have, and the confirm button stays **disabled** until you lower the
amount or top up. This catches the awkward case where you have *almost* enough but
not quite enough to also pay the miner.

> **Tip** — Fee figures in the preview are always shown to **8 decimal places**
> (for example `0.00100000`), so you can read them exactly without rounding
> surprises.

> **Note** — There are **no platform fees**. The Corkboard charges nothing to post
> or trade. The only cost is the ordinary on-chain mining fee you'd pay for any
> Bitcoin-style transaction, and that's only paid once a swap actually runs.

### The wallet-lock check

Satchel also checks, before it posts, that the node wallet for the coin you're
**giving** isn't locked. If that wallet is encrypted and currently locked, posting
is refused up front with a clear message telling you to **unlock it first** — run
`walletpassphrase` on the node — and to keep it unlocked until the swap completes.

> **Warning** — A locked wallet can still *read* its balance but cannot *sign* the
> funding transaction. Without this check the offer could be taken and then strand
> at funding, unable to lock your side. Unlock the give-coin's wallet before you
> post, and leave it unlocked for the life of the swap.

## What happens after you post

Your offer **fans out to all of your noticeboards at once** — every Corkboard and
every Nostr relay you've configured — so the widest possible audience can see it.
Satchel then takes you to the Corkboard, where your own offer is marked with a
"your offer" tag and a "mine here" dot at its price level.

From there you simply wait. If someone takes your offer, a swap begins and you'll
see it in **Your active swaps** and on the **Swaps** page. If you change your mind
first, find the offer (the **Mine** filter helps) and click **Withdraw** — it's
instant and free.
