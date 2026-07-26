# crier

A read-only Discord bot that cries the Pact orderbook. It watches the public
offer adverts (Nostr kind 31510) and maker revocations (kind 5) on the same
relays Satchel uses, rebuilds the book, and:

- answers **`/book`** (price ladder), **`/top`** (best bid/ask per pair), and
  **`/status`** (relay health) on demand;
- **announces top-of-book changes** for configured pairs into a channel —
  debounced, so maker refresh churn stays silent.

Prices are unit prices (what 1 BTCX costs, in mBTC by default), with an
optional USD reference annotation. crier holds no funds, has no identity, and
never takes offers; it only verifies other people's signatures. It can only
say what is provable from relay data — an offer that disappears is reported
as *gone/revoked*, never "filled" (takes are private and not observable).

## Build

```sh
cd crier
cargo build --release        # binary: target/release/crier
```

Requires the sibling `pact-nostr`/`pact-proto` crates (i.e. build from a
checkout of this repo).

## Try it without Discord

```sh
./crier --dry-run
```

Connects to the relays, prints the live book for the configured pairs, and
prints every announcement it WOULD post. No token needed. Do this first —
both to sanity-check connectivity and to review the message formats.

## Deploy

### 1. Create the Discord application

1. <https://discord.com/developers/applications> → **New Application** → name
   it (e.g. `crier`).
2. **Bot** tab: no privileged intents are needed (leave Presence/Members/
   Message Content OFF). **Reset Token** and copy it — this is
   `CRIER_DISCORD_TOKEN`.
3. Invite it: **OAuth2 → URL Generator**, scopes `bot` +
   `applications.commands`; bot permissions **Send Messages** and
   **Embed Links**. Open the generated URL and add the bot to your server.

### 2. Get the ids

Discord → User Settings → Advanced → enable **Developer Mode**. Then:

- right-click your server → *Copy Server ID* → `discord.guild_id`
  (setting it registers slash commands instantly in that guild; `0` registers
  globally, which can take up to an hour to propagate),
- right-click the announce channel → *Copy Channel ID* →
  `discord.announce_channel_id` (leave `0` for commands-only, no announcer).

### 3. Configure

```sh
cp crier.example.toml crier.toml    # edit: guild_id, announce_channel_id, pairs
export CRIER_DISCORD_TOKEN='<the bot token>'   # never commit the token
```

Defaults: mainnet, Satchel's public relay set, pair `btc/btcx`, prices in
mBTC, USD annotations on.

### 4. Verify, then run

```sh
./crier --dry-run     # book appears? relays connect? formats look right?
./crier               # the real thing
```

On start you should see `slash commands registered…`; `/status` in your
server shows relay connectivity. The announcer stays quiet for
`initial_sync_secs` after start and only posts genuine diffs vs its persisted
state (`crier-state.json`) — restarts don't cause announcement storms.

### systemd

`/etc/systemd/system/crier.service` (adjust paths/user; see
[`crier.service`](crier.service) in this directory):

```sh
sudo cp crier.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now crier
journalctl -u crier -f          # logs
```

The unit reads the token from `/etc/crier/crier.env`
(`CRIER_DISCORD_TOKEN=...`, mode 0600, root-owned).

### Docker

```sh
docker build -t crier -f crier/Dockerfile .        # from the REPO ROOT
docker run -d --name crier --restart unless-stopped \
  -e CRIER_DISCORD_TOKEN='<token>' \
  -v /srv/crier:/data \
  crier
```

`/data` holds `crier.toml` (optional) and `crier-state.json`.

### Upgrade

Stop, replace the binary (or image), start. The state file is
forward-compatible junk-tolerant JSON — worst case crier re-announces the
current top once.

## Operations notes

- **Logs**: `RUST_LOG=debug ./crier` for verbose ingest logging; default is
  `info` with serenity noise suppressed.
- **Relays**: crier is read-only and cheap (one fetch per 30 s poll). Edit
  `relays = [...]` to add your own.
- **Multiple networks**: run one crier per network (separate config + state
  file + channel); there is deliberately no multi-network mode.
- **USD reference**: CoinGecko with Coinbase-spot fallback, refreshed every
  5 min. It is a display reference only; when it is stale/unreachable, the
  USD annotations disappear rather than mislead.
