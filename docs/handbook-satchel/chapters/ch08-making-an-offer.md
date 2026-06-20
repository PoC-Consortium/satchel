# Making an Offer

Instead of taking someone else's offer, you can post your own and let others come
to you. Click **Post an offer** in the left navigation to open the offer form.

Posting an offer locks nothing. It's just a signed advert pinned to the board. A
swap only starts when someone takes your offer and both sides fund their legs —
and you can withdraw the offer any time before that. So there's no risk in
posting; take your time getting the terms right.

![The Post an offer form: give and receive coins, price, swap type, and safety timelock.](images/processed/ch08-offer-form.png){width=70%}

## Filling in the form

### You give / You receive

Pick the coin you're offering under **You give** and the coin you want under
**You receive**, each with an amount. Beneath each amount field Satchel shows your
**live balance** in that coin (pulled from your own node), so you can see what you
have to work with. The two coins must be different, and both their nodes need to
be live — if a node is down, Satchel blocks the post and tells you to start it.

### Price

The **Price** field lets you work the way an exchange does: enter a unit price and
one amount, and Satchel fills in the other amount for you. It works in both
directions — type the receive amount and it back-fills the price. The label under
the field reminds you of the units (quote coin per give coin).

### Swap type

If the pair you chose supports more than one kind of swap, a **Swap type**
dropdown appears so you can pick:

- **Standard (HTLC)** — the classic atomic swap.
- **Private (Taproot)** — a newer style that looks like an ordinary payment
  on-chain.

If the pair only supports one type, Satchel just shows it as a line of text —
there's nothing to choose. Both are equally safe; the safety chapter explains the
difference.

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

> **Note** — There are **no platform fees**. The Corkboard charges nothing to post
> or trade. The only cost is the ordinary on-chain mining fee you'd pay for any
> Bitcoin-style transaction, and that's only paid once a swap actually runs.

## What happens after you post

Your offer **fans out to all of your noticeboards at once** — every Corkboard and
every Nostr relay you've configured — so the widest possible audience can see it.
Satchel then takes you to the Corkboard, where your own offer is marked with a
"your offer" tag and a "mine here" dot at its price level.

From there you simply wait. If someone takes your offer, a swap begins and you'll
see it in **Your active swaps** and on the **Swaps** page. If you change your mind
first, find the offer (the **Mine** filter helps) and click **Withdraw** — it's
instant and free.
