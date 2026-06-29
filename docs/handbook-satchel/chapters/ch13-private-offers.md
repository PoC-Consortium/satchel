# Private (Off-Market) Offers

Sometimes you don't want to post to the whole board — you want to trade with one
specific person. Maybe you've agreed a price with a friend, or you simply prefer
not to advertise. *Private offers* let you do exactly that. Instead of pinning an
offer to the Corkboard, you create a small code called a **slip** and hand it to
your counterparty directly.

A slip is the same kind of signed offer you'd post publicly — it's just delivered
person-to-person instead of broadcast. The swap that results is identical: still
atomic, still auto-refunding, still safe.

## Creating a slip

1. Click **Create slip** in the left navigation.
2. Fill in the **same offer form** you'd use to post publicly — coins, amounts,
   price, swap type, safety timelock. (See the chapter on making an offer for the
   field-by-field details.)
3. Review and confirm. Satchel produces a slip: a string that begins with
   `pactoffer1:`, shown in a copyable box.
4. Click **Copy** and send the slip to your counterparty over **any chat** —
   messaging app, email, whatever you both use. Satchel doesn't send it for you.

![Create slip: the offer form produces a copyable pactoffer1: slip.](images/processed/ch12-create-slip.png){width=70%}

The slip box also notes that it **expires in about 24 hours**. After that it's no
good and your counterparty will have to ask for a fresh one.

## Taking a slip

If someone sends *you* a slip:

1. Click **Take a slip** in the left navigation.
2. Paste the `pactoffer1:` string into the box.
3. Click **Review & take**. Satchel decodes it, shows you the same take
   confirmation dialog you'd get from a board offer (amounts, protocol, refund
   times, fees), and re-checks the maker's signature.
4. Confirm. From there it's an ordinary swap — follow it on the **Swaps** page.

## Tracking your slips: My slips

Click **My slips** to see the private offers you've handed out. Each card shows
the amounts and a countdown to expiry, with a **Cancel** button.

Cancelling stops you honouring that slip: a friend who still holds it can no longer
take it. (Slips also expire on their own after roughly 24 hours, so an unused one
lapses even if you forget.) If you have no slips out, the screen offers a button
to create one.

![My slips: outstanding private offers with expiry countdowns and Cancel.](images/processed/ch12-my-slips.png){width=70%}

## A note on slip safety: bearer offers

A slip is a *bearer* instrument: **whoever holds it can take it**, until it expires
or you cancel it. There's no name baked into a slip — if you forward it to the
wrong person, or it's intercepted, they could take it on the agreed terms.

That sounds scarier than it is, because of what a slip *can't* do:

- **The terms are fixed.** A slip locks in the exact coins, amounts, price, and
  timelock. Nobody can take it for different terms than you set.
- **The swap is still atomic.** Whoever takes it, the trade either completes for
  both sides or refunds — nobody can take your funds without giving you theirs.
- **You can cancel.** Use **My slips → Cancel** any time before it's taken, and it
  becomes worthless.

> **Tip** — Treat a slip like a price you've quoted to one person: only send it to
> the counterparty you meant, and cancel it if plans change. The worst case is
> that someone else accepts the *exact trade you offered* — never a worse one.

## When to use private versus public

- **Post an offer (public)** when you want the best chance of a fill and don't mind
  the offer being visible. The whole board can take it.
- **Create slip (private)** when you've already agreed terms with someone, or you'd
  rather not advertise — an over-the-counter, trade-with-a-friend deal.

Either way the protection is the same. Private offers change *who can see the
offer*, not *how safe the swap is*.
