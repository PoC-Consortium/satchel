# Tracking Your Swaps

Once a swap is under way, you can follow it from start to finish. There are two
places to look: the **Swaps** page, which is your read-only ledger, and the
**active-swaps dock**, which appears beneath every page and is where the
occasional action buttons live.

> **Tip** — You don't have to keep the window in front, either. When Satchel is
> in the background, swap milestones also arrive as **desktop notifications**
> (configurable under **Settings → Notifications**), and the **tray icon**'s
> tooltip always shows how many swaps are in flight. While the window has focus,
> notifications stay silent — the dock is already telling you the story.

## The Swaps page

Click **Swaps** in the left navigation. This page is a **read-only ledger** — it
shows you everything but has no action buttons (those live in the active-swaps dock,
covered below). It's split into two sections, newest first:

- **In flight** — swaps currently running.
- **History** — swaps that have finished, refunded, or been aborted.

![The Swaps page: in-flight swaps above, finished trades below.](images/processed/ch10-swaps.png){width=90%}

> **Note** — The ledger lists **this machine's** swaps. If you run the same
> recovery phrase on more than one machine (see the *Backup, Seeds & Safety*
> chapter), the other machine's swaps appear read-only in the active-swaps dock
> under **Another machine** — they join this ledger only once you take them
> over.

### The columns

Each row shows:

| Column | What it tells you |
|---|---|
| **swap** | The swap's short id. |
| **maker** | The party who posted the offer — shown as a small identicon and short public key. |
| **taker** | The party who took it — likewise an identicon and short key. |
| **gives → receives** | What you gave and what you got. |
| **state** | Where the swap is in its lifecycle. |
| **when** | The timestamp. |
| **final tx** | The settling transaction once it completes. |

Both parties are shown explicitly, side by side, rather than as a single "your
role" label: the **maker** is whoever posted the offer and the **taker** is
whoever took it, regardless of which one is you. Your own side is marked with a
**(you)** tag next to its key, so you can always tell which party you are at a
glance. A party Satchel hasn't recorded yet (occasionally the case for an older
record) shows as **unknown**.

Every swap carries a **protocol chip** so you can tell the two kinds apart at a
glance: a muted **Standard (HTLC)** chip for a classic swap, and a highlighted
**Private (Taproot)** chip for a private one. The same chip appears on each card
in the active-swaps dock.

### Reading the state

The **state** is colour-coded so you can scan the page quickly:

- **Finalizing** — your claim has been broadcast and you're just waiting for it to
  bury under enough confirmations to be safe. The coins are effectively yours, but
  the engine still needs to run, so the swap stays **active** (it keeps a card in
  the dock, counts toward your in-flight total, and the exit gate still warns you).
  Keep the app open until it finishes.
- **Completed** — green. The swap finished, your claim is **buried and safe**, and
  the engine is done — you can close the app freely. ("Completed" deliberately
  appears only once the claim has confirmed deep enough, not the instant it's
  broadcast.)
- **Refunded** — amber. The swap didn't complete, so your locked funds came back
  to you. No loss.
- **Aborted** — red. The swap was cancelled before any funds were locked.

Alongside the state, Satchel shows the engine's own narration — a plain-language,
verbatim description of what happened at each step. This is the same calm,
running commentary you see in the activity log.

### The live progress line

While a swap is in flight, a compact **progress line** sits just beneath the
narration — both here and on each card in the active-swaps dock. It turns the
qualitative story into a number you can watch tick up, and it shows only while
there's genuinely something to wait on (it disappears the moment a step is final,
so it never sits "stuck" once a leg has buried). Depending on whose move you're
waiting for, you'll see one of:

- **Awaiting their lock** / **Awaiting their claim** — you've done your part and
  are waiting on the counterparty. There's no fixed target to count toward, so the
  bar is indeterminate and a small **+N blocks** shows how many blocks have passed
  while you wait.
- **Locking your {coin} — unlock wallet if stalled** — *your own* funding of a leg
  is pending or retrying: it hasn't broadcast or confirmed yet. The bar is
  indeterminate, with a **+N blocks** liveness count beside it; a count that keeps
  growing flags a fund that's genuinely stuck, most often because the give-coin's
  node wallet is locked. This state is honest about *whose* action is outstanding —
  it's yours. (Previously a stuck taker in this position mis-displayed as though it
  were the maker's turn; now it names the wait correctly.)
