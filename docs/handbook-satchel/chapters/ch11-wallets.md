# Your Wallets

The **Wallets** page gives you a quick, at-a-glance balance for each coin you've
connected. Click **Wallets** in the left navigation to open it.

![The Wallets page: per-coin balances with Send and Receive.](images/processed/ch11-wallets.png){width=80%}

You'll see one card per configured coin, each with the coin's icon, name, symbol,
and a large balance figure. The balances come live from your own coin nodes.

Each card also tells you **where that coin's wallet lives**:

- A coin connected to **your own node** (RPC) uses the node's wallet. If you've
  named one (in **Settings → Coins**), the card shows **"wallet · {name}"** —
  every RPC for that coin uses exactly that wallet. If you haven't, a small
  warning, **"default wallet (not scoped)"**, says the coin falls back to the
  node's default wallet. On a node with more than one wallet loaded, naming it
  removes any doubt about which balance you're looking at.
- A coin connected via **Electrum servers** shows **"pact seed wallet"**: there
  is no node — the wallet lives on your Pact seed (a standard BIP-86 branch of
  the same recovery phrase), and the servers only supply chain data. They never
  see your keys.

## Send, Receive and Activity

Every card carries **Receive** (a fresh address each time — old ones keep
working, fresh is better for privacy) and **Send**. The send form takes the
recipient address and amount, then lets you pick the network fee — added on
top of the amount — as **Slow / Normal / Fast** presets priced from the live
fee market, or a **Custom** sat/vB rate. When the market has no estimates
(a quiet or brand-new chain), the presets grey out and the form falls back to
a custom rate at the coin's minimum. A **Review** step shows recipient,
amount, estimated fee and total before anything is broadcast — transactions
are irreversible, so check the address there. For a node-backed coin these
drive the node's own wallet; for an Electrum coin they drive your pact-seed
wallet directly.

Electrum coins add a third button, **Activity** — the wallet's transaction
history (direction, amount, fee, confirmations), including anything still
pending. A pending send you made carries a **Bump** action: every send is
broadcast replaceable (RBF), so if it's stuck you can re-price it to a higher
sat/vB rate and the replacement takes its place. Node-backed coins skip the
Activity dialog: your node's own software already keeps that history (and its
own fee-bump tooling), and the transactions that matter for trading — funding
and settlement — appear on the **Swaps** page with full on-chain detail either
way.

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
