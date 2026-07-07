#!/usr/bin/env python3
"""Managed-Satchel playground — a two-sided market for click-testing.

Brings up regtest nodes + a Corkboard + TWO headless counterparties, so the
Corkboard shows a real two-sided book and you (Alice, in Satchel) can take
EITHER side:

  * Bob   — the BUY side: gives BTC, wants POCX  (give-BTC/get-POCX). Funded
            with BTC. Taking his offer, you give POCX.
  * Carol — the SELL side: gives POCX, wants BTC (give-POCX/get-BTC). Funded
            with POCX. Taking her offer, you give BTC.

Bob and Carol never interact (opposite directions; the board never matches).
Each posts a varied spread so each side of the book is rate-sorted.

You run ONE Satchel in MANAGED mode as "Alice" (Satchel owns its own pactd):
go through the merchant wizard, confirm the Coins tab, then take a buy-side
*or* sell-side offer and watch it auto-complete on the Swaps tab. Alice needs
BOTH coins, so this script funds alice_btc too (the shared Harness leaves it
empty; we top it up here without touching the e2e suite's funding layout).

All parties run auto-fund + a 5s scheduler, so once Alice takes, the swap
completes hands-off. The agent pre-seeds Alice's satchel.json and launches
Satchel separately; this script runs the infra + Bob + Carol and keeps mining
so confirmations and timelocks advance.
"""

import base64
import json
import os
# Regtest seeds take the obfuscation wrap (#120), off the dev keychain.
os.environ.setdefault("PACT_DISABLE_KEYRING", "1")
import sys
import time
import urllib.request

sys.stdout.reconfigure(line_buffering=True)

from regtest_harness import (
    ElectrsServer, Harness, ELECTRS_ELECTRUM_PORT, ELECTRS_MONITORING_PORT)
from test_swap_e2e import build_workspace, Corkboard, Party, COINS_TOML

# Timing model mirrored from the nostr playground: per-chain block cadence at
# mainnet RATIOS scaled ~20x (fast btcx, slower btc/ltc) instead of a uniform
# instant tick, plus mainnet-like confirmation depths — so fee bumping, the
# multi-tick-per-block window and the Satchel progress display are exercised
# realistically. e2e suites are unaffected (they pass no coin_confs).
BLOCK_SECS = {"btcx": 6, "btc": 12, "ltc": 12}
BASE_BLOCK_SECS = 6  # miner granularity = the fastest chain's interval
PLAYGROUND_CONFS = {"btcx": 10, "btc": 6, "ltc": 6}
REPOST_EVERY_SECS = 30

# --nodeless (playground-nodeless.ps1): Alice's btcx becomes the pact-seed bdk
# wallet over a live electrs (epic #58) — PoCX node on :18443 (+REST), electrs
# leg, and a faucet for her wizard-created wallet. Default (playground-cork.ps1)
# is the classic all-Core layout on :19443.
NODELESS = "--nodeless" in sys.argv[1:]


def _arg_val(flag, default):
    """Read `--flag VALUE` out of argv (VALUE follows the flag)."""
    a = sys.argv[1:]
    return a[a.index(flag) + 1] if flag in a and a.index(flag) + 1 < len(a) else default


# --electrs-count N (nodeless only, default 1): stand up N INDEPENDENT electrs
# instances over the SAME regtest PoCX node, so the Electrum active-set /
# failover path (issue #98 and its follow-ups) can be exercised for real —
# kill one server and watch a standby get promoted, or wire a dead endpoint and
# confirm the coin stays green. Electrum/monitoring ports step by 2 from the
# base (19750/1, 19752/3, 19754/5, …), staying below the BTC electrs at 19760.
ELECTRS_COUNT = int(_arg_val("--electrs-count", "1")) if NODELESS else 1

# Alice's managed pactd (Satchel regtest offset) + its cookie, for the faucet.
ALICE_RPC = "http://127.0.0.1:9739/"
ALICE_COOKIE = os.path.join(
    os.environ.get("LOCALAPPDATA", ""), "org.pocx.satchel", "regtest", "pactd", ".cookie")
FAUCET_BTCX = 100.0


def alice_rpc(method, *params):
    with open(ALICE_COOKIE, encoding="utf-8") as fh:
        cookie = fh.read().strip()
    body = json.dumps(
        {"jsonrpc": "2.0", "id": "pg", "method": method, "params": list(params)}).encode()
    req = urllib.request.Request(ALICE_RPC, data=body, method="POST")
    req.add_header("Content-Type", "application/json")
    req.add_header("Authorization", "Basic " + base64.b64encode(cookie.encode()).decode())
    with urllib.request.urlopen(req, timeout=10) as resp:
        data = json.loads(resp.read())
    if data.get("error"):
        raise RuntimeError(data["error"]["message"])
    return data["result"]


