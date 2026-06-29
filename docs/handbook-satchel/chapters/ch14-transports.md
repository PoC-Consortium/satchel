# Trading over Nostr & Corkboard

When you post an offer, it has to reach other people somehow. Satchel uses a
*noticeboard* for this — a public place where offers are pinned up so others can
browse them. Think of a physical cork noticeboard in a town square: you tack up a
card saying what you want to trade, people walk past and read it, and someone who
likes the deal takes your card down and contacts you.

Satchel can use two kinds of noticeboard, and they work side by side. You don't
have to choose — and most people never think about this screen at all, because
the default just works.

![Settings, Network tab: the Nostr relays and Corkboards lists.](images/processed/ch13-network-tab.png){width=85%}

## The two kinds of noticeboard

### Nostr — the default, nothing to run

*Nostr* is an open, public messaging network made up of independent servers
called *relays*. It is not owned by anyone; anybody can run a relay, and they all
speak the same simple language. Satchel uses Nostr as its default noticeboard,
and **it is ready the moment you install** — there is nothing for you to set up
or run.

A fresh copy of Satchel comes with **six recommended relays already wired in**.
When you post an offer, Satchel sends a copy to *all* of them at once. This is
deliberate. If one relay is slow, offline, or decides to ignore you, the others
still carry your offer — so no single server can quietly silence you. People call
this *censorship-resistance*, and it is the main reason Nostr is the default.

> **Note** — The six default relays are public, community-run servers on the open
> Nostr network. You can change the list, add your own, or remove ones you don't
> like in **Settings → Network**. The chapter *"Settings"* shows you how.

You can tell Nostr is working from the small **relay dot** in the header at the
top of the window. Green means at least one relay is reachable; amber means none
are responding right now. The dot only appears when you have relays configured.

### Corkboard — a noticeboard a community can run

A *Corkboard* is a small, self-hostable noticeboard. A community, a club, or a
group of friends can run one on a server they control, and then everyone points
their Satchel at its web address. It is the same idea as Nostr — a public list of
signed offers — but it lives at one address that the community owns and operates.

Corkboard is **opt-in**: a fresh install has none configured. If your community
runs one, someone will give you its URL (a web address like
`https://board.example.com`). You add it in **Settings → Network**, Satchel
checks it, and from then on your offers appear there too.

> **Tip** — Corkboard is handy when you want a smaller, more focused marketplace —
> say, just the members of one community — rather than the wide-open Nostr
> network. You can run both at once.

## How they work together

The two kinds of noticeboard are equal partners. Here's the simple model:

- **Posting fans out to everything.** When you post an offer, Satchel sends it to
  *every* noticeboard you have configured — all your Nostr relays *and* every
  Corkboard at once. You post once; it lands everywhere.
- **Browsing shows one at a time.** When you look at the **Corkboard** screen, you
  are looking at *one* board's offers. A board selector lets you switch between
  them — your Nostr offers, or a particular Corkboard — so you can see who is
  trading where.

Because posting fans out but browsing is per-board, there is one rule worth
remembering:

> **Note** — You and the person you want to trade with only need **one
> noticeboard in common**. If you both have Nostr enabled (the default), you're
> already covered. If your community runs a Corkboard and you've both added it,
> that works too. You don't need to match every board — just one.

## What a noticeboard can and cannot see

This is the part that matters most, so it's worth being clear. A noticeboard —
whether a Nostr relay or a Corkboard — is a **dumb pinboard**. It carries two
kinds of thing, and nothing else:

1. **Your public offers**, which are signed by you. An offer says "I'll give X for
   Y, valid for an hour" — and it is *meant* to be public, so people can find and
   take it. Anyone can read it; only you could have created it.
2. **Sealed messages** passed between two traders mid-swap. These are *encrypted*
   before they ever leave your machine. The board stores them and hands them on,
   but it cannot open them.

What a noticeboard **never** sees:

- **Your keys.** Your private keys and recovery phrase never leave your computer.
  Nothing on a board can derive them.
- **Your coins.** A noticeboard holds no money. It cannot touch your funds, and
  there is nothing in your wallet for it to take.
- **The contents of sealed messages.** Mid-swap messages are encrypted to the
  recipient only. The board sees scrambled bytes — not who said what.

And just as importantly, a noticeboard **cannot match you with anyone or execute
a trade**. It has no power to pair offers, move coins, or settle a swap. All of
that happens directly between you and your counterparty, enforced by the
blockchains themselves (the chapter *"Understanding Atomic Swaps"* explains how).
The board is only a place to find each other.

> **Warning** — Treat every offer on a noticeboard as a *claim*, not a promise.
> The board does not vet anyone. Your safety never comes from trusting the board
> or the other trader — it comes from the swap itself, which is built so that
> either both sides get paid or neither does. Always confirm the amounts and the
> safety timelock in the take dialog before you commit.

## Do I need to think about any of this?

Usually not. For most people the honest answer is: **install Satchel and start
trading.** Nostr is on by default with six relays prewired, so you have a working
noticeboard from minute one. You only need to open **Settings → Network** if you
want to add a community Corkboard, trim the relay list, or turn Nostr off
entirely — all of which the next chapter, *"Settings"*, walks through.
