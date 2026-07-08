# Backup, Seeds & Safety

This is the most important chapter in the book. Trustless trading puts you in
complete control of your money — and that control comes with responsibility. No
company holds your coins, so no company can lose them, freeze them, or hand them
to a thief. The flip side is that **no one can recover them for you, either.**
Almost all of your safety comes down to one small list of words and a few simple
habits. Please read this chapter slowly.

## Your recovery phrase

When you create a trading identity, Satchel shows you a *recovery phrase* — a list
of 12 or 24 ordinary words, in a specific order. (You may also hear it called a
*seed* or *seed phrase*; they're the same thing.) Those words *are* your wallet.
Everything Satchel does on your behalf — your trading keys, your addresses, your
ability to refund a swap — is mathematically derived from them. Lose the words and
you lose access; let someone else have them and they have your money.

There are exactly two rules. They are non-negotiable.

> **Warning** — **Write your recovery phrase down on paper the moment Satchel
> shows it to you**, and store it somewhere safe and private — ideally more than
> one place. This is the *only* way to recover your funds if your computer is
> lost, stolen, or breaks. Do it before you click past the screen.

> **Warning** — **Never type or paste your recovery phrase into any website, chat,
> email, or "support" form. Ever.** Anyone who gets those words can take
> everything, instantly and irreversibly. No genuine support person, and no one
> from this project, will *ever* ask you for it. Satchel itself only ever asks for
> it on its own first-run screen, never on a web page.

A few practical do's and don'ts:

- **Do** write it by hand on paper (or stamp it into metal, if you want it
  fireproof). Pen and paper beats a screenshot.
- **Don't** take a photo of it, save it in a notes app, email it to yourself, or
  store it in a password manager that syncs to the cloud. Each of those is a copy
  a thief could reach.
- **Do** double-check you copied every word correctly and in order before you
  continue. Satchel asks you to confirm a few of the words for exactly this
  reason.

## Encrypting with a passphrase

When you set up your identity, Satchel offers to protect your seed with a
*passphrase* — a password of your choosing. This is **optional but strongly
recommended**, especially on a computer others might use.

Here's what the passphrase does and doesn't do:

- **It protects the copy of your seed on this computer.** With encryption on, your
  seed is stored scrambled, and Satchel asks for your passphrase once each session
  to unlock it. Someone who steals your laptop can't trade or drain your funds
  without it.
- **It is not your recovery phrase, and it is not a backup.** The passphrase only
  guards the local copy. Your recovery phrase is still what restores your funds
  anywhere.
- **Even without a passphrase, your seed isn't left as plain text.** On Windows
  and macOS, Satchel locks the on-disk copy with a key held in your operating
  system's secure store, so the file is useless if copied off your machine. That
  key stays on this computer, so it doesn't stop someone already logged in as you
  — a passphrase is what adds that extra lock. (On Linux the no-passphrase seed is
  only lightly scrambled; treat it as unencrypted and use a passphrase if the
  machine isn't solely yours.)

> **Warning** — If you forget your passphrase, **Satchel cannot reset or recover
> it** — there's no "forgot password" link, by design. You would need to restore
> from your recovery phrase to set a new one. So: choose a passphrase you'll
> remember, and keep your written recovery phrase safe as your ultimate fallback.

## What the engine holds, and what never leaves your machine

Behind Satchel runs *the engine* (its technical name is `pactd`). The engine is
the trustworthy worker that does the real swap work on your computer:

- It holds your **keys**, unlocked in memory for the session, and signs
  transactions with them.
- It **builds and watches** your swaps from start to finish, broadcasting the
  right transaction at the right moment.
- It **auto-refunds** you if a swap doesn't complete (more on that below).

Crucially, all of this happens **locally, on your machine**. What is *never* sent
to any server, noticeboard, or other person:

- your recovery phrase and private keys;
- your passphrase;
- the contents of your wallet.

The only things that ever go out are your **public, signed offers** (which are
meant to be seen) and **encrypted messages** to your counterparty mid-swap (which
the noticeboard can't read). See the chapter *"Trading over Nostr & Corkboard"*
for exactly what a board can and can't see.

## Keep Satchel running until a swap completes

This rule prevents the most avoidable kind of loss.

> **Warning** — **While a swap is in progress, keep Satchel (and your nodes)
> running.** A swap is a sequence of timed on-chain steps, and the engine has to
> be awake to broadcast each transaction — including the refund — at the right
> moment. If you close everything mid-swap, those steps can't fire on time.

A swap isn't instant: each side locks funds on a blockchain, waits for
confirmations, and then claims. The engine babysits all of this for you, but it
can only do so while it's running. This is why, when you try to quit during a live
swap, Satchel stops you with a clear warning and offers to **Keep running** in the
background instead. Take that option. (Quitting from the tray icon's menu goes
through the same warning — no exit path skips it.) While the engine works in the
background, the tray icon keeps watch and swap milestones arrive as desktop
notifications, so backgrounding a swap never means losing sight of it.

> **Tip** — If you genuinely must shut the machine down, the **Keep running**
> choice on quit lets the engine continue in the background. And even in the worst
> case, your seed and the swap's timelocks protect your funds — see *"If your
> machine dies mid-swap"* below.

## A note on hot seeds

When you open the **Wallets** screen, a banner reminds you that these balances
belong to your own nodes' wallets, and nudges you about *hot seeds*. A "hot" seed
is simply one that lives on an internet-connected, running machine — which is
exactly what a live trader needs. That's normal and fine, but it carries the usual
caution:

> **Warning** — Don't keep more on a hot, always-on trading machine than you're
> comfortable having exposed. Treat your trading balance like the cash in your
> pocket, not your life savings. Keep long-term holdings in cold storage (an
> offline wallet), and move funds to your trading machine as you need them.

## If your machine dies mid-swap

Hardware fails, power cuts happen, laptops get left on trains. Three things
protect you, and it's worth knowing what each one covers:

1. **Your recovery phrase protects your identity and keys.** Every key is derived
   from your seed, so restoring from your recovery phrase on a new machine brings
   your trading identity back and lets you trade again.
2. **A quiet backup of your in-flight swaps rides along on the relays.** While a
   swap is under way, Satchel's engine also backs up just enough of its state to
   the Nostr relays you're already connected to — **encrypted so only you can
   read it** — so a swap survives even if the machine that ran it never comes
   back. Nobody but you can decrypt these backups; not the relay, not anyone
   else. This is what lets a *brand-new* machine, holding only your recovery
   phrase, pick a swap back up.
3. **The timelock guarantees a refund — as long as some machine of yours is
   watching.** Every swap has a built-in deadline called a *timelock*. Funds you
   locked become refundable to you after that deadline, and **nobody can take
   them before then.** The refund still has to be broadcast by a running engine
   that knows about the swap, which is exactly what the relay backup restores.

Putting those together, if a machine dies mid-swap:

- **Install Satchel on a new (or the same, repaired) machine and restore your
  recovery phrase.** Your identity and keys come back immediately.
- **The engine notices if it finds rescuable swaps.** On unlocking your restored
  identity, the engine checks the relays for any in-flight swap it doesn't
  already know about, and — if it finds one — logs a warning rather than
  silently resuming it. This is deliberate: if the old machine is still alive
  and quietly working the same swap, having two machines drive it at once could
  cost you money. **Only proceed once you're sure the old machine is genuinely
  retired** (wiped, destroyed, or definitely never coming back online).
- **Completing the restore is currently a technical, one-time step**, run
  through the engine's command-line tool (`pact-cli restore`) rather than a
  Satchel button — this handbook's companion, the *Pact Developer Handbook*,
  covers it, or the chapter "Where to Get Help" points you at the community
  channels if you'd rather have someone walk you through it. Once restored, the
  engine drives the swap on its own exactly like any other — completing it if
  it still can, or refunding you once the timelock allows — with no data folder
  to carry over by hand.

> **Warning** — This safety net only covers swaps **started after this
> feature shipped**. A swap that was already in flight when you upgraded was
> never backed up to the relays and is not retroactively rescuable — for those,
> the data folder is still what carries the swap across machines (see below).
> And in the rare case where you were the **maker of a Private (Taproot)** swap
> and your machine died in the narrow window after accepting but before the
> final signatures were assembled, the swap cannot be completed even after
> restoring — the assembled signatures are the one piece of data that can't be
> recreated from your seed. Your funds are never at risk either way: the
> timelock refund is always the fallback.

> **Note** — Keeping Satchel's **data folder** as an actual backup (copying it
> alongside your recovery phrase) still works too, and restores a swap
> immediately with no relay round-trip or waiting period — useful if you're
> migrating a working machine deliberately rather than recovering from a loss.
> But for an unplanned loss, you no longer *need* it: the relay-based rescue
> above is the automatic safety net, keyed off your recovery phrase alone.

> **Note** — Three things, three jobs: the **timelock** protects funds *locked in
> a swap*; your **recovery phrase** protects your *identity and keys* and, via the
> relay rescue, lets a new machine rediscover an *in-flight swap* too; your
> **data folder**, if you happen to still have it, restores a swap instantly with
> no waiting. Back up the phrase always — it's now doing more work than ever.
> Without it, a lost computer is a disaster — write it down.
