"""rc16 reproduction: force the microseconds-apart concurrent-drain double-send.

The taker gets PACT_TEST_OUTBOX_DRAIN_DELAY_MS set, which widens the window
between reading the outbox and marking it sent. So boardtake's flush_nostr pass
and the next tick's pass BOTH read the same still-unsent `take` and each
publishes it -> the maker receives the take twice (take->init + take-duplicate).
In a clean env (no delay) this never happens. Proves the #176 race, and becomes
the red->green check for the single-flight lock fix."""
import os, sys, time

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import test_swap_e2e as e2e  # noqa: E402


def count(resp, counts):
    events = resp.get("events", []) if isinstance(resp, dict) else (resp or [])
    for e in events:
        a = e.get("action", "")
        counts[a] = counts.get(a, 0) + 1
        if a in ("take->init", "take-duplicate", "take-rejected"):
            print(f"  [maker] {a}: {e.get('detail','')}", flush=True)


e2e.build_workspace()
with e2e.Harness(keep=True) as h:
    relay = e2e.NostrRelay(h.workdir)
    relay.start()
    maker = e2e.Party("dmk", h, h.workdir, "alice_pocx", "alice_btc",
                      nostr_relays=relay.ws_url, auto_fund=True).start()
    # Delay ONLY the taker's outbox drains.
    os.environ["PACT_TEST_OUTBOX_DRAIN_DELAY_MS"] = "800"
    taker = e2e.Party("dtk", h, h.workdir, "bob_pocx", "bob_btc",
                      nostr_relays=relay.ws_url, auto_fund=True).start()
    del os.environ["PACT_TEST_OUTBOX_DRAIN_DELAY_MS"]
    counts = {}
    try:
        offer_id = maker.rpc("boardpostoffer", f"btcx:{e2e.GIVE_POCX}", f"btc:{e2e.GET_BTC}",
                             4 * 3600, 2 * 3600, "pact-htlc-v1")["offer_id"]
        for _ in range(20):
            maker.rpc("tick")
            taker.rpc("tick")
            if any(o["swap_id"] == offer_id for o in taker.rpc("boardlistoffers")["offers"]):
                break
        print("taker.boardtake -> fires flush_nostr (pass A, delayed 800ms)", flush=True)
        taker.rpc("boardtake", offer_id)
        # Pass B: a tick right away, while pass A is still inside its delay.
        taker.rpc("tick")
        # Drive; count the maker's take events.
        for _ in range(25):
            count(maker.rpc("tick"), counts)
            h.pocx.generate(1, "alice_pocx")
            h.btc.generate(1, "bob_btc")
            taker.rpc("tick")
            done = sum(1 for s in maker.rpc("listswaps") if s["state"] == "completed")
            if counts.get("take-duplicate", 0) and done:
                break
    finally:
        maker.stop()
        taker.stop()
        relay.stop()
    ti = counts.get("take->init", 0)
    td = counts.get("take-duplicate", 0)
    tr = counts.get("take-rejected", 0)
    print(f"\nMAKER take->init={ti}  take-duplicate={td}  take-rejected={tr}", flush=True)
    print("RESULT:", "REPRODUCED — concurrent-drain double-send" if (td + tr) > 0
          else "NOT reproduced (no duplicate take seen)", flush=True)
