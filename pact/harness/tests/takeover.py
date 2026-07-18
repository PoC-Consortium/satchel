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


def _relay_handshake(h, maker, taker, protocol, observers=()):
    """Post a v1/v2 offer, propagate it over the relay, take it. Returns the
    offer_id. Observers are ticked along so they see the mailbox live."""
    offer_id = maker.rpc(
        "boardpostoffer", f"btcx:{GIVE_POCX}", f"btc:{GET_BTC}",
        4 * 3600, 2 * 3600, protocol)["offer_id"]
    for _ in range(25):
        maker.rpc("tick")
        taker.rpc("tick")
        for o in observers:
            o.rpc("tick")
        if any(o["swap_id"] == offer_id
               for o in taker.rpc("boardlistoffers")["offers"]):
            break
    else:
        raise AssertionError(f"{protocol} offer never propagated to the taker")
    taker.rpc("boardtake", offer_id)
    return offer_id


def _await_follow(standby, sid, tag, rounds=30, others=()):
    """Tick the standby until it holds `sid` as a FOLLOWED (read-only) record.
    `others` are ticked along — needed while the owner is still alive and its
    snapshot may not have reached the relay yet (v1 snapshots exactly once, in
    the owner's accept-processing tick)."""
    for _ in range(rounds):
        tick_all(tag, standby, *others)
        rec = swap_of(standby, sid)
        if rec is not None:
            assert rec.get("source") == "foreign", \
                f"standby must follow read-only, not drive: {rec}"
            return rec
        time.sleep(0.5)
    raise AssertionError(f"standby never followed swap {sid}")


def _kill(party):
    party.proc.kill()
    party.proc.wait(timeout=15)


def scenario_taker_hot_standby_v1(h, ep, eb):
    """The TAKER-side twin of scenario_hot_standby_takeover_v1: every existing
    takeover cell adopts the MAKER role, but a taker standby inherits the other
    half of the protocol (leg-B custody, the leg-A redeem after the reveal).
    Both legs fund, the taker is hard-killed, its warm standby takes over and
    completes on the ORIGINAL leg-B funding."""
    relay = NostrRelay(h.workdir)
    maker_bx, maker_bt = multi_urls(h, ep, eb, "alice_pocx", "alice_btc")
    taker_bx, taker_bt = multi_urls(h, ep, eb, "bob_pocx", "bob_btc")
    maker = Party("thmk", h, h.workdir, "alice_pocx", "alice_btc",
                  nostr_relays=relay.ws_url, auto_fund=True, auto_init=True,
                  pocx_url=maker_bx, btc_url=maker_bt)
    taker = Party("thtk", h, h.workdir, "bob_pocx", "bob_btc",
                  nostr_relays=relay.ws_url, auto_fund=True, auto_init=False,
                  pocx_url=taker_bx, btc_url=taker_bt)
    standby = Party("thsb", h, h.workdir, "bob_pocx", "bob_btc",
                    nostr_relays=relay.ws_url, auto_fund=True, auto_init=False,
                    pocx_url=taker_bx, btc_url=taker_bt)
    try:
        relay.start()
        maker.start()
        taker.start()
        standby.start()
        mnemonic = taker.setup_seed()
        standby.setup_seed(mnemonic=mnemonic)

        _relay_handshake(h, maker, taker, "pact-htlc-v1", observers=(standby,))

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
                sid = swap_of(taker)["swap_id"]
                break
            if handshake_done(maker, taker):
                mine_and_sync(h, ep, eb)
        assert sid, "swap never reached both-legs-funded"
        pre_b = swap_of(taker, sid)["htlc_b_txid"]
        for bad in ("auto-fund", "funded-a", "funded-b", "adaptor-fund-b"):
            assert bad not in standby_events, \
                f"taker standby committed funds while following: {standby_events}"

        for _ in range(2):
            taker.rpc("tick")
            time.sleep(1)
        print(f"[takeover-e2e] both funded (B {pre_b[:16]}) — hard-killing the taker")
        _kill(taker)

        _await_follow(standby, sid, "import")
        standby.rpc("takeover", sid)
        assert swap_of(standby, sid).get("source") == "local", \
            "takeover did not adopt on the taker standby"
        done = False
        for _ in range(40):
            tick_all("finish", standby, maker)
            s, m = swap_of(standby, sid), swap_of(maker, sid)
            if s and m and s["state"] == "completed" and m["state"] == "completed":
                done = True
                break
            mine_and_sync(h, ep, eb)
        assert done, (f"taker-standby takeover never completed: "
                      f"standby={swap_of(standby, sid)} maker={swap_of(maker, sid)}")
        assert swap_of(standby, sid)["htlc_b_txid"] == pre_b, \
            "taker standby re-funded leg B (double-fund!)"
        print("[takeover-e2e] taker hot-standby OK: took over a killed taker, "
              "completed on the original leg-B funding")
    finally:
        for p in (maker, taker, standby):
            try:
                p.stop()
            except Exception:  # noqa: BLE001
                pass
        relay.stop()


