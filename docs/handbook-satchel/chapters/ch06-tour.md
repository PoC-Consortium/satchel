# A Tour of the Interface

Now that Satchel is set up, let's walk through the screen so you know where
everything lives. Nothing here changes your funds — this is just an orientation
tour. Open the app and follow along.

Satchel's window is split into three regions: a **left navigation rail** down
the side, a thin **header** across the top, and the **main content area** that
fills the rest. The chapters that follow each dig into one screen; this one shows
you the map.

![The Satchel main window: left navigation, header, and the Corkboard in the content area.](images/processed/ch06-overview.png){width=95%}

## The left navigation

The rail on the left is how you move between screens. You can collapse it to a
slim strip with the menu toggle in the header (handy on a small window), and on a
narrow window it slides out as an overlay. At the very top sits the **Satchel**
logo with the version number underneath; if a newer release is available, a small
update badge appears there — click it to see what's new.

The navigation items are grouped so related tasks sit together:

**Public** — trading out in the open, where anyone can see and take your offers:

- **Corkboard** — the order book. Browse every offer you can trade and take one.
  This is where the app opens.
- **Post an offer** — advertise a trade for anyone to take.

**Private** — trading with one specific person instead of the whole board:

- **Create slip** — build a private offer and get a shareable code (a *slip*).
- **Take a slip** — paste a slip a friend sent you.
- **My slips** — track and cancel the private offers you've handed out.

Below the groups:

- **Swaps** — your full trade history and the swaps currently running.
- **Wallets** — per-coin balances with **Send** and **Receive** (node-backed, or a pact-seed wallet over Electrum servers; Electrum coins add an **Activity** history).
- **Contacts** — your private nicknames and standings for the people you trade
  with, kept only on this device.
- **Network** — a read-only monitor of your connections: one tab for your Nostr
  relays and one tab per Electrum-backed coin, so you can see at a glance which
  relays and servers are connected and how healthy they are.

And at the foot of the rail:

- **Settings** — appearance, language, your coin connections, the noticeboards
  Satchel talks to, fee-bumping preferences, and desktop notifications. Its tabs
  are **General**, **Coins**, **Network**, **Fees**, **Notifications**, and
  **About**.

> **Tip** — If you ever feel lost, click **Corkboard**. It's the home base and it's
> where your live swaps and activity log are docked (more on that below).

The header also carries a **globe** menu — the language picker. Click it to switch
Satchel's display language on the spot; it offers all **26 languages** Satchel
ships with, listed under their own native names. It's always there, so you can
change language any time, not just during first-run setup.

## The active merchant, and switching between merchants

A *merchant* is one trading identity — its own recovery seed, its own swap
history, kept separate from any other identity you create. Think of it like a
single wallet profile. You set up your first merchant during onboarding.

Just under the logo you'll see a clickable **active-merchant block**: a little
identicon, the merchant's name, and a shortened id. This always tells you which
identity you're trading as. There's also a matching merchant chip on the right of
the header.

You can **rename the active merchant in place**: hover the name in the block and
click the little pencil (or click the name) to edit it inline — type a new label
and press **Enter** to save, **Esc** to cancel. Only the label changes; the
merchant's id, identity, and seed are untouched, so a rename is safe even with a
swap in flight.

To switch, click either one. The merchant chip opens a dropdown with
**Manage Merchants…** and a list of your other merchants. Choosing **Manage
Merchants…** opens the full manager, where you can create a new merchant, import
one from a recovery phrase, or load a different one.

> **Note** — Satchel won't let you switch away from a merchant that has a swap in
> flight. That's a safety rule: a running swap needs its own seed available to
> finish or refund. Wait for it to complete, then switch.

## The Cashrate chip

Just to the **left of the merchant chip** sits the **Cashrate chip** — your own
price reference. When it's on and you've set a rate it reads something like
`~ 91,400.00 · BTC`; otherwise it just says **Cashrate (SYM)**. Click it and a
small popover opens with an on/off toggle and a rate field: enter what *you*
call one unit of the coin in your own money — EUR, USD, RMB, whatever you think
in. Satchel never names a currency and never fetches a price; the rate is yours
alone, remembered per coin.

The chip always binds to the **quote coin of the pair on the current screen** —
the Corkboard pair, or the pair in an offer form. On screens with no coin
context (Swaps, Wallets, Settings…) it greys out but keeps showing the last
coin's rate. With a rate set, muted **~Cash** figures appear beside prices and
amounts across the app; the Corkboard chapter covers exactly where and how to
read them.

## The header status indicators

The left of the header carries a row of small status lights. At a glance they
tell you whether Satchel can do its job. Hover any of them for a plain-language
tooltip.

- **Engine reachable** — a hub icon that is green when Satchel can talk to the
  engine (the core that holds your keys and runs swaps) and red when it can't. If
  this is red, nothing else will work until it recovers.
