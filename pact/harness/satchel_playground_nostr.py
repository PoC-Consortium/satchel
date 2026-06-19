#!/usr/bin/env python3
"""Managed-Satchel playground — RELAYS-ONLY (Nostr) variant.

Same two-sided book as satchel_playground.py, but with NO corkboard: Bob and
Carol (and your Satchel "Alice") trade over a single local Nostr relay only.
That's deliberate — it proves offers flow over Nostr *alone* (the demo's target
config, with the corkboard server dropped). If the relay/websocket path is
broken you see an empty board, not a false pass masked by the corkboard.

Two things differ from the corkboard playground:
  * A local Nostr relay stands in for the Corkboard. The relay BINARY is built
    in a dedicated session, so this harness launches whatever PACT_NOSTR_RELAY_CMD
    names ({port}/{dir} substituted) and talks to it at ws://127.0.0.1:19788.
    Its data dir lives under the (temp) workspace → wiped on teardown, so
    "clear the playground → orders gone" actually holds.
  * Offers use the engine's default TTL — the local relay is wiped on teardown,
    so there's nothing to keep short (a short TTL only made offers churn).

The ps1 wrapper (playground-nostr.ps1) pre-seeds Alice's satchel.json with
nostr_relays=[the relay] and board_urls=[] (relays-only) and launches Satchel.
"""

import os
import shlex
import socket
import subprocess
import sys
import time

sys.stdout.reconfigure(line_buffering=True)

from regtest_harness import Harness, HERE, EXE
from test_swap_e2e import build_workspace, Party, COINS_TOML

BLOCK_EVERY_SECS = 4
REPOST_EVERY_SECS = 30
# Use the engine's DEFAULT offer TTL (omit the ttl arg). The short TTL was to
# avoid stale offers lingering on PUBLIC relays — but this playground runs a
# LOCAL relay that's wiped on teardown, so a short TTL only made offers churn
# (expire + re-post) mid-session for no benefit.
NOSTR_RELAY_PORT = 19788

PROTOCOLS = ["pact-htlc-v1", "pact-htlc-v2"]

# Same book as the corkboard playground (see satchel_playground.py for the rate
# rationale). Bob = BUY side (gives BTC), Carol = SELL side (gives POCX).
BOB_OFFERS = [
    ("btc:0.0005", "btcx:24"),
    ("btc:0.001",  "btcx:47"),
    ("btc:0.001",  "btcx:50"),
    ("btc:0.0015", "btcx:72"),
    ("btc:0.002",  "btcx:102"),
    ("btc:0.003",  "btcx:153"),
]
CAROL_OFFERS = [
    ("btcx:25",  "btc:0.0005"),
    ("btcx:50",  "btc:0.00104"),
    ("btcx:50",  "btc:0.00098"),
    ("btcx:75",  "btc:0.00156"),
    ("btcx:100", "btc:0.00196"),
]

# Litecoin sub-book — a two-sided spread on BOTH LTC pairs (BTC<->LTC and
# BTCX<->LTC), so those boards aren't near-empty next to BTCX<->BTC. Mirrors the
# BUY/SELL split: Bob BUYS ltc (gives BTC or BTCX; funds bob_btc / bob_pocx),
# Carol SELLS ltc (gives LTC; funds carol_ltc). LTC is a file-added coin
# (satchel/coins.toml); pinned to v1 HTLC (the classic CLTV+P2WSH path).
BOB_LTC_OFFERS = [
    ("btc:0.0005", "ltc:1"),      # buy LTC with BTC: 1 LTC = 0.0005 BTC
    ("btc:0.001",  "ltc:2"),
    ("btc:0.0015", "ltc:3"),
    ("btcx:25", "ltc:1"),         # buy LTC with BTCX: 1 LTC = 25 BTCX
    ("btcx:50", "ltc:2"),
    ("btcx:75", "ltc:3"),
]
CAROL_LTC_OFFERS = [
    ("ltc:1", "btc:0.00052"),     # sell LTC for BTC (asks a touch above Bob's bid)
    ("ltc:2", "btc:0.00104"),
    ("ltc:3", "btc:0.00156"),
    ("ltc:1", "btcx:26"),         # sell LTC for BTCX
    ("ltc:2", "btcx:52"),
    ("ltc:3", "btcx:78"),
]


