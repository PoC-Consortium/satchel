#!/usr/bin/env python3
"""Observer playground — watch a BACKUP machine follow a main machine's swaps.

A live multi-machine stack for eyeballing the observer experience (#166 →
#175): a MAIN Satchel ("Alice") drives make/take swaps for both protocols,
and a second OBSERVER Satchel on the SAME seed + SAME (seed-derived) wallet
but a distinct machine scope follows them read-only — the dock, the narrate
story and the progress line should track Alice's, step for step, then the
followed row self-purges when the swap settles deep.

Topology (all over one local Nostr relay, NODELESS so both coins are Electrum
— history-classifiable regardless of who redeems, so every swap RESOLVES for
the observer instead of aging out on a Core-only leg):

  * regtest PoCX + BTC nodes, each with an electrs; a local Nostr relay.
  * Bob + Carol — node-backed counterparties posting a two-sided book AND
    auto-taking Alice's own offers (so Alice can MAKE, not just take).
  * Alice (MAIN) — a managed Satchel on :9739, seeded with FOLLOW_MNEMONIC,
    nodeless btcx/btc wallets on that seed.
  * Alice-observer (OBSERVER) — a second managed Satchel on :9740 launched
    with SATCHEL_DATA_DIR pointed at its own dir → its own machine.json scope
    → Alice's swaps read as `source=foreign` and it follows them. Same seed
    (so same npub → it sees Alice's encrypted-to-self snapshots) and the same
    Electrum coins (same seed → same wallet → shared).

This PYTHON side owns the nodes/relay/electrs/bots/mining + seeding both
Satchels' pactd and the auto-taker. The `.ps1` wrapper launches the two
Satchel GUIs. Paced mining (per-coin block cadence) + mainnet-like confs keep
each state on screen long enough to watch on both machines.
"""

import os

os.environ.setdefault("PACT_DISABLE_KEYRING", "1")  # regtest seeds → obfs wrap (#120)
import time  # noqa: E402

import sys  # noqa: E402

# Force UTF-8: prints below contain non-ASCII (arrows/ellipses). When Satchel's
# launcher redirects this driver's stdout to a file, Windows would otherwise use
# the locale codepage (cp1252) and raise UnicodeEncodeError mid-run, killing it.
sys.stdout.reconfigure(encoding="utf-8", line_buffering=True)

from framework.daemon import Party  # noqa: E402
from framework.services import PLAYGROUND_NOSTR_RELAY_PORT, NostrRelay  # noqa: E402
from framework.stack import COINS_TOML, build_workspace  # noqa: E402
from framework.util import pactd_rpc_or_none  # noqa: E402
from regtest_harness import (  # noqa: E402
    BTC_ELECTRS_ELECTRUM_PORT,
    BTC_ELECTRS_MONITORING_PORT,
    ElectrsServer,
    Harness,
    find_btc_electrs,
)

# Alice's (and the observer's) shared Pact seed. A CHECKSUM-VALID BIP39 test
# vector so both Satchels import the SAME identity with no copy-paste. Local
# ephemeral relay → no cross-talk with other playgrounds' seeds. NOT for funds.
FOLLOW_MNEMONIC = (
    "legal winner thank year wave sausage worth useful legal winner thank yellow"
)

# Managed-Satchel pactd ports (regtest offset): MAIN 9739, OBSERVER 9740.
ALICE_PORT = 9739
OBSERVER_PORT = 9740

# Starter balances for Alice's nodeless (seed) wallets — plenty for a session
# of make/take on both legs. The OBSERVER shares these (same seed → same
# addresses), it just never spends them.
FAUCET_BTCX = 200.0
FAUCET_BTC = 0.05

# Paced mining: per-coin block cadence (seconds) + granularity, mirroring the
# nostr playground but a touch slower so the confirmation count is comfortable
# to watch tick up on BOTH machines. Mainnet-like depths (not regtest's 1).
BLOCK_SECS = {"btcx": 8, "btc": 12}
BASE_BLOCK_SECS = 4
PLAYGROUND_CONFS = {"btcx": 6, "btc": 4}
REPOST_EVERY_SECS = 30

# Both protocols get offers so you can take v1 AND v2; the auto-taker pins
# Alice's own offers to whatever protocol Alice chose (it just takes them).
PROTOCOLS = ["pact-htlc-v1", "pact-htlc-v2"]