- **Your lock confirming · 2/6** — *your own* funding is burying toward the depth
  your counterparty needs before they'll lock their side. You'll see this as the
  maker of either kind of swap: the wait is on your lock maturing, not on a slow
  counterparty, so Satchel shows it honestly as a count on your own chain rather
  than an indeterminate "awaiting their lock." (A Private swap's maker locks the
  moment its offer is taken, so this count starts during the brief signing
  handshake and runs until your lock is deep enough — only when the taker's lock
  is actually *seen* on the network does the line switch to **Their lock
  confirming**, never before.)
- **Their lock confirming · 3/6** — their funding is burying toward the depth you
  need before you act. The numbers are *confirmations so far / confirmations
  needed*. Each side picks its own confirmation depth in coin setup, and the two
  of you exchange those choices at the start of the swap purely for display — so
  the target in a "your lock" count is the depth *they* chose to wait for, and
  the target in a "their lock" count is *yours*. The exchange only makes the
  counters precise; what each side actually waits for is always its own setting.
- **Securing your BTC · 2/6** — your own claim — a redeem *or* a refund — is
  burying toward final. When it reaches the needed depth the swap flips to
  **Completed** (or **Refunded**).

Where it's relevant, the line also shows the current settlement **feerate** (for
example `· 2 sat/vB`), so any automatic fee-bumping is visible to you rather than
hidden. It's purely informational — the engine drives the swap either way.

> **Note** — If a funding transaction fails to broadcast — for instance because the
> node wallet got locked partway through a swap — the engine **re-attempts it
> automatically on every tick**. So if you see **"Locking your {coin}"** sitting
> stuck, just unlock the wallet (`walletpassphrase`) and the swap **self-heals** on
> the next pass, with no manual step from you. The retry is idempotent — it
> locates any funding already on chain first, so it never double-funds — and this
> locate-first guard now covers **both** Standard (HTLC) and Private (Taproot)
> swaps. A **Standard** swap also auto-retries its funding on every tick. A
> **Private (Taproot)** swap instead broadcasts *your* leg only once both sides
> have signed **and** the counterparty's leg is confirmed on-chain — so you never
> lock first-in-the-dark — and a Taproot swap that funds and then stalls
> auto-refunds at its timelock, so nothing strands unattended either way.

### On-chain detail

