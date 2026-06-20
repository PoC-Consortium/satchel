# Understanding Atomic Swaps

You can trade happily with Satchel without ever reading this chapter — the app
handles the machinery for you. But many people sleep better once they understand
*why* a trade with a stranger is safe even though there's no exchange in the
middle. That's what this chapter is for. It's conceptual and friendly; if you want
the real cryptography, the companion *Pact Developer & Integrator Handbook* has
every byte.

## The problem: trading without trust

Imagine you want to swap some BTCX for someone else's BTC. You've never met them.
Whoever sends first is taking a risk: what if the other person just keeps the
coins and vanishes? On a traditional exchange, a company sits in the middle holds
both sides' money and makes sure the trade is fair — but then you have to trust
*that company* not to lose, freeze, or steal your funds.

An *atomic swap* removes the middleman without adding that risk. "Atomic" means
**all-or-nothing**: the trade is built so that either *both* sides receive their
coins, or *neither* does. There is no in-between where one person walks away with
both halves.

## How "all-or-nothing" works

The trick is to **link the two payments together** so that claiming one
automatically makes the other claimable. Picture two locked boxes — one holding
your BTCX, one holding their BTC — fitted with a clever shared lock. The moment one
box is opened, opening it reveals the secret that unlocks the other. Neither box
can be opened on its own.

So the sequence is safe at every step:

- Both sides lock their coins into these linked arrangements on their respective
  blockchains. Locked funds can't be spent by anyone yet.
- When the first party claims their side, the act of claiming **reveals a secret**
  on the blockchain.
- The other party sees that secret and uses it to claim their side.

If everyone plays along, both get paid. And if someone tries to cheat by claiming
their half without letting the other claim theirs — the maths simply doesn't allow
it. The very action that releases one half is what makes the other half
releasable.

> **Note** — Crucially, the blockchains themselves enforce all of this. No
> noticeboard, no server, and no person can override it. That's why your safety
> never depends on trusting the board or the other trader.

## The safety net: timelocks

There's one more thing to handle: what if the other side simply does nothing —
locks up, gets cold feet, or disappears halfway through? You don't want your coins
stuck in a locked box forever.

That's what the *timelock* is for. Every locked box has a **built-in deadline**.
If the swap hasn't completed by then, the lock releases your funds **back to you**.
You get a full refund, automatically, with no cooperation needed from the other
side.

> **Tip** — This is why Satchel asks you to pick a **safety timelock** (Short,
> Medium, or Long) when you post or take an offer. It's choosing how long to wait
> before that refund deadline kicks in. **Medium** is a sensible default. Longer
> gives a slow swap more breathing room; shorter gets your money back sooner if
> something stalls.

The two halves of a swap have slightly different deadlines on purpose — the engine
arranges them so that the person who could be left exposed always has time to
react or refund. You don't have to think about this; Satchel sets it up correctly
and watches the clock for you. (The one thing it needs from you is to **stay
running** until the swap finishes — see the chapter *"Backup, Seeds & Safety"*.)

So the worst realistic case isn't "I lose my coins." It's "the trade didn't
happen, and after the timelock I got my coins back." That's the confidence atomic
swaps are designed to give you.

## Two flavours of swap

Satchel knows two ways to build a swap. The cryptography differs under the hood,
but what matters to *you* comes down to privacy and how the trade looks on the
blockchain. You usually don't pick — Satchel chooses the best method a given
trading pair supports — but you'll see them labelled on offers.

### Standard (HTLC)

The classic, well-proven method, labelled **Standard (HTLC)** on offers. It uses a
special kind of locked output that anyone inspecting the blockchain can recognise
as a swap. It's robust, battle-tested, and the default for the first pair,
BTCX ↔ BTC.

### Private (Taproot)

A newer method, labelled **Private (Taproot)**. When everything goes smoothly, a
swap built this way looks like an ordinary, everyday transaction on the
blockchain — there's no obvious "this was a swap" fingerprint for outside observers
to see. It uses a more modern Bitcoin feature called Taproot to achieve this.

> **Note** — Both methods are equally *atomic* — equally all-or-nothing, equally
> protected by timelocks. The difference is privacy and on-chain footprint, not
> safety. Where a pair supports both, the offer form lets you choose; where it
> supports only one, Satchel picks it for you.

## Want the real cryptography?

This chapter is the comfortable, plain-language version. If you want to know
exactly how the secret is constructed, how the locking scripts are written, how
the Taproot signatures combine, and how the timelocks are computed down to the
second, that all lives in the **Pact Developer & Integrator Handbook**. It's
written for developers, but it's there if curiosity strikes.

> **Note** — One honest caveat: Satchel is *alpha* software under external
> security review. Both swap methods run on mainnet today, but the audit is still
> ongoing. Trade with funds you're prepared to put at risk while the software
> matures — and lean on the safety habits in *"Backup, Seeds & Safety"*.