def scenario_taker_committed_takeover_v2(h, ep, eb):
    """v2 TAKER adopted mid-flight: the taker dies right after committing leg B
    (its adaptor accept + funding are on the wire, nothing settled). The
    standby must finish the participant side — wait out the maker's reveal and
    claim leg A — WITHOUT re-funding leg B. The maker's btc=3 conf gate holds
    it at Signed so the kill lands inside the committed window."""
    relay = NostrRelay(h.workdir)
    maker_bx, maker_bt = multi_urls(h, ep, eb, "alice_pocx", "alice_btc")
    taker_bx, taker_bt = multi_urls(h, ep, eb, "bob_pocx", "bob_btc")
    maker = Party("tcmk", h, h.workdir, "alice_pocx", "alice_btc",
                  nostr_relays=relay.ws_url, auto_fund=True, auto_init=True,
                  coin_confs={"btc": 3}, pocx_url=maker_bx, btc_url=maker_bt)
    taker = Party("tctk", h, h.workdir, "bob_pocx", "bob_btc",
                  nostr_relays=relay.ws_url, auto_fund=True, auto_init=False,
                  pocx_url=taker_bx, btc_url=taker_bt)
    standby = Party("tcsb", h, h.workdir, "bob_pocx", "bob_btc",
                    nostr_relays=relay.ws_url, auto_fund=True, auto_init=False,
                    pocx_url=taker_bx, btc_url=taker_bt)
    try:
        relay.start()
        maker.start()
        taker.start()
        standby.start()
        mnemonic = taker.setup_seed()
        standby.setup_seed(mnemonic=mnemonic)

        _relay_handshake(h, maker, taker, "pact-htlc-v2", observers=(standby,))

        def taker_committed():
            # Chain probe, not the transient funding_b_broadcast flag (same
            # rationale as the rescue suite's committed_leg_b).
            t = swap_of(taker)
            if t is None or not t.get("funding_b_txid"):
                return False
            return h.btc.rpc("gettxout", t["funding_b_txid"],
                             t.get("funding_b_vout") or 0) is not None

        sid = None
        for _ in range(60):
            tick_all("drive", maker, taker)
            tick_all("standby", standby)
            if taker_committed():
                sid = swap_of(taker)["swap_id"]
                break
            if handshake_done(maker, taker):
                mine_and_sync(h, ep, eb)
        assert sid, "taker never committed leg B"
        pre_b = swap_of(taker, sid)["funding_b_txid"]
        print(f"[takeover-e2e] taker committed leg B ({pre_b[:16]}) — killing it")
        _kill(taker)

        _await_follow(standby, sid, "import")
        standby.rpc("takeover", sid)
        assert swap_of(standby, sid).get("source") == "local", \
            "takeover did not adopt on the v2 taker standby"
        done = False
        for _ in range(40):
            tick_all("finish", standby, maker)
            s, m = swap_of(standby, sid), swap_of(maker, sid)
            if s and m and s["state"] == "completed" and m["state"] == "completed":
                done = True
                break
            mine_and_sync(h, ep, eb)
        assert done, (f"v2 taker takeover never completed: "
                      f"standby={swap_of(standby, sid)} maker={swap_of(maker, sid)}")
        assert swap_of(standby, sid)["funding_b_txid"] == pre_b, \
            "v2 taker standby re-funded leg B (double-fund!)"
        print("[takeover-e2e] v2 taker-committed takeover OK: adopted, completed "
              "on the original leg-B funding")
    finally:
        for p in (maker, taker, standby):
            try:
                p.stop()
            except Exception:  # noqa: BLE001
                pass
        relay.stop()