Each swap row can be expanded to reveal its **on-chain detail** — the audit trail.
This shows both funding transactions and *your* settlement or refund (never the
counterparty's settlement, and never the swap secret). Each transaction id has a
**copy** button, so you can paste it into a block explorer if you ever want to see
it on-chain yourself. The expanded row also carries a **Dump logs** button for
support — see *"Dump logs: diagnostics for support"* below.

> **Tip** — You never *need* to check a block explorer; the swap completes on its
> own. The on-chain detail is there for peace of mind and your own records.

If you have no swaps yet, the page simply says so and points you to the Corkboard
to take your first offer.

## Where the actions live: the active-swaps dock

The Swaps page shows you everything but doesn't have buttons. The handful of
actions a swap might ask of you live in the **active-swaps dock**, which is docked
beneath every page. Each live swap gets a card there with its state, the two
parties shown as **maker ↔ taker** (each an identicon and short key, your own
side tagged **(you)**; hover the arrow to see which side is which), the amounts,
the engine's narration, and a "refund *when*" note.

The dock keeps just one action button, plus diagnostics:

| Button | When it appears | What it does |
|---|---|---|
| **cancel** | While nothing of yours is locked yet (no funding on either leg) | Abandons the swap; you lose nothing, since nothing of yours is locked. |
| **dump logs** | Always | Copies a secret-free diagnostics bundle for support (see below). |

There is no **redeem**, **refund**, or **fund** button. Funding, redeeming, and
refunding are all automatic — the engine funds your side as the trade begins,
auto-redeems the instant it's safe, and auto-refunds anything past its timelock.
Every one of those steps is also **chain-gated** (by confirmations and timelocks),
so a manual button could never make them happen sooner or differently — only fail
or double-act. The one genuinely human decision is backing *out* before any funds
are committed, which is what **cancel** is for. It shows a confirmation dialog
first.

> **Note** — **cancel** is offered only while nothing of yours is locked yet, so
> it's always safe — the offer simply won't complete. It's gated on there being no
> funding on either leg (not on a particular state name), so it behaves correctly
> for both **Standard (HTLC)** and **Private (Taproot)** swaps, and it works even
> on an "initiating" pre-swap that's still waiting on the maker to answer. Once a
> leg is funded, your safety net is the **"refunds at *when*"** time shown on the
> card: the engine reclaims your funds automatically after that deadline.

> **Tip** — You don't strictly have to press **cancel** on a handshake that's
> going nowhere. A **Private (Taproot)** swap stuck waiting on the other side —
> before anything is funded — clears itself automatically after about 15
> minutes, on both sides independently, with nothing lost either way. Cancel is
> there for when you don't want to wait even that long.

![The active-swaps dock, with its cancel and dump-logs buttons.](images/processed/ch10-dock.png){width=85%}

### Swaps from another machine

If you run the same recovery phrase on more than one machine (see *"One seed on
more than one machine"* in the *Backup, Seeds & Safety* chapter), the dock also
shows the *other* machine's in-flight swaps, grouped per machine under a heading
like **Another machine · M-7f3a**. Those cards are **read-only**: this machine
watches them on-chain but never acts on them, and they don't appear in the Swaps
ledger. Each group carries one **Take over** button. Press it — and confirm that
the other machine really is stopped — and this machine adopts the group's swaps
and starts driving them, ledger and all. The one kind it can't adopt this way is
a **Private (v2)** swap paid out to a node wallet this machine doesn't control:
it's skipped, with a note to point this machine at that wallet (or add an
Electrum view for the coin) and take over again. All v1 swaps, and v2 swaps that
pay a seed-derived address, adopt without conditions.

> **Note** — Upgrading from an older Satchel needs no ceremony here: your
> **finished** swaps stay in the ledger automatically. A swap that was still
> **in flight** when you upgraded shows up once under **Another machine** —
> one **Take over** and it carries on as this machine's swap.

### Dump logs: diagnostics for support

Every swap has a **Dump logs** button — on its card in the active-swaps dock, and
on its expandable detail row on the **Swaps** page. Press it and Satchel copies a
**diagnostics bundle** for that one swap to your clipboard: the swap's own record
plus the engine log lines that mention it. Paste it into a bug report or a message
to a developer when something has gone wrong.

> **Tip** — The bundle is **secret-free and safe to share**. Secrets are scrubbed
> out before it's copied, so it never contains your seed, your recovery phrase, the
> swap preimage, or any nonces — just the information a developer needs to see what
> happened.

## Closing the app mid-swap: the exit gate

If you try to close Satchel while a swap is in flight, an **exit gate** dialog
stops you and explains the situation: the swap is governed by on-chain timelocks,
and the engine must keep running to redeem or refund before the deadline. The
gate guards every way out — choosing **Quit** from the tray icon's menu runs
through exactly the same dialog, so the tray can never sidestep it.

Your choices:

- **Keep running, close window** (recommended) — the window closes but the engine
  keeps working in the background until the swap finishes. Reopen Satchel any time
  to check on it.
- **Cancel** — go back; don't close after all.
- **Force-quit** — stops the engine entirely. This is deliberately hard: you must
  type the word `QUIT` to confirm, and the dialog warns you in plain terms that
  doing so can lose funds.

> **Warning** — Choose **Keep running** whenever a swap is live. **Force-quit**
> kills the engine mid-flight, and with nothing left to redeem or refund before the
> timelock, funds can be lost. Only force-quit when you have no swaps in flight.

If instead you only have *offers* posted (no live swap), the exit gate is gentler:
it lets you withdraw your offers and exit, or keep the engine running so
counterparties can still take them while Satchel is closed.

> **Note** — With nothing to protect — no live swap and no posted offers — there's
> no gate to show, so the window simply closes. If you're only browsing the board
> without any offers or swaps of your own, closing Satchel is always immediate.