class NostrRelay:
    """A local Nostr relay standing in for the Corkboard.

    Default: the bundled `nostr-rs-relay` (pact/harness/bin/nostr-rs-relay) with
    a generated minimal config (our chosen port + an ephemeral db under the
    workspace). Override with PACT_NOSTR_RELAY_BIN (a different binary) or
    PACT_NOSTR_RELAY_CMD (a full command template, {port}/{dir} substituted).
    Ephemeral: the db lives under the (temp) workspace, wiped on teardown."""

    def __init__(self, workdir, port=NOSTR_RELAY_PORT):
        self.port = port
        self.host = "127.0.0.1"
        self.ws_url = f"ws://{self.host}:{port}"
        self.dir = os.path.join(workdir, "nostr-relay")
        os.makedirs(self.dir, exist_ok=True)
        self.proc = None

    def _build_cmd(self):
        # Escape hatch: a full command template.
        tmpl = os.environ.get("PACT_NOSTR_RELAY_CMD")
        if tmpl:
            return shlex.split(
                tmpl.replace("{port}", str(self.port)).replace("{dir}", self.dir))
        # Default: bundled nostr-rs-relay + a generated config (its port lives in
        # the config file, not a flag).
        relay_bin = os.environ.get("PACT_NOSTR_RELAY_BIN") or \
            os.path.join(HERE, "bin", "nostr-rs-relay" + EXE)
        if not os.path.exists(relay_bin):
            raise RuntimeError(
                f"nostr-rs-relay not found at {relay_bin}.\n"
                "Set PACT_NOSTR_RELAY_BIN to the binary, or PACT_NOSTR_RELAY_CMD "
                "to a full launch command ({port}/{dir} substituted).")
        cfg = os.path.join(self.dir, "config.toml")
        db = self.dir.replace(os.sep, "/")
        with open(cfg, "w", encoding="utf-8") as fh:
            fh.write(
                f'[info]\nrelay_url = "{self.ws_url}/"\nname = "pact-playground"\n\n'
                f'[network]\naddress = "{self.host}"\nport = {self.port}\n\n'
                f'[database]\ndata_directory = "{db}"\n')
        return [relay_bin, "--config", cfg, "--db", self.dir]

    def start(self):
        cmd = self._build_cmd()
        self.proc = subprocess.Popen(
            cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        deadline = time.time() + 30
        while time.time() < deadline:
            if self.proc.poll() is not None:
                raise RuntimeError(
                    f"nostr relay exited early: {self.proc.returncode} (cmd: {cmd})")
            try:
                with socket.create_connection((self.host, self.port), timeout=2):
                    print(f"[nostr-pg] relay up on :{self.port} ({self.ws_url})")
                    return self
            except OSError:
                time.sleep(0.2)
        raise TimeoutError("nostr relay did not come up")

    def stop(self):
        if self.proc:
            self.proc.terminate()
            try:
                self.proc.wait(timeout=15)
            except subprocess.TimeoutExpired:
                self.proc.kill()
            self.proc = None


def chain_time(node):
    # Tip block time, used to keep mocktime monotonic across all three chains.
    # litecoind is an older Bitcoin Core fork whose getblockchaininfo has no
    # "time" field (pocx/btc on Core v30 do) — fall back to "mediantime", which
    # every version reports.
    info = node.rpc("getblockchaininfo")
    return int(info.get("time", info["mediantime"]))


def main():
    build_workspace()
    with Harness(keep=False, with_ltc=True) as h:
        relay = NostrRelay(h.workdir)
        relay.start()

        # Same extra wallets as the corkboard playground (two-sided book + Alice
        # funded on ALL THREE coins). See satchel_playground.py for the rationale.
        # bob_pocx is funded too (the harness leaves it empty) so Bob can GIVE
        # BTCX on the BTCX<->LTC board.
        h.pocx.create_wallet("carol_pocx")
        h.btc.create_wallet("carol_btc")
        h.pocx.generate(110, "carol_pocx")
        h.pocx.generate(110, "bob_pocx")
        h.btc.generate(110, "alice_btc")

        # Litecoin leg. alice_ltc + carol_ltc are funded (each gives LTC on some
        # offer); bob_ltc is a receive-only sweep target. LTC coinbase matures at
        # 100, so 110 deep is spendable.
        h.ltc.create_wallet("alice_ltc")
        h.ltc.create_wallet("bob_ltc")
        h.ltc.create_wallet("carol_ltc")
        h.ltc.generate(110, "alice_ltc")
        h.ltc.generate(110, "carol_ltc")
        print("[nostr-pg] funded carol_pocx + alice_btc + alice_ltc/carol_ltc "
              f"(carol_pocx: {h.pocx.rpc('getbalance', wallet='carol_pocx')} POCX, "
              f"alice_btc: {h.btc.rpc('getbalance', wallet='alice_btc')} BTC, "
              f"alice_ltc: {h.ltc.rpc('getbalance', wallet='alice_ltc')} LTC)")

        # RELAYS-ONLY: nostr_relays set, board_url omitted. Brisk 2s tick — fine
        # against the LOCAL relay (public relays would need a slower tick;
        # tick_secs is per-config, default 30s). Bob/Carol get an LTC leg too
        # (own wallet on the LTC node) so they post/serve LTC offers over the
        # relay; a file coin needs --coins-file (coins_file) + the extra --coin.
        bob = Party("bob", h, h.workdir, "bob_pocx", "bob_btc",
                    nostr_relays=relay.ws_url, auto_fund=True, tick_secs=2,
                    coins_file=COINS_TOML,
                    extra_coins=[("ltc", h.ltc.rpc_url(wallet="bob_ltc"))]).start()
        carol = Party("carol", h, h.workdir, "carol_pocx", "carol_btc",
                      nostr_relays=relay.ws_url, auto_fund=True, tick_secs=2,
                      coins_file=COINS_TOML,
                      extra_coins=[("ltc", h.ltc.rpc_url(wallet="carol_ltc"))]).start()

        posted = {"bob": [], "carol": [], "bob_ltc": [], "carol_ltc": []}

        def topup(party, key, offers, pin_proto=None):
            # Non-destructive refresh (see satchel_playground.py): prune ids that
            # have lapsed and refill to target.
            try:
                live = {o["swap_id"] for o in party.rpc("boardlistoffers")["offers"]}
            except Exception:  # noqa: BLE001
                return
            posted[key][:] = [oid for oid in posted[key] if oid in live]
            deficit = len(offers) - len(posted[key])
            for i, (give, get) in enumerate(offers[:max(0, deficit)]):
                proto = pin_proto or PROTOCOLS[i % len(PROTOCOLS)]
                try:
                    # boardpostoffer: give, get, t1_secs, t2_secs, protocol
                    # (ttl omitted → engine default; local relay is ephemeral).
                    r = party.rpc("boardpostoffer", give, get, 4 * 3600, 2 * 3600, proto)
                    posted[key].append(r["offer_id"])
                except Exception as e:  # noqa: BLE001
                    print(f"[nostr-pg] {key} post failed ({give} -> {get}, {proto}): {e}")

        def post_offers():
            topup(bob, "bob", BOB_OFFERS)
            topup(carol, "carol", CAROL_OFFERS)
            # LTC sub-book, pinned to v1 HTLC.
            topup(bob, "bob_ltc", BOB_LTC_OFFERS, pin_proto="pact-htlc-v1")
            topup(carol, "carol_ltc", CAROL_LTC_OFFERS, pin_proto="pact-htlc-v1")
            ltc_live = len(posted["bob_ltc"]) + len(posted["carol_ltc"])
            print(f"[nostr-pg] {len(posted['bob'])} buy-side (Bob) + "
                  f"{len(posted['carol'])} sell-side (Carol) + "
                  f"{ltc_live} LTC offers live on the relay")

        post_offers()

        bar = "=" * 70
        print(f"""
{bar}
  SATCHEL NOSTR (RELAYS-ONLY) PLAYGROUND IS UP   (Ctrl+C to stop)

  No corkboard — Bob, Carol and your Satchel "Alice" trade over ONE local
  Nostr relay only:
    Bob   (:{bob.port}) BUY side — {len(BOB_OFFERS)} give-BTC/get-POCX + {len(BOB_LTC_OFFERS)} give-BTC/get-LTC
    Carol (:{carol.port}) SELL side — {len(CAROL_OFFERS)} give-POCX/get-BTC + {len(CAROL_LTC_OFFERS)} LTC offers
  Relay {relay.ws_url} | POCX :19443 | BTC :19543 | LTC :19643
  Offers use the default TTL; taken offers refill every {REPOST_EVERY_SECS}s.

  In the Satchel window (managed "Alice", relays-only, funded on ALL THREE coins):
    1. Wizard -> Create a merchant.
    2. Coins tab -> BTCX + BTC + LTC connected.
    3. Corkboard tab -> board source is the Nostr relay; two-sided market incl.
       LTC pairs; take any side (give POCX, give BTC, or trade LTC either way).
    4. Swaps tab -> watch it walk to 'completed'.
  Offers may take a few seconds to appear (the relay poll cycle).
{bar}
""")
        start_wall = time.time()
        legs = ((h.pocx, "alice_pocx"), (h.btc, "bob_btc"), (h.ltc, "alice_ltc"))
        base = max(chain_time(n) for n, _ in legs)
        last_post = time.time()
        # Per-tick mining is BEST-EFFORT: a transient node error (e.g. a momentary
        # `bad-txns-vin-empty` on CreateNewBlock) must NOT crash the driver — that
        # would unwind the Harness and tear every node down, leaving Satchel on a
        # dead stack (the spurious coin-setup gate). Each chain advances on its
        # own; failures are logged and skipped, and the next tick retries.
        try:
            while True:
                time.sleep(BLOCK_EVERY_SECS)
                tip = base
                for node, _ in legs:
                    try:
                        tip = max(tip, chain_time(node))
                    except Exception:  # noqa: BLE001
                        pass
                now = max(tip, base + int(time.time() - start_wall)) + 1
                for node, wallet in legs:
                    try:
                        node.set_mocktime(now)
                        node.generate(1, wallet)
                    except Exception as e:  # noqa: BLE001
                        print(f"[nostr-pg] mine skipped ({wallet}): {e}")
                if time.time() - last_post > REPOST_EVERY_SECS:
                    try:
                        post_offers()
                    except Exception as e:  # noqa: BLE001
                        print(f"[nostr-pg] post_offers skipped: {e}")
                    last_post = time.time()
        except KeyboardInterrupt:
            print("\n[nostr-pg] shutting down ...")
        finally:
            bob.stop()
            carol.stop()
            relay.stop()


if __name__ == "__main__":
    main()
