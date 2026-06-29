# Glossary

Plain-language definitions of the terms you'll meet in Satchel and in this
handbook. Where a term has its own chapter, that's the place to go for more.

**Adaptor swap** — See *Taproot swap*. The technical name for the **Private
(Taproot)** swap method, where the two payments are linked using a hidden
adaptor secret.

**Atomic swap** — A trade between two cryptocurrencies that is all-or-nothing:
either both sides receive their coins or neither does, with no middleman holding
funds. The blockchain itself enforces it. See *"Understanding Atomic Swaps"*.

**Base / quote** — The two sides of a trading pair. In **BTCX/BTC**, BTCX is the
*base* (the thing being priced) and BTC is the *quote* (what it's priced in). The
Corkboard uses this convention to lay out bids and asks.

**BTCX / PoCX** — **BTCX** is the ticker for **Bitcoin-PoCX** (PoCX = Proof-of-
Capacity eXtended), the first coin Satchel trades. BTCX ↔ BTC is the first
supported pair.

**Confirmation** — A blockchain's way of recording that a transaction is settled.
Each new block adds one confirmation; more confirmations mean the transaction is
more firmly final. Swaps wait for a set number of confirmations before proceeding.

**Corkboard** — A self-hostable *noticeboard* a community can run on its own
server. You point Satchel at its web address to post and browse offers there. See
*"Trading over Nostr & Corkboard"*.

**Engine (pactd)** — The local program that does the real swap work on your
machine: holding your keys for the session, building and watching swaps, and
auto-refunding if needed. Satchel's UI drives it; you'll see it called "the
engine." Its technical name is `pactd`.

**HTLC** — *Hash Time-Locked Contract*: the classic, well-proven way to build an
atomic swap, labelled **Standard (HTLC)** on offers. Recognisable on the
blockchain as a swap.

**Maker / taker** — The *maker* is the person who posts an offer; the *taker* is
the person who accepts it. You can be either, depending on whether you post or take.

**Merchant** — A trading identity in Satchel: one seed, one set of keys, one data
directory. You can have several merchants (separate identities) and switch between
them in the Merchant Manager.

**Mining fee** — The small fee every blockchain charges to confirm a transaction,
paid to miners — not to Satchel. The only cost of a swap, beyond the coins
themselves.

**Node** — The software that connects you to a cryptocurrency's network and holds
your wallet for that coin (for example, Bitcoin Core for BTC). Satchel talks to
the nodes you run. See *"Setting Up Your Coins"*.

**Nostr** — An open, public messaging network of independent servers (*relays*).
Satchel's default *noticeboard*, prewired with six relays. No setup needed. See
*"Trading over Nostr & Corkboard"*.

**Noticeboard** — A public place where offers are posted and browsed. Satchel
supports two kinds: *Nostr* and *Corkboard*. A noticeboard never holds coins or
keys and can't match or execute trades.

**Offer** — A signed statement of a trade you're willing to make: "I'll give X for
Y, valid for this long." Public offers go on a noticeboard; private offers become
*slips*.

**Pair** — Two coins that can be traded against each other, for example BTCX/BTC.
**Settings → Coins** shows which pairs are ready to trade.

**Passphrase** — An optional password that encrypts the copy of your seed on this
computer. It protects against someone using your machine — but it is *not* your
recovery phrase and *not* a backup. Forget it and you must restore from your
recovery phrase. See *"Backup, Seeds & Safety"*.

**Recovery phrase (seed / seed phrase)** — The list of 12 or 24 words that *is*
your wallet. Write it down offline; never share it. It restores all your funds on
any machine. The single most important thing to protect. See *"Backup, Seeds &
Safety"*.

**Redeem** — Claiming your side of a completed swap — taking the coins you traded
for, once the linked payments unlock.

**Refund** — Getting your own locked funds back when a swap doesn't complete. The
*timelock* makes refunds automatic and guaranteed after its deadline.

**Relay** — One server on the *Nostr* network. Satchel posts to several at once so
no single relay can silence you. The header's relay dot shows whether any are
reachable.

**RPC** — *Remote Procedure Call*: the technical channel Satchel's engine uses to
talk to your nodes. You'll meet the term only when entering a node's connection
details (RPC host, port, credentials) in **Settings → Coins**.

**Slip** — A private, unposted offer in text form (it starts with `pactoffer1:`).
You hand it directly to one person — paste it into a chat — and only someone
holding it can take it.

**Taker** — See *Maker / taker*.

**Taproot swap** — The newer swap method, labelled **Private (Taproot)**. When it
completes smoothly it looks like an ordinary transaction on the blockchain, giving
better privacy. Also called an *adaptor swap*. See *"Understanding Atomic Swaps"*.

**Timelock** — A built-in deadline on the funds locked in a swap. If the swap
doesn't complete by then, your funds become refundable to you automatically. The
safety net behind every swap. You pick a **Short / Medium / Long** preset when
trading.

**Trustless** — Describes a trade where you don't have to trust the other person or
any middleman, because the rules are enforced by the blockchain itself. The whole
point of atomic swaps.
