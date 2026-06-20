# Setting Up Your Coins

A swap always involves **two** blockchains — you give one coin and receive
another. So before Satchel will let you trade, it needs **at least two coins
connected and live**. This chapter walks you through connecting them.

If this is your first launch, Satchel brings you here automatically with a
**"Connect your coins"** wizard. You can also reach the same setup any time from
**Settings → Coins**.

> **Note** — *Live* means a coin is connected **and** its node is up and
> answering. The wizard shows a running count — "1 of 2 coins connected" — and the
> **Continue** button stays disabled until two coins are live. That's the safety
> gate: no trading until you genuinely have two working chains.

## The coins screen

Whether from the first-run wizard or **Settings → Coins**, you'll see a card for
each coin Satchel knows about. The first pair is **BTCX** (Bitcoin-PoCX) and
**BTC** (Bitcoin). Each card shows:

- The coin's **icon, name, and symbol**.
- A **status pill** — *Not set up*, *Connected* (with the node's current block
  height, called the *tip*), or *Connection error*.
- **Capability chips** — small tags showing what the coin can do (more on these
  below).
- A **Set up** button (or **Edit connection** if it's already configured), and a
  way to remove it.

To connect a coin, click **Set up** on its card.

![The Coins screen, with one coin connected and one still to set up.](images/processed/ch05-coins-screen.png){width=85%}

## The connection form

Setting up a coin means telling Satchel **where your node is** and **how to log
in to it**. This is the form that does it. It's filled in with sensible defaults,
so often you'll only change a field or two.

The fields, in order:

- **RPC host** — the address of the machine running the node. The default is
  `127.0.0.1`, which means "this same computer". If your node runs on another
  machine, put its address here.

- **RPC port** — the port number the node listens on for RPC. Each coin and
  network has its own usual port; the form is pre-filled with the expected one,
  but check it matches your node's configuration.

- **Authentication** — how Satchel proves to your node that it's allowed to
  connect. Pick one of two cards:

  - **Cookie file** *(the default)* — your node writes a small `.cookie` file
    containing a one-time login, and Satchel reads it automatically. Nothing to
    type, nothing to store. When you choose this, a **Node data directory** field
    appears — point it at your node's data folder, and Satchel finds the cookie
    inside (it shows you the exact path it will read).

  - **User / password** — if your node is set up with a fixed `rpcuser` and
    `rpcpassword` (common for a node on another machine), choose this and enter
    the **RPC username** and **RPC password** from your node's config.

- **Wallet name** *(optional)* — if your node has more than one wallet loaded,
  name the one Satchel should use for this coin. Leave it blank to use the
  default.

- **Confirmations before final** *(optional)* — how many blocks deep a payment
  on this chain must be before Satchel treats it as settled. Leave it blank to
  use the recommended default for the coin. Higher numbers are a little safer
  against rare blockchain reshuffles, but make swaps slower.

![The coin connection form, set to read login from the node's cookie file.](images/processed/ch05-coin-setup.png){width=80%}

> **Tip** — On the same machine, the **Cookie file** option is the easiest and
> most secure choice: there's no password to type or store, and the login rotates
> automatically. Reach for **User / password** mainly when connecting to a node
> elsewhere.

## Validate before you save

Here is the part that keeps you safe from a costly mistake: **Satchel checks it's
talking to the right blockchain before it saves anything.**

1. With the form filled in, click **Validate node**.
2. Satchel connects to the node and reads its *genesis block* — the unique first
   block that identifies a chain. It compares that to the genesis it expects for
   this coin and network.
3. You'll see one of:
   - **Checking the node…** while it works.
   - **Genesis matched — this is the right chain**, along with the node's current
     **tip height** and the genesis hash. Success.
   - **Rejected — not saving**, if the genesis doesn't match — meaning the node is
     on the wrong network (or it's the wrong coin entirely).
4. The **Save** button only becomes available **after** validation passes. If you
   change any field afterward, you'll need to validate again.

![Validation succeeded: the node is on the correct chain, showing its tip height.](images/processed/ch05-validate.png){width=80%}

> **Warning** — This genesis check exists so your funds can **never** be sent into
> the wrong chain by a mis-typed port or a node running on the wrong network.
> Nothing is saved until the check passes — so if validation is rejected, **don't
> try to force it.** Fix the connection details (usually the port or the data
> directory) and validate again.

Click **Save**, and Satchel records the connection and reconnects the engine to
that node. Repeat for your second coin.

## Capabilities and trading pairs

Two more things appear on the Coins screen once a coin is connected.

**Capability chips** — small tags like **CLTV**, **SegWit**, and **Taproot** —
describe technical features the coin supports. You don't need to understand them
to trade; they simply tell Satchel which kinds of swap it can build with that
coin. (If you're curious: *CLTV* enables the time-based safety refund, *SegWit*
and *Taproot* are transaction formats — Taproot powers the newer, more private
swap style.)

**Trading pairs** — below the coin cards, Satchel lists the pairs you can trade,
derived automatically from what your connected coins can do. There's no fixed
list. Each pair shows a readiness label:

- **Ready to trade** — both coins are connected and live. You're good to go.
- **Connect *(coin)*** — you still need to set up one of the two coins.
- **Not buildable yet** — the coins can't form a swap together (for example, one
  lacks a needed capability).

When **BTCX ↔ BTC** reads **Ready to trade**, you're set.

![The Trading pairs list showing BTCX and BTC ready to trade.](images/processed/ch05-pairs.png){width=80%}

## Your credentials stay local

A reasonable worry: you've just typed in node details, maybe a password. Where
does that go?

> **Note** — Your node credentials are **read and used locally, by the engine on
> your own machine — never shown back inside the app's window and never sent
> anywhere.** When you use a cookie file, Satchel reads it directly on your
> computer; the secret never even reaches the part of the app that draws the
> screen. Connections are shared across all your merchants, so you set each coin
> up once.

With two coins live and your pair reading **Ready to trade**, the first-run
wizard's **Continue** button lights up. Click it, and you arrive at the
**Corkboard**, ready for your first trade — which is exactly where Part 2 begins.
