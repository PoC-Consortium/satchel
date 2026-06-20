# Frequently Asked Questions

Short answers to the questions people ask most. Each one points to a fuller
chapter if you want the detail.

## Is there a fee?

**No platform fee.** Satchel and the project behind it take *nothing* from your
trades — there is no commission, no spread, no membership cost. The only cost you
pay is the ordinary **mining fee** (also called a network or transaction fee) that
every blockchain charges to confirm a transaction. That fee goes to the
blockchain's miners, never to us, and you'd pay it for any on-chain transaction.

## Who holds my coins?

**You do — always.** At no point does Satchel, a noticeboard, or your trading
partner hold your funds. Your coins stay in wallets you control, on nodes you run,
secured by keys only you have. During a swap they're briefly locked into an
all-or-nothing arrangement enforced by the blockchain itself — not handed to anyone.

## Do I need to run nodes?

**Yes, for now.** Satchel works by talking to your own cryptocurrency nodes (your
Bitcoin node, your BTCX node, and so on), which is what keeps everything fully in
your control. Setting them up is a one-time job covered in the chapter *"Setting Up
Your Coins"*.

> **Note** — A future *nodeless* build is planned, which will let Satchel act as a
> self-contained wallet without you running full nodes. For this alpha, nodes are
> required.

## Is it safe to close the app?

**Not during a swap.** While a swap is in progress, keep Satchel and your nodes
running — the engine needs to be awake to broadcast each step (including your
refund) on time. If you try to quit mid-swap, Satchel warns you and offers **Keep
running** in the background; take that. When no swap is active, you can close it
freely. See *"Backup, Seeds & Safety"*.

## What if the other side disappears?

**You get refunded, automatically.** Every swap has a *timelock* — a built-in
deadline. If your counterparty walks away and the swap can't complete, your locked
funds become refundable to you once that deadline passes, with no cooperation
needed from them. The chapter *"Understanding Atomic Swaps"* explains why this is
guaranteed.

## What's the difference between public and private offers?

A **public offer** is posted to a noticeboard (Nostr or a Corkboard) where anyone
can browse and take it — that's the **Corkboard** screen. A **private offer** is a
*slip* you create and hand directly to one person (paste it into a chat, say); it
isn't posted publicly, and only someone holding the slip can take it. Use public
offers to find traders; use private offers to trade with a specific friend. Both
settle with the same atomic-swap safety.

## Can I trade on mainnet?

**Yes.** Both swap types — **Standard (HTLC)** and **Private (Taproot)** — run on
real mainnet today.

> **Warning** — Satchel is **alpha** software still under external security review.
> Mainnet trading works, but treat it accordingly: start small and only trade
> funds you're prepared to put at risk while the software and its audit mature.

## What coins are supported?

The first supported pair is **BTCX ↔ BTC** (BTCX is Bitcoin-PoCX). **Litecoin
(LTC)** was the first added third coin. The list grows over time: coins are defined
in a plain text file (`coins.toml`) that ships beside the app, so new ones can be
added without rebuilding Satchel. **Settings → Coins** shows what's available and
which pairs are ready to trade.

## Where are my keys and seed stored?

**On your own machine, never in the cloud.** Your recovery phrase and private keys
are held locally by the engine and, if you chose a passphrase, stored encrypted.
Satchel never sends them to any server, noticeboard, or person — and your settings
file (`satchel.json`) never contains them. The chapter *"Backup, Seeds & Safety"*
covers this in full, including the two golden rules for your recovery phrase.

## Will anyone from the project ask for my recovery phrase?

**Never.** No genuine support person and no one from this project will *ever* ask
for your recovery phrase. Anyone who does is trying to steal from you. Satchel only
asks for it on its own first-run screen — never on a website, chat, or form.
