# First Launch & Setup

The first time you open Satchel, it walks you through a short setup, one screen
at a time, like a friendly wizard. You can't skip ahead and you can't get it
wrong by accident — each step unlocks the next. This chapter follows that
sequence in order:

1. Create (or import) a **merchant** — your trading identity.
2. Back up your **recovery phrase**, then choose whether to protect it with a
   passphrase.
3. (On later launches only) **unlock** if you chose a passphrase.

After that, Satchel checks your coins are connected and hands off to the next
chapter.

> **Tip** — Prefer to do this in another language? Look for the **globe** menu in
> the **top-right corner of the welcome dialog**. Click it and pick from the list —
> Satchel ships in **26 languages**, and the choice applies right away, so you can
> switch before or even partway through setup. The same picker is always available
> later from the header.

## Step 1 — Create your merchant

The first screen welcomes you and explains that Satchel trades under a
*merchant*. A merchant is simply **your trading identity**. Behind that one word
sits a tidy rule:

> **Note** — **One merchant = one seed = one data folder.** A *seed* is the
> master secret that all your keys come from (more on it in the next step). Each
> merchant has its own seed, its own swap history, and its own folder on disk —
> much like separate wallets. Keeping trades under different merchants keeps them
> from being linked together, which is good for privacy.

You have two choices on this screen:

- **Create new** — make a brand-new identity with a fresh seed. Choose this if
  you're starting out.
- **Import** — restore an identity you already have, using its recovery phrase.
  Choose this if you've used Satchel before and want to bring an existing
  merchant onto this machine.

Let's create a new one.

1. Click **Create new**.
2. Satchel asks for a **Merchant name** — just a friendly label so you can tell
   your merchants apart (for example, `Main`). It's only for your own eyes.
3. Click to continue. Satchel creates the merchant and moves you straight to the
   seed step.

![Naming your first merchant.](images/processed/ch04-merchant-name.png){width=70%}

## Step 2 — Your recovery phrase

This is the single most important step in the whole book, so we'll take it
slowly.

### What a recovery phrase is

When Satchel creates your merchant, it generates a *recovery phrase* (also called
a *seed phrase* or *mnemonic*) — a list of **12 or 24 ordinary words**, like
`ripple cabin lunar quote …`. Those words are a human-readable backup of your
seed, the master secret behind every key your merchant uses.

> **Warning** — Anyone who has these words **controls this merchant's funds.**
> Satchel keeps no copy you can recover from — if you lose the words and lose your
> computer, the funds are gone for good. This is the trade-off for being your own
> bank.

### Write it down

Satchel shows you the words on a **"Write down your recovery phrase"** screen,
numbered in order. A toggle above the words lets you choose a **12- or 24-word**
phrase — the default is **12**, and 12 is plenty: this is a hot transit wallet,
not cold storage. Pick 24 if you prefer the longer phrase. Switching the toggle
generates a fresh phrase, so settle on a length *before* you start writing.

1. Get a pen and paper. (Paper, not a photo and not a text file — a backup that
   lives only on a device can be lost or stolen along with it.)
2. Write the words down **in order**, with their numbers.
3. Double-check what you wrote against the screen.
4. Tick the box **"I have written down my recovery phrase."**
5. Continue.

![The recovery phrase, shown for you to copy down.](images/processed/ch04-seed-reveal.png){width=80%}

> **Warning** — **Never share these words with anyone, and never type them into a
> website.** No genuine support person will ever ask for them. Satchel itself will
> never ask you to re-enter your phrase on a web page.

### Confirm you wrote it down

To make sure your backup is good, Satchel next asks you to **confirm a few
words** — it names a couple of positions (for example, "Word #4") and you type
the matching words back. As you type, it offers the valid words so a typo can't
slip through.

If a word doesn't match, Satchel tells you so you can check your written copy
before going on. This little check has saved many people from a backup with a
wrong word in it.

> **Note** — On a practice (regtest) network, this confirmation step is skipped,
> because there's no real money at stake. On the real network you'll always be
> asked to confirm.

### Importing instead

If you chose **Import** back in Step 1, this step looks a little different: rather
than revealing words, Satchel gives you a **numbered grid of word cells** —
arranged in three columns, one cell per word — to **enter your existing 12- or
24-word phrase**. A toggle at the top lets you say whether your phrase is **12 or
24 words**, so the grid has exactly the right number of slots.