# A small two-sided book (btcx<->btc only — LTC omitted to keep the observer
# demo focused). Bob BUYS btcx (gives btc), Carol SELLS btcx (gives btcx).
BOB_OFFERS = [
    ("btc:0.001", "btcx:47"),
    ("btc:0.0015", "btcx:72"),
    ("btc:0.002", "btcx:102"),
]
CAROL_OFFERS = [
    ("btcx:50", "btc:0.00104"),
    ("btcx:75", "btc:0.00156"),
    ("btcx:100", "btc:0.00196"),
]


def rpc(port, method, *params, timeout=30):
    """One JSON-RPC call to a managed Satchel pactd (cookie auth from its data
    dir). Returns None on any error so the driver loop never crashes on a
    not-yet-up daemon. The config dirs match playground-observer.ps1: MAIN
    under org.pocx.satchel, OBSERVER under org.pocx.satchel-observer (its
    SATCHEL_DATA_DIR), each nesting <base>/regtest/pactd."""
    base = "org.pocx.satchel" if port == ALICE_PORT else "org.pocx.satchel-observer"
    cookie_path = os.path.join(
        os.environ["LOCALAPPDATA"], base, "regtest", "pactd", ".cookie"
    )
    return pactd_rpc_or_none(f"http://127.0.0.1:{port}/", method, *params,
                             cookie_path=cookie_path, timeout=timeout)


def chain_time(node):
    info = node.rpc("getblockchaininfo")
    return int(info.get("time", info["mediantime"]))


