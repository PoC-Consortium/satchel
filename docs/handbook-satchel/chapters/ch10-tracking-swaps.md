# Tracking Your Swaps

Once a swap is under way, you can follow it from start to finish. There are two
places to look: the **Swaps** page, which is your read-only ledger, and the
**active-swaps dock** under the Corkboard, which is where the occasional action
buttons live.

## The Swaps page

Click **Swaps** in the left navigation. This page is a **read-only ledger** — it
shows you everything but has no action buttons (those live in the Corkboard dock,
covered below). It's split into two sections, newest first:

- **In flight** — swaps currently running.
- **History** — swaps that have finished, refunded, or been aborted.

![The Swaps page: in-flight swaps above, finished trades below.](images/processed/ch10-swaps.png){width=90%}

### The columns

Each row shows:

| Column | What it tells you |
|---|---|
| **swap** | The swap's short id. |
| **role** | Whether you were the maker or the taker. |
| **state** | Where the swap is in its lifecycle. |
| **gives → receives** | What you gave and what you got. |
| **when** | The timestamp. |
| **final tx** | The settling transaction once it completes. |

Private swaps carry a **Private (Taproot)** chip so you can tell them apart at a
glance.

### Reading the state

The **state** is colour-coded so you can scan the page quickly:

- **Completed** — green. The swap finished and you received your coins.
- **Refunded** — amber. The swap didn't complete, so your locked funds came back
  to you. No loss.
- **Aborted** — red. The swap was cancelled before any funds were locked.

Alongside the state, Satchel shows the engine's own narration — a plain-language,
verbatim description of what happened at each step. This is the same calm,
running commentary you see in the activity log.

### On-chain detail

Each swap row can be expanded to reveal its **on-chain detail** — the audit trail.
This shows both funding transactions and *your* settlement or refund (never the
counterparty's settlement, and never the swap secret). Each transaction id has a
**copy** button, so you can paste it into a block explorer if you ever want to see
it on-chain yourself.

> **Tip** — You never *need* to check a block explorer; the swap completes on its
> own. The on-chain detail is there for peace of mind and your own records.

If you have no swaps yet, the page simply says so and points you to the Corkboard
to take your first offer.

## Where the actions live: the active-swaps dock

The Swaps page shows you everything but doesn't have buttons. The handful of
actions a swap might ask of you live in the **active-swaps dock**, which is docked
under the **Corkboard**. Each live swap gets a card there with its state, amounts,
your role, the engine's narration, and a "refund *when*" note.

The action buttons appear **only when it's your turn**, gated by the swap's state:

| Button | When it appears | What it does |
|---|---|---|
| **fund** | When it's your turn to lock your side | Funds your leg of the swap. |
| **redeem** | When the other side has funded and you can claim | Claims your coins. |
| **cancel** | Before you've funded anything | Abandons the swap; you lose nothing, since nothing of yours is locked. |
| **refund** | After the safety timelock has passed on a leg you funded | Pulls your locked funds back now. |

Each button shows a confirmation dialog first. In normal operation the engine
handles funding, redeeming, and refunding **automatically** — you'll rarely press
anything. The buttons are there for the cases where you want to act manually, for
example to cancel early or to refund the instant the timelock allows rather than
waiting for the engine's automatic pass.

> **Note** — **cancel** is only offered while nothing of yours is locked, so it's
> always safe — the offer simply won't complete. **refund** only appears once the
> timelock has passed, and the engine also fires it automatically after the
> deadline, so you're covered either way.

![The active-swaps dock under the Corkboard, with state-gated action buttons.](images/processed/ch10-dock.png){width=85%}

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
