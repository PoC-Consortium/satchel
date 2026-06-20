# Private Offers

A *private offer* lets you trade off-market — with a specific friend instead of posting to a public board. Instead of advertising terms on a [transport](Transports), you produce a signed **slip** and send it directly, over any channel you like (chat, email, paste).

For the full treatment see the private-offers chapters in both handbooks: **Pact** <https://github.com/PoC-Consortium/satchel/tree/master/docs/handbook-pact> and **Satchel** <https://github.com/PoC-Consortium/satchel/tree/master/docs/handbook-satchel>.

## The `pactoffer1:` slip

A slip is the same signed offer envelope used on the boards, just base64url-encoded with a `pactoffer1:` prefix — no new wire fields, nothing posted anywhere. When your counterparty takes it, the engine re-verifies the signature before doing anything.

## Make / take / cancel

1. **Make** — in Satchel's **Create slip** screen (or `makeprivateoffer`), fill the offer form and get a copyable `pactoffer1:…` string. It is stored locally and **not** posted to any board.
2. **Send** — paste the slip to your counterparty over whatever channel you trust.
3. **Take** — they paste it into **Take a slip** (or `takeoffer`). The engine decodes, verifies the signature, applies the gates, and sends a sealed take back to the maker via the relay.
4. **Cancel** — withdraw an outstanding slip from **My slips** (or `cancelprivateoffer`).

## Relay-assisted flow

The take travels back as a sealed blob through the relay/board, so the **maker must stay online and polling** to receive it and continue the swap:

```text
maker: makeprivateoffer ──► paste slip in chat ──► friend: takeoffer
   ▲                                                      │
   └────────── sealed take via relay mailbox ◄────────────┘
   (maker polls, then the swap proceeds on-chain)
```

## Bearer-slip safety

A slip is a **bearer instrument** — whoever holds it can take it. That is safe because the terms are fixed and the settlement is atomic:

- The offer's terms (amounts, coins, timelocks) are baked into the signed slip and can't be altered.
- Settlement is the same trustless atomic swap — the maker funds first, the chain enforces the deal, and neither side can run off with the other's coins.
- Slips carry a TTL (~24h) and auto-expire, so a leaked-but-untaken slip simply lapses.

## See also

- [Transports](Transports) · [Satchel User Guide](Satchel-User-Guide)
