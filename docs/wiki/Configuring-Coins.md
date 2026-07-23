# Configuring Coins

A *coin* in Pact is a chain the engine can swap on. Two coins are built in — **btcx** (Bitcoin-PoCX) and **btc** (Bitcoin) — and more can be added without recompiling. To trade you connect each coin either to **your own node** (Core RPC) or to **Electrum servers** (nodeless: the wallet lives on the Pact seed), and tradeable *pairs* are derived automatically from which coins are live and what they support.

For the full details see both handbooks: **Pact** <https://github.com/PoC-Consortium/satchel/tree/master/docs/handbook-pact> and **Satchel** <https://github.com/PoC-Consortium/satchel/tree/master/docs/handbook-satchel>.

## Built-ins and `coins.toml`

The engine ships with `btcx` and `btc` compiled in. Additional coins are declared in a **`coins.toml`** file that sits next to the executable — add a `[[coin]]` block plus an icon and the coin appears, no recompile. The bundled file ships three: **btcx**, **btc**, and **ltc** (Litecoin, the first added third coin). A file-coin whose id collides with a built-in is dropped.

`coins.toml` is read by both pactd (consensus parameters) and Satchel (connection defaults + icon). Satchel resolves `<exe>/coins.toml` first, then a user-editable copy under the config dir, falling back to a baked-in default if parsing fails.

