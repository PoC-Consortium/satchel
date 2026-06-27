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
> *These docs were verified against commit `c48112b`.*

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
