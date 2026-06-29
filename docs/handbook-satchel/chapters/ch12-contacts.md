# Contacts

Trade with the same people often enough and you'll want to recognise them.
**Contacts** is Satchel's private address book: it lets you put a name to a
counterparty so you're not squinting at a string of hex every time. Click
**Contacts** in the left navigation to open it.

Every counterparty in Satchel is identified by a cryptographic public key — the
same key that draws the little *identicon* and short fingerprint you see on offer
cards, on the Swaps page, and in the active-swaps dock. A *contact* maps one of
those keys to a **nickname** you choose, an optional freeform **note**, and a
**standing**: **Trusted**, **Neutral**, or **Blocked**. That's the whole feature
— a way to remember who's who.

> **Note** — Contacts are **local to this device.** They live in Satchel's
> `satchel.json` settings file and are **never** shared, published, signed, or
> sent to a relay; the engine never even sees them. Your nicknames are yours
> alone, and they survive clearing the webview cache. Nobody else learns what you
> called them, and nobody can read your list off the network.

## What a nickname is — and isn't

A nickname is a convenience label, nothing more. It's shown *alongside* the
identicon and fingerprint — it never **replaces** them.

> **Warning** — The identicon and fingerprint remain the real, spoof-proof
> identity; the nickname is only a note you've pinned to it. Never treat a name
> as proof of who you're dealing with. If you only went by the label, a hostile
> party could generate a fresh key, and there is nothing stopping *them* from
> being the one you happened to nickname "alice." Always check the identicon and
> fingerprint match the counterparty you expect before you act.

## What standing does — and doesn't

Standing is a *soft, personal* signal — your own memory of how a past trade went.
It's deliberately the human judgement the protocol itself leaves out: in Pact,
trust is **atomicity only**. The swap either completes for both sides or refunds,
no matter who the counterparty is.

So it's worth being honest about the limit here: **Blocked never stops a trade.**
Blocking someone only changes your local view and adds warnings — it does not
prevent you, or them, from trading. What actually protects you is the atomic swap,
not this list. Use standing to keep your own notes tidy, not as a safety
mechanism.

## Adding and editing a contact

You don't go to the Contacts page to add someone — you do it wherever you already
see them. Click any counterparty's identicon or tag — on a Corkboard offer card,
on the **Swaps** page, in the active-swaps dock, or in the take-confirm dialog —
and a small menu opens:

- **Add to contacts…** / **Edit contact…** — opens the edit dialog, with a
  **Nickname** field and a multi-line **Notes** field.
- **Mark as trusted** / **Mark as neutral** / **Block** — sets the standing in one
  click.
- **Copy public key** — copies the full key to your clipboard.
- **Open in Contacts** — jumps to this contact's row on the Contacts page.

## The Contacts page

The page is a searchable table of everyone you've saved. The search box matches on
**nickname, note, or key**, and filter chips — **All / Trusted / Blocked** — narrow
the list. The columns are:

| Column | What it shows |
|---|---|
| **Identity** | The identicon and short fingerprint — the real identity. |
| **Nickname** | The label you chose. |
| **Notes** | Your freeform note. |
| **Standing** | Trusted, Neutral, or Blocked. |
| **Added** | When you first saved them. |

You can edit a row inline, or remove a contact entirely (Satchel asks you to
confirm a removal first). If you haven't saved anyone yet, the page shows a short
empty state telling you to click a counterparty's identicon anywhere it appears to
add your first one.

## Where standing shows up

Once you've saved a contact, their standing follows them around the app:

- **On the counterparty tag** — wherever a counterparty renders, the tag shows
  your nickname and a small status dot for their standing.
- **On the Corkboard** — once you've blocked anyone, a **Hide blocked offers**
  toggle appears above the board. Turn it on to drop blocked makers' offers from
  the ladder so you don't keep seeing them.
- **At take-confirm** — if you take an offer from someone you've blocked, the
  confirmation dialog shows a warning ("You blocked this counterparty") and asks
  you to confirm a second time. It's a personal reminder, not a barrier: blocking
  does not stop the trade, and the swap is atomic either way.
