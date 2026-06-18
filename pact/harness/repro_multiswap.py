#!/usr/bin/env python3
"""Repro for C13 (ASYNC): mirrors the live playground — real self-scheduler
(tick_secs) on each party + a SEPARATE async block-miner — then one taker
takes 3 offers back-to-back with no wait. The synchronous tick->mine->tick
harness can't reproduce the live -25 (bad-txns-inputs-missingorspent); this
async setup should, because the scheduler and miner interleave.

Run: python repro_multiswap.py
"""
import sys
import threading
import time

sys.stdout.reconfigure(line_buffering=True)

from regtest_harness import Harness
from test_swap_e2e import build_workspace, Corkboard, Party

OFFERS = [("btcx:5", "btc:0.001"), ("btcx:6", "btc:0.001"), ("btcx:7", "btc:0.001")]
BLOCK_EVERY = 4
TICK_SECS = 5
WATCH_SECS = 150


def main():
    build_workspace()
    with Harness(keep=False) as h:
        board = Corkboard(h.workdir)
        board.start()
        maker = Party("mk", h, h.workdir, "alice_pocx", "alice_btc",
                      board_url=board.url, auto_fund=True, tick_secs=TICK_SECS).start()
        taker = Party("tk", h, h.workdir, "bob_pocx", "bob_btc",
                      board_url=board.url, auto_fund=True, tick_secs=TICK_SECS).start()

        stop = threading.Event()
        start_wall = time.time()
        base = max(int(h.pocx.rpc("getblockchaininfo")["time"]),
                   int(h.btc.rpc("getblockchaininfo")["time"]))

        def miner():
            while not stop.is_set():
                tip = max(int(h.pocx.rpc("getblockchaininfo")["time"]),
                          int(h.btc.rpc("getblockchaininfo")["time"]))
                now = max(tip, base + int(time.time() - start_wall)) + 1
                try:
                    h.pocx.set_mocktime(now); h.btc.set_mocktime(now)
                    h.pocx.generate(1, "alice_pocx"); h.btc.generate(1, "bob_btc")
                except Exception as e:  # noqa: BLE001
                    print(f"[repro] miner: {e}")
                stop.wait(BLOCK_EVERY)

        mt = threading.Thread(target=miner, daemon=True); mt.start()

        try:
            for give, get in OFFERS:
                maker.rpc("boardpostoffer", give, get, 4 * 3600, 2 * 3600)
            offers = [o["swap_id"] for o in taker.rpc("boardlistoffers")["offers"]]
            print(f"[repro] {len(offers)} offers up; taker takes ALL back-to-back (no wait)")
            for oid in offers:
                taker.rpc("boardtake", oid)   # no wait, no tick between

            # Let the self-schedulers drive; poll states.
            deadline = time.time() + WATCH_SECS
            while time.time() < deadline:
                time.sleep(TICK_SECS)
                sa = maker.rpc("listswaps"); st = taker.rpc("listswaps")
                ca = sorted(s["state"] for s in sa); ct = sorted(s["state"] for s in st)
                print(f"[repro] t+{int(time.time()-start_wall):3}s maker={ca} taker={ct}")
                if (len(sa) == len(OFFERS) and len(st) == len(OFFERS)
                        and all(s["state"] == "completed" for s in sa)
                        and all(s["state"] == "completed" for s in st)):
                    print(f"[repro] ALL {len(OFFERS)} swaps completed — not reproduced (good)")
                    return
            print("\n[repro] ===== TIMEOUT: not all completed — likely C13 =====")
            # Capture the error detail by driving one explicit tick each.
            for who in (maker, taker):
                for ev in who.rpc("tick")["events"]:
                    if ev["action"] == "error":
                        print(f"[repro]   {who.name} ERROR {ev['swap_id'][:8]}: {ev['detail']}")
            for who, label in ((maker, "maker"), (taker, "taker")):
                rows = [(s["swap_id"][:8], s["state"]) for s in who.rpc("listswaps")]
                print(f"[repro] {label} swaps: {rows}")
        finally:
            stop.set(); time.sleep(0.2)
            maker.stop(); taker.stop(); board.stop()


if __name__ == "__main__":
    main()
