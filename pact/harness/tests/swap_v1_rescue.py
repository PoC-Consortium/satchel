#!/usr/bin/env python3
"""Seed-only mid-swap rescue matrix (#54): {maker,taker} x {accepted,
funded_a, committed, post_reveal} x {v1,v2} cells + the refund variant, split
out of the former test_swap_e2e.py. Bodies verbatim; each cell runs on its
own fresh cached stack, so the per-cell distinct mnemonics are belt-and-
suspenders now (kept: they also guard against a leaked relay's stale DB).

Run:  python tests/swap_v1_rescue.py [--filter SUBSTR] [--keep] [--no-build]
"""

import shutil
import time
import os
import sys

sys.path.insert(0, os.path.normpath(
    os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")))

from framework.daemon import Party  # noqa: E402
from framework.services import NostrRelay  # noqa: E402
from framework.testbase import PactTestFramework, run_scenarios  # noqa: E402
from framework.util import FEE_SLACK, GET_BTC, GIVE_POCX, balances  # noqa: E402


# Fixed BIP39 test vectors — deterministic identities so a wiped party can be
# re-provisioned with the SAME seed (thus same npub + swap keys) after its data
# dir is destroyed. DISTINCT per scenario: same seed ⇒ same npub ⇒ the rescue
# would (correctly!) also pull the OTHER scenario's snapshot, muddying the test.
# Standard BIP39 English test vectors, except M2/R1: deterministic phrases from
# entropy 0x00010203…0e0f / 0x10111213…1e1f (checksum-valid). NOT for real funds.
RESCUE_MNEMONIC_V1 = ("abandon abandon abandon abandon abandon abandon abandon "
                      "abandon abandon abandon abandon about")
RESCUE_MNEMONIC_V2 = ("legal winner thank year wave sausage worth useful legal "
                      "winner thank yellow")
RESCUE_MNEMONIC_M1 = ("letter advice cage absurd amount doctor acoustic avoid "
                      "letter advice cage above")
RESCUE_MNEMONIC_T1 = "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong"
RESCUE_MNEMONIC_T2 = ("ozone drill grab fiber curtain grace pudding thank "
                      "cruise elder eight picnic")
RESCUE_MNEMONIC_M2 = ("abandon amount liar amount expire adjust cage candy "
                      "arch gather drum buyer")
RESCUE_MNEMONIC_R1 = ("avoid mass luggage choice fabric argue gather cash "
                      "brand thought elegant divide")
RESCUE_MNEMONIC_T2V2 = ("army van defense carry jealous true garbage claim "
                        "echo media make crunch")


def _rescue_scenario(h, protocol, tag, mnemonic, victim="taker",
                     stage="committed", refund=False, fund_refused_when_blind=False):
    """Seed-only mid-swap rescue (#54) over a live Nostr relay — one cell of
    the wipe-stage matrix {maker,taker} × {accepted, funded_a, committed,
    post_reveal} × {v1,v2}, plus a refund variant.

    Drive a board swap until `stage` is reached, then DESTROY the victim's
    pactd data dir — its swap state, relay cursors and Pact seed, exactly like
    a dead laptop. Re-provision a fresh pactd with the SAME seed, assert the
    rescue surfaces (pactd detects + warns; the relay scan auto-imports the
    now-foreign snapshot as a FOLLOWED read-only record — #163 — but nothing
    auto-ADOPTS), adopt via the explicit `takeover`, and drive to completion —
    or, with `refund=True`, jump past the timelocks and drive to the timeout
    reclaim.
    Asserts the victim re-broadcasts NOTHING it already funded: its own leg's
    txid is IDENTICAL before the wipe and after settlement (adopted, not
    re-funded).

    Stages (when the wipe lands):
      accepted     the victim's record (and its accept snapshot) exists;
                   nothing of the victim's is on chain yet.
      funded_a     the maker's leg A is on chain (v1 only here: v1 funds
                   serially, so leg B is guaranteed not committed yet).
      committed    the taker's leg B is on the wire — funds at stake on both
                   legs, swap not settled.
      post_reveal  the maker has redeemed leg B, publishing the secret; a
                   wiped taker must finish leg A from the chain alone.

    Balance probes use the clean RECEIVING wallets only (bob_pocx, alice_btc);
    the funding wallets creep from maturing coinbase and mining rewards, so
    settlement and refunds are asserted via swap state / outpoint spends
    where balances can't signal.
    """
    # The victim gets the fixed seed (auto_init off → importseed); its funding
    # behavior must survive the restart identically. The refund variant keeps
    # the taker from ever funding leg B, so the maker times out.
    relay = NostrRelay(h.workdir)
    # v2 maker@committed: on 1-conf regtest the maker races signed →
    # redeemed_b → completed inside one round, so the wipe window never opens.
    # A deeper leg-B requirement holds the maker at Signed for a few blocks —
    # exactly the state whose snapshot (assembled adaptor sigs) is under test.
    maker_confs = ({"btc": 3} if victim == "maker" and stage == "committed"
                   and protocol.endswith("v2") else None)
    maker = Party(f"mk{tag}", h, h.workdir, "alice_pocx", "alice_btc",
                  nostr_relays=relay.ws_url, auto_fund=True,
                  coin_confs=maker_confs,
                  auto_init=(victim != "maker"))
    taker = Party(f"tk{tag}", h, h.workdir, "bob_pocx", "bob_btc",
                  nostr_relays=relay.ws_url, auto_fund=not refund,
                  auto_init=(victim != "taker"))
    victim_party = maker if victim == "maker" else taker
    victim_data_dir = victim_party.data_dir

    def swap_of(p, sid=None):
        # After the restore, the replayed relay history can spawn a ghost
        # handshake record (the rescued node lost its relay cursor with the
        # wipe) — pin post-restore reads to the swap under test via `sid`.
        sw = (p.rpc("listswaps") or []) + (p.rpc("listadaptorswaps") or [])
        if sid is not None:
            sw = [s for s in sw if s["swap_id"] == sid]
        return sw[0] if sw else None

    def leg_a_txid(s):
        # v1 HTLC vs v2 adaptor field names.
        return None if s is None else (s.get("htlc_a_txid") or s.get("funding_a_txid"))

    def leg_b_txid(s):
        return None if s is None else (s.get("htlc_b_txid") or s.get("funding_b_txid"))

    def own_leg_txid(s):
        # The leg the victim itself funds: maker → A, taker → B.
        return leg_a_txid(s) if victim == "maker" else leg_b_txid(s)

    def committed_leg_b(s):
        # "Leg B is on the wire", with a wide detection window that persists to
        # completion (a mempool probe only hits for the single unconfirmed round).
        # v1 sets htlc_b_txid only at the funding broadcast; v2 records
        # funding_b_txid at BUILD (too early) but flips funding_b_broadcast at the
        # two-phase broadcast.
        if s is None:
            return False
        if s.get("htlc_b_txid"):  # v1 HTLC
            return True
        return s.get("funding_b_broadcast") is True  # v2 adaptor

    def stage_reached():
        if stage == "accepted":
            return swap_of(victim_party) is not None
        if stage == "funded_a":
            return leg_a_txid(swap_of(maker)) is not None
        if stage == "committed":
            if not committed_leg_b(swap_of(taker)):
                return False
            if victim == "maker" and protocol.endswith("v2"):
                # The v2 maker assembles (Signed) one relay round AFTER the
                # taker commits — wipe only once ITS Signed snapshot exists. A
                # maker wiped inside the accept→Signed gap cannot complete by
                # design (the assembled adaptor sigs are the one datum that is
                # neither seed- nor chain-derivable); it falls back to the
                # timelock refund, which the refund cell covers.
                m = swap_of(maker)
                return m is not None and m["state"] in (
                    "signed", "funded_a", "funded_b")
            return True
        if stage == "post_reveal":
            m = swap_of(maker)
            return m is not None and m["state"] == "redeemed_b"
        raise AssertionError(f"unknown wipe stage: {stage}")

    try:
        # Everything runs inside the try — a failure during startup/seed setup
        # must still tear down relay + pactds, or the leaked relay poisons
        # every later scenario on the same fixed port with its stale DB.
        relay.start()
        maker.start()
        taker.start()
        victim_party.setup_seed(mnemonic=mnemonic)
        before = balances(h)

        offer_id = maker.rpc(
            "boardpostoffer", f"btcx:{GIVE_POCX}", f"btc:{GET_BTC}",
            4 * 3600, 2 * 3600, protocol)["offer_id"]
        # Relay propagation is async (publish our outbox / fetch theirs per tick);
        # poll until the taker sees the offer, then take it.
        for _ in range(25):
            maker.rpc("tick")
            taker.rpc("tick")
            if any(o["swap_id"] == offer_id
                   for o in taker.rpc("boardlistoffers")["offers"]):
                break
        else:
            raise AssertionError("offer never propagated to the taker over the relay")
        taker.rpc("boardtake", offer_id)

        # Drive until the wipe stage is reached. Break the INSTANT it is, so
        # the wipe lands mid-flight.
        reached = False
        for _round in range(50):
            for party in (maker, taker):
                evs = party.rpc("tick")["events"]
                for ev in evs:
                    print(f"[e2e]   {tag}[{party.name}]: {ev['action']} {ev['detail'][:70]}")
            if stage_reached():
                reached = True
                break
            h.pocx.generate(1, "alice_pocx")
            h.btc.generate(1, "bob_btc")
        assert reached, f"stage '{stage}' never reached before the wipe"
        pre_rec = swap_of(victim_party)
        pre_own_leg = own_leg_txid(pre_rec)
        if stage not in ("accepted",):
            assert pre_own_leg or victim == "taker" and stage == "funded_a", \
                f"no own-leg txid recorded pre-wipe at stage {stage}: {pre_rec}"
        # The snapshot rides the Nostr outbox and the on-tick relay pass is
        # asynchronous — give it two more ticks and a beat so the snapshot is
        # ON the relay before the wipe destroys the outbox.
        for _ in range(2):
            victim_party.rpc("tick")
            time.sleep(1)
        sid = pre_rec["swap_id"]
        print(f"[e2e] {tag}: {victim} @ {stage} "
              f"(own leg: {str(pre_own_leg)[:16]}) — wiping mid-swap")

        # --- the crash: destroy the victim's pactd state entirely ---
        victim_party.stop()
        for attempt in range(10):  # Windows can lag releasing the dead proc's handles
            try:
                shutil.rmtree(victim_data_dir)
                break
            except PermissionError:
                time.sleep(0.5)
        else:
            shutil.rmtree(victim_data_dir)  # last try: surface the error

        # --- the rescue: a fresh pactd on the SAME (now-wiped) data dir, same
        # seed. The Bitcoin Core wallets are the node's, untouched by the wipe. ---
        fresh = Party(victim_party.name, h, h.workdir,
                      "alice_pocx" if victim == "maker" else "bob_pocx",
                      "alice_btc" if victim == "maker" else "bob_btc",
                      nostr_relays=relay.ws_url,
                      auto_fund=(victim == "maker" or not refund),
                      coin_confs=maker_confs if victim == "maker" else None,
                      auto_init=False)
        assert fresh.data_dir == victim_data_dir, "restart must reuse the wiped data dir"
        fresh.start()
        if victim == "maker":
            maker = fresh
        else:
            taker = fresh
        victim_party = fresh
        # Importing the seed re-establishes our identity. #54 decision 1: pactd
        # only DETECTS recoverable snapshots — nothing may auto-ADOPT (a still-
        # live machine on the same seed could be driving the swap; two drivers
        # can double-fund it). Assert `rescuestatus` reports the snapshot as
        # pending WITH the two-machines warning. Deterministic: the followed-
        # import scan below runs only on a tick (this daemon has --tick-secs 0)
        # and none has run yet, so the swap list is still empty here.
        victim_party.setup_seed(mnemonic=mnemonic)
        st = None
        for _ in range(20):
            st = victim_party.rpc("rescuestatus")
            if st["pending"] > 0:
                break
            time.sleep(0.5)
        assert st and st["pending"] >= 1, \
            f"rescuestatus never saw the relay snapshot: {st}"
        assert st.get("warning"), "a pending rescue must carry the two-machines warning"
        # Multi-machine (#122/#134/#163): the wipe destroyed machine.json too,
        # so the fresh install minted a NEW derive scope — the old snapshot
        # carries the OLD scope and reads as ANOTHER MACHINE's swap. The
        # periodic relay scan therefore auto-imports it as a FOLLOWED record
        # WITHOUT any confirm: visibility is ungated by design; the confirm
        # gate protects DRIVING. Assert it appears, is read-only
        # (source=foreign — i.e. not driven), and stays that way until the
        # explicit takeover.
        rec = None
        for _ in range(20):
            victim_party.rpc("tick")  # each tick kicks a followed-import scan
            rec = swap_of(victim_party, sid)
            if rec is not None:
                break
            time.sleep(0.5)
        assert rec is not None, \
            "foreign snapshot never auto-imported as a followed record (#163)"
        assert rec.get("source") == "foreign", \
            f"auto-imported record must be FOLLOWED (read-only), not driven: {rec}"
        # The explicit restore is now a no-op for this swap (local record wins)
        # and — crucially — must NOT flip it to driven: adoption only ever
        # happens via `takeover`.
        r = victim_party.rpc("restorefromrelay")
        assert r["restored"] == 0, \
            f"restorefromrelay re-imported an already-followed swap: {r}"
        rec = swap_of(victim_party, sid)
        assert rec.get("source") == "foreign", \
            f"restorefromrelay must not adopt a followed record: {rec}"
        # `takeover` is the explicit dead-is-dead confirm that adopts the swap
        # — true here by construction (the old pactd is stopped and its state
        # destroyed). Without it the rescued party would observe the swap
        # forever and never redeem/refund.
        victim_party.rpc("takeover", sid)
        rec = swap_of(victim_party, sid)
        assert rec.get("source") == "local", \
            f"takeover did not adopt the restored swap: {rec}"

        if fund_refused_when_blind:
            # F3 fail-closed belt: this Core-only backup (no script index) is
            # blind to the counterparty's leg, so it CANNOT prove its own leg is
            # unfunded — it must REFUSE to broadcast a fresh funding rather than
            # risk re-funding a swap that may have already settled (whose secret
            # would be public → theft). The old "blind backup continues a
            # pre-funding swap" path is deliberately gone. Drive a while; assert
            # the backup NEVER broadcasts its leg and the swap never completes —
            # it safely stalls (and would time out both sides).
            for _ in range(15):
                for party in (maker, taker):
                    party.rpc("tick")
                h.pocx.generate(1, "alice_pocx")
                h.btc.generate(1, "bob_btc")
                v = swap_of(victim_party, sid)
                assert own_leg_txid(v) is None, \
                    f"blind backup re-funded its leg despite the F3 belt: {v}"
                assert v is None or v["state"] != "completed", \
                    f"blind adopted swap completed via an unverified re-fund: {v}"
            print(f"[e2e] {tag}: F3 belt held — blind backup refused to fund its leg")

            # No forever ghost: past the §7.4 fund/confirm deadline the stalled
            # adopted record auto-aborts (nothing of ours is committed — the
            # counterparty reclaims its own leg at its timelock), so it leaves the
            # active dock instead of lingering. These deadlines are chain-time, so
            # advance_time triggers them.
            h.advance_time(5 * 3600)
            terminal = False
            for _ in range(30):
                for party in (maker, taker):
                    party.rpc("tick")
                h.pocx.generate(1, "alice_pocx")
                h.btc.generate(1, "bob_btc")
                v = swap_of(victim_party, sid)
                if v is not None and v["state"] in ("aborted", "refunded"):
                    terminal = True
                    break
            assert terminal, \
                f"stalled adopted swap never terminalized — forever ghost: {swap_of(victim_party, sid)}"
            assert own_leg_txid(swap_of(victim_party, sid)) is None, \
                "re-funded its leg on the way to the abort"
            print(f"[e2e] {tag}: stalled swap auto-aborted past the deadline — no ghost")
            return

        # The snapshot was taken at `accept`, BEFORE any funding — the rescued
        # record has no funding pointers; the tick rediscovers them on chain
        # below. (The no-double-fund check compares txids after settlement.)
        print(f"[e2e] {tag}: swap {sid[:16]} auto-followed from relay snapshot + taken over")

        # Whatever was on the wire at the wipe may still be unconfirmed.
        # find_funding is confirmed-only, so bury it first — otherwise the
        # rescued party wouldn't SEE its own funding and might re-fund it. This
        # is the exact ordering a real recovery faces.
        for _ in range(4):
            h.pocx.generate(1, "alice_pocx")
            h.btc.generate(1, "bob_btc")

        if refund:
            # The taker never funds leg B (auto_fund off): jump past both
            # timelocks and let the RESCUED maker's scheduler time out and
            # reclaim its leg A — the C8 timeout path re-driven from a rescued
            # record whose funding pointer had to be rediscovered on chain.
            a_txid = pre_rec.get("htlc_a_txid") or pre_rec.get("funding_a_txid")
            a_vout = pre_rec.get("htlc_a_vout")
            if a_vout is None:
                a_vout = pre_rec.get("funding_a_vout")
            h.advance_time(5 * 3600)
            done = False
            for _ in range(40):
                for party in (maker, taker):
                    evs = party.rpc("tick")["events"]
                    for ev in evs:
                        print(f"[e2e]   {tag}[{party.name}]: {ev['action']} {ev['detail'][:70]}")
                h.pocx.generate(1, "alice_pocx")
                h.btc.generate(1, "bob_btc")
                v = swap_of(victim_party, sid)
                # The funding wallet's balance can't signal (coinbase creep):
                # refunded = terminal state + the leg-A outpoint SPENT on chain.
                spent = h.pocx.rpc("gettxout", a_txid, a_vout) is None
                if v is not None and v["state"] in ("aborted", "refunded") and spent:
                    done = True
                    break
            assert done, f"rescued maker never reclaimed leg A: {swap_of(victim_party, sid)}"
            post_own_leg = own_leg_txid(swap_of(victim_party, sid))
            assert post_own_leg == pre_own_leg, \
                f"maker re-funded leg A (double-fund!): {pre_own_leg} -> {post_own_leg}"
            print(f"[e2e] {tag}: seed-only rescue refunded; leg A reclaimed, not re-funded")
            return

        # Drive both to completion — the rescued party rediscovers funding on
        # chain and settles via chain-watch alone.
        done = False
        for _ in range(40):
            for party in (maker, taker):
                evs = party.rpc("tick")["events"]
                for ev in evs:
                    print(f"[e2e]   {tag}[{party.name}]: {ev['action']} {ev['detail'][:70]}")
            h.pocx.generate(1, "alice_pocx")
            h.btc.generate(1, "bob_btc")
            now = balances(h)
            # Receiving legs are clean: taker got the POCX leg, maker got the BTC leg.
            if (now["bob_pocx"] >= before["bob_pocx"] + float(GIVE_POCX) - FEE_SLACK
                    and now["alice_btc"] >= before["alice_btc"] + float(GET_BTC) - 0.0005):
                done = True
                break
        after = balances(h)
        assert done, f"rescued swap did not complete: before={before}, after={after}"
        # No double-fund: the rescued victim must have ADOPTED its existing
        # funding, not broadcast a second one — its own leg's txid is unchanged.
        # Exception: rediscovery only re-points UNSPENT funding, so a leg that
        # was already spent at the wipe (post_reveal) legitimately stays
        # pointerless — prove settlement went through the ORIGINAL funding by
        # its outpoint being spent on chain instead.
        if pre_own_leg is not None:
            post_own_leg = own_leg_txid(swap_of(victim_party, sid))
            assert post_own_leg in (pre_own_leg, None), \
                f"{victim} re-funded its leg (double-fund!): {pre_own_leg} -> {post_own_leg}"
            if post_own_leg is None:
                node = h.pocx if victim == "maker" else h.btc
                keys = (("htlc_a_vout", "funding_a_vout") if victim == "maker"
                        else ("htlc_b_vout", "funding_b_vout"))
                vout = pre_rec.get(keys[0])
                if vout is None:
                    vout = pre_rec.get(keys[1])
                assert node.rpc("gettxout", pre_own_leg, vout) is None, \
                    "own-leg pointer lost but the original funding is still unspent"
        print(f"[e2e] {tag}: seed-only rescue completed; own leg adopted, not re-funded")
    finally:
        maker.stop()
        taker.stop()
        relay.stop()


def test_swap_rescue_v1(h):
    """v1 HTLC: wipe the taker mid-swap (leg B committed), restore, complete."""
    _rescue_scenario(h, "pact-htlc-v1", "rcv1", RESCUE_MNEMONIC_V1)


def test_swap_rescue_v2(h):
    """v2 Taproot/adaptor: wipe the taker mid-swap (leg B committed), restore."""
    _rescue_scenario(h, "pact-htlc-v2", "rcv2", RESCUE_MNEMONIC_V2)


def test_rescue_v1_maker_funded_a(h):
    """v1: wipe the MAKER right after it funded leg A — the rescued initiator
    must rediscover its own funding, restore its counter and still complete
    (deterministic preimage re-derived from the restored index)."""
    _rescue_scenario(h, "pact-htlc-v1", "rcm1", RESCUE_MNEMONIC_M1,
                     victim="maker", stage="funded_a")


def test_rescue_v1_taker_accepted(h):
    """v1: wipe the taker at ACCEPT, before anything is on chain. The snapshot is
    still detected and auto-followed, and takeover ADOPTS — but the F3 belt then
    REFUSES to fund: a Core-only backup is blind to the counterparty's leg and
    can't prove its own leg is unfunded, so re-funding could pay a lock whose
    secret is already public on a settled swap. The backup safely stalls instead
    of re-funding. (Completing a pre-funding swap on the backup now requires a
    chain view (Electrum) for both legs, so the leg is provably unfunded.)"""
    _rescue_scenario(h, "pact-htlc-v1", "rct1", RESCUE_MNEMONIC_T1,
                     victim="taker", stage="accepted", fund_refused_when_blind=True)


def test_rescue_v1_taker_post_reveal(h):
    """v1: wipe the taker AFTER the maker revealed the preimage redeeming leg
    B — the rescued taker must find the spend, extract the secret and claim
    leg A from the chain alone."""
    _rescue_scenario(h, "pact-htlc-v1", "rct2", RESCUE_MNEMONIC_T2,
                     victim="taker", stage="post_reveal")


def test_rescue_v2_maker_committed(h):
    """v2: wipe the MAKER once the taker's leg B is on the wire (the maker is
    Signed — its snapshot carries the assembled adaptor sigs); the rescued
    maker must rediscover both legs and reveal t to completion."""
    _rescue_scenario(h, "pact-htlc-v2", "rcm2", RESCUE_MNEMONIC_M2,
                     victim="maker", stage="committed")


def test_rescue_v2_taker_post_reveal(h):
    """v2: wipe the TAKER after the maker revealed t (redeemed leg B) — the
    v2 analog of the v1 post-reveal cell, and the owner-reveals-then-dies
    takeover the audit flagged as uncovered. The rescued participant restores
    from its Signed snapshot, then must find the maker's leg-B cooperative
    redeem on chain, EXTRACT t from its witness (t replaces the v1 hash
    preimage) and claim leg A — proving the payout-gate change still lets a
    wallet-OWNING machine complete the redeem post-takeover."""
    _rescue_scenario(h, "pact-htlc-v2", "rct2v2", RESCUE_MNEMONIC_T2V2,
                     victim="taker", stage="post_reveal")


def test_rescue_v1_maker_refund(h):
    """v1 refund variant: the taker never funds leg B; the maker is wiped at
    funded_a and, once past the timelocks, the RESCUED maker must time out and
    reclaim leg A (C8 abort re-driven from a rescued record)."""
    _rescue_scenario(h, "pact-htlc-v1", "rcr1", RESCUE_MNEMONIC_R1,
                     victim="maker", stage="funded_a", refund=True)


class RescueTakerCommittedV1(PactTestFramework):
    def run_test(self):
        test_swap_rescue_v1(self.h)


class RescueTakerCommittedV2(PactTestFramework):
    def run_test(self):
        test_swap_rescue_v2(self.h)


class RescueMakerFundedAV1(PactTestFramework):
    def run_test(self):
        test_rescue_v1_maker_funded_a(self.h)


class RescueTakerAcceptedV1(PactTestFramework):
    def run_test(self):
        test_rescue_v1_taker_accepted(self.h)


class RescueTakerPostRevealV1(PactTestFramework):
    def run_test(self):
        test_rescue_v1_taker_post_reveal(self.h)


class RescueMakerCommittedV2(PactTestFramework):
    def run_test(self):
        test_rescue_v2_maker_committed(self.h)


class RescueMakerRefundV1(PactTestFramework):
    def run_test(self):
        test_rescue_v1_maker_refund(self.h)


class RescueTakerPostRevealV2(PactTestFramework):
    def run_test(self):
        test_rescue_v2_taker_post_reveal(self.h)


SCENARIOS = [
    RescueTakerCommittedV1,
    RescueTakerCommittedV2,
    RescueMakerFundedAV1,
    RescueTakerAcceptedV1,
    RescueTakerPostRevealV1,
    RescueMakerCommittedV2,
    RescueMakerRefundV1,
    RescueTakerPostRevealV2,
]


if __name__ == "__main__":
    run_scenarios(SCENARIOS)
