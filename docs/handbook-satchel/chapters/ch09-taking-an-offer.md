# Taking an Offer & How a Swap Runs

This chapter follows what happens from the moment you take an offer to the moment
the swap completes. The good news is short: once you confirm, the engine does the
work. You don't have to babysit it.

## Taking an offer from the Corkboard

On the Corkboard, open the price level you want, find the offer, and click
**Take offer**. Satchel shows a **confirmation dialog** before committing
anything. It lays out:

- **You give** and **You receive** — the exact amounts, from your side.
- **Counterparty** — who you'd be trading with.
- **The protocol** — **Standard (HTLC)** or **Private (Taproot)**.
- **Safety refund** — the auto-refund deadlines (`t2h / t1h`).
- **A network-cost preview** — the on-chain fees you'll pay.

Crucially, it also reassures you about order of operations: *the maker locks their
coins first — you never send first.* You can still cancel before you fund your own
side, and if anything stalls the engine auto-refunds you after the safety
timelock.

Just like posting an offer, the dialog runs a **funds check** first: it confirms
your wallet can cover the **amount plus the funding fee** for your side, and if it
can't, it shows a "Not enough … (amount + funding fee)" alert and blocks the take
until you can.

It also runs the same **wallet-lock check** posting does, on the coin you'd be
**getting** (the side you fund): if that node wallet is encrypted and locked, the
take is refused up front, asking you to unlock it first with `walletpassphrase`.

> **Warning** — A locked wallet can read its balance but can't sign your funding
> transaction, so taking with it locked would strand the swap at funding. Unlock
> the get-coin's wallet before you take, and keep it unlocked until the swap
> completes.

![The take-offer confirmation dialog.](images/processed/ch09-take-confirm.png){width=65%}

If the offer was posted by someone you've marked **Blocked** in your contacts, the
dialog also shows a warning — **"You blocked this counterparty"** — and asks you to
confirm a second time. This does **not** hard-block the trade: blocking is only a
personal reminder, and an atomic swap protects you regardless of who you trade
with. See the chapter *"Contacts"*.

Read it over and confirm. The swap is now under way.

## How the swap runs, in plain language

A swap is an *atomic* trade: either both sides happen, or neither does. There's no
moment where one person has both coins. Here's the sequence, without the
cryptography:

1. **The maker funds first.** The party who posted the offer locks their coins on
   their chain before you lock anything. You are never the one exposed first.
2. **Your side is funded automatically.** When the swap reaches the point where it
   needs *your* coins, Satchel funds your side for you — you don't have to click
   anything. This is how every swap works — there is nothing to configure.
3. **The engine watches both chains.** Satchel's engine monitors the blockchains
   for both coins, waiting for each funding to confirm to a safe depth.
4. **The funds are released atomically.** Once both sides are locked, the engine
   completes the exchange so that each of you can only claim your coin in a way
   that simultaneously lets the other claim theirs. Neither side can run off with
   both.
5. **If anything stalls, you're auto-refunded.** If a counterparty disappears or a
   chain gets stuck, the engine waits for the safety timelock and then
   automatically pulls your locked funds back to you. You don't have to do
   anything to get a refund — it's built in.

> **Note** — The whole flow is automatic: after you take an offer you won't touch
> a button at all — funding, redeeming, and (if needed) refunding all happen on
> their own. The **fund** and **refund** actions still appear in the active-swaps
> dock if you ever want to act manually — see the chapter on tracking your swaps.

## You don't have to babysit it — but keep the app running

You don't need to sit and watch a swap. But the **engine must keep running** until
the swap finishes, because it's the engine that redeems or refunds at the right
moment. Those moments are governed by on-chain timelocks with real deadlines.

If you try to close Satchel while a swap is live, it offers to **Keep running** —
this leaves the engine working quietly in the background (even with the window
closed) so the swap finishes on its own. This is the recommended choice. You can
reopen Satchel any time to check progress.

> **Warning** — Force-quitting in the middle of a swap stops the engine and can
> lose funds, because nothing is left running to redeem or refund before the
> deadline. Always choose **Keep running** when prompted. The exit dialog is
> covered in the chapter on tracking your swaps.

## Where to go next

- To watch a swap progress and act on it if needed, see the chapter **Tracking
  Your Swaps**.
- For the reassurance of *why* this is safe — what atomic means, what the
  timelocks guarantee — see the safety chapter.

If you're curious about the cryptographic machinery (HTLCs, adaptor signatures,
the actual scripts), that's all in the companion **Pact** handbook. You don't need
any of it to trade safely.
