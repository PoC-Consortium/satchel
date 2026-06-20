# Coins, Pairs & Capabilities

Pact trades between *coins*. A coin is a Bitcoin-Core-compatible chain the engine
knows how to verify, derive keys for, and build swap outputs on. Trading *pairs*
are not curated — they are **derived** from what each pair of coins can do. This
chapter covers the built-in coins, adding coins as data, attaching backends with
`--coin`, setting confirmation depths, and how capabilities turn into pairs.

## Built-in coins

Two coins ship in the engine code itself, fully trusted: `btcx` (Bitcoin-PoCX)
and `btc` (Bitcoin). These are the engine's *registry* coins — their consensus
parameters are compiled in and cannot be redirected by a stray file.

## Adding coins with `coins.toml`

Any Bitcoin-Core-compatible chain can be added **as data**, with no recompile,
through a `coins.toml` passed with `--coins-file`. The file is loaded at startup
and merged with the built-ins; a file coin whose id collides with a built-in is
dropped (so a stray file can never redirect `btc` or `btcx`), and a malformed
file logs an error and falls back to the built-ins rather than refusing to boot.

Only the **consensus** fields the engine needs are read — enough to verify
genesis, build and parse addresses, and derive keys. A minimal entry looks like:

```toml
schema_version = 1

[[coin]]
coin_id        = "ltc"
display_name   = "Litecoin"
symbol         = "LTC"
decimals       = 8
bip32_coin_type = 2
target_spacing_secs = 150

[coin.capabilities]
cltv      = true
segwit_v0 = true
taproot   = true

[coin.mainnet.consensus]
genesis_hash = "12a765e31ffd4059bada1e25190f6e98c99d9714d334efa41a195a7e7e04bfe2"
bech32_hrp   = "ltc"

[coin.regtest.consensus]
genesis_hash = "530827f38f93b43ed12af0b3ad25a288dc02ed74d6d7857862df51fc56c416f9"
bech32_hrp   = "rltc"
```

> **Note** — The consensus values in `coins.toml` are trusted by whoever edits
> the file, but they are not taken on faith at runtime: before any funds move,
> the engine validates each coin's `genesis_hash` against the live node
> (`getblockhash 0`). A wrong genesis hash fails the chain check, not a swap.
> A `coins.toml` entry may also carry a `connection` sub-table with RPC defaults;
> that is Satchel's concern and is ignored by the engine.

## Attaching backends with `--coin`

A coin in the registry is *configured* by attaching a chain backend with
`--coin id=url[,url]`:

- The **first URL is the wallet-qualified Core-RPC primary** — the
  bitcoind-style RPC, including the wallet path, that actually funds and
  broadcasts swap transactions. For example:
  `--coin btcx=http://user:pass@127.0.0.1:9332/wallet/swap`.
- Any **additional URLs may be Electrum** backends (`tcp://` or `ssl://`), used
  for light chain queries.

The coin id must already be in the registry (built-in or `coins.toml`-added);
the flag is repeatable, once per coin, and the last `--coin` for a given id
wins. This is the single, uniform way every coin is wired — there is no
per-coin special-casing.

## Confirmation depth with `--coin-confs`

Each coin has a *confirmation depth* `N` — how many confirmations a funding or
redeem needs before the engine treats it as final. This gates auto-redeem and
swap completion in both v1 and v2, and it is your reorg-safety knob. Override it
per coin with `--coin-confs id=N` (`N ≥ 1`); coins you don't override use a
default heuristic based on the network and the chain's block spacing:

| Chain profile | Default `N` |
|---|---|
| Regtest (any coin) | `1` |
| Fast chain — block spacing under 5 minutes (e.g. BTCX, 120 s) | `10` |
| Slow chain — block spacing 5 minutes or more (e.g. BTC, 600 s) | `6` |

> **Note** — The fast-chain default is deliberately higher than the slow-chain
> one: a faster chain produces more blocks in the same wall-clock reorg window,
> so it needs more confirmations to reach comparable finality. Whatever you set,
> the floor is `1`.

## Capabilities and how pairs are derived

Each coin declares three capabilities: `cltv` (OP_CHECKLOCKTIMEVERIFY,
i.e. usable timelocks), `segwit_v0` (P2WSH outputs), and `taproot` (P2TR /
v1 segwit outputs). These capabilities determine which *protocols* a pair of
coins can use, and therefore which pairs are tradable at all — the engine
**derives** the pair list, it never curates one:

- A pair where both coins have `cltv` + `segwit_v0` can run the **v1 HTLC**
  protocol.
- A pair where both coins have `taproot` can run the **v2 adaptor** protocol.

When both protocols are possible, the engine **prefers HTLC**; the v2 adaptor
protocol is selected only for Taproot pairs that lack an HTLC option. The shipped
BTCX ↔ BTC pair therefore defaults to HTLC. (Note that v2 adaptor swaps are
enabled on every network, including mainnet, under external audit.)

## Inspecting coins and pairs

Three RPC methods report the live state of your coin and pair configuration —
covered in full in the API part of this handbook, but worth knowing here:

- `listcoins` — every registry coin with its capabilities, whether it is
  `configured`, a live `status` probe, tip height, genesis hash, bech32 HRP, and
  its effective and default confirmation depth.
- `listpairs` — every derivable pair with its protocol list (`htlc` / `adaptor`),
  whether both legs are configured, and whether it is currently available.
- `validatecoin` — a genesis check of a *proposed* backend for a coin, without
  touching the engine's live configuration; use it to confirm a node before you
  attach it.

*See the chapter on coins-and-pairs RPCs for the full field lists and return
shapes.*
