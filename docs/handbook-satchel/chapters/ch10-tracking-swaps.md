# Tracking Your Swaps

Once a swap is under way, you can follow it from start to finish. There are two
places to look: the **Swaps** page, which is your read-only ledger, and the
**active-swaps dock**, which appears beneath every page and is where the
occasional action buttons live.

## The Swaps page

Click **Swaps** in the left navigation. This page is a **read-only ledger** — it
shows you everything but has no action buttons (those live in the active-swaps dock,
covered below). It's split into two sections, newest first:

- **In flight** — swaps currently running.
- **History** — swaps that have finished, refunded, or been aborted.

![The Swaps page: in-flight swaps above, finished trades below.](images/processed/ch10-swaps.png){width=90%}

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

Private swaps carry a **Private (Taproot)** chip so you can tell them apart at a
glance.

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
- **Their lock confirming · 3/6** — their funding is burying toward the depth you
  need before you act. The numbers are *confirmations so far / confirmations
  needed*.
- **Securing your BTC · 2/6** — your own claim is burying toward final. When this
  reaches the needed depth the swap flips to **Completed**.

Where it's relevant, the line also shows the current settlement **feerate** (for
example `· 2 sat/vB`), so any automatic fee-bumping is visible to you rather than
hidden. It's purely informational — the engine drives the swap either way.

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

The action buttons appear **only when it's your turn**, gated by the swap's state:

| Button | When it appears | What it does |
|---|---|---|
| **redeem** | When the other side has funded and you can claim | Claims your coins. |
| **cancel** | Before you've funded anything | Abandons the swap; you lose nothing, since nothing of yours is locked. |
| **refund** | After the safety timelock has passed on a leg you funded | Pulls your locked funds back now. |

Each button shows a confirmation dialog first. In normal operation the engine
handles **funding, redeeming, and refunding automatically** — you'll rarely press
anything. **Funding is always automatic**, so there is no Fund button: your side
of a swap is locked for you as soon as the trade begins. The buttons that remain
are there for the cases where you want to act manually — to cancel early, nudge a
claim, or refund the instant the timelock allows rather than waiting for the
engine's automatic pass.

> **Note** — **cancel** is only offered while nothing of yours is locked, so it's
> always safe — the offer simply won't complete. **refund** only appears once the
> timelock has passed, and the engine also fires it automatically after the
> deadline, so you're covered either way.

![The active-swaps dock, with state-gated action buttons.](images/processed/ch10-dock.png){width=85%}

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
and the engine must keep running to redeem or refund before the deadline.

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

> **Note** — In **watch-only mode** the exit gate never appears: a watch-only
> session holds no offer liveness and can hold no live swap, so there is nothing
> to protect and the window simply closes.