Type into the cells one word at a time, and Satchel **suggests the matching word**
as you go (the words come from a fixed list, so it can autocomplete each one). Any
cell holding a word that isn't on that list is **highlighted** so you can spot a
typo at a glance. In a hurry? You can **paste your whole phrase** into the first
cell and Satchel spreads the words across the grid for you.

As you fill it in, a **status line** at the bottom keeps you posted:

- *"Enter all 12 words."* (or 24) while the grid isn't full yet.
- *"Some words aren't in the BIP39 wordlist — check the highlighted ones."* if a
  cell holds a word it doesn't recognise.
- *"Checksum doesn't match — re-check your words and their order."* if every word
  is valid but the phrase as a whole doesn't add up — usually a word in the wrong
  place.
- *"Recovery phrase looks valid."* once everything checks out.

Only when you see **"Recovery phrase looks valid."** can you continue — Satchel
won't let you import a phrase it can't verify, which protects you from a small
slip costing you the import. Then you carry on to the passphrase choice, the same
as everyone else.

## Step 3 — Passphrase: protect the seed, or not

Last, Satchel asks how to store your seed on disk. Two cards:

- **No passphrase (recommended)** — you never have to type anything to start
  trading, and Satchel's automatic safety net (the part that gives you your money
  back if a swap stalls) keeps working even after a reboot, with nothing for you
  to do. The seed still isn't left as plain text on disk: on Windows and macOS
  Satchel locks it with a key kept in your operating system's secure store, so
  the file alone is useless if copied to another machine. That key lives on this
  computer, though, so it doesn't protect you from someone who is already signed
  in as you — for that, choose a passphrase below.

- **Encrypt with a passphrase** — the seed is locked with a passphrase you
  choose. It's safer if someone gets access to your computer's files, but there's
  a cost: you must re-enter the passphrase each time you start Satchel, and the
  automatic safety net stays paused until you do.

> **Note** — Satchel **never stores your passphrase.** If you encrypt and then
> forget the passphrase, no one can unlock the seed for you — your recovery phrase
> is the only way back in. Either way, your written recovery phrase remains your
> real backup.

> **Tip** — The seed in a merchant is a *hot* seed — it holds only the temporary
> keys needed to move coins through a swap, not a long-term vault. For most
> people, **No passphrase** is the right, convenient choice. Whichever you pick,
> sweep any sizeable proceeds out to your own main wallet rather than letting them
> pile up here.

Make your choice and finish. Your merchant is ready.

## Later launches: unlocking

If you chose **No passphrase**, Satchel goes straight to trading every time you
open it.

> **Note** — One rare exception: if this computer can no longer unlock the seed
> stored on disk — typically because the data folder was moved to a new machine,
> or the system keychain was reset — Satchel opens a **guided re-import dialog**
> at startup explaining exactly that. Re-enter your recovery phrase and you're
> back; **your funds and swaps are not affected**. The *Backup, Seeds & Safety*
> chapter has the details.

If you chose **Encrypt with a passphrase**, the next time you start Satchel you'll
see an **Unlock merchant** screen. Type your passphrase to unlock the seed for
this session. Satchel holds it in memory only and forgets it when you close the
app — it is never written down. From this screen you can also switch to a
different merchant instead.

## Managing more than one merchant

You're not limited to one identity. Later, from the merchant control at the top
of the window, you can **create or import more merchants** and **switch** between
them — each with its own seed, history, and folder. This is handy for keeping,
say, a main identity and a separate one for private trades. We cover the
**Merchant Manager** in Part 2.

> **Note** — Satchel won't let you switch away from a merchant that has a swap in
> progress — that's a safety measure, so an in-flight trade is never abandoned.

## What's next

With your merchant created and your seed backed up, Satchel drops you straight
into the app. On this first run it offers a **one-time, skippable coin-setup
dialog** to help you connect two coins — but it never blocks: click **Later** to
go straight to browsing with zero coins configured, or set them up now. It won't
appear again once dismissed. Connecting coins is exactly what the next chapter,
*"Setting Up Your Coins"*, covers.
