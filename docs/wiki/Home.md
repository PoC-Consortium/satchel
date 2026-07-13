# Satchel

**Trustless, peer-to-peer trading for cryptocurrencies via atomic swaps.** No exchange, no custody, no fees, no matching engine — just a protocol, some relays, and a desktop app. **Pact** is the swap engine; **Satchel** is the desktop app; **Corkboard** and **Nostr** are the transports that carry offers. Keys never leave your machine, the chain enforces the deal, and whatever is hosted sees only signed offers and encrypted blobs. The first supported pair is **BTCX ↔ BTC**; more coins follow.

```
  ┌─────────────────────────────┐         ┌──────────────────────────┐
  │  Your machine               │         │  Hosted (untrusted)      │
  │                             │         │                          │
  │  Satchel (desktop GUI)      │ signed  │  Nostr relays            │
  │      │ JSON-RPC (loopback)  │ offers  │   (default transport)    │
  │      ▼                      │   +     │                          │
  │  pactd (swap engine)────────┼────────►│  ...or a Corkboard       │
  │      │ owns BIP39 seed,     │ sealed  │   instance               │
  │      │ keys, refunds        │  blobs  │   (self-hostable)        │
  │      ▼                      │         │                          │
  │  BTCX node + BTC backend    │         └──────────────────────────┘
  └─────────────────────────────┘
```

> **Status** — live. v1 (hash-locked HTLC) and v2 (Taproot/MuSig2 adaptor) are **reviewed and running on mainnet**. As with any self-custody software, you alone hold your keys — keep your recovery phrase safe.
>
> *These docs were verified against commit `205ee74`.*

> **Upgrading** — every offer and handshake message carries its protocol's **wire epoch**, and incompatible peers are refused up-front (offers badged un-takeable on the Corkboard, a mixed-version take rejected with a clear reason) so nothing fails mid-swap. Two flag-days matter: **rc10** changed the v2 (Taproot) cooperative redeem and set the epochs to v1 = 1, v2 = 2; the **rc12 recut** made confirmation depths per-side and bumped both epochs to **v1 = 2, v2 = 3**. Because both epochs move in the recut, an updated build and a pre-recut build cannot open **either** v1 or v2 swaps with each other — **settle or abort any live swaps before upgrading**; swaps already past the handshake finish on the version that made them. Also from rc10: v2 fundings and redeems broadcast **non-replaceable** (the engine bumps a stuck one via CPFP); the timelock refund keeps RBF. **rc13** is wire-compatible with rc12 — no flag day, epochs unchanged (v1 = 2, v2 = 3). On first start after upgrading, your finished swap history is claimed automatically; a swap still active from before the upgrade appears under **"Another machine"** and needs one **Take over** click.

## Start here

| You want to… | Go to |
|---|---|
| Understand what this is and how the pieces fit | [Architecture](Architecture) · [How Atomic Swaps Work](How-Atomic-Swaps-Work) |
| Know what's safe and what's trusted | [Security Model](Security-Model) |
| Install Satchel and make your first swap | [Getting Started](Getting-Started) · [Satchel User Guide](Satchel-User-Guide) |
| Connect coins / add a new one | [Configuring Coins](Configuring-Coins) |
| Trade off-market with a friend | [Private Offers](Private-Offers) |
| Run the engine and drive it yourself | [Running pactd](Running-pactd) · [pact-cli](pact-cli) · [JSON-RPC API](JSON-RPC-API) |
| Host your own board | [Self-Hosting a Corkboard](Self-Hosting-Corkboard) |
| Build from source | [Building from Source](Building-from-Source) |
| Look up a term or a common question | [Glossary](Glossary) · [FAQ](FAQ) |

**The deep docs** (everything the wiki only summarizes):

- **Users → the Satchel User Handbook:** <https://github.com/PoC-Consortium/satchel/tree/master/docs/handbook-satchel>
- **Developers / integrators / operators → the Pact Developer Handbook:** <https://github.com/PoC-Consortium/satchel/tree/master/docs/handbook-pact>

The naming theme is the village market square: a **pact** is the trustless deal, posted on the **corkboard**, settled into your **satchel**. Deliberately no "exchange" / "DEX" branding.
