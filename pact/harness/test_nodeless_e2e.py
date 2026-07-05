#!/usr/bin/env python3
"""Nodeless wallet e2e parity suite (epic #58): real swaps where one side's
btcx wallet is the NODELESS bdk wallet over live electrs — no Core-RPC URL.

Scenarios:
  1. v1 HTLC swap, nodeless maker    (Alice gives btcx from the bdk wallet:
     wallet_send funds leg A over Electrum broadcast; completion both sides)
  2. v2 adaptor swap, nodeless taker (Alice gives btcx as participant: the
     bdk wallet_build_funding two-phase leg B — build at accept, broadcast
     only after leg A is deep; completion both sides)
  3. v2 cancel pre-broadcast         (leg B BUILT in the bdk wallet, maker's
     leg A never confirms; abort must take the commitment-semantics path and
     wallet_cancel_funding must RELEASE the reserved inputs: balance restored)
  4. v1 swap nodeless<->nodeless     (both parties' btcx wallets are bdk over
     the SAME electrs, different seeds)

Stack: PoCX node on :18443 with -rest (bindex hardcodes that port) + BTC node
+ one electrs + Corkboard. Alice's btcx URL is tcp:// (nodeless); her btc and
all of Bob's coins stay Core-RPC — except scenario 4.

Run:  python test_nodeless_e2e.py
Env:  POCX_BITCOIND / BTC_BITCOIND / PACT_ELECTRS_BIN (see regtest_harness.py)
"""

import subprocess
import sys
import time

from regtest_harness import ELECTRS_ELECTRUM_PORT, ElectrsServer, Harness
from test_swap_e2e import PACT_DIR, Corkboard, Party, build_workspace

NODELESS_URL = f"tcp://127.0.0.1:{ELECTRS_ELECTRUM_PORT}"

GIVE_POCX = "50.0"
GIVE_POCX_SAT = 50 * 100_000_000
GET_BTC = "0.001"


def fund_bdk_wallet(h, electrs, party, coins="60.0"):
    """Fund a party's nodeless btcx (bdk) wallet from the harness's funded
    core wallet, and wait until the party can see it through electrs.

    getbalance is a pure CACHE read since the background sync worker
    (issue #87): an EXTERNALLY-originated payment like this one becomes
    visible at the worker's cadence (scripthash notification or ~15s tick),
    so poll bounded instead of asserting immediately. Our OWN broadcasts
    stay promptly visible (the wallet ops poke the worker)."""
    addr = party.rpc("getnewaddress", "btcx")["address"]
    assert addr.startswith("rpocx1p"), f"expected a regtest P2TR pocx address: {addr}"
    h.pocx.rpc("sendtoaddress", addr, float(coins), wallet="alice_pocx")
    h.pocx.generate(1, "alice_pocx")
    electrs.wait_synced(h.pocx.rpc("getblockcount"))
    deadline = time.time() + 30
    while True:
        bal = party.rpc("getbalance", "btcx")["balance_sat"]
        if bal > 0:
            break
        assert time.time() < deadline, f"bdk wallet not funded (balance {bal})"
        time.sleep(0.5)
    print(f"[nodeless] {party.name}: bdk wallet funded with {bal} sat")
    return bal


def swaps_of(party):
    """v1 + v2 records of a party (v2 lives in its own table)."""
    return party.rpc("listswaps") + party.rpc("listadaptorswaps")


def drive_to_completion(h, electrs, a, b, rounds=30, label="swap"):
    """Tick both parties, mine both chains, keep electrs in step; return the
    completed swap id."""
    for round_no in range(rounds):
        for party in (a, b):
            for ev in party.rpc("tick")["events"]:
                print(f"[nodeless]   {party.name}: {ev['action']} {ev['detail'][:70]}")
        h.pocx.generate(1, "alice_pocx")
        h.btc.generate(1, "bob_btc")
        electrs.wait_synced(h.pocx.rpc("getblockcount"))
        swaps_a = swaps_of(a)
        swaps_b = swaps_of(b)
        if swaps_a and swaps_b:
            sid = swaps_a[0]["swap_id"]
            states = (swaps_a[0]["state"], swaps_b[0]["state"])
            if states == ("completed", "completed"):
                print(f"[nodeless] {label} {sid} completed in {round_no + 1} rounds")
                return sid
    raise AssertionError(
        f"{label} did not complete: a={swaps_of(a)}, b={swaps_of(b)}")