> **Per-coin minimum feerate** — each `[coin.<network>]` block takes an optional **`min_feerate_sat_vb`** (decimal sat/vB, e.g. `0.1`; default `1` for file coins). pactd floors every feerate at it, so chains with a higher wallet floor still fund. The bundled **ltc** ships `10` on all networks (litecoind's `-mintxfee`, exposed by no RPC); the built-in **btc**/**btcx** floor at `0.1` (Core 30+ relays it). Estimator/preset-driven rates additionally never go below `1` sat/vB (the miner-revenue floor) — only an explicitly user-chosen Custom rate may price between the coin floor and `1`.

## Connecting a coin

### pactd flag form

Point each coin at a node backend with `--coin`:

```sh
pactd --coin btcx=http://__cookie__:<hex>@127.0.0.1:19443/wallet/alice \
      --coin btc=http://__cookie__:<hex>@127.0.0.1:19543/wallet/alice
```

With an `http://` first URL, that wallet-qualified Core-RPC primary funds swaps and extra comma-separated URLs may be Electrum (`tcp://`/`ssl://`) chain views. An **Electrum-only list** (no `http://`) flips the coin to **nodeless**: a bdk wallet on the Pact seed's BIP-86 branch serves all wallet operations over the first server, the rest are cross-checking chain views — **mainnet requires ≥ 2** — and every server passes a capability handshake (protocol 1.4+, genesis match, pruned servers refused). The coin id must exist in the registry; the last `--coin` for a given id wins.

### Multiple Electrum servers & failover

A nodeless coin's server list runs as an **active set**: one **wallet home** (the bdk wallet is pinned to it) plus a couple of **views** (independent read-only cross-checks) are active at a time, and any extras sit as cold **standbys** — so a list can grow to a dozen servers without adding latency. Health is **passive** (derived from real traffic; nothing probes on a schedule): a failed server trips a backoff and is skipped instantly rather than stalling on its timeout. Failover is automatic — a downed view is replaced by a standby, and a downed wallet home is re-elected onto a healthy, already-verified view. The rule is **tolerate absence, never tolerate disagreement**: an unreachable server is skipped and the others cover (more servers = *more* robust), but servers that answer and disagree about an on-chain fact fail the read closed. On **mainnet**, money-critical reads need **two agreeing servers**, so a coin on a single reachable server is *degraded* (balances show, swaps wait); a coin is only **offline** when nothing answers. Watch it per coin with `serverstatus` or Satchel's **Network** screen.

### Satchel coin-setup form

In Satchel, **Settings → Coins** starts with a **connection type** choice — *Your own node* or *Electrum*. Electrum mode is a URL-per-line server list (pre-filled from the coin's shipped defaults); node mode is the structured RPC form: **RPC host/port**, **Cookie file** (auto-reads `.cookie`) or **User/password** auth, an optional **wallet name**, and a **Confirmations before final** field. The flow is **validate-genesis-then-save**: clicking **Validate node** runs `validatecoin` (a genesis-hash check) and **Save** stays disabled until it passes — nothing is persisted until the genesis matches. Editing a validated form invalidates it again.

Each configured coin's card wears a **connection-kind chip** — *RPC (local)*, *RPC (remote)*, *Electrum (local)*, or *Electrum (remote)* — so you can see at a glance how a coin is backed.

> **Switching a funded Electrum coin to node mode** — one wallet serves each coin, never both (the node's wallet on RPC, the pact-seed wallet on Electrum). If the coin's pact-seed wallet still holds funds when you save a switch to *Your own node*, Satchel warns first: the coins stay safe on your seed and reappear the moment you switch back to Electrum, but until then they won't show up or fund swaps — consider sending them somewhere first. (If the Electrum servers are already unreachable, the balance can't be read and the save proceeds without the warning.)

The data-dir / cookie path field understands `~`, `%LOCALAPPDATA%`, `%APPDATA%`, and the **`%NODEDIR%/<Name>`** token. `%NODEDIR%/<Name>` resolves to the node's real per-OS default data dir — Windows `%LOCALAPPDATA%\<Name>`, macOS `~/Library/Application Support/<Name>`, Linux `~/.<name>` — so cookie auth works out-of-the-box on every OS. The bundled templates use it (`%NODEDIR%/Bitcoin-PoCX`, `%NODEDIR%/Bitcoin`, `%NODEDIR%/Litecoin`), so the form is prefilled with the correct path and Windows users no longer have to hand-fix it.

## Confirmation depth

Each coin has a confirmation depth for reorg finality, which gates auto-redeem and swap completion. It is **per-side**: each party sets its own depth from its own config and it gates only that party's own actions and risk — nobody adopts the counterparty's value. The two sides exchange their chosen depths so each can show the other's confirmation counter precisely, but a value out of range is refused up-front rather than adopted. Set it per-coin with `--coin-confs <id>=<N>` (or the Satchel **Confirmations before final** field). Left blank, a default heuristic applies — which is also the **maximum**:

| Network / chain | Default confirmations (= maximum) |
|---|---|
| regtest | 1 |
| fast chain | 10 |
| slow chain | 6 |

On mainnet/testnet the allowed range is **2 up to the default** (spec §7.3 as amended for the rc12 recut): 0- and 1-confirmation trading is disallowed (a single stale block is routine on both chains), and there is no benefit to demanding more than the trustless default, so it is the ceiling. Values outside the band are **clamped** — below 2 rises to 2, above the default drops to the default. Regtest keeps a floor of 1 and no ceiling so the test suite can drive arbitrary depths.

## Capabilities and pairs

`listcoins` reports each coin's **capabilities** — **CLTV**, **SegWit v0**, and **Taproot**. Pairs are **derived, not curated**: `listpairs` walks live coins and reports which protocols a pair supports (v1 HTLC needs CLTV+SegWit; v2 adaptor needs Taproot). A pair only shows as ready when both coins are configured and live. The first supported pair is **BTCX ↔ BTC**.

> **Note** — in Satchel, browsing the board never requires coins: with **zero coins configured** the Corkboard shows every pair automatically. Trading is gated **per action** and server-side (`ensure_chains_live`) rather than at app entry — you need at least two coins with status `ok` to post an offer or create a slip, and both of a pair's coins live to take one. See [Satchel User Guide](Satchel-User-Guide).

## See also

- [Running pactd](Running-pactd) · [JSON-RPC API](JSON-RPC-API) · [Satchel User Guide](Satchel-User-Guide)
