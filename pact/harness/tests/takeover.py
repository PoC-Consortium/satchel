#!/usr/bin/env python3
"""Multi-machine takeover coverage the rescue/follow suites don't reach: the
#173 payout-gate REFUND-ONLY branch (every other e2e shares the node wallet, so
`wallet_owns_address` is always true and the branch never runs).

Scenario — owner dies mid-v2-swap, backup shares the SEED but NOT the owner's
btc node wallet:

  1. Maker (owner) posts a v2 offer; taker takes; both legs fund; the maker is
     held at `Signed` (btc=3 conf gate) — its snapshot carries the assembled
     adaptor sigs.
  2. The owner's pactd is killed.
  3. A backup pactd on the SAME seed but a DIFFERENT btc wallet (so it does NOT
     own the cooperative-redeem sweep `sweep_b`) auto-imports the swap as
     FOLLOWED and takes it over.
     - Pre-fix, `takeover` HARD-BAILED here ("payout pays a wallet this machine
       does not control"), stranding the funded leg with nobody to reclaim it.
     - Post-fix it ADOPTS refund-only.
  4. Even with leg B buried deep enough to cooperatively redeem, the backup
     NEVER reveals t / redeems to the foreign wallet; past T1 it reclaims its
     own leg A (refund → a fresh address it owns), on the ORIGINAL funding.

Run:  python tests/takeover.py [--filter SUBSTR] [--keep] [--no-build]
"""

import os
import sys
import time