def test_v1_nodeless_maker(h, electrs, board):
    """Alice (nodeless btcx) posts a v1 offer giving btcx; Bob (all Core)
    takes. Leg A is funded BY THE BDK WALLET and broadcast over Electrum."""
    alice = Party("nl-alice1", h, h.workdir, "alice_pocx", "alice_btc",
                  board_url=board.url, auto_fund=True,
                  pocx_url=NODELESS_URL).start()
    bob = Party("nl-bob1", h, h.workdir, "bob_pocx", "bob_btc",
                board_url=board.url, auto_fund=True).start()
    try:
        bal_before = fund_bdk_wallet(h, electrs, alice)
        offer_id = alice.rpc("boardpostoffer", f"btcx:{GIVE_POCX}",
                             f"btc:{GET_BTC}", 4 * 3600, 2 * 3600,
                             "pact-htlc-v1")["offer_id"]
        bob.rpc("boardtake", offer_id)
        drive_to_completion(h, electrs, alice, bob, label="v1 nodeless-maker")

        bal_after = alice.rpc("getbalance", "btcx")["balance_sat"]
        spent = bal_before - bal_after
        assert GIVE_POCX_SAT <= spent < GIVE_POCX_SAT + 100_000, (
            f"alice's bdk wallet should be down by ~{GIVE_POCX_SAT}: {spent}")
        # Her activity feed carries the leg-A lock as a plain send.
        txs = alice.rpc("listtransactions", "btcx")["transactions"]
        assert any(t["direction"] == "sent" and t["amount_sat"] == GIVE_POCX_SAT
                   for t in txs), f"leg-A lock missing from activity: {txs}"
        print("[nodeless] v1 nodeless-maker OK (balance + activity verified)")
    finally:
        alice.stop()
        bob.stop()


def test_v2_nodeless_taker(h, electrs, board):
    """Bob (Core) posts a v2 offer giving btc, getting btcx; Alice (nodeless
    btcx) takes -> she is the participant and leg B (btcx) goes through the
    bdk two-phase wallet_build_funding + delayed Electrum broadcast."""
    alice = Party("nl-alice2", h, h.workdir, "alice_pocx", "alice_btc",
                  board_url=board.url, auto_fund=True,
                  pocx_url=NODELESS_URL).start()
    bob = Party("nl-bob2", h, h.workdir, "bob_pocx", "bob_btc",
                board_url=board.url, auto_fund=True).start()
    try:
        # Bob needs btc (he gives btc): bob_btc is the funded btc wallet.
        bal_before = fund_bdk_wallet(h, electrs, alice)
        offer_id = bob.rpc("boardpostoffer", f"btc:{GET_BTC}",
                           f"btcx:{GIVE_POCX}", 4 * 3600, 2 * 3600,
                           "pact-htlc-v2")["offer_id"]
        alice.rpc("boardtake", offer_id)
        drive_to_completion(h, electrs, alice, bob, label="v2 nodeless-taker")

        bal_after = alice.rpc("getbalance", "btcx")["balance_sat"]
        spent = bal_before - bal_after
        assert GIVE_POCX_SAT <= spent < GIVE_POCX_SAT + 100_000, (
            f"alice's bdk wallet should be down by ~{GIVE_POCX_SAT}: {spent}")
        print("[nodeless] v2 nodeless-taker OK (two-phase bdk leg B verified)")
    finally:
        alice.stop()
        bob.stop()


def test_v2_cancel_releases_bdk_inputs(h, electrs, board):
    """The phantom-funding fix, live: Alice takes a v2 offer, her leg B is
    BUILT in the bdk wallet (inputs reserved) but the maker's leg A never
    confirms enough, so it is never broadcast. Abort must succeed (commitment
    semantics) and wallet_cancel_funding must restore her spendable balance."""
    alice = Party("nl-alice3", h, h.workdir, "alice_pocx", "alice_btc",
                  board_url=board.url, auto_fund=True,
                  pocx_url=NODELESS_URL).start()
    bob = Party("nl-bob3", h, h.workdir, "bob_pocx", "bob_btc",
                board_url=board.url, auto_fund=True).start()
    try:
        bal_before = fund_bdk_wallet(h, electrs, alice)
        offer_id = bob.rpc("boardpostoffer", f"btc:{GET_BTC}",
                           f"btcx:{GIVE_POCX}", 4 * 3600, 2 * 3600,
                           "pact-htlc-v2")["offer_id"]
        alice.rpc("boardtake", offer_id)

        # Drive the handshake WITHOUT mining: leg A gets no confirmations, so
        # alice's built leg B must never broadcast. Wait until the build shows
        # up as a reserved-balance drop.
        built = False
        for _ in range(12):
            for party in (bob, alice):
                for ev in party.rpc("tick")["events"]:
                    print(f"[nodeless]   {party.name}: {ev['action']} {ev['detail'][:70]}")
            swaps = alice.rpc("listadaptorswaps")
            if swaps and swaps[0].get("funding_b_txid"):
                built = True
                break
            time.sleep(0.3)
        assert built, f"alice never built leg B: {alice.rpc('listadaptorswaps')}"
        sid = alice.rpc("listadaptorswaps")[0]["swap_id"]

        bal_reserved = alice.rpc("getbalance", "btcx")["balance_sat"]
        assert bal_reserved < bal_before - GIVE_POCX_SAT + 100_000, (
            f"built leg B should reserve ~{GIVE_POCX_SAT}: "
            f"{bal_before} -> {bal_reserved}")
        print(f"[nodeless] leg B built: {bal_before - bal_reserved} sat reserved")

        # No btcx tx may be in the node mempool (leg B must NOT be broadcast).
        assert not h.pocx.rpc("getrawmempool"), "leg B leaked to the network!"

        # Abort: must take the built-but-uncommitted path and release.
        alice.rpc("abort", sid, "e2e cancel test")
        bal_released = alice.rpc("getbalance", "btcx")["balance_sat"]
        assert bal_released == bal_before, (
            f"cancel must restore the reserved inputs: "
            f"{bal_before} -> {bal_released}")
        state = alice.rpc("listadaptorswaps")[0]["state"]
        assert state == "aborted", f"swap not aborted: {state}"
        print("[nodeless] v2 cancel OK: reserved inputs released, balance restored")
    finally:
        alice.stop()
        bob.stop()