- **Nostr relay health** — a dot that only appears when you have relays
  configured. Green means at least one relay is connected; amber means none are
  reachable right now. (Relays are one way your offers travel — see *Settings* in
  the relevant chapter.)
- **Per-coin health** — a small glyph for each coin you've connected, bordered by
  its status. The tooltip tells you whether that coin's node is connected (and its
  current block height), not set up, or in error.
- **Live-swaps counter** — a gently pulsing swaps icon with a number badge showing
  how many swaps are running right now. Click it to jump straight to the Corkboard
  where you can act on them.

![The header status indicators: engine, Nostr relays, per-coin health, and the live-swaps counter.](images/processed/ch06-status.png){width=70%}

## The network stamp

In the centre of the header you may see a network stamp such as **RegTest** or
**TestNet**, reminding you that you are *not* using real funds. On **mainnet**
(real coins) this stamp is deliberately hidden — there's nothing to warn you
about, because that's the real thing.

> **Warning** — If there's no network stamp, you are on mainnet and trading with
> real money. Double-check amounts before you confirm anything.

If you haven't connected coins yet, you can still browse the board freely —
nothing walls off the app. What's gated is the money-moving actions, and they're
gated **per action** rather than app-wide: posting, taking, and funding stay out
of reach until you connect two live coins, with each action pointing you to
**Settings → Coins** at the moment you reach for it. (See *"Setting Up Your
Coins"* for the details.)

## The Network monitor

Click **Network** in the navigation to open a read-only health screen for your
connections. It has one tab for your **Nostr relays** and one tab per
**Electrum-backed coin**. It's a **monitor only** — it never dials or probes;
every status you see comes from the connections your normal activity already
uses.

### Nostr relays

The relays tab lists one row per relay you've configured, so you can see whether
your offers have a way out onto the network. The header reads
**"{up} / {total} connected"** — how many of your relays are live right now —
with a refresh button beside it. Each row then shows:

- A **status dot** — green when the relay is connected, amber while it's
  connecting, red if it was terminated or banned, grey otherwise.
- The **relay URL**.
- A short **status label**, the connection **latency** in milliseconds, and how
  long it's been **up**.

If you have no relays configured (or none are reachable), the screen points you to
**Settings** to add some.

![The Network monitor's Nostr tab: one row per relay with status, latency, and uptime.](images/processed/ch06-relays.png){width=80%}

### Electrum servers (per coin)

Each coin that runs over Electrum servers gets its own tab, named for the coin.
It lists every server you've configured for that coin, and shows how the app is
using them right now:

- A **health dot** — green for healthy, red for a server in back-off (with a
  short retry countdown), grey for one that's simply never been needed yet.
- The **server URL** and its **role**: **wallet** (the one your balance and
  sends run through), **view** (an independent server the app cross-checks
  against), or **standby** (configured but idle, ready to step in).
- The current **state** and **latency**.

You don't manage anything here — the app picks a wallet server and a couple of
views automatically, and if one drops it promotes a standby and moves on, so a
single server going down never interrupts you. The **"{healthy} / {total}"**
count in the header tells you how much headroom you have. It's the place to look
if a coin ever shows a connection warning: as long as at least one server is
healthy you're covered, and on mainnet a swap keeps two independent servers
agreeing before it trusts anything on-chain.

> **Note** — This screen only *shows* you relay health; you don't add or remove
> relays here. To change which relays Satchel uses, go to **Settings → Network**
> (covered in the *Settings* chapter).

## The activity log and active-swaps dock

Two panels are docked along the bottom of the content area, visible on every
page:

- **Your active swaps** — a live card for each swap in progress, showing its
  state, the two parties as **maker ↔ taker** (your own side tagged **(you)**),
  the amounts, a live confirmation-progress line, and — while nothing of yours is
  locked yet — a **cancel** button to back out before funding. Funding, redeeming,
  and refunding all happen automatically, so there are no buttons for them; a
  **dump logs** button is always there for diagnostics. The chapter on tracking
  your swaps explains them.
- **Activity log** — a running, collapsible feed of what the engine is doing,
  narrated in plain language. You don't need to watch it, but it's reassuring to
  glance at when a swap is mid-flight.

## The tray icon

Satchel also keeps a small icon in your **system tray**. Hover it and the
tooltip mirrors the header's live-swaps counter — *"Satchel — no swaps in
flight"*, *"Satchel — 1 swap in flight"*, and so on — so you can check on things
without opening the window. A **left-click** brings the window back, and the
icon's menu offers **Open Satchel** and **Quit**.

> **Note** — Quitting from the tray menu goes through exactly the same
> fund-safety exit gate as closing the window: if a swap is in flight, Satchel
> warns you and offers **Keep running** first. The tray can never bypass that
> protection. (If your desktop has no tray area, Satchel simply runs without the
> icon — nothing else changes.)

That's the whole map. The next chapters take each screen in turn, starting with
the Corkboard.
