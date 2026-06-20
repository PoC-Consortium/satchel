# Installing Satchel

Installing Satchel is a two-part job. The app itself is a single download. But
because Satchel trades using **your own coin nodes**, you also need those nodes
running before you can trade. This chapter walks through both, gently.

## What you need first: your own coin nodes

Satchel is a *node-backed* app. A *node* is your own personal copy of a
cryptocurrency's network — the software that holds a wallet, knows the current
state of the blockchain, and can send and receive coins. When you swap, Satchel
uses the wallets on your nodes to fund your side of the trade and to receive the
proceeds.

To trade the first pair, you need **two** nodes running on your machine (or on a
machine Satchel can reach):

- **Bitcoin-PoCX** (BTCX) — a `bitcoin-pocx` node.
- **Bitcoin** (BTC) — a standard Bitcoin Core node, or anything compatible with
  it.

Each node must have its **RPC interface reachable**. *RPC* (Remote Procedure
Call) is simply the doorway through which other programs — like Satchel's engine
— talk to your node. In practice this means the node is running and configured to
accept local connections. You will point Satchel at each node's address and port
later, in the chapter *"Setting Up Your Coins"*; for now, the goal is just to have
both nodes installed and running.

> **Note** — Don't worry about the exact RPC settings yet. The chapter *"Setting
> Up Your Coins"* covers the connection form field by field, including how
> Satchel reads your node's login automatically from its *cookie file* so you
> usually don't have to type a password.

> **Tip** — You need at least **two** live coins before Satchel will let you
> trade, because a swap involves two chains. If you only have one node up, that's
> fine for installing — you'll just finish connecting the second before your
> first trade.

## Getting the app

Satchel is distributed as a ready-to-run *bundle* — a single download containing
everything the app needs.

1. Go to the project's **releases** page.
2. Download the bundle for your operating system:
   - **Windows** — the Windows bundle.
   - **Linux** — the Linux bundle.
3. Unpack it if needed, and run the **Satchel** application.

That's the whole installation. There is nothing to register and no account to
create.

> **Note** — **macOS is not supported yet.** A macOS build is planned but out of
> scope for this alpha. For now, use Windows or Linux.

### What's inside the bundle

The bundle contains two things working together:

- **Satchel** — the app you interact with.
- **The bundled engine** — Satchel ships *Pact*, the swap engine, alongside
  itself and starts it for you automatically. You don't install or run it
  separately; Satchel launches it, supervises it, and shuts it down when you're
  done.

So a single download gives you both the face and the engine. The only thing you
provide is your coin nodes.

## Windows: the SmartScreen prompt

Satchel's alpha builds are **not yet code-signed** — signing is a paperwork step
the project will complete closer to a stable release. Because of this, Windows
**SmartScreen** may show a blue warning the first time you run the app, saying
something like *"Windows protected your PC"*.

This is expected for unsigned software and does not mean anything is wrong. To
run it:

1. Click **More info** in the SmartScreen dialog.
2. Click **Run anyway**.

Satchel will then start normally.

> **Warning** — Only do this for a bundle you downloaded from the **official**
> releases page. Treat any copy of "Satchel" from somewhere else with suspicion —
> a tampered build could put your funds at risk. When in doubt, download fresh
> from the official source.

## First-time prerequisites

On a current, up-to-date copy of **Windows**, you generally need nothing extra:
Satchel uses a component called **WebView2** to draw its window, and WebView2
already ships with modern Windows. If for some reason it is missing, Windows will
offer to install it.

On **Linux**, the bundle is self-contained; follow any notes on the releases page
for your distribution.

Beyond that, the only real prerequisite is the one from the top of this chapter:
**your two coin nodes, up and reachable.**

## For developers: building from source

If you'd rather build Satchel yourself from the source code, that path is for
developers and is documented elsewhere:

- The project **`README`** covers the prerequisites (Rust, Node, the Tauri CLI)
  and the build commands.
- The companion *Pact Developer & Integrator Handbook* covers the engine in
  depth.

Most people don't need this — the prebuilt bundle above is all it takes to start
trading. With Satchel installed, the next chapter walks you through your very
first launch.