def scenario_prefund_takeover_aborts_blind(h, ep, eb):
    """The fire-and-forget promise of the takeover dialog's pre-funding note,
    UNPROVABLE branch: takeover of a swap with nothing locked, on a standby
    whose backend cannot positively prove the leg unfunded (Core-RPC only, no
    electrs view). The #191 F3 belt must block any funding broadcast and the
    (test-shrunk) pre-funding timeout must end the swap in a CLEAN abort that
    also reaches the counterparty. Nothing may hit the chain."""
    relay = NostrRelay(h.workdir)
    maker = Party("pbmk", h, h.workdir, "alice_pocx", "alice_btc",
                  nostr_relays=relay.ws_url, auto_fund=False, auto_init=False)
    taker = Party("pbtk", h, h.workdir, "bob_pocx", "bob_btc",
                  nostr_relays=relay.ws_url, auto_fund=True, auto_init=True)
    # Core-only URLs (the Party default) — the blind-backup shape. The tiny
    # pre-funding window makes the clean abort observable within the test.
    standby = Party("pbsb", h, h.workdir, "alice_pocx", "alice_btc",
                    nostr_relays=relay.ws_url, auto_fund=True, auto_init=False,
                    extra_env={"PACT_TEST_PREFUNDING_TIMEOUT_SECS": "5"})
    try:
        relay.start()
        maker.start()
        taker.start()
        standby.start()
        mnemonic = maker.setup_seed()
        standby.setup_seed(mnemonic=mnemonic)

        _relay_handshake(h, maker, taker, "pact-htlc-v1", observers=(standby,))

        # Handshake to accepted/accepted — NO mining, nothing funds (maker
        # auto_fund=False holds the swap pre-funding indefinitely). Wait for
        # the maker to PROCESS the accept ("accepted"): that tick publishes
        # its one v1 snapshot, the standby's only follow source pre-funding.
        sid = None
        for _ in range(40):
            tick_all("drive", maker, taker)
            tick_all("standby", standby)
            m, t = swap_of(maker), swap_of(taker)
            if m and t and m["state"] == "accepted":
                sid = m["swap_id"]
                break
        assert sid, "pre-funding handshake never established on both sides"
        rec = swap_of(maker, sid)
        assert not rec.get("htlc_a_txid") and not rec.get("htlc_b_txid"), \
            f"scenario must hold PRE-funding, but a leg funded: {rec}"

        _await_follow(standby, sid, "import", others=(maker, taker))
        print(f"[takeover-e2e] pre-funding swap {sid[:16]} followed — killing the maker")
        _kill(maker)

        standby.rpc("takeover", sid)
        assert swap_of(standby, sid).get("source") == "local", \
            "pre-funding takeover did not adopt"

        # Fire and forget: no funding may appear (belt), and within the shrunk
        # window the standby must abort CLEANLY and notify the taker.
        aborted = False
        for _ in range(40):
            tick_all("abort", standby, taker)
            s, t = swap_of(standby, sid), swap_of(taker, sid)
            s_dead = s is None or s["state"] == "aborted"
            t_dead = t is None or t["state"] == "aborted"
            if s_dead and t_dead:
                aborted = True
                break
            time.sleep(0.5)
        assert aborted, (f"pre-funding takeover did not end in a clean abort: "
                         f"standby={swap_of(standby, sid)} taker={swap_of(taker, sid)}")
        final = swap_of(standby, sid)
        assert final is None or not final.get("htlc_a_txid"), \
            f"blind standby broadcast a funding for an adopted pre-funding swap: {final}"
        print("[takeover-e2e] blind pre-funding takeover OK: belt blocked funding, "
              "clean abort reached both sides")
    finally:
        for p in (maker, taker, standby):
            try:
                p.stop()
            except Exception:  # noqa: BLE001
                pass
        relay.stop()