def test_v1_nodeless_both_sides(h, electrs, board):
    """Both parties' btcx wallets are nodeless bdk wallets over the SAME
    electrs (different seeds). Alice gives btcx, Bob receives his btcx redeem
    into his own bdk wallet (redeem sweep -> wallet_new_address on bdk)."""
    alice = Party("nl-alice4", h, h.workdir, "alice_pocx", "alice_btc",
                  board_url=board.url, auto_fund=True,
                  pocx_url=NODELESS_URL).start()
    bob = Party("nl-bob4", h, h.workdir, "bob_pocx", "bob_btc",
                board_url=board.url, auto_fund=True,
                pocx_url=NODELESS_URL).start()
    try:
        bal_before = fund_bdk_wallet(h, electrs, alice)
        bob_btcx_before = bob.rpc("getbalance", "btcx")["balance_sat"]
        offer_id = alice.rpc("boardpostoffer", f"btcx:{GIVE_POCX}",
                             f"btc:{GET_BTC}", 4 * 3600, 2 * 3600,
                             "pact-htlc-v1")["offer_id"]
        bob.rpc("boardtake", offer_id)
        drive_to_completion(h, electrs, alice, bob, label="v1 nodeless-both")

        spent = bal_before - alice.rpc("getbalance", "btcx")["balance_sat"]
        assert GIVE_POCX_SAT <= spent < GIVE_POCX_SAT + 100_000, spent
        # Bob's redeem pays him from a FOREIGN input (the HTLC), so bdk
        # counts it as trusted_spendable only once CONFIRMED — which needs a
        # worker pass AFTER the confirming block. The completing tick pokes
        # the worker (issue #87 swap-event pokes) but the sync is
        # asynchronous, so poll briefly instead of asserting instantly.
        deadline = time.time() + 20
        while True:
            gained = bob.rpc("getbalance", "btcx")["balance_sat"] - bob_btcx_before
            if gained > 0 or time.time() > deadline:
                break
            time.sleep(0.5)
        assert 0 < gained <= GIVE_POCX_SAT, (
            f"bob's bdk wallet should hold the leg-A redeem: {gained}")
        print(f"[nodeless] v1 nodeless-both OK (bob's bdk redeem: {gained} sat)")
    finally:
        alice.stop()
        bob.stop()


def main():
    build_workspace()
    failures = 0
    tests = (test_v1_nodeless_maker, test_v2_nodeless_taker,
             test_v2_cancel_releases_bdk_inputs, test_v1_nodeless_both_sides)
    with Harness(keep=True, pocx_rest=True) as h:
        electrs = ElectrsServer(h.workdir, h.pocx)
        board = Corkboard(h.workdir)
        try:
            electrs.start()
            electrs.wait_synced(h.pocx.rpc("getblockcount"))
            print("[nodeless] electrs synced")
            board.start()
            for test in tests:
                try:
                    test(h, electrs, board)
                except Exception as exc:  # noqa: BLE001 — report and continue
                    failures += 1
                    print(f"[nodeless] FAIL {test.__name__}: {exc}", file=sys.stderr)
        finally:
            board.stop()
            electrs.stop()
    if failures:
        print(f"\n[nodeless] RED: {failures}/{len(tests)} scenario(s) failing.",
              file=sys.stderr)
        sys.exit(1)
    print(f"\n[nodeless] GREEN: all {len(tests)} nodeless parity scenarios pass.")


if __name__ == "__main__":
    main()