def faucet_alice_btcx(h):
    """Alice's btcx is NODELESS (epic #58): her wallet is the bdk one on the
    seed she creates in the wizard, so it cannot be pre-funded like a node
    wallet. Poll until her pactd serves a wallet (merchant created + unlocked),
    then send a starter balance once. Returns True when done."""
    try:
        if alice_rpc("getbalance", "btcx")["balance_sat"] > 0:
            return True
        addr = alice_rpc("getnewaddress", "btcx")["address"]
        h.pocx.rpc("sendtoaddress", addr, FAUCET_BTCX, wallet="alice_pocx")
        print(f"[satchel-pg] faucet: {FAUCET_BTCX} BTCX -> Alice's nodeless "
              f"wallet ({addr[:24]}…) — confirms next block")
        return True
    except Exception:  # noqa: BLE001 — not up yet / wizard pending: retry
        return False

# Each side posts a SPREAD at varied sizes + implied rates so the book is
# rate-sorted, not one card repeated. Both sides hover ~47k–51k POCX/BTC so
# they read as a coherent market. Offer strings use the wire coin_id ("btcx",
# "btc"); the UI shows the symbols (BTCX / BTC). Every offer is completable:
# the maker funds what it GIVES, the taker funds what it gives in return.
#
# Each offer also alternates its swap protocol (v1 HTLC ↔ v2 Taproot/MuSig2
# adaptor) so the book carries BOTH routes and you can click-test either. The
# suite defaults to HTLC; v2 is pinned explicitly here.
PROTOCOLS = ["pact-htlc-v1", "pact-htlc-v2"]

# Bob = BUY side: gives BTC, wants POCX (funded with BTC). Taker gives POCX.
BOB_OFFERS = [
    ("btc:0.0005", "btcx:24"),    # 1 BTC = 48,000 POCX  (small)
    ("btc:0.001",  "btcx:47"),    #         47,000       (cheap for taker)
    ("btc:0.001",  "btcx:50"),    #         50,000       (canonical)
    ("btc:0.0015", "btcx:72"),    #         48,000
    ("btc:0.002",  "btcx:102"),   #         51,000
    ("btc:0.003",  "btcx:153"),   #         51,000       (large)
]

# Carol = SELL side: gives POCX, wants BTC (funded with POCX). Taker gives BTC.
CAROL_OFFERS = [
    ("btcx:25",  "btc:0.0005"),   # 1 BTC = 50,000 POCX
    ("btcx:50",  "btc:0.00104"),  #         ~48,000      (asks a bit more BTC)
    ("btcx:50",  "btc:0.00098"),  #         ~51,000
    ("btcx:75",  "btc:0.00156"),  #         ~48,000
    ("btcx:100", "btc:0.00196"),  #         ~51,000
]

# Litecoin sub-book — a two-sided spread on BOTH LTC pairs (BTC<->LTC and
# BTCX<->LTC) so those boards aren't near-empty next to BTCX<->BTC. Mirrors the
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


def chain_time(node):
    # Tip block time, used to keep mocktime monotonic across all three chains.
    # litecoind is an older Bitcoin Core fork whose getblockchaininfo has no
    # "time" field (pocx/btc on Core v30 do) — fall back to "mediantime", which
    # every version reports.
    info = node.rpc("getblockchaininfo")
    return int(info.get("time", info["mediantime"]))


