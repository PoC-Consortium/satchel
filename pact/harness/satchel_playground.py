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

import sys
import time

sys.stdout.reconfigure(line_buffering=True)

from regtest_harness import Harness
from test_swap_e2e import build_workspace, Corkboard, Party

BLOCK_EVERY_SECS = 4
REPOST_EVERY_SECS = 60

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


def main():
    build_workspace()
    with Harness(keep=False) as h:
        board = Corkboard(h.workdir)
        board.start()

        # Extra wallets for the two-sided book — created HERE, not in the shared
        # Harness, so the e2e suite's 2-party funding layout stays untouched:
        #   carol_pocx — the SELLER's POCX (she gives POCX); funded.
        #   carol_btc  — receive-only sweep target for Carol's redeemed BTC.
        #   alice_btc  — funded so the tester can take SELL-side offers too
        #                (Alice gives BTC); Harness leaves it empty.
        # Mining their own coinbase avoids draining Alice/Bob; >100 deep matures.
        h.pocx.create_wallet("carol_pocx")
        h.btc.create_wallet("carol_btc")
        h.pocx.generate(110, "carol_pocx")
        h.btc.generate(110, "alice_btc")
        print("[satchel-pg] funded carol_pocx + alice_btc "
              f"(carol_pocx: {h.pocx.rpc('getbalance', wallet='carol_pocx')} POCX, "
              f"alice_btc: {h.btc.rpc('getbalance', wallet='alice_btc')} BTC)")

        bob = Party("bob", h, h.workdir, "bob_pocx", "bob_btc",
                    board_url=board.url, auto_fund=True, tick_secs=5).start()
        carol = Party("carol", h, h.workdir, "carol_pocx", "carol_btc",
                      board_url=board.url, auto_fund=True, tick_secs=5).start()

        posted = {"bob": [], "carol": []}

        def topup(party, key, offers):
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
                # Alternate protocols by slot so both v1 and v2 are always live.
                proto = PROTOCOLS[i % len(PROTOCOLS)]
                try:
                    r = party.rpc("boardpostoffer", give, get, 4 * 3600, 2 * 3600, proto)
                    posted[key].append(r["offer_id"])
                except Exception as e:  # noqa: BLE001
                    print(f"[satchel-pg] {key} post failed ({give} -> {get}, {proto}): {e}")

        def post_offers():
            topup(bob, "bob", BOB_OFFERS)
            topup(carol, "carol", CAROL_OFFERS)
            print(f"[satchel-pg] {len(posted['bob'])} buy-side (Bob) + "
                  f"{len(posted['carol'])} sell-side (Carol) offers live")

        post_offers()

        bar = "=" * 70
        print(f"""
{bar}
  SATCHEL MANAGED PLAYGROUND IS UP   (Ctrl+C to stop)

  Two headless counterparties make a two-sided book:
    Bob   (:{bob.port}) BUY side — {len(BOB_OFFERS)} give-BTC/get-POCX offers
    Carol (:{carol.port}) SELL side — {len(CAROL_OFFERS)} give-POCX/get-BTC offers
  Corkboard {board.url} | POCX node :19443 | BTC node :19543
  Blocks every {BLOCK_EVERY_SECS}s; both top up taken offers every {REPOST_EVERY_SECS}s (live IDs stable).

  In the Satchel window (managed "Alice", funded on BOTH coins):
    1. Wizard -> Create a merchant (write down the mnemonic; pick
       encrypted or not).
    2. Coins tab -> BTCX + BTC should show configured + connected.
    3. Corkboard tab -> two-sided rate-sorted market; take EITHER side:
       a buy-side offer (you give POCX) or a sell-side one (you give BTC).
    4. Swaps tab -> watch it walk to 'completed' on its own.
{bar}
""")
        start_wall = time.time()
        base = max(int(h.pocx.rpc("getblockchaininfo")["time"]),
                   int(h.btc.rpc("getblockchaininfo")["time"]))
        last_post = time.time()
        try:
            while True:
                time.sleep(BLOCK_EVERY_SECS)
                tip = max(int(h.pocx.rpc("getblockchaininfo")["time"]),
                          int(h.btc.rpc("getblockchaininfo")["time"]))
                now = max(tip, base + int(time.time() - start_wall)) + 1
                h.pocx.set_mocktime(now)
                h.btc.set_mocktime(now)
                h.pocx.generate(1, "alice_pocx")
                h.btc.generate(1, "bob_btc")
                if time.time() - last_post > REPOST_EVERY_SECS:
                    post_offers()
                    last_post = time.time()
        except KeyboardInterrupt:
            print("\n[satchel-pg] shutting down ...")
        finally:
            bob.stop()
            carol.stop()
            board.stop()


if __name__ == "__main__":
    main()