sys.path.insert(0, os.path.normpath(
    os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")))
# Sibling test module for the shared electrs/relay scaffold + tiny helpers.
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from framework.daemon import Party  # noqa: E402
from framework.services import NostrRelay  # noqa: E402
from framework.testbase import run_scenarios  # noqa: E402
from framework.util import GET_BTC, GIVE_POCX, handshake_done  # noqa: E402
from follow import (  # noqa: E402  (shared scaffold, verbatim helpers)
    _FollowScenario,
    mine_and_sync,
    multi_urls,
    swap_of,
    tick_all,
)


def scenario_refund_only_takeover_v2(h, ep, eb):
    """The #173 payout-gate refund-only branch, end to end (see module docstring)."""
    relay = NostrRelay(h.workdir)
    # The backup's btc PRIMARY wallet is a fresh one that never saw the owner's
    # sweep_b — so `wallet_owns_address(sweep_b)` is false and the gate trips.
    # (Its pocx wallet IS shared, so it can still reclaim leg A.)
    h.btc.create_wallet("alice_btc2")
    owner_bx, owner_bt = multi_urls(h, ep, eb, "alice_pocx", "alice_btc")
    backup_bx, backup_bt = multi_urls(h, ep, eb, "alice_pocx", "alice_btc2")
    taker_bx, taker_bt = multi_urls(h, ep, eb, "bob_pocx", "bob_btc")

    # btc=3 holds the maker at Signed for a few blocks (1-conf regtest otherwise
    # races Signed → redeemed_b → completed within a round, closing the window).
    # Owner: auto_init off so we mint the seed explicitly and can re-import the
    # SAME mnemonic into the backup. Taker keeps its own auto-created seed.
    maker = Party("tomk", h, h.workdir, "alice_pocx", "alice_btc",
                  nostr_relays=relay.ws_url, auto_fund=True, auto_init=False,
                  coin_confs={"btc": 3}, pocx_url=owner_bx, btc_url=owner_bt)
    taker = Party("totk", h, h.workdir, "bob_pocx", "bob_btc",
                  nostr_relays=relay.ws_url, auto_fund=True, auto_init=True,
                  pocx_url=taker_bx, btc_url=taker_bt)
    backup = None
    try:
        relay.start()
        maker.start()
        taker.start()
        # Fresh seed on the owner; the backup imports the SAME mnemonic.
        mnemonic = maker.setup_seed()

        offer_id = maker.rpc(
            "boardpostoffer", f"btcx:{GIVE_POCX}", f"btc:{GET_BTC}",
            4 * 3600, 2 * 3600, "pact-htlc-v2")["offer_id"]
        for _ in range(25):
            maker.rpc("tick")
            taker.rpc("tick")
            if any(o["swap_id"] == offer_id
                   for o in taker.rpc("boardlistoffers")["offers"]):
                break
        else:
            raise AssertionError("v2 offer never propagated to the taker")
        taker.rpc("boardtake", offer_id)

        # Drive until the maker is Signed with BOTH legs on the wire — break the
        # instant leg B is broadcast (still shallow), before the maker reveals t.
        def signed_committed():
            m = swap_of(maker)
            return (m is not None
                    and m.get("state") in ("signed", "funded_a", "funded_b")
                    and m.get("funding_a_txid")
                    and (m.get("funding_b_broadcast") is True
                         or m.get("funding_b_txid")))

        sid = None
        for _ in range(60):
            tick_all("drive", maker, taker)
            if signed_committed():
                sid = swap_of(maker)["swap_id"]
                break
            if handshake_done(maker, taker):
                mine_and_sync(h, ep, eb)
        assert sid, "maker never reached Signed with both legs committed"
        pre = swap_of(maker, sid)
        pre_a = pre.get("funding_a_txid")
        pre_a_vout = pre.get("funding_a_vout")
        assert pre_a is not None, f"maker has no leg-A funding pre-kill: {pre}"

        # Give the Signed snapshot (assembled adaptor sigs) time to reach the relay.
        for _ in range(2):
            maker.rpc("tick")
            time.sleep(1)
        print(f"[takeover-e2e] maker Signed (leg A {pre_a[:16]}) — killing the owner")
        # Hard kill (machine fallout): no graceful outbox drain.
        maker.proc.kill()
        maker.proc.wait(timeout=15)

        # Backup: SAME seed, FOREIGN btc primary wallet (no sweep_b custody).
        backup = Party("tobk", h, h.workdir, "alice_pocx", "alice_btc2",
                       nostr_relays=relay.ws_url, auto_fund=True, auto_init=False,
                       coin_confs={"btc": 3},
                       pocx_url=backup_bx, btc_url=backup_bt)
        backup.start()
        backup.setup_seed(mnemonic=mnemonic)

        rec = None
        for _ in range(30):
            tick_all("import", backup)
            rec = swap_of(backup, sid)
            if rec is not None:
                break
            time.sleep(0.5)
        assert rec is not None, "foreign v2 snapshot never auto-imported (#163)"
        assert rec.get("source") == "foreign", f"import must be read-only: {rec}"

        # HEADLINE (Fix): takeover must ADOPT, not bail on the foreign payout.
        backup.rpc("takeover", sid)
        rec = swap_of(backup, sid)
        assert rec.get("source") == "local", \
            f"refund-only takeover did not adopt (payout-gate bail?): {rec}"
        print(f"[takeover-e2e] takeover adopted {sid[:16]} refund-only (foreign sweep)")

        # Bury leg B deep enough that a payout-OWNING machine would redeem — the
        # backup must still refuse (never reveal t to a wallet it can't spend).
        mine_and_sync(h, ep, eb, n=4)
        for _ in range(6):
            tick_all("no-redeem", backup, taker)
            mine_and_sync(h, ep, eb)
        assert swap_of(backup, sid).get("final_txid_b") is None, \
            "backup revealed t / cooperatively redeemed to a wallet it does not own"

        # Ride to the leg-A refund: jump past T1 and drive both sides to terminal.
        h.advance_time(5 * 3600)
        done = False
        for _ in range(40):
            tick_all("refund", backup, taker)
            mine_and_sync(h, ep, eb)
            v = swap_of(backup, sid)
            spent = h.pocx.rpc("gettxout", pre_a, pre_a_vout) is None
            if v is not None and v["state"] in ("aborted", "refunded") and spent:
                done = True
                break
        assert done, f"backup never reclaimed leg A after T1: {swap_of(backup, sid)}"
        # Reclaimed the ORIGINAL funding (no double-fund) and never redeemed.
        post = swap_of(backup, sid)
        assert post.get("funding_a_txid") in (pre_a, None), \
            f"backup re-funded leg A (double-fund!): {pre_a} -> {post.get('funding_a_txid')}"
        assert post.get("final_txid_b") is None, \
            "backup cooperatively redeemed on the refund path"
        print("[takeover-e2e] refund-only takeover OK: adopted, rode to leg-A "
              "refund on the original funding, never redeemed to a foreign wallet")
    finally:
        for p in (maker, taker, backup):
            if p is not None:
                try:
                    p.stop()
                except Exception:  # noqa: BLE001
                    pass
        relay.stop()


def scenario_hot_standby_takeover_v1(h, ep, eb):
    """A warm STANDBY on the owner's seed, running the WHOLE time the owner
    drives — the emergency shape (a second machine already up, not a fresh
    boot). While the owner is alive the standby holds the swap as FOLLOWED
    (read-only) and must NEVER commit funds for it (§2 / the #164 double-fund
    hole: both machines share the identity mailbox, so the standby also
    receives the handshake messages). When the owner is hard-killed mid-swap
    the standby takes over and completes on the ORIGINAL funding."""
    relay = NostrRelay(h.workdir)
    owner_bx, owner_bt = multi_urls(h, ep, eb, "alice_pocx", "alice_btc")
    taker_bx, taker_bt = multi_urls(h, ep, eb, "bob_pocx", "bob_btc")
    maker = Party("hsmk", h, h.workdir, "alice_pocx", "alice_btc",
                  nostr_relays=relay.ws_url, auto_fund=True, auto_init=False,
                  pocx_url=owner_bx, btc_url=owner_bt)
    taker = Party("hstk", h, h.workdir, "bob_pocx", "bob_btc",
                  nostr_relays=relay.ws_url, auto_fund=True, auto_init=True,
                  pocx_url=taker_bx, btc_url=taker_bt)
    # Same seed + same node wallets as the owner, own data dir (own scope).
    standby = Party("hssb", h, h.workdir, "alice_pocx", "alice_btc",
                    nostr_relays=relay.ws_url, auto_fund=True, auto_init=False,
                    pocx_url=owner_bx, btc_url=owner_bt)
    try:
        relay.start()
        maker.start()
        taker.start()
        standby.start()
        mnemonic = maker.setup_seed()
        standby.setup_seed(mnemonic=mnemonic)  # the SAME seed → same identity

        offer_id = maker.rpc(
            "boardpostoffer", f"btcx:{GIVE_POCX}", f"btc:{GET_BTC}",
            4 * 3600, 2 * 3600, "pact-htlc-v1")["offer_id"]
        for _ in range(25):
            maker.rpc("tick")
            taker.rpc("tick")
            standby.rpc("tick")
            if any(o["swap_id"] == offer_id
                   for o in taker.rpc("boardlistoffers")["offers"]):
                break
        else:
            raise AssertionError("v1 offer never propagated to the taker")
        taker.rpc("boardtake", offer_id)

        def both_funded():
            m, t = swap_of(maker), swap_of(taker)
            return (m is not None and m.get("htlc_a_txid")
                    and t is not None and t.get("htlc_b_txid"))

        standby_events = []
        sid = None
        for _ in range(60):
            tick_all("drive", maker, taker)
            standby_events += tick_all("standby", standby)
            if both_funded():
                sid = (swap_of(maker) or swap_of(taker))["swap_id"]
                break
            if handshake_done(maker, taker):
                mine_and_sync(h, ep, eb)
        assert sid, "swap never reached both-legs-funded"
        pre_a = swap_of(maker, sid)["htlc_a_txid"]

        # While the owner drove, the standby must have FOLLOWED read-only and
        # never committed funds (the §2 ownership + #164 double-fund guards).
        srec = swap_of(standby, sid)
        assert srec is not None and srec.get("source") == "foreign", \
            f"standby must hold the live swap as FOLLOWED, not drive it: {srec}"
        for bad in ("auto-fund", "funded-a", "funded-b", "adaptor-fund-b"):
            assert bad not in standby_events, \
                f"standby committed funds while merely following: {standby_events}"

        for _ in range(2):
            maker.rpc("tick")
            time.sleep(1)
        print(f"[takeover-e2e] both legs funded (A {pre_a[:16]}); standby followed "
              "read-only — hard-killing the owner")
        maker.proc.kill()
        maker.proc.wait(timeout=15)

        # The standby takes over (owner provably dead) and finishes with the taker.
        standby.rpc("takeover", sid)
        assert swap_of(standby, sid).get("source") == "local", \
            "takeover did not adopt on the standby"
        done = False
        for _ in range(40):
            tick_all("finish", standby, taker)
            s, t = swap_of(standby, sid), swap_of(taker, sid)
            if s and t and s["state"] == "completed" and t["state"] == "completed":
                done = True
                break
            mine_and_sync(h, ep, eb)
        assert done, (f"standby takeover never completed: "
                      f"standby={swap_of(standby, sid)} taker={swap_of(taker, sid)}")
        assert swap_of(standby, sid)["htlc_a_txid"] == pre_a, \
            "takeover re-funded leg A (double-fund!)"
        print("[takeover-e2e] hot-standby OK: followed live, took over a killed "
              "owner, completed on the original funding")
    finally:
        for p in (maker, taker, standby):
            if p is not None:
                try:
                    p.stop()
                except Exception:  # noqa: BLE001
                    pass
        relay.stop()


class RefundOnlyTakeoverV2(_FollowScenario):
    scenario = staticmethod(scenario_refund_only_takeover_v2)


class HotStandbyTakeoverV1(_FollowScenario):
    scenario = staticmethod(scenario_hot_standby_takeover_v1)


SCENARIOS = [
    RefundOnlyTakeoverV2,
    HotStandbyTakeoverV1,
]


if __name__ == "__main__":
    run_scenarios(SCENARIOS)
