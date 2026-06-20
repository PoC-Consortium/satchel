# Troubleshooting

Things occasionally don't go to plan — a node is down, an address has a typo, a
swap seems stuck. This chapter walks through the situations you're most likely to
meet and how to put them right. Work top to bottom; each entry is independent, so
jump to the one that matches what you're seeing.

> **Tip** — The header at the top of the window is your dashboard. The
> engine-reachability dot, the Nostr relay dot, the per-coin health glyphs, and the
> live swap count all tell you at a glance what's healthy and what isn't. Hover any
> of them for a tooltip.

## "Engine not running" or disconnected

If Satchel shows a disconnected state — the engine indicator is red, and screens
can't load data — the app can't reach its swap engine. The usual causes:

1. **A node is down.** The engine talks to your cryptocurrency nodes (Bitcoin,
   BTCX, and so on). If one isn't running, start it and give it a moment to come
   up.
2. **The wrong RPC credentials.** If a node is running but the engine can't log in
   to it, the connection details are off. Open **Settings → Coins**, edit the coin,
   and re-check the host, port, and authentication (cookie file path, or
   username/password). See the chapter *"Setting Up Your Coins"*.
3. **The engine simply needs a restart.** Closing and reopening Satchel relaunches
   the engine cleanly.

> **Note** — Satchel needs the engine to be reachable before it can show balances,
> offers, or swaps. Fixing the engine connection usually fixes several symptoms at
> once.

## A coin shows a connection error

In **Settings → Coins**, each coin carries a status pill. **Connection error**
(rather than **Connected**) means Satchel reached the engine but the engine can't
talk to that particular node.

To fix it:

1. Confirm the node itself is running and has finished starting up.
2. Open the coin's **Edit connection**, check the host, port, and authentication,
   then press **Validate node**.
3. A green "Genesis matched" verdict means it's good — press **Save**. A rejection
   means the details point at the wrong node or network; correct them and validate
   again.

The chapter *"Setting Up Your Coins"* covers the validation flow and what each
field means in detail.

## "Fewer than 2 coins connected"

Satchel needs at least **two live coins** before you can trade — after all, a swap
is an exchange between two of them. If you see a "X of 2 coins connected" gate, or
the app won't let you past setup, one or both of your coins isn't fully connected.

- The progress line tells you how many are live. Pick a coin that isn't connected
  and complete its setup.
- A coin counts as *live* only when it's both configured **and** showing
  **Connected**. A coin with a **Connection error** doesn't count — fix it as
  above.

Once two coins are green, the gate clears and you can continue.

## No offers on the Corkboard

If the **Corkboard** screen is empty, it's usually one of these:

- **No board or relay configured.** If you see "No Corkboard connected," open
  **Settings → Network** and make sure you have at least one Nostr relay or one
  Corkboard. A fresh install ships with six Nostr relays, so this is rare unless
  you cleared the list.
- **You're browsing the wrong board.** The board selector at the top switches
  between your noticeboards. If your counterparty posted on a specific Corkboard,
  select that board.
- **The wrong pair, or genuinely no offers.** The Corkboard filters to the trading
  pair you've selected. Try another pair. A message like "{n} more… Settings →
  Coins" means there are offers for pairs you haven't connected the coins for yet.
- **It's just quiet.** Sometimes nobody's posted recently. Consider posting your
  own offer — see the chapter on posting offers.

## A relay shows amber or red

The Nostr relay dot in the header turns **amber** when none of your relays are
reachable, and the **Settings → Network** list can flag individual ones. Usually:

- **A typo in the address.** Relay addresses must start with `wss://`. Re-check the
  one you added.
- **That relay is temporarily down.** Public relays come and go. If you have
  several configured (the default has six), the others keep you running — one amber
  relay isn't a problem. If *all* are amber, check your internet connection.
- Press **Save & reconnect** after any change to retry the connections.

## A swap looks stuck

A swap that seems frozen is usually just *waiting* — for blockchain confirmations,
or for the other side to take their step. Swaps are not instant.

1. **Keep Satchel and your nodes running.** This is the single most important
   thing. The engine needs to be awake to broadcast the next step — including the
   refund — at the right time.
2. **Let it run its course.** Check the **Swaps** screen; each row shows a plain-
   language narration of where it is. If the other side has gone quiet, the engine
   is already watching the *timelock* on your behalf.
3. **Trust the refund.** If the swap can't complete, the timelock guarantees your
   locked funds come back to you automatically once its deadline passes. You don't
   need to do anything except keep the app running. See the chapter *"Backup, Seeds
   & Safety"*.

> **Warning** — Do **not** close Satchel to "reset" a stuck swap. The engine has to
> be running to finish or refund it. If you must shut down, choose **Keep running**
> on the quit dialog so the engine continues in the background.

## I can't take an offer

If the **Take offer** button is disabled, or a take fails, the offer isn't
available to you right now. Common reasons:

- **It's your own offer.** You can't take an offer you posted — you can only
  **Withdraw** it.
- **A leg is down.** If one of the two coins involved isn't connected, the take is
  blocked until you fix that coin (**Settings → Coins**).
- **It expired.** Offers have a "valid for" window. An expired offer can't be
  taken; refresh the board and look for a current one.
- **It was already taken or withdrawn.** Someone else may have taken it first, or
  the maker pulled it. The offer's state chip tells you which.

## Windows SmartScreen warning

The first time you run Satchel on Windows, **SmartScreen** may show a blue
"Windows protected your PC" box. This appears for new applications that aren't yet
widely recognised by Microsoft — it isn't a sign anything is wrong with the
download.

To proceed, click **More info**, then **Run anyway**.

> **Tip** — Only do this for a copy of Satchel you downloaded from the official
> project repository. If in doubt about where your download came from, stop and
> verify the source first — see the chapter *"Where to Get Help"* for the official
> links.

## Still stuck?

If none of the above fixes it, the chapter *"Where to Get Help"* lists the project
repository and community channels, and explains how to file a useful bug report —
including which logs to attach (and how to be sure they contain **no** seed or
passphrase).
