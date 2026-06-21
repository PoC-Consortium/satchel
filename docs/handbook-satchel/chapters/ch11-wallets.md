# Your Wallets

The **Wallets** page gives you a quick, at-a-glance balance for each coin you've
connected. Click **Wallets** in the left navigation to open it.

![The Wallets page: read-only per-coin balances.](images/processed/ch11-wallets.png){width=80%}

You'll see one card per configured coin, each with the coin's icon, name, symbol,
and a large balance figure. The balances come live from your own coin nodes.

Each card also tells you **which node wallet that coin is scoped to**. If you've
named a wallet for the coin (in **Settings → Coins**), the card shows
**"wallet · {name}"** — every RPC for that coin uses exactly that wallet. If you
*haven't*, the card shows a small warning, **"default wallet (not scoped)"**,
meaning the coin falls back to the node's default wallet; the tooltip points you to
**Settings → Coins** to set one explicitly. On a node with more than one wallet
loaded, naming the wallet removes any doubt about which balance you're looking at.

## Why it's read-only

There's no **Send** or **Receive** button here, and that's on purpose. These
aren't wallets that Satchel owns — **they are the wallets of your own nodes**, the
same nodes the engine uses to fund swaps and receive proceeds. Your keys, your
machine; Satchel never holds your coins.

Because the coins live in your node's own wallet, sending and receiving already
belongs to your node's own software — Satchel duplicating it would just be a
second, confusing front end for the same wallet. And the transactions that *do*
matter for trading — your funding and settlement — already appear on the **Swaps**
page with full on-chain detail. So the Wallets page sticks to what Satchel is best
placed to show you: a clean balance readout.

> **Note** — A full send/receive wallet *is* planned, but it arrives with the
> future **nodeless** build of Satchel — a version that carries its own wallet
> instead of leaning on a separate node. Until then, use your node software to move
> coins.

## The hot-seed warning

At the top of the page is a warning banner reminding you that a merchant's seed is
a **hot** spending seed, not a vault. It holds transit keys for in-flight swaps,
which means it has to be available and unencrypted enough for the engine to act
quickly.

> **Warning** — Don't park large balances in a merchant wallet. Sweep sizable
> proceeds to your own cold or main wallet after a trade. The merchant seed is hot
> by design — treat it as a working float, not savings.

## Loading and error states

The balance on each card updates independently, so one coin having trouble never
blocks the others:

- **`…`** — the balance is still loading.
- **`—`** — Satchel couldn't read this coin's balance. Hover the dash for a tooltip
  explaining the error; usually it means that coin's node is unreachable, so start
  it (or check **Settings → Coins**).

If you haven't connected a merchant yet, or haven't set up any coins, the page
shows a short prompt with a link to **Coins** in Settings rather than empty cards.