def main():
    build_workspace()
    with Harness(keep=False, pocx_rest=True, btc_rest=True) as h:
        # electrs for BOTH coins → both are Electrum (nodeless), so every leg
        # is history-classifiable and the observer resolves whichever side
        # redeems.
        pocx_electrs = ElectrsServer(h.workdir, h.pocx)
        pocx_electrs.start()
        pocx_electrs.wait_synced(h.pocx.rpc("getblockcount"))
        btc_electrs = ElectrsServer(
            h.workdir,
            h.btc,
            electrum_port=BTC_ELECTRS_ELECTRUM_PORT,
            monitoring_port=BTC_ELECTRS_MONITORING_PORT,
            network="testnet",
            binary=find_btc_electrs(),
            name="btc-electrs",
        )
        btc_electrs.start()
        btc_electrs.wait_synced(h.btc.rpc("getblockcount"))
        print(f"[obs-pg] electrs up: btcx {pocx_electrs.url} | btc {btc_electrs.url}")

        relay = NostrRelay(h.workdir, port=PLAYGROUND_NOSTR_RELAY_PORT,
                           name="pact-playground")
        relay.start()

        # Bob/Carol counterparties (node wallets, node-backed). They post a book
        # AND auto-take Alice's own offers.
        h.pocx.create_wallet("carol_pocx")
        h.btc.create_wallet("carol_btc")
        h.pocx.generate(110, "carol_pocx")
        h.pocx.generate(110, "bob_pocx")
        h.btc.generate(110, "alice_btc")  # spare
        bob = Party(
            "bob",
            h,
            h.workdir,
            "bob_pocx",
            "bob_btc",
            nostr_relays=relay.ws_url,
            auto_fund=True,
            tick_secs=2,
            coins_file=COINS_TOML,
            coin_confs=PLAYGROUND_CONFS,
        ).start()
        carol = Party(
            "carol",
            h,
            h.workdir,
            "carol_pocx",
            "carol_btc",
            nostr_relays=relay.ws_url,
            auto_fund=True,
            tick_secs=2,
            coins_file=COINS_TOML,
            coin_confs=PLAYGROUND_CONFS,
        ).start()

        posted = {"bob": [], "carol": []}

        def topup(party, key, offers):
            try:
                live = {o["swap_id"] for o in party.rpc("boardlistoffers")["offers"]}
            except Exception:  # noqa: BLE001
                return
            posted[key][:] = [oid for oid in posted[key] if oid in live]
            for i, (give, get) in enumerate(offers[len(posted[key]) :], len(posted[key])):
                try:
                    r = party.rpc(
                        "boardpostoffer", give, get, 4 * 3600, 2 * 3600,
                        PROTOCOLS[i % len(PROTOCOLS)],
                    )
                    posted[key].append(r["offer_id"])
                except Exception as e:  # noqa: BLE001
                    print(f"[obs-pg] {key} post failed: {e}")

        def post_offers():
            topup(bob, "bob", BOB_OFFERS)
            topup(carol, "carol", CAROL_OFFERS)

        post_offers()

        print(
            "\n"
            + "=" * 70
            + "\n  OBSERVER PLAYGROUND STACK IS UP   (nodes + electrs + relay + bots)\n\n"
            f"  Relay {relay.ws_url} | electrs btcx {pocx_electrs.url} | btc {btc_electrs.url}\n"
            f"  Shared seed (Alice + observer): {FOLLOW_MNEMONIC}\n"
            "  The .ps1 wrapper now launches the two Satchel windows.\n"
            + "=" * 70
            + "\n"
        )

        # Track which of Alice's offers we've already auto-taken (so a bot
        # takes each Alice-made offer exactly once). Alice's identity is read
        # from her pactd once she is seeded.
        alice_id = None
        taken_alice_offers = set()
        alice_seeded = observer_seeded = alice_funded = False

        legs = [(h.pocx, "carol_pocx", "btcx"), (h.btc, "bob_btc", "btc")]
        start_wall = time.time()
        elapsed = 0
        last_post = time.time()
        base = max(chain_time(n) for n, _, _ in legs)
        try:
            while True:
                time.sleep(BASE_BLOCK_SECS)
                elapsed += BASE_BLOCK_SECS
                tip = base
                for node, _, _ in legs:
                    try:
                        tip = max(tip, chain_time(node))
                    except Exception:  # noqa: BLE001
                        pass
                now = max(tip, base + int(time.time() - start_wall)) + 1
                for node, wallet, coin in legs:
                    try:
                        node.set_mocktime(now)
                        if elapsed % BLOCK_SECS[coin] == 0:
                            node.generate(1, wallet)
                    except Exception as e:  # noqa: BLE001
                        print(f"[obs-pg] mine skipped ({wallet}): {e}")

                # Onboarding is USER-DRIVEN (the Satchel wizard runs its own
                # merchant+seed flow; an RPC importseed behind its back does not
                # reflect in the UI). We just WATCH for each window to be seeded
                # (identity present) — you import FOLLOW_MNEMONIC in BOTH wizards.
                if not alice_seeded:
                    info = rpc(ALICE_PORT, "getinfo")
                    if info and info.get("identity"):
                        alice_seeded = True
                        print("[obs-pg] Alice (MAIN :9739) seeded by the wizard")
                if not observer_seeded:
                    info = rpc(OBSERVER_PORT, "getinfo")
                    if info and info.get("identity"):
                        observer_seeded = True
                        print("[obs-pg] observer (:9740) seeded — now following Alice")

                # Fund Alice's nodeless (seed) wallets so she can fund swaps;
                # the observer shares them (same seed) but never spends.
                if alice_seeded and not alice_funded:
                    ok = True
                    for coin, node, wallet, amount in (
                        ("btcx", h.pocx, "carol_pocx", FAUCET_BTCX),
                        ("btc", h.btc, "bob_btc", FAUCET_BTC),
                    ):
                        bal = rpc(ALICE_PORT, "getbalance", coin)
                        if bal is None:
                            ok = False
                            continue
                        if bal.get("balance_sat", 0) > 0:
                            continue
                        addr = rpc(ALICE_PORT, "getnewaddress", coin)
                        if addr is None:
                            ok = False
                            continue
                        node.rpc("sendtoaddress", addr["address"], amount, wallet=wallet)
                        print(f"[obs-pg] faucet: {amount} {coin} → Alice ({addr['address'][:20]}…)")
                    alice_funded = ok

                # Read Alice's identity once, then auto-take HER offers so the
                # MAKE path works (a bot funds the counterparty leg).
                if alice_id is None and alice_seeded:
                    info = rpc(ALICE_PORT, "getinfo")
                    alice_id = info.get("identity") if info else None
                if alice_id:
                    try:
                        board = carol.rpc("boardlistoffers")["offers"]
                    except Exception:  # noqa: BLE001
                        board = []
                    for o in board:
                        if o.get("from") == alice_id and o["swap_id"] not in taken_alice_offers:
                            taken_alice_offers.add(o["swap_id"])
                            # Carol takes it (she holds both coins); she funds
                            # whichever leg the offer's `get` side names.
                            try:
                                carol.rpc("boardtake", o["swap_id"])
                                print(f"[obs-pg] Carol auto-took Alice's offer {o['swap_id'][:12]}")
                            except Exception as e:  # noqa: BLE001
                                print(f"[obs-pg] auto-take failed: {e}")

                if time.time() - last_post > REPOST_EVERY_SECS:
                    try:
                        post_offers()
                    except Exception as e:  # noqa: BLE001
                        print(f"[obs-pg] post_offers skipped: {e}")
                    last_post = time.time()
        except KeyboardInterrupt:
            print("\n[obs-pg] shutting down …")
        finally:
            bob.stop()
            carol.stop()
            relay.stop()
            pocx_electrs.stop()
            btc_electrs.stop()


if __name__ == "__main__":
    main()
