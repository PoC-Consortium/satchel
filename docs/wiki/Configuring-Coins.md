# Configuring Coins

A *coin* in Pact is a chain the engine can swap on. Two coins are built in — **btcx** (Bitcoin-PoCX) and **btc** (Bitcoin) — and more can be added without recompiling. To trade you connect each coin either to **your own node** (Core RPC) or to **Electrum servers** (nodeless: the wallet lives on the Pact seed), and tradeable *pairs* are derived automatically from which coins are live and what they support.

For the full details see both handbooks: **Pact** <https://github.com/PoC-Consortium/satchel/tree/master/docs/handbook-pact> and **Satchel** <https://github.com/PoC-Consortium/satchel/tree/master/docs/handbook-satchel>.

## Built-ins and `coins.toml`

The engine ships with `btcx` and `btc` compiled in. Additional coins are declared in a **`coins.toml`** file that sits next to the executable — add a `[[coin]]` block plus an icon and the coin appears, no recompile. The bundled file ships three: **btcx**, **btc**, and **ltc** (Litecoin, the first added third coin). A file-coin whose id collides with a built-in is dropped.

`coins.toml` is read by both pactd (consensus parameters) and Satchel (connection defaults + icon). Satchel resolves `<exe>/coins.toml` first, then a user-editable copy under the config dir, falling back to a baked-in default if parsing fails.

> **Per-coin minimum feerate** — each `[coin.<network>]` block takes an optional **`min_feerate_sat_vb`** (default `1`). pactd floors every funding/spend feerate at it, so chains with a higher wallet floor still fund. The bundled **ltc** ships `10` on all networks (litecoind's `-mintxfee`, exposed by no RPC); **btc**/**btcx** keep the default `1`.

## Connecting a coin

### pactd flag form

Point each coin at a node backend with `--coin`:

```sh
pactd --coin btcx=http://__cookie__:<hex>@127.0.0.1:19443/wallet/alice \
      --coin btc=http://__cookie__:<hex>@127.0.0.1:19543/wallet/alice
```

With an `http://` first URL, that wallet-qualified Core-RPC primary funds swaps and extra comma-separated URLs may be Electrum (`tcp://`/`ssl://`) chain views. An **Electrum-only list** (no `http://`) flips the coin to **nodeless**: a bdk wallet on the Pact seed's BIP-86 branch serves all wallet operations over the first server, the rest are cross-checking chain views — **mainnet requires ≥ 2** — and every server passes a capability handshake (protocol 1.4+, genesis match, pruned servers refused). The coin id must exist in the registry; the last `--coin` for a given id wins.

### Satchel coin-setup form

In Satchel, **Settings → Coins** starts with a **connection type** choice — *Your own node* or *Electrum*. Electrum mode is a URL-per-line server list (pre-filled from the coin's shipped defaults); node mode is the structured RPC form: **RPC host/port**, **Cookie file** (auto-reads `.cookie`) or **User/password** auth, an optional **wallet name**, and a **Confirmations before final** field. The flow is **validate-genesis-then-save**: clicking **Validate node** runs `validatecoin` (a genesis-hash check) and **Save** stays disabled until it passes — nothing is persisted until the genesis matches. Editing a validated form invalidates it again.

The data-dir / cookie path field understands `~`, `%LOCALAPPDATA%`, `%APPDATA%`, and the **`%NODEDIR%/<Name>`** token. `%NODEDIR%/<Name>` resolves to the node's real per-OS default data dir — Windows `%LOCALAPPDATA%\<Name>`, macOS `~/Library/Application Support/<Name>`, Linux `~/.<name>` — so cookie auth works out-of-the-box on every OS. The bundled templates use it (`%NODEDIR%/Bitcoin-PoCX`, `%NODEDIR%/Bitcoin`, `%NODEDIR%/Litecoin`), so the form is prefilled with the correct path and Windows users no longer have to hand-fix it.

## Confirmation depth

Each coin has a confirmation depth for reorg finality (`N≥1`), which gates auto-redeem and swap completion. Set it per-coin with `--coin-confs <id>=<N>` (or the Satchel **Confirmations before final** field). Left blank, a default heuristic applies:

| Network / chain | Default confirmations |
|---|---|
| regtest | 1 |
| fast chain | 10 |
| slow chain | 6 |

## Capabilities and pairs

`listcoins` reports each coin's **capabilities** — **CLTV**, **SegWit v0**, and **Taproot**. Pairs are **derived, not curated**: `listpairs` walks live coins and reports which protocols a pair supports (v1 HTLC needs CLTV+SegWit; v2 adaptor needs Taproot). A pair only shows as ready when both coins are configured and live. The first supported pair is **BTCX ↔ BTC**.

> **Note** — in Satchel, the **first-run ≥2-live-coins gate** means you must have at least two coins with status `ok` before you can reach the trading screens. See [Satchel User Guide](Satchel-User-Guide).

> **Watch-only escape hatch** — you can skip coin setup entirely and **browse the board with zero coins configured** via watch-only mode. You can read offers and withdraw any you already own, but trading (post/take/fund) still requires two live coins. See [Satchel User Guide](Satchel-User-Guide).

## See also

- [Running pactd](Running-pactd) · [JSON-RPC API](JSON-RPC-API) · [Satchel User Guide](Satchel-User-Guide)
