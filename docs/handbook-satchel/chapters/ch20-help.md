# Where to Get Help

Stuck on something this handbook didn't cover, or found a bug? Here's where to
turn — and one safety reminder that's worth repeating before anything else.

> **Warning** — **No one from this project will ever ask for your recovery
> phrase**, your passphrase, or your private keys — not in a chat, not in an
> "support" message, not anywhere. Anyone who asks is trying to steal from you.
> Genuine help never needs your seed. See *"Backup, Seeds & Safety"*.

## The project repository

The home of Satchel is its public code repository:

> **github.com/PoC-Consortium/satchel**

There you'll find the latest releases and download links, release notes, the issue
tracker, and the source code itself. When you want the official build, this is the
place to get it — and the only place to trust a download from.

> **Tip** — Satchel can also tell you when a new version exists: check
> **Settings → About**, which shows your version and an update notice when one's
> available.

## Filing an issue

If something's broken or behaving oddly, a good bug report helps it get fixed
fast. Open an issue on the repository's issue tracker, and include:

1. **Your Satchel version** — from **Settings → About** (for example, 0.1).
2. **Which network** you were on (mainnet, testnet, or regtest) and which **coins
   and pair** were involved.
3. **What you did, what you expected, and what actually happened** — step by step.
4. **Relevant logs**, if you have them — they often pin down the cause quickly.

> **Warning** — Before you paste any log, configuration, or screenshot into a bug
> report, **double-check it contains no recovery phrase, passphrase, or private
> key.** Satchel keeps secrets out of its logs by design, but always glance over
> anything you share. When in doubt, leave it out.

> **Note** — A first quick stop before filing: the chapter *"Troubleshooting"*
> covers the most common snags — a node that won't connect, an empty Corkboard, a
> swap that looks stuck — and often saves you the trip.

## Community channels

Beyond the repository, the project's community is the place to ask questions, share
what you're building, and get a hand from other users. The repository's README and
release notes link to the current community channels — check there for the
up-to-date list, since the venues can change as the project grows.

When you ask for help, the same etiquette applies as for bug reports: describe what
you're seeing clearly, mention your version and network — and never share your
seed.

## For developers: the Pact handbook

This book is the *user's* guide. If you're a developer or integrator who wants the
machinery underneath — how swaps are constructed transaction by transaction, the
exact protocol messages, the engine's RPC interface, key derivation, and the
cryptography — that all lives in the companion book:

> **Pact Developer & Integrator Handbook**

It's dense and precise, written for people comfortable with Bitcoin and code. If
you've ever wondered *exactly* how the all-or-nothing guarantee is enforced, or you
want to build on top of the engine, that's your reference.

## A final word

Satchel is live, and the project is small and moving fast. Your reports genuinely
help shape what comes next. Thanks for trading carefully, keeping your recovery
phrase safe, and helping the project improve.
