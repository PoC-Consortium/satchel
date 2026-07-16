#!/usr/bin/env python3
"""Pact playground: the full stack on regtest, for clicking around.

Starts: PoCX node + BTC node (funded wallets), a Corkboard, and two
pactds with the UI — Alice (has 100+ POCX) and Bob (has 500+ BTC).
Mines a block on each chain every few seconds and advances the mock
clocks with wall time, so confirmations, the scheduler, and even
timelock expiries behave like a tiny live network.

    python playground.py

Then open the two printed URLs, paste each token, and:
  1. Alice  -> Corkboard tab -> post an offer (defaults are fine)
  2. Bob    -> Corkboard tab -> take it
  3. Watch the Swaps tab on both sides — the schedulers do the rest.

Tip: post an offer with T1=0.2 / T2=0.1 (hours) and DON'T take it from
Bob until it expires, or take it and stop one side, to watch refunds.

Ctrl+C stops and cleans up everything.
"""

import sys
import time

# Keep output visible even when stdout is piped (background runs).
sys.stdout.reconfigure(line_buffering=True)

from framework.daemon import Party
from framework.services import Corkboard
from framework.stack import build_workspace
from regtest_harness import Harness

BLOCK_EVERY_SECS = 4


def main():
    build_workspace()
    with Harness(keep=False) as h:
        board = Corkboard(h.workdir)
        board.start()
        # Hands-off: scheduler every 5s; auto_fund is a playground/demo
        # convenience — production pactds default to manual funding. Each
        # party's pactd auto-inits its seed on start.
        alice = Party("alice", h, h.workdir, "alice_pocx", "alice_btc",
                      board_url=board.url, auto_fund=True, tick_secs=5).start()
        bob = Party("bob", h, h.workdir, "bob_pocx", "bob_btc",
                    board_url=board.url, auto_fund=True, tick_secs=5).start()

        bar = "=" * 68
        # pactd is JSON-RPC now (no web UI — that returns in Satchel,
        # Phase 3). Drive it with pact-cli:
        #   pact-cli --rpc http://127.0.0.1:PORT --data-dir DIR getinfo
        #   pact-cli ... board post --give btcx:1.0 --get btc:0.0001
        #   pact-cli ... board sync        (one scheduler pass)
        print(f"""
{bar}
  PACT PLAYGROUND IS UP   (Ctrl+C to stop)

  Alice (maker, has POCX):  JSON-RPC http://127.0.0.1:{alice.port}/
    --data-dir {alice.data_dir}

  Bob   (taker, has BTC):   JSON-RPC http://127.0.0.1:{bob.port}/
    --data-dir {bob.data_dir}

  Corkboard: {board.url}   |  blocks every {BLOCK_EVERY_SECS}s on both chains
  (GUI returns in Satchel — Phase 3; the daemons auto-fund/redeem on their
   own, so a board-driven swap completes here without any clicking.)
{bar}
""")

        start_wall = time.time()
        base_chain = max(int(h.pocx.rpc("getblockchaininfo")["time"]),
                         int(h.btc.rpc("getblockchaininfo")["time"]))
        try:
            while True:
                time.sleep(BLOCK_EVERY_SECS)
                # Chain clocks follow wall time so timelocks really mature —
                # but never move backwards: PoCX forging may have advanced
                # the mock clock beyond wall pace on its own.
                tip = max(int(h.pocx.rpc("getblockchaininfo")["time"]),
                          int(h.btc.rpc("getblockchaininfo")["time"]))
                now = max(tip, base_chain + int(time.time() - start_wall)) + 1
                h.pocx.set_mocktime(now)
                h.btc.set_mocktime(now)
                h.pocx.generate(1, "alice_pocx")
                h.btc.generate(1, "bob_btc")
        except KeyboardInterrupt:
            print("\n[playground] shutting down ...")
        finally:
            alice.stop()
            bob.stop()
            board.stop()


if __name__ == "__main__":
    main()
