# Settings

Everything you can configure in Satchel lives behind the **Settings** gear, which
you'll find in the top-right header and again at the foot of the left-hand
navigation. Settings is organised into four tabs — **General**, **Coins**,
**Network**, and **About** — and this chapter takes them in turn.

![The Settings screen with its four tabs.](images/processed/ch14-settings-tabs.png){width=85%}

Most of what's here you'll set once and forget. None of it touches your coins or
your recovery phrase — these are preferences and connection details, nothing
more.

## General

The **General** tab holds your look-and-feel preferences.

- **Theme** — choose **Light**, **Dark**, or **System**. **System** follows
  whatever your computer is set to (dark in the evening, light by day, if your OS
  does that). The change applies instantly and is remembered next time you open
  the app.
- **Language** — selects the app's display language. At the moment **English** is
  the only language available; more may come later.
- **Auto-fund swaps** — controls whether Satchel locks *your* side of a swap for
  you. It's **on by default**, and the change applies live (no restart). When
  **on**, the moment a swap you've made or taken needs your coins, the engine funds
  your side automatically — you never click **fund**. When **off**, Satchel waits
  for you to fund each swap by hand from the active-swaps dock, alerting you with a
  tone and a banner when it's your turn (see *"Tracking Your Swaps"*).

> **Note** — Leaving **Auto-fund swaps** on is safe because offers are *one-shot*:
> once an offer is taken it can't be taken again, so the most you can ever fund is
> the size of the offers you posted. Turn it off only if you prefer to confirm
> every swap by hand.

> **Tip** — There's also a quick theme and language reach from the header, but the
> **General** tab is the canonical place to set them.

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
*"Trading over Nostr & Corkboard"*.) It shows your current network at the top, and
then two editable lists.

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
