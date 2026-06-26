# Settings

Everything you can configure in Satchel lives behind the **Settings** gear, which
you'll find in the top-right header and again at the foot of the left-hand
navigation. Settings is organised into five tabs — **General**, **Coins**,
**Network**, **Fees**, and **About** — and this chapter takes them in turn.

![The Settings screen with its tabs.](images/processed/ch14-settings-tabs.png){width=85%}

Most of what's here you'll set once and forget. None of it touches your coins or
your recovery phrase — these are preferences and connection details, nothing
more.

## General

The **General** tab holds your look-and-feel preferences.

- **Theme** — choose **Light**, **Dark**, or **System**. **System** follows
  whatever your computer is set to (dark in the evening, light by day, if your OS
  does that). The change applies instantly and is remembered next time you open
  the app.
- **Language** — selects the app's display language. Satchel ships in **26
  languages**, each listed under its own native name; pick one and the app
  switches straight away. (There's also a globe picker in the header for changing
  language on the fly.)
- **Mode** — a **Watch-only mode** toggle. Switch it **on** and Satchel becomes a
  browse-only window: you can look around the **Corkboard** and **withdraw your
  own offers**, but you can't **post**, **take**, or **fund** a swap — handy for
  watching the market before you connect any coins. Switch it **off** to trade
  normally. Either way Satchel **restarts the session** to apply the change, so
  give it a moment to come back.
> **Tip** — There's also a quick theme and language reach from the header, but the
> **General** tab is the canonical place to set them.

> **Note** — In **Watch-only mode** a **"Watch only"** badge shows in the header,
> and the post, take, and fund actions are switched off until you turn the toggle
> back off and have **two live coins** connected.

## Coins

The **Coins** tab is where you tell Satchel how to reach the cryptocurrency nodes
you trade with — your Bitcoin node, your BTCX node, and so on. Because this is a
big topic with its own setup flow, it has a dedicated chapter.

> **Note** — See the chapter *"Setting Up Your Coins"* for the full walkthrough:
> adding a coin, entering its connection details, validating it, and reading the
> status pills and trading-pair list. The same screen appears both there and here
> under **Settings → Coins**.

In short: each coin shows a status pill — **Not set up**, **Connected**, or
**Connection error** — and a **Set up** or **Edit connection** button. Below the
coins, a **Trading pairs** list tells you which pairs are ready to trade.

## Network

The **Network** tab controls your noticeboards — the places your offers are
posted and browsed. (For the bigger picture of how these work, see the chapter
*"Trading over Nostr & Corkboard"*.) It holds just two editable lists. (The old
read-only "network mode" row is gone — which network you're on is fixed when
Satchel launches and is shown in the top-bar badge instead.)

### Nostr relays

This is the list of *Nostr relays* — the public servers that carry your offers on
the open Nostr network. A fresh install arrives with six recommended relays
already in the list, so you normally don't need to touch this.

- To **add** a relay, type its address. A valid relay address starts with `wss://`
  (for example `wss://relay.damus.io`). Satchel checks the format as you type.
- To **remove** one, delete it from the list.

> **Note** — If you clear the Nostr relays list completely, **Nostr is turned
> off**. That's a valid choice — for instance, if you only ever trade over a
> private Corkboard — but with an empty list your offers won't reach the open
> Nostr network at all.

> **Tip** — This is the place you **add and remove** relays, but to *watch* how
> they're doing — which are connected, their latency and uptime — open the
> top-level **Relays** screen in the left navigation (described in the *Tour of
> the Interface* chapter). That screen is monitor-only.

### Corkboards

This is the list of *Corkboard* noticeboards — the self-hostable boards a
community might run. A fresh install has none; you add one only if you have its
address.

- To **add** a Corkboard, type its web address. A valid Corkboard address starts
  with `https://` (or `http://`). If the list is empty, the field reads **None
  configured** — which is perfectly fine.
- To **remove** one, delete it from the list.

### Saving your changes

Both lists are edited together. When you're happy, press **Save & reconnect**.
Satchel saves the new relay and Corkboard lists and reconnects to them right away —
you'll see the header's relay dot update to reflect what's now reachable.

> **Tip** — If a freshly added relay or board shows amber or red after saving,
> double-check the address for a typo. The chapter *"Troubleshooting"* has a short
> section on noticeboards that won't connect.

## Fees

The **Fees** tab holds one advanced, optional section — **Fee bumping** — that
sets the limits Satchel works within when it raises an on-chain fee to get a stuck
swap transaction confirmed. Satchel now does the bumping automatically, tracking
the going fee market for you rather than stepping up by a fixed amount, so these
are just the guard-rails. **You don't need to touch this to trade.** The defaults
are sensible; they're safety-versus-cost trade-offs for the rare swap that needs a
nudge.

A couple of things to know before you change anything:

- The settings apply to your **active merchant**.
- New values affect **future** bumps only. A swap that's already funded keeps the
  policy it was funded under, so changing these won't disturb a trade in flight.

The three knobs are:

- **Max feerate (sat/vB)** — the ceiling for every fee bump, so a runaway fee
  market can never drain you. Range 1–500; **default 500** (also the hard system
  maximum).
- **Funding bump reservation (×)** — how much extra balance the funds check sets
  aside as headroom for a possible future bump. Range 1–100; **default 3**.
- **Redeem over-provision (×)** — a multiplier on the market rate a Private
  (Taproot) redeem commits to up front (its fee is fixed when the swap is signed
  and can't be raised by ordinary replacement). Range 1–100; **default 1** — i.e.
  it commits at the live market rate, with no padding, and the engine accelerates
  it with a child transaction only if the market actually climbs while it's
  pending. Raise it above 1 if you'd rather pre-pay a cushion. Applies to new
  swaps only.

Press **Save** to apply your changes — they take effect **live, with no restart** —
or **Reset to defaults** to put all three back the way they came.

![The Fees tab: the fee-bumping knobs with Save and Reset to defaults.](images/processed/ch14-fees-tab.png){width=80%}

> **Note** — If you've never heard of fee bumping or RBF, that's fine — leave this
> tab alone. Satchel handles fees for you out of the box; these knobs exist only
> for people who want fine control over the cost of a stalled swap.

## About

The **About** tab is your reference corner.

- **Version** — the exact Satchel version you're running, with an **Up to date**
  badge or an update notice. If a newer release exists, Satchel tells you here (and
  with a small badge by the logo in the navigation); use **Check for updates** to
  look again.
- **Where your keys live** — a short trust note reminding you that your keys and
  recovery phrase stay on your own machine and are never sent to Satchel, to any
  noticeboard, or to anyone else.
- **Risk disclaimer** — the self-custody notice. Satchel is provided without
  warranty; you alone hold your keys and are responsible for safeguarding your
  recovery phrase and the funds you trade.

## Where your settings are stored

All of these preferences — your theme, language, coin connections, relay list,
and Corkboard list — are saved in a small file named `satchel.json` on your own
computer. Nothing is stored in the cloud or on any server.

> **Note** — `satchel.json` holds *settings only* — never your recovery phrase or
> any private key. Those are handled separately and far more carefully; the
> chapter *"Backup, Seeds & Safety"* explains exactly where they live.
