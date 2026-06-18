# PoCX Trading

Trustless P2P trading for PoCX via atomic swaps. No exchanges, no custody,
no fees, no matching engine — a protocol plus a noticeboard.

- Plan: [TRADING_ROADMAP.md](docs/TRADING_ROADMAP.md)
- Design: [ARCHITECTURE.md](docs/ARCHITECTURE.md)

## Components

| Folder | Name | What it is | Phase |
|--------|------|-----------|-------|
| [`spec/`](spec/) | — | Atomic swap protocol spec (v1 + v2) + test vectors | 1 |
| [`pact-proto/`](pact-proto/) | — | Wire format + crypto: signed envelopes, canonical JSON, BIP340, relay sealing | 1 |
| [`pact/`](pact/) | **Pact** | Swap engine (Rust): `libswap` crate, `pactd` daemon, `pact-cli` client | 1 |
| [`corkboard/`](corkboard/) | **Corkboard** | Order board: signed offers + blind relay | 2 |
| [`satchel/`](satchel/) | **Satchel** | Desktop app (Tauri, React/MUI frontend) + light BTC wallet; owns the GUI | 3 |
| [`tools/`](tools/) | — | Dev tooling (e.g. `relay-prober` — Nostr relay eligibility) | — |

Naming theme: the village market square. A **pact** is the trustless deal,
posted on the **corkboard**, settled into your **satchel**. Deliberately no
"exchange"/"DEX" branding.

## Hard constraints (apply to every component)

1. Zero changes to bitcoin-pocx core — stock node, RPC/ZMQ only.
2. Keys never leave the user's machine; hosted components see signed
   offers and encrypted blobs only.
3. One engine, many faces — swap logic exists exactly once, in Pact.
4. Corkboard never matches, executes, custodies, or charges.

**Technology: Rust across all components** (cargo workspaces; Tauri for
Satchel, whose frontend is React + Vite + TypeScript + MUI).
