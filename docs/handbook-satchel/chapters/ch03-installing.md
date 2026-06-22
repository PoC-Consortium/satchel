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

On **Windows**, the installer also tucks the engine's command-line tools —
`pact-cli` and `pactd` — onto your **user PATH**, so you can run them from any
terminal. You won't need them for ordinary trading, but if you do want them,
**open a new terminal** afterwards: an already-open window won't see the change
until it's reopened.

> **Note** — **macOS is not supported yet.** A macOS build is planned. For now,
> use Windows or Linux.

> **Warning** — If you installed an **earlier release-candidate build** of
> Satchel, a now-fixed installer bug could have truncated your Windows user PATH,
> dropping unrelated entries past a certain length (so a tool like `cargo` might
> suddenly seem to have vanished from the command line). Current builds fix this.
> If something went missing, just **re-add it once** — for example, add
> `%USERPROFILE%\.cargo\bin` back to your PATH — and it'll stick.

### What's inside the bundle

The bundle contains two things working together:

- **Satchel** — the app you interact with.
- **The bundled engine** — Satchel ships *Pact*, the swap engine, alongside
  itself and starts it for you automatically. You don't install or run it
  separately; Satchel launches it, supervises it, and shuts it down when you're
  done.

So a single download gives you both the face and the engine. The only thing you
provide is your coin nodes.

> **Note** — On **Windows**, Satchel keeps its merchants, your recovery seed, and
> your settings under **`%LOCALAPPDATA%`** (your machine-local app-data folder),
> not the roaming `%APPDATA%`. This is deliberate: a spending seed should stay
> tied to this one machine and never roam to others. You don't have to do anything
> with this — it's just where Satchel stores things.

## Windows: the SmartScreen prompt

Satchel's builds are **not yet code-signed** — signing is a paperwork step the
project is still completing. Because of this, Windows **SmartScreen** may show a
blue warning the first time you run the app, saying something like *"Windows
protected your PC"*.

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
