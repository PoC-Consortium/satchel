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

### Nodeless (Electrum-only) coins

A coin whose backend list has **no `http://` primary** runs **nodeless**
(epic #58): the first `tcp://`/`ssl://` URL becomes a bdk wallet on the Pact
seed's BIP-86 branch (`m/86'/<bip32_coin_type>'/0'`), synced over the same raw
Electrum calls the chain-data backends use; any further URLs join as
independent chain views. All nine `wallet_*` operations — funding, the v2
two-phase build, CPFP, RBF bump — are served by that wallet, and the
`listtransactions` RPC exposes its activity feed. Rules and guarantees:

- **Mainnet requires ≥ 2 Electrum servers** — a single lying or withholding
  server must never be the only view of the chain while funds move (spec §10).
- Every server passes a **capability handshake** before use:
  `server.version` (protocol 1.4+), `server.features` cross-checks
  (`genesis_hash` must match; **pruned servers are refused** — a restored
  seed's history scan would be silently incomplete), and a deep check that
  fetches header 0 and hashes it locally (which also validates PoCX's 286-byte
  headers against that server).
- **Address handout is capped** (`MAX_UNUSED_AHEAD = 20`, `wallet_bdk.rs`):
  `getnewaddress` reveals fresh external addresses only while fewer than 20
  revealed-but-unused ones are outstanding; past the cap it recycles the
  oldest unused address instead. The restore scan's `STOP_GAP = 25` therefore
  always covers the real address gap by construction — a wallet restored from
  the seed alone finds every coin, with no deep-rescan affordance needed.
- A locked or absent seed degrades to **chain-reads-only** and surfaces as
  `wallet_locked` — exactly like an encrypted, locked Core wallet.
- Default server lists ship per coin in `coins.toml`
  (`connection.electrum = [...]`) and pre-fill Satchel's setup form.
- An Electrum-FIRST list must be Electrum-only; with a Core-RPC primary,
  Electrum URLs remain plain chain-data views as before.

### Multiple Electrum servers & failover

A nodeless coin's Electrum list is not a fixed primary-plus-spares — the engine
runs it as an **active set** (issue #98). At any moment only a few servers are
*active*: one **wallet home** (the bdk wallet and its sync worker are pinned to
it) plus a couple of **views** (independent read-only cross-checks). The rest
sit as cold **standbys**, holding no socket until they are promoted. This is
what lets the list grow to a dozen servers without adding latency — only the
active few are ever dialed.

Health is **passive**: a server's state (`healthy` / `down` / `untested`) is
derived from whether real requests to it succeed. Nothing probes on a schedule,
and opening the Network monitor (`serverstatus`) never dials. A server that
fails trips a backoff (`retry_in_secs`) and is skipped *instantly* by later
requests — no stalling on its connect timeout — then half-opens for a retry
once the window expires.

Failover is automatic and role-aware:

- A **view** that goes down is benched and a healthy standby is promoted in its
  place.
- If the **wallet home** goes down, the home is re-elected onto a healthy,
  already-verified *view* — a warm socket, so the wallet migrates with a sync
  gap of seconds, never onto a cold or unverified server. A home that is dead
  from the start is skipped the same way at boot.

The governing principle is **tolerate absence, never tolerate disagreement**.
An unreachable server is simply skipped and the others cover, so *more servers
make a coin more robust, not more fragile*. But when servers that **do** answer
disagree about an on-chain fact, the read fails closed rather than trust either
(spec §10). On **mainnet**, a nodeless coin's money-critical reads — confirming
a counterparty's funding, finality depth, the deadline clocks — require **two
agreeing responders**, so running on a single reachable server is a *degraded*
state: balances still display, but a swap will not start or advance until a
second view returns. A coin is only fully **offline** when *no* configured
server answers. (On test networks the quorum relaxes to one, so a single server
both displays and trades.)

Watch it live per coin with `serverstatus`, or Satchel's Network screen; the
`servers_total` / `servers_healthy` / `servers_down` / `wallet_server_state`
fields on `listcoins` summarize the same registry.

### Per-coin minimum feerate

pactd derives its funding and spend feerate from the node's `estimatesmartfee`,
falling back to `1 sat/vB` (Bitcoin's floor) on a quiet or fresh chain. Some
chains bake a higher wallet floor: litecoind's `-mintxfee` defaults to ~10
sat/vB, and because that floor is exposed by **no RPC**, the engine cannot
discover it at runtime — a spend below it is simply rejected (`-6`, "lower than
the minimum fee rate setting"), so swaps on that coin can't fund at all.

Carry the floor as data instead. Each `[coin.<network>]` block takes an optional
**`min_feerate_sat_vb`** (decimal sat/vB, e.g. `0.1`; default `1` for file
coins, whose node version is unknown); the engine floors every feerate at it,
so a coin's own floor always wins:

```toml
[coin.mainnet]
min_feerate_sat_vb = 10   # litecoind's wallet floor; optional, defaults to 1
consensus = { genesis_hash = "12a7…", bech32_hrp = "ltc", … }
```

The bundled `ltc` coin ships `min_feerate_sat_vb = 10` on mainnet, testnet, and
regtest; the built-in `btc` and `btcx` coins floor at **0.1 sat/vB** (Core 30+
relays it). Two floors apply, deliberately distinct: **estimator/preset-driven**
rates (market estimates, target fallbacks) never go below **1 sat/vB** — the
miner-revenue floor — while an **explicitly user-chosen** rate (the send form's
Custom field) respects only the coin floor, so `0.1` survives to the wire.
Internally feerates are integer **sat/kvB** (`min_feerate_sat_kvb`, 100 = 0.1
sat/vB); the decimal form exists only at human-facing edges like this file.

### The `%NODEDIR%` datadir token

A `connection` sub-table's `datadir` value may use a `%NODEDIR%/<Name>` token.
Satchel expands it to the node's **real per-OS default data directory** for a
Bitcoin-Core-family node named `<Name>`, which is where that node writes its
`.cookie`:

| OS | `%NODEDIR%/<Name>` resolves to |
|---|---|
| Windows | `%LOCALAPPDATA%\<Name>` |
| macOS | `~/Library/Application Support/<Name>` |
| Linux | `~/.<name>` (lowercased) |

This is why the bundled templates use `%NODEDIR%/Bitcoin-PoCX`, `%NODEDIR%/Bitcoin`,
and `%NODEDIR%/Litecoin`: each resolves to the correct cookie path on every OS
without per-platform editing. The `datadir` field also understands `~`,
`%LOCALAPPDATA%`, and `%APPDATA%`; anything else is left literal.

## Attaching backends with `--coin`

A coin in the registry is *configured* by attaching a chain backend with
`--coin id=url[,url]`:

- The **first URL is the wallet-qualified Core-RPC primary** — the
  bitcoind-style RPC, including the wallet path, that actually funds and
  broadcasts swap transactions. For example:
  `--coin btcx=http://user:pass@127.0.0.1:9332/wallet/swap`.
- Any **additional URLs may be Electrum** backends (`tcp://` or `ssl://`), used
  for light chain queries.
- A list with **no** `http://` URL at all runs the coin **nodeless** — see
  "Nodeless (Electrum-only) coins" above.

The coin id must already be in the registry (built-in or `coins.toml`-added);
the flag is repeatable, once per coin, and the last `--coin` for a given id
wins. This is the single, uniform way every coin is wired — there is no
per-coin special-casing.

## Confirmation depth with `--coin-confs`

Each coin has a *confirmation depth* `N` — how many confirmations a funding or
redeem needs before the engine treats it as final. This gates auto-redeem and
swap completion in both v1 and v2, and it is your reorg-safety knob. Override it
per coin with `--coin-confs id=N`; coins you don't override use a default
heuristic based on the network and the chain's block spacing:

| Chain profile | Default `N` |
|---|---|
| Regtest (any coin) | `1` |
| Fast chain — block spacing under 5 minutes (e.g. BTCX, 120 s) | `10` |
| Slow chain — block spacing 5 minutes or more (e.g. BTC, 600 s) | `6` |

The default is also the **maximum** (spec §7.3 as amended for the rc12 recut):
on mainnet and testnet the accepted range is **`2` up to the chain's default**,
and the engine *clamps* values outside it — below `2` is raised to `2`, above
the default is lowered to the default. On regtest the floor is `1` and there
is no cap. The floor exists because 1-block reorgs and stale blocks are
routine, so 0/1-conf trading is disallowed; the cap exists because a
fat-fingered depth (say, 100) would stall every swap on that coin for hours.

> **Note** — The fast-chain default is deliberately higher than the slow-chain
> one: a faster chain produces more blocks in the same wall-clock reorg window,
> so it needs more confirmations to reach comparable finality. Your depth is
> **yours alone**: it gates only your own side's actions, and is exchanged with
> the counterparty purely so their UI can show your progress precisely — they
> never adopt it, and you never adopt theirs.

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
enabled on every network, including mainnet.)

## Inspecting coins and pairs

Three RPC methods report the live state of your coin and pair configuration —
covered in full in the API part of this handbook, but worth knowing here:

- `listcoins` — every registry coin with its capabilities, whether it is
  `configured`, a live `status` probe, tip height, genesis hash, bech32 HRP, its
  effective and default confirmation depth, and — for nodeless coins — the
  Electrum fleet summary (`servers_total` / `servers_healthy` / `servers_down` /
  `wallet_server_state`).
- `listpairs` — every derivable pair with its protocol list (`htlc` / `adaptor`),
  whether both legs are configured, and whether it is currently available.
- `validatecoin` — a genesis check of a *proposed* backend for a coin, without
  touching the engine's live configuration; use it to confirm a node before you
  attach it.
- `serverstatus` — per-server Electrum health for one nodeless coin (roles,
  state, latency, backoff), a passive in-memory read that never dials. Backs the
  Network monitor and the failover model described above.

*See the chapter on coins-and-pairs RPCs for the full field lists and return
shapes.*
