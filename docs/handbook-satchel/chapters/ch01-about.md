# About this Handbook

Welcome. This is the **Satchel User Handbook** — a friendly, step-by-step
guide to trading cryptocurrency directly with another person, with no exchange
in the middle and no one ever holding your coins but you.

If that sounds technical, don't worry. This handbook assumes **no background in
cryptocurrency**. Wherever a new idea comes up — a *swap*, a *seed*, a
*recovery phrase* — we explain it the first time you meet it, in plain
language, before moving on. You will need to be comfortable installing a
desktop application and following on-screen steps; everything beyond that, we
walk you through.

## Who this is for

This handbook is for **anyone who wants to trade one cryptocurrency for another
without trusting a middleman**. You might be:

- Someone curious about *trustless trading* — swapping coins peer-to-peer in a
  way where the blockchain itself enforces the deal, so neither side can run off
  with the other's money.
- Someone who already holds Bitcoin or Bitcoin-PoCX and wants to trade between
  them safely, on your own machine, on your own terms.
- A member of a community that runs its own trading noticeboard and wants to use
  it.

You do **not** need to know how Bitcoin works under the hood, what a
"transaction" is, or any programming. If you ever do want the deep technical
detail — how the swap transactions are built, the exact protocol — that lives in
a separate companion book, the *Pact Developer & Integrator Handbook*. This
handbook points you there only when it is genuinely useful.

## How this handbook is organised

The handbook is in four parts, moving from "getting set up" to "trading" to
"the finer points":

1. **Getting Started** — what Satchel is, installing it, creating your trading
   identity, and connecting your coins. Start here. *(You are reading this
   part.)*
2. **Trading** — a tour of the app, browsing the **Corkboard**, posting an
   offer, taking someone else's, watching a swap complete, your wallet view, and
   private one-to-one offers.
3. **Transports & Network** — how your offers reach other people (over **Nostr**
   or a **Corkboard**), and the settings that control it.
4. **Safety & Reference** — keeping your funds safe, what really happens during a
   swap, troubleshooting, a glossary of terms, and where to get help.

You do not have to read straight through. If you just want to get trading, work
through Part 1 once, then dip into Part 2 as you go.

## Conventions used in this book

We use a few simple conventions throughout.

- **Callouts** flag things worth pausing on:

  > **Tip** — a shortcut or a friendlier way to do something.

  > **Note** — background, a clarification, or a pointer to another chapter.

  > **Warning** — something that could cost you funds if you get it wrong. Please
  > read these carefully.

- **Text styles** carry meaning. **Bold** marks something you see on screen — a
  button, a field, a menu, or the name of a screen. `Monospace` marks anything
  you type exactly, a file path, or a web address. *Italics* introduce a new term
  the first time it appears.

- **Numbered steps** are procedures to follow in order. Bulleted lists are just
  lists.

- **Screenshots** show you what a screen looks like. Satchel is young and
  changing quickly, so a screenshot may differ slightly from what you see — the
  labels and the order of steps are what matter, and we keep those accurate.

- **Cross-references** name another chapter by its title — for example, *see the
  chapter "Setting Up Your Coins"* — rather than by number, because numbers shift
  as the book grows.

## What this edition tracks

Rather than a release-version number, this handbook tracks the **source revision**
it was checked against: it was verified against commit `424834b` (June 2026). The
commit hash on the copyright page is the single status marker — when the code
moves, the hash is bumped and the affected pages are updated.

> **Note** — The first trading pair is **BTCX ↔ BTC** (BTCX is Bitcoin-PoCX).
> Screens, labels, and details evolve with the code; where an exact detail
> matters, the app itself is the final word.

## A word on safety

Trustless trading puts you in full control — which also means **you alone are
responsible for your funds**. This is different from an exchange, where a
company holds your coins (and can lose or freeze them). With Satchel, no one can
freeze or seize your funds. The flip side is that no one can recover them for
you either. Two rules carry almost all the safety you need:

> **Warning** — When you create your trading identity, Satchel shows you a
> *recovery phrase* — a list of 12 or 24 words. **Write it down on paper and keep
> it somewhere safe.** It is the only way to recover your funds if your computer
> is lost or breaks.

> **Warning** — **Never share your recovery phrase with anyone, and never type it
> into a website.** Anyone who has those words can take your funds. No genuine
> support person will ever ask for it. Satchel will never ask you to enter it on
> a web page.

We come back to safety in detail in the chapter *"Staying Safe"*, but those two
rules are worth carrying with you from page one.

## Where to get help

- The project's repository and website carry release notes, design notes, and
  the latest news.
- The chapter *"Troubleshooting"* covers the most common snags — a node that
  won't connect, an offer that won't post — with fixes.
- The chapter *"Getting Help"* lists the community channels where you can ask
  questions.
- If you want the technical "how it works" underneath Satchel, the companion
  *Pact Developer & Integrator Handbook* is the place to look.
