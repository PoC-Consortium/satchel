#!/usr/bin/env python3
"""Upgrade-path e2e: swaps STARTED on the last released pactd (v0.1.0-rc16,
built from the immutable tag into bin/pactd-rc16) must finish on the CURRENT
build — the compat promise a stable release makes. Three cells:

  UpgradeMidSwapV1     both parties on rc16 drive a v1 swap to both-funded,
                       both daemons stop, the SAME datadirs relaunch on the
                       current build and the swap completes on the ORIGINAL
                       fundings (rc16-written DB: swaps, outbox, meta,
                       machine.json all reopened + migrated).
  UpgradeMidSwapV2     the v2 twin, upgraded at committed — the state-richest
                       DB an rc16 could leave behind (nonce sessions +
                       assembled adaptor sigs), held observable by the
                       maker's btc=3 conf gate.
  MixedVersionSwapV1   an rc16 maker completes a whole v1 swap against a
                       CURRENT taker — the wire-compat direction (an upgraded
                       user trades with a not-yet-upgraded one).

Run:  python tests/upgrade.py [--filter SUBSTR] [--keep] [--no-build]
"""

import os
import sys

sys.path.insert(0, os.path.normpath(
    os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")))
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from framework.binaries import find_pactd_rc16  # noqa: E402
from framework.daemon import Party  # noqa: E402
from framework.services import NostrRelay  # noqa: E402
from framework.testbase import PactTestFramework, run_scenarios  # noqa: E402
from framework.util import GET_BTC, GIVE_POCX, handshake_done  # noqa: E402


def swap_of(p, sid=None):
    sw = (p.rpc("listswaps") or []) + (p.rpc("listadaptorswaps") or [])
    if sid is not None:
        sw = [s for s in sw if s["swap_id"] == sid]
    return sw[0] if sw else None


def tick_all(tag, *parties):
    for p in parties:
        if p.proc is None:
            continue
        for ev in p.rpc("tick")["events"]:
            print(f"[upgrade-e2e]   {tag}[{p.name}]: {ev['action']} {ev['detail'][:70]}")


def relay_handshake(maker, taker, protocol):
    offer_id = maker.rpc(
        "boardpostoffer", f"btcx:{GIVE_POCX}", f"btc:{GET_BTC}",
        4 * 3600, 2 * 3600, protocol)["offer_id"]
    for _ in range(25):
        maker.rpc("tick")
        taker.rpc("tick")
        if any(o["swap_id"] == offer_id
               for o in taker.rpc("boardlistoffers")["offers"]):
            break
    else:
        raise AssertionError(f"{protocol} offer never propagated to the taker")
    taker.rpc("boardtake", offer_id)
    return offer_id


def mine(h, n=1):
    h.pocx.generate(n, "alice_pocx")
    h.btc.generate(n, "bob_btc")


def drive_to_completion(h, a, b, sid, rounds=40, what="swap"):
    for _ in range(rounds):
        tick_all("finish", a, b)
        sa, sb = swap_of(a, sid), swap_of(b, sid)
        if (sa and sb and sa["state"] == "completed"
                and sb["state"] == "completed"):
            return
        mine(h)
    raise AssertionError(f"{what} never completed: "
                         f"a={swap_of(a, sid)} b={swap_of(b, sid)}")


def scenario_upgrade_mid_swap_v1(h):
    """v1 upgraded at both-funded (see module docstring)."""
    rc16 = find_pactd_rc16()
    relay = NostrRelay(h.workdir)
    maker = Party("upmk1", h, h.workdir, "alice_pocx", "alice_btc",
                  nostr_relays=relay.ws_url, auto_fund=True, auto_init=True,
                  pactd_bin=rc16)
    taker = Party("uptk1", h, h.workdir, "bob_pocx", "bob_btc",
                  nostr_relays=relay.ws_url, auto_fund=True, auto_init=True,
                  pactd_bin=rc16)
    try:
        relay.start()
        maker.start()
        taker.start()
        relay_handshake(maker, taker, "pact-htlc-v1")

        sid = None
        for _ in range(60):
            tick_all("rc16", maker, taker)
            m, t = swap_of(maker), swap_of(taker)
            if (m and t and m.get("htlc_a_txid") and t.get("htlc_b_txid")):
                sid = m["swap_id"]
                break
            if handshake_done(maker, taker):
                mine(h)
        assert sid, "rc16 pair never reached both-legs-funded"
        pre_a = swap_of(maker, sid)["htlc_a_txid"]
        pre_b = swap_of(taker, sid)["htlc_b_txid"]

        # --- the upgrade: stop BOTH rc16 daemons, relaunch the same datadirs
        # (same ports, same DBs) on the current build. ---
        print(f"[upgrade-e2e] both funded on rc16 ({sid[:16]}) — upgrading")
        maker.stop()
        taker.stop()
        maker.pactd_bin = None  # current workspace build
        taker.pactd_bin = None
        maker.start()
        taker.start()

        drive_to_completion(h, maker, taker, sid, what="upgraded v1 swap")
        post_m, post_t = swap_of(maker, sid), swap_of(taker, sid)
        assert post_m["htlc_a_txid"] == pre_a and post_t["htlc_b_txid"] == pre_b, \
            (f"upgrade re-funded a leg: A {pre_a}->{post_m['htlc_a_txid']} "
             f"B {pre_b}->{post_t['htlc_b_txid']}")
        print("[upgrade-e2e] v1 upgrade OK: rc16-started swap completed on the "
              "current build, original fundings intact")
    finally:
        maker.stop()
        taker.stop()
        relay.stop()


def scenario_upgrade_mid_swap_v2(h):
    """v2 upgraded at committed — rc16 wrote the nonce sessions and assembled
    adaptor sigs the current build must reload to complete (not just refund)."""
    rc16 = find_pactd_rc16()
    relay = NostrRelay(h.workdir)
    # btc=3 holds the maker at Signed (the same committed-window hold as the
    # rescue/takeover suites) so the upgrade lands mid-flight deterministically.
    maker = Party("upmk2", h, h.workdir, "alice_pocx", "alice_btc",
                  nostr_relays=relay.ws_url, auto_fund=True, auto_init=True,
                  coin_confs={"btc": 3}, pactd_bin=rc16)
    taker = Party("uptk2", h, h.workdir, "bob_pocx", "bob_btc",
                  nostr_relays=relay.ws_url, auto_fund=True, auto_init=True,
                  pactd_bin=rc16)
    try:
        relay.start()
        maker.start()
        taker.start()
        relay_handshake(maker, taker, "pact-htlc-v2")

        def leg_b_on_wire():
            t = swap_of(taker)
            return (t is not None and t.get("funding_b_txid") is not None
                    and h.btc.rpc("gettxout", t["funding_b_txid"],
                                  t.get("funding_b_vout") or 0) is not None)

        sid = None
        for _ in range(60):
            tick_all("rc16", maker, taker)
            m = swap_of(maker)
            if (m is not None and m.get("funding_a_txid")
                    and m.get("state") in ("signed", "funded_a", "funded_b")
                    and leg_b_on_wire()):
                sid = m["swap_id"]
                break
            # Hold leg B shallow while the maker assembles Signed (the
            # committed-window collapse — see takeover.py).
            if handshake_done(maker, taker) and not leg_b_on_wire():
                mine(h)
        assert sid, "rc16 pair never reached v2 committed"
        pre_a = swap_of(maker, sid)["funding_a_txid"]
        pre_b = swap_of(taker, sid)["funding_b_txid"]

        print(f"[upgrade-e2e] v2 committed on rc16 ({sid[:16]}) — upgrading")
        maker.stop()
        taker.stop()
        maker.pactd_bin = None
        taker.pactd_bin = None
        maker.start()
        taker.start()

        drive_to_completion(h, maker, taker, sid, what="upgraded v2 swap")
        post_m, post_t = swap_of(maker, sid), swap_of(taker, sid)
        assert (post_m["funding_a_txid"] == pre_a
                and post_t["funding_b_txid"] == pre_b), \
            "v2 upgrade re-funded a leg"
        assert post_m.get("final_txid_b"), \
            "upgraded maker never cooperatively redeemed (adaptor sigs lost?)"
        print("[upgrade-e2e] v2 upgrade OK: completed via the rc16-written "
              "adaptor state, original fundings intact")
    finally:
        maker.stop()
        taker.stop()
        relay.stop()


def scenario_mixed_version_swap_v1(h):
    """Wire compat: an rc16 maker and a CURRENT-build taker complete a whole
    v1 swap against each other over the relay — the mid-rollout reality where
    one side upgraded first."""
    rc16 = find_pactd_rc16()
    relay = NostrRelay(h.workdir)
    maker = Party("mxmk", h, h.workdir, "alice_pocx", "alice_btc",
                  nostr_relays=relay.ws_url, auto_fund=True, auto_init=True,
                  pactd_bin=rc16)
    taker = Party("mxtk", h, h.workdir, "bob_pocx", "bob_btc",
                  nostr_relays=relay.ws_url, auto_fund=True, auto_init=True)
    try:
        relay.start()
        maker.start()
        taker.start()
        relay_handshake(maker, taker, "pact-htlc-v1")

        sid = None
        for _ in range(60):
            tick_all("mixed", maker, taker)
            m, t = swap_of(maker), swap_of(taker)
            if m and t:
                sid = m["swap_id"]
            if (sid and m["state"] == "completed"
                    and t["state"] == "completed"):
                break
            if handshake_done(maker, taker):
                mine(h)
        else:
            raise AssertionError(
                f"mixed-version swap never completed: "
                f"rc16-maker={swap_of(maker)} current-taker={swap_of(taker)}")
        print("[upgrade-e2e] mixed-version OK: rc16 maker <-> current taker "
              "completed a v1 swap over the wire")
    finally:
        maker.stop()
        taker.stop()
        relay.stop()


class UpgradeMidSwapV1(PactTestFramework):
    def run_test(self):
        scenario_upgrade_mid_swap_v1(self.h)


class UpgradeMidSwapV2(PactTestFramework):
    def run_test(self):
        scenario_upgrade_mid_swap_v2(self.h)


class MixedVersionSwapV1(PactTestFramework):
    def run_test(self):
        scenario_mixed_version_swap_v1(self.h)


SCENARIOS = [
    UpgradeMidSwapV1,
    UpgradeMidSwapV2,
    MixedVersionSwapV1,
]


if __name__ == "__main__":
    run_scenarios(SCENARIOS)