def scenario_prefund_takeover_funds_provable(h, ep, eb):
    """The fire-and-forget promise, PROVABLE branch: the same pre-funding
    takeover on a standby WITH an electrs chain view. The F3 belt's positive
    `Unfunded` proof must let the adopted swap fund automatically and run to
    completion — the "continues only if this wallet can verify" half of the
    dialog copy."""
    relay = NostrRelay(h.workdir)
    maker_bx, maker_bt = multi_urls(h, ep, eb, "alice_pocx", "alice_btc")
    taker_bx, taker_bt = multi_urls(h, ep, eb, "bob_pocx", "bob_btc")
    maker = Party("ppmk", h, h.workdir, "alice_pocx", "alice_btc",
                  nostr_relays=relay.ws_url, auto_fund=False, auto_init=False,
                  pocx_url=maker_bx, btc_url=maker_bt)
    taker = Party("pptk", h, h.workdir, "bob_pocx", "bob_btc",
                  nostr_relays=relay.ws_url, auto_fund=True, auto_init=True,
                  pocx_url=taker_bx, btc_url=taker_bt)
    standby = Party("ppsb", h, h.workdir, "alice_pocx", "alice_btc",
                    nostr_relays=relay.ws_url, auto_fund=True, auto_init=False,
                    pocx_url=maker_bx, btc_url=maker_bt)
    try:
        relay.start()
        maker.start()
        taker.start()
        standby.start()
        mnemonic = maker.setup_seed()
        standby.setup_seed(mnemonic=mnemonic)

        _relay_handshake(h, maker, taker, "pact-htlc-v1", observers=(standby,))

        sid = None
        for _ in range(40):
            tick_all("drive", maker, taker)
            tick_all("standby", standby)
            m, t = swap_of(maker), swap_of(taker)
            if m and t and m["state"] == "accepted":
                sid = m["swap_id"]
                break
        assert sid, "pre-funding handshake never established on both sides"
        assert not swap_of(maker, sid).get("htlc_a_txid"), \
            "scenario must hold PRE-funding"

        _await_follow(standby, sid, "import", others=(maker, taker))
        print(f"[takeover-e2e] pre-funding swap {sid[:16]} followed — killing the maker")
        _kill(maker)

        standby.rpc("takeover", sid)
        assert swap_of(standby, sid).get("source") == "local", \
            "pre-funding takeover did not adopt"

        # The belt's positive Unfunded proof (electrs view) lets auto-fund
        # proceed; the swap must then complete normally with the taker.
        done = False
        for _ in range(40):
            tick_all("finish", standby, taker)
            s, t = swap_of(standby, sid), swap_of(taker, sid)
            if s and t and s["state"] == "completed" and t["state"] == "completed":
                done = True
                break
            mine_and_sync(h, ep, eb)
        assert done, (f"provable pre-funding takeover never completed: "
                      f"standby={swap_of(standby, sid)} taker={swap_of(taker, sid)}")
        assert swap_of(standby, sid).get("htlc_a_txid"), \
            "completed without a leg-A funding pointer on the standby?"
        print("[takeover-e2e] provable pre-funding takeover OK: belt proved "
              "unfunded, standby funded and completed the swap")
    finally:
        for p in (maker, taker, standby):
            try:
                p.stop()
            except Exception:  # noqa: BLE001
                pass
        relay.stop()


