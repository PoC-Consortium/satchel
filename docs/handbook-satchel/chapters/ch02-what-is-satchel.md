# What Satchel Is

Satchel is a desktop app for **trading one cryptocurrency for another, directly
with another person**. There is no company in the middle. No one takes custody
of your coins, no one matches your trades, and there are no trading fees. You and
your counterparty swap directly, and the blockchain itself makes sure neither of
you can cheat.

This chapter explains, in plain language, what that means and how the pieces fit
together. You don't need any of it to start trading — but it helps to know what
is actually happening to your money.

## Trading without a middleman

On a normal exchange, you hand your coins to the exchange, it holds them, and it
arranges trades on your behalf. You are trusting that company not to lose your
funds, not to freeze your account, and not to disappear. Plenty have.

Satchel works differently. It uses an *atomic swap* — a way of trading two
coins so that the exchange happens **all at once or not at all**. Either both
sides get what they agreed to, or both sides keep what they started with. There
is no in-between where one person has paid and the other has run off. The word
*atomic* just means "indivisible": the trade cannot be left half-finished.

Because the blockchain enforces this, you never have to trust the other person —
and you never have to trust Satchel with your coins either. This is what we mean
by *trustless*: not "no one is trustworthy", but "you don't need to trust anyone
for your money to be safe".

What this gives you:

- **No custody.** Your coins stay under your control the entire time. Satchel
  never holds them.
- **No exchange, no account.** There is nothing to sign up for and no company
  that can freeze you out.
- **No fees.** Satchel takes nothing. You pay only the ordinary network fee that
  every blockchain charges to process a transaction — the same fee you'd pay
  sending coins to a friend.
- **No matching engine.** Nobody decides who trades with whom. You browse offers
  and pick the one you like.

## The village market

Satchel borrows its names from an old-fashioned village market square, because
the picture is a good one.

- A *pact* is the **deal** — the agreement to swap so much of one coin for so
  much of another. (Behind the scenes, "Pact" is also the name of the engine
  that carries out the deal; more on that below.)
- The *corkboard* is the **noticeboard** in the square where people pin up their
  offers for everyone to see. You browse it, and when you see a deal you like,
  you take it.
- Your *satchel* is your **bag** — where your trades settle and where you keep
  track of what you've done.

So in one sentence: a *pact* is posted on the *corkboard*, and settled into your
*satchel*. You will see all three words in the app, and now you know what each
one means.

## The three moving parts

Under the hood, Satchel has three pieces. The important thing to understand is a
single hard rule: **your coins and your keys never leave your own machine.**

![How the parts fit together: everything that touches your coins runs on your machine; the hosted side only carries signed notes.](images/processed/ch02-architecture.png){width=85%}

**On your machine** (and only here):

1. **Satchel** — the app you look at. It shows you offers, balances, and the
   progress of your swaps. It is the friendly face; it does not hold your keys.
2. **The engine** — the part that does the real work of a swap. It holds your
   keys, builds and watches the swap transactions, and — importantly —
   automatically gives you your money back if the other side walks away. (Its
   proper name is *Pact*; the app simply calls it "the engine".)
3. **Your coin nodes** — your own copies of the Bitcoin-PoCX and Bitcoin
   networks. Satchel is a *node-backed* app, which means it trades using wallets
   that live on nodes you run. We cover setting these up in the chapter
   *"Installing Satchel"* and connecting them in *"Setting Up Your Coins"*.

**Hosted elsewhere** (and trusted with almost nothing):

4. A **transport** — either a **Nostr** relay or a **Corkboard** — that simply
   carries your offers and a few sealed coordination messages between you and
   your counterparty. That's all it does. It never touches your coins, never
   matches trades, never charges anything, and cannot read your private
   messages. It is a noticeboard and a postbox, nothing more.

> **Note** — This wall is the whole point. Everything that could touch your money
> runs on your computer. The hosted side sees only signed offers and sealed
> envelopes — never your coins, never your keys. We come back to it in the
> chapter *"Transports"*.

## What you can trade today

The first supported pair is **BTCX ↔ BTC** — that is, Bitcoin-PoCX traded
against Bitcoin. *BTCX* is the ticker for Bitcoin-PoCX. More coins will follow,
and the app is built so new coins can be added without a new release.

## Setting expectations

Satchel is **alpha** software — an early release. The underlying swap protocols
(both the original style and the newer, more private Taproot style) are under
external security review. The design is careful and the safety guarantees are
real, but this is new software handling real value.

> **Warning** — Because Satchel is alpha and under audit, treat it accordingly:
> start small, keep amounts modest while you learn, and never put in more than
> you are comfortable risking on early software. Above all, follow the safety
> rules in the chapter *"Staying Safe"* — write down your recovery phrase, and
> never share it.