def main():
    build_workspace()
    # pocx_rest (nodeless only): bindex (electrs' indexer) hardcodes the node's
    # REST endpoint at regtest-default :18443.
    with Harness(keep=False, with_ltc=True, pocx_rest=NODELESS) as h:
        board = Corkboard(h.workdir)
        board.start()
        electrs = None
        electrs_all = []
        if NODELESS:
            want_h = h.pocx.rpc("getblockcount")
            for i in range(ELECTRS_COUNT):
                e = ElectrsServer(
                    h.workdir, h.pocx,
                    electrum_port=ELECTRS_ELECTRUM_PORT + 2 * i,
                    monitoring_port=ELECTRS_MONITORING_PORT + 2 * i,
                    name="electrs" if i == 0 else f"electrs{i + 1}")
                e.start()
                e.wait_synced(want_h)
                electrs_all.append(e)
            electrs = electrs_all[0]  # home / wallet leg
            if len(electrs_all) == 1:
                print(f"[satchel-pg] electrs up on {electrs.url} (Alice's nodeless btcx)")
            else:
                print(f"[satchel-pg] {len(electrs_all)} electrs up: "
                      f"{', '.join(e.url for e in electrs_all)} "
                      "(Alice's nodeless btcx active-set fleet)")

        # Extra wallets for the two-sided book — created HERE, not in the shared
        # Harness, so the e2e suite's 2-party funding layout stays untouched:
        #   carol_pocx — the SELLER's POCX (she gives POCX); funded.
        #   carol_btc  — receive-only sweep target for Carol's redeemed BTC.
        #   alice_btc  — funded so the tester can take SELL-side offers too
        #                (Alice gives BTC); Harness leaves it empty.
        # Mining their own coinbase avoids draining Alice/Bob; >100 deep matures.
        # bob_pocx is funded too (the harness leaves it empty) so Bob can GIVE
        # BTCX on the BTCX<->LTC board.
        h.pocx.create_wallet("carol_pocx")
        h.btc.create_wallet("carol_btc")
        h.pocx.generate(110, "carol_pocx")
        h.pocx.generate(110, "bob_pocx")
        h.btc.generate(110, "alice_btc")

        # Litecoin leg. alice_ltc + carol_ltc are funded (each gives LTC on some
        # offer); bob_ltc is a receive-only sweep target. Litecoin coinbase
        # matures at 100, so 110 deep is spendable.
        h.ltc.create_wallet("alice_ltc")
        h.ltc.create_wallet("bob_ltc")
        h.ltc.create_wallet("carol_ltc")
        h.ltc.generate(110, "alice_ltc")
        h.ltc.generate(110, "carol_ltc")
        print("[satchel-pg] funded carol_pocx + alice_btc + alice_ltc/carol_ltc "
              f"(carol_pocx: {h.pocx.rpc('getbalance', wallet='carol_pocx')} POCX, "
              f"alice_btc: {h.btc.rpc('getbalance', wallet='alice_btc')} BTC, "
              f"alice_ltc: {h.ltc.rpc('getbalance', wallet='alice_ltc')} LTC)")

        # Bob/Carol get an LTC leg too (their own wallet on the LTC node) so they
        # can post and serve LTC offers. A file coin needs --coins-file, passed
        # via coins_file; the leg itself is a generic extra --coin.
        bob = Party("bob", h, h.workdir, "bob_pocx", "bob_btc",
                    board_url=board.url, auto_fund=True, tick_secs=2,
                    coins_file=COINS_TOML, coin_confs=PLAYGROUND_CONFS,
                    extra_coins=[("ltc", h.ltc.rpc_url(wallet="bob_ltc"))]).start()
        carol = Party("carol", h, h.workdir, "carol_pocx", "carol_btc",
                      board_url=board.url, auto_fund=True, tick_secs=2,
                      coins_file=COINS_TOML, coin_confs=PLAYGROUND_CONFS,
                      extra_coins=[("ltc", h.ltc.rpc_url(wallet="carol_ltc"))]).start()

        posted = {"bob": [], "carol": [], "bob_ltc": [], "carol_ltc": []}

        def topup(party, key, offers, pin_proto=None):
            # NON-destructive refresh: never revoke a live offer (that would
            # churn offer IDs and make the taker's click race a revoke — the
            # "offer not on any configured board" failure). Only PRUNE ids that
            # are already gone (taken → C5 auto-revoke, or expired) and refill
            # back up to the target count with fresh offers. Live offers keep
            # their IDs, so a take never races a repost.
            try:
                live = {o["swap_id"] for o in party.rpc("boardlistoffers")["offers"]}
            except Exception:  # noqa: BLE001
                return
            posted[key][:] = [oid for oid in posted[key] if oid in live]
            deficit = len(offers) - len(posted[key])
            for i, (give, get) in enumerate(offers[:max(0, deficit)]):
                # Alternate protocols by slot so both v1 and v2 are always live,
                # unless the caller pins one (LTC offers pin v1 HTLC).
                proto = pin_proto or PROTOCOLS[i % len(PROTOCOLS)]
                try:
                    r = party.rpc("boardpostoffer", give, get, 4 * 3600, 2 * 3600, proto)
                    posted[key].append(r["offer_id"])
                except Exception as e:  # noqa: BLE001
                    print(f"[satchel-pg] {key} post failed ({give} -> {get}, {proto}): {e}")

        def post_offers():
            topup(bob, "bob", BOB_OFFERS)
            topup(carol, "carol", CAROL_OFFERS)
            # LTC sub-book, pinned to v1 HTLC.
            topup(bob, "bob_ltc", BOB_LTC_OFFERS, pin_proto="pact-htlc-v1")
            topup(carol, "carol_ltc", CAROL_LTC_OFFERS, pin_proto="pact-htlc-v1")
            ltc_live = len(posted["bob_ltc"]) + len(posted["carol_ltc"])
            print(f"[satchel-pg] {len(posted['bob'])} buy-side (Bob) + "
                  f"{len(posted['carol'])} sell-side (Carol) + "
                  f"{ltc_live} LTC offers live")

        post_offers()

        bar = "=" * 70
        print(f"""
{bar}
  SATCHEL MANAGED PLAYGROUND IS UP   (Ctrl+C to stop)

  Two headless counterparties make a two-sided book + an LTC sub-book:
    Bob   (:{bob.port}) BUY side — {len(BOB_OFFERS)} give-BTC/get-POCX + {len(BOB_LTC_OFFERS)} give-BTC/get-LTC
    Carol (:{carol.port}) SELL side — {len(CAROL_OFFERS)} give-POCX/get-BTC + {len(CAROL_LTC_OFFERS)} LTC offers
  Corkboard {board.url} | POCX :{h.pocx.rpc_port}{" (+REST)" if NODELESS else ""} | BTC :19543 | LTC :19643{f'''
  electrs {electrs.url} — Alice's BTCX is NODELESS (pact-seed bdk wallet)''' if NODELESS else ""}
  Blocks: btcx every {BLOCK_SECS["btcx"]}s, btc/ltc every {BLOCK_SECS["btc"]}s (mainnet ratios, ~20x);
  confirmations btcx {PLAYGROUND_CONFS["btcx"]} / btc {PLAYGROUND_CONFS["btc"]} / ltc {PLAYGROUND_CONFS["ltc"]}; taken offers refill every {REPOST_EVERY_SECS}s.

  In the Satchel window (managed "Alice"):
    1. Wizard -> Create a merchant (write down the mnemonic; pick
       encrypted or not).{f'''
    2. Coins tab -> BTCX shows "pact seed wallet" (nodeless via electrs);
       BTC + LTC are node-backed as before.
    3. Wallets tab -> the BTCX card has Receive / Send / Activity; a
       faucet drops {FAUCET_BTCX} BTCX into your wallet right after the
       wizard (watch the balance appear).
    4. Corkboard tab -> two-sided market incl. LTC pairs; take any side.
    5. Swaps tab -> watch it walk to 'completed' on its own — BTCX legs
       fund straight from your pact-seed wallet.''' if NODELESS else '''
    2. Coins tab -> BTCX + BTC + LTC should show configured + connected.
    3. Corkboard tab -> two-sided market incl. LTC pairs; take any side:
       give POCX, give BTC, or trade LTC either way.
    4. Swaps tab -> watch it walk to 'completed' on its own.'''}
{bar}
""")
        start_wall = time.time()
        legs = ((h.pocx, "alice_pocx", "btcx"), (h.btc, "bob_btc", "btc"),
                (h.ltc, "alice_ltc", "ltc"))
        base = max(chain_time(n) for n, _, _ in legs)
        last_post = time.time()
        alice_funded = False
        elapsed = 0
        # Per-tick mining is BEST-EFFORT: a transient node error (e.g. a momentary
        # `bad-txns-vin-empty` on CreateNewBlock) must NOT crash the driver — that
        # would unwind the Harness and tear every node down, leaving Satchel on a
        # dead stack (the spurious coin-setup gate). Each chain advances on its
        # own; failures are logged and skipped, and the next tick retries.
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
                # Advance every chain's clock each tick (keeps timelocks moving),
                # but only mine when this chain's own cadence is due — several
                # scheduler ticks per block, like mainnet, not instant finality.
                for node, wallet, coin in legs:
                    try:
                        node.set_mocktime(now)
                        if elapsed % BLOCK_SECS[coin] == 0:
                            node.generate(1, wallet)
                    except Exception as e:  # noqa: BLE001
                        print(f"[satchel-pg] mine skipped ({wallet}): {e}")
                if NODELESS and not alice_funded:
                    alice_funded = faucet_alice_btcx(h)
                if time.time() - last_post > REPOST_EVERY_SECS:
                    try:
                        post_offers()
                    except Exception as e:  # noqa: BLE001
                        print(f"[satchel-pg] post_offers skipped: {e}")
                    last_post = time.time()
        except KeyboardInterrupt:
            print("\n[satchel-pg] shutting down ...")
        finally:
            bob.stop()
            carol.stop()
            board.stop()
            for e in electrs_all:
                e.stop()


if __name__ == "__main__":
    main()