def scenario_taker_post_reveal_takeover_v1(h, ep, eb):
    """Takeover in the REDEEM phase: the maker has redeemed leg B (the secret
    is public on chain) and the taker dies before claiming leg A. The standby
    adopts and must finish leg A from the chain-visible reveal — the takeover
    twin of the rescue suite's post_reveal cells."""
    relay = NostrRelay(h.workdir)
    maker_bx, maker_bt = multi_urls(h, ep, eb, "alice_pocx", "alice_btc")
    taker_bx, taker_bt = multi_urls(h, ep, eb, "bob_pocx", "bob_btc")
    maker = Party("prmk", h, h.workdir, "alice_pocx", "alice_btc",
                  nostr_relays=relay.ws_url, auto_fund=True, auto_init=True,
                  pocx_url=maker_bx, btc_url=maker_bt)
    taker = Party("prtk", h, h.workdir, "bob_pocx", "bob_btc",
                  nostr_relays=relay.ws_url, auto_fund=True, auto_init=False,
                  pocx_url=taker_bx, btc_url=taker_bt)
    standby = Party("prsb", h, h.workdir, "bob_pocx", "bob_btc",
                    nostr_relays=relay.ws_url, auto_fund=True, auto_init=False,
                    pocx_url=taker_bx, btc_url=taker_bt)
    try:
        relay.start()
        maker.start()
        taker.start()
        standby.start()
        mnemonic = taker.setup_seed()
        standby.setup_seed(mnemonic=mnemonic)

        _relay_handshake(h, maker, taker, "pact-htlc-v1", observers=(standby,))

        sid = None
        for _ in range(60):
            tick_all("drive", maker, taker)
            tick_all("standby", standby)
            m = swap_of(maker)
            if m is not None and m["state"] == "redeemed_b":
                sid = m["swap_id"]
                break
            if handshake_done(maker, taker):
                mine_and_sync(h, ep, eb)
        assert sid, "maker never revealed (redeemed_b)"
        pre_b = swap_of(taker, sid)["htlc_b_txid"]
        print(f"[takeover-e2e] reveal is public ({sid[:16]}) — killing the taker "
              "before its leg-A claim")
        _kill(taker)

        _await_follow(standby, sid, "import")
        standby.rpc("takeover", sid)
        assert swap_of(standby, sid).get("source") == "local", \
            "post-reveal takeover did not adopt"
        done = False
        for _ in range(40):
            tick_all("finish", standby, maker)
            s, m = swap_of(standby, sid), swap_of(maker, sid)
            if s and m and s["state"] == "completed" and m["state"] == "completed":
                done = True
                break
            mine_and_sync(h, ep, eb)
        assert done, (f"post-reveal takeover never completed: "
                      f"standby={swap_of(standby, sid)} maker={swap_of(maker, sid)}")
        post = swap_of(standby, sid)
        assert post["htlc_b_txid"] == pre_b, \
            "post-reveal standby re-funded leg B (double-fund!)"
        # The leg-A HTLC outpoint must be SPENT — the standby's claim, made
        # from the chain-visible secret alone.
        assert h.pocx.rpc("gettxout", post["htlc_a_txid"],
                          post.get("htlc_a_vout") or 0) is None, \
            f"leg-A HTLC still unspent — the standby never claimed it: {post}"
        print("[takeover-e2e] post-reveal taker takeover OK: adopted, claimed "
              "leg A from the chain-visible secret")
    finally:
        for p in (maker, taker, standby):
            try:
                p.stop()
            except Exception:  # noqa: BLE001
                pass
        relay.stop()


class RefundOnlyTakeoverV2(_FollowScenario):
    scenario = staticmethod(scenario_refund_only_takeover_v2)


class HotStandbyTakeoverV1(_FollowScenario):
    scenario = staticmethod(scenario_hot_standby_takeover_v1)


class TakerHotStandbyV1(_FollowScenario):
    scenario = staticmethod(scenario_taker_hot_standby_v1)


class TakerCommittedTakeoverV2(_FollowScenario):
    scenario = staticmethod(scenario_taker_committed_takeover_v2)


class PrefundTakeoverAbortsBlind(_FollowScenario):
    scenario = staticmethod(scenario_prefund_takeover_aborts_blind)


class PrefundTakeoverFundsProvable(_FollowScenario):
    scenario = staticmethod(scenario_prefund_takeover_funds_provable)


class TakerPostRevealTakeoverV1(_FollowScenario):
    scenario = staticmethod(scenario_taker_post_reveal_takeover_v1)


SCENARIOS = [
    RefundOnlyTakeoverV2,
    HotStandbyTakeoverV1,
    TakerHotStandbyV1,
    TakerCommittedTakeoverV2,
    PrefundTakeoverAbortsBlind,
    PrefundTakeoverFundsProvable,
    TakerPostRevealTakeoverV1,
]


if __name__ == "__main__":
    run_scenarios(SCENARIOS)
