#!/usr/bin/env python3
"""v1 HTLC end-to-end scenarios on regtest — the former test_swap_e2e.py
monolith minus the rescue matrix (tests/swap_v1_rescue.py). Scenario bodies
are verbatim; each class wrapper runs one scenario on its own fresh cached
stack (framework/testbase.py, plan section 2.3).

Run:  python tests/swap_v1.py [--filter SUBSTR] [--keep] [--no-build]
"""

import json
import os
import sys

sys.path.insert(0, os.path.normpath(
    os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")))

from framework.daemon import Party  # noqa: E402
from framework.services import Corkboard, NostrRelay  # noqa: E402
from framework.testbase import PactTestFramework, run_scenarios  # noqa: E402
from framework.util import (  # noqa: E402
    FEE_SLACK,
    GET_BTC,
    GIVE_POCX,
    assert_htlc_spent,
    balances,
    drive_until,
    expect_fail,
    handshake_and_fund,
    handshake_done,
    load_msg,
    msg,
    outpoint_from,
    regtest_timelocks,
    save_msg,
    swap_id_from,
)


def test_complete_swap(h):
    """Happy path, fully manual: the Phase 1 definition of done.
    Each party runs its own pactd; pact-cli drives it (bitcoin-cli style)."""
    alice = Party("alice", h, h.workdir, "alice_pocx", "alice_btc").start()
    bob = Party("bob", h, h.workdir, "bob_pocx", "bob_btc").start()
    try:
        before = balances(h)

        sid, m_funded_a, m_funded_b = handshake_and_fund(h, alice, bob, "01")
        alice.cli("recv", "--in", m_funded_b)

        alice.cli("redeem", "--swap", sid)          # reveals s on the BTC chain
        h.btc.generate(1, "bob_btc")
        bob.cli("redeem", "--swap", sid)            # engine extracted s from chain B
        h.pocx.generate(1, "alice_pocx")

        assert_htlc_spent(h.pocx, m_funded_a, "chain-A")
        assert_htlc_spent(h.btc, m_funded_b, "chain-B")

        after = balances(h)
        assert after["bob_pocx"] >= float(GIVE_POCX) - FEE_SLACK, \
            f"Bob did not receive POCX: {after}"
        assert after["alice_btc"] >= float(GET_BTC) - FEE_SLACK, \
            f"Alice did not receive BTC: {after}"
        assert after["alice_pocx"] <= before["alice_pocx"] - float(GIVE_POCX) + 10 * 3, \
            f"Alice's POCX did not decrease plausibly: {after}"
        print("[e2e] complete-swap scenario OK")
    finally:
        alice.stop()
        bob.stop()


def test_refund(h):
    """Manual refund path + negative safety checks."""
    alice = Party("alice2", h, h.workdir, "alice_pocx", "alice_btc").start()
    bob = Party("bob2", h, h.workdir, "bob_pocx", "bob_btc").start()
    try:
        before = balances(h)

        sid, m_funded_a, m_funded_b = handshake_and_fund(h, alice, bob, "11")

        # Premature refunds must be rejected (MTP < T2 < T1).
        expect_fail(bob, "premature Bob refund", "refund", "--swap", sid)
        expect_fail(alice, "premature Alice refund", "refund", "--swap", sid)

        # Alice goes silent. Push both chains' MTP past T1 (> T2 too).
        h.advance_time(5 * 3600)

        # §7.4 reveal deadline: with MTP past T2, redeeming would risk both
        # legs — the engine must refuse even though the HTLC is still there.
        # (Alice never received funded_b, so first deliver it to set FundedB.)
        alice.cli("recv", "--in", m_funded_b)
        expect_fail(alice, "late Alice redeem past T2", "redeem", "--swap", sid)

        bob.cli("refund", "--swap", sid)     # valid once MTP >= T2
        h.btc.generate(1, "bob_btc")
        alice.cli("refund", "--swap", sid)   # valid once MTP >= T1
        h.pocx.generate(1, "alice_pocx")

        assert_htlc_spent(h.pocx, m_funded_a, "chain-A")
        assert_htlc_spent(h.btc, m_funded_b, "chain-B")

        after = balances(h)
        # Mining rewards accrue to alice_pocx/bob_btc, so check the *other*
        # side of each leg: nobody ended up with counterparty funds.
        assert after["bob_pocx"] <= before["bob_pocx"] + FEE_SLACK, \
            f"Bob must not gain POCX in refund scenario: {after}"
        assert after["alice_btc"] <= before["alice_btc"] + FEE_SLACK, \
            f"Alice must not gain BTC in refund scenario: {after}"
        print("[e2e] refund scenario OK")
    finally:
        alice.stop()
        bob.stop()


def test_daemon_autopilot_swap(h):
    """Alice runs entirely through pactd's JSON-RPC API (with duplicated
    backends to exercise the spec §10 multi-backend path); redeems on both
    sides happen via the scheduler, with an RBF fee-bump while Alice's
    redeem sits unconfirmed."""
    alice = Party("alice3", h, h.workdir, "alice_pocx", "alice_btc",
                  duplicate_backends=True).start()
    bob = Party("bob3", h, h.workdir, "bob_pocx", "bob_btc").start()
    try:
        before = balances(h)

        t2, t1 = regtest_timelocks(h)
        m_init = msg(h.workdir, "21_init.json")
        m_accept = msg(h.workdir, "22_accept.json")
        m_funded_a = msg(h.workdir, "23_funded_a.json")
        m_funded_b = msg(h.workdir, "24_funded_b.json")

        # Auth: a request with no/invalid cookie must be rejected (401).
        try:
            alice.rpc("listswaps", auth=False)
            raise AssertionError("API accepted a request without auth")
        except RuntimeError as exc:
            assert "401" in str(exc), f"expected 401, got: {exc}"
            print("[e2e] correctly rejected: JSON-RPC call without cookie")

        # Wallet RPCs: balance, fresh address, send (self-send).
        balance = alice.rpc("getbalance", "btcx")["balance_sat"]
        assert balance > 0, f"alice should have POCX: {balance}"
        addr = alice.rpc("getnewaddress", "btcx")["address"]
        assert addr.startswith("rpocx1"), f"unexpected address: {addr}"
        r = alice.rpc("sendtoaddress", "btcx", addr, "1.0")
        assert len(r["txid"]) == 64, f"send failed: {r}"
        # Wrong-chain address must be refused before money moves.
        btc_addr = alice.rpc("getnewaddress", "btc")["address"]
        try:
            alice.rpc("sendtoaddress", "btcx", btc_addr, "1.0")
            raise AssertionError("sent POCX to a BTC address")
        except RuntimeError:
            pass
        print("[e2e] wallet RPCs OK (balance/receive/send + chain guard)")

        r = alice.rpc("offer", f"btcx:{GIVE_POCX}", f"btc:{GET_BTC}", t1, t2)
        sid = r["record"]["swap_id"]
        save_msg(m_init, r["envelope"])

        bob.cli("accept", "--in", m_init, "--out", m_accept)
        alice.rpc("recv", load_msg(m_accept))

        r = alice.rpc("fund", sid)
        save_msg(m_funded_a, r["envelope"])
        h.pocx.generate(1, "alice_pocx")

        bob.cli("recv", "--in", m_funded_a)
        bob.cli("fund", "--swap", sid, "--out", m_funded_b)
        h.btc.generate(1, "bob_btc")
        alice.rpc("recv", load_msg(m_funded_b))

        # Alice's scheduler auto-redeems chain B.
        events = alice.tick()
        assert any(e["action"] == "auto-redeem" for e in events), f"no auto-redeem: {events}"

        # The redeem went out at the 1 sat/vB fallback; raise the market (regtest
        # has none) so the next scheduler pass sees it under-priced and RBF-bumps.
        alice.rpc("_settestfeerate", 10)

        # While the redeem sits unconfirmed, the next pass must RBF-bump it.
        events = alice.tick()
        assert any(e["action"] == "fee-bump" for e in events), f"no fee-bump: {events}"
        h.btc.generate(1, "bob_btc")

        # Bob's scheduler pass: detects s on chain B, redeems chain A.
        events = bob.tick()
        assert any(e["action"] == "auto-redeem" for e in events), f"no auto-redeem: {events}"
        h.pocx.generate(1, "alice_pocx")

        # Alice's next pass books the swap as completed.
        events = alice.tick()
        assert any(e["action"] == "completed" for e in events), f"no completed: {events}"
        state = alice.rpc("getswap", sid)["state"]
        assert state == "completed", f"alice state {state}"

        assert_htlc_spent(h.pocx, m_funded_a, "chain-A")
        assert_htlc_spent(h.btc, m_funded_b, "chain-B")
        after = balances(h)
        assert after["bob_pocx"] >= before["bob_pocx"] + float(GIVE_POCX) - FEE_SLACK
        assert after["alice_btc"] >= before["alice_btc"] + float(GET_BTC) - FEE_SLACK
        print("[e2e] daemon-autopilot swap scenario OK")
    finally:
        alice.stop()
        bob.stop()


def test_daemon_autopilot_refund(h):
    """Both parties go OFFLINE after funding; when their schedulers return after
    the timelocks, each reclaims its own leg — the roadmap's refund-UX
    requirement. NOTE: funding is chain-watched, so an *online* initiator would
    correctly COMPLETE the swap once it sees chain B funded (covered by the
    autopilot *swap* test). "Walking away" therefore means offline here: we do
    NOT tick through the completion window, only after the timelocks pass."""
    alice = Party("alice4", h, h.workdir, "alice_pocx", "alice_btc").start()
    bob = Party("bob4", h, h.workdir, "bob_pocx", "bob_btc").start()
    try:
        before = balances(h)

        sid, m_funded_a, m_funded_b = handshake_and_fund(h, alice, bob, "31")

        # Simulate both offline through the completion window: jump past the
        # timelocks WITHOUT ticking, then let each scheduler reclaim its leg.
        h.advance_time(5 * 3600)

        events = bob.tick()
        assert any(e["action"] == "auto-refund" for e in events), f"bob: {events}"
        events = alice.tick()
        assert any(e["action"] == "auto-refund" for e in events), f"alice: {events}"
        h.btc.generate(1, "bob_btc")
        h.pocx.generate(1, "alice_pocx")

        assert_htlc_spent(h.pocx, m_funded_a, "chain-A")
        assert_htlc_spent(h.btc, m_funded_b, "chain-B")
        after = balances(h)
        assert after["bob_pocx"] <= before["bob_pocx"] + FEE_SLACK
        assert after["alice_btc"] <= before["alice_btc"] + FEE_SLACK
        print("[e2e] daemon-autopilot refund scenario OK")
    finally:
        alice.stop()
        bob.stop()


def test_chain_watched_funding(h):
    """The `funded` relay messages never arrive after the handshake, yet the
    swap completes — driven entirely by chain-watched funding detection in
    tick(): each leg is discovered on-chain by its derivable HTLC script. This
    is the robustness guarantee: no single post-init message is load-bearing."""
    alice = Party("alicecw", h, h.workdir, "alice_pocx", "alice_btc").start()  # initiator
    bob = Party("bobcw", h, h.workdir, "bob_pocx", "bob_btc").start()          # participant
    try:
        before = balances(h)
        t2, t1 = regtest_timelocks(h)
        m_init = msg(h.workdir, "cw_init.json")
        m_accept = msg(h.workdir, "cw_accept.json")
        # funded_* envelopes are written but NEVER delivered to the counterparty.
        m_dump_a = msg(h.workdir, "cw_funded_a.json")
        m_dump_b = msg(h.workdir, "cw_funded_b.json")

        # Handshake (init/accept) only.
        alice.cli("offer", "--give", f"btcx:{GIVE_POCX}", "--get", f"btc:{GET_BTC}",
                  "--t1", str(t1), "--t2", str(t2), "--out", m_init)
        sid = swap_id_from(m_init)
        bob.cli("accept", "--in", m_init, "--out", m_accept)
        alice.cli("recv", "--in", m_accept)

        # Alice funds chain A; her funded_a message is NEVER given to Bob.
        alice.cli("fund", "--swap", sid, "--out", m_dump_a)
        h.pocx.generate(1, "alice_pocx")

        # Bob discovers chain A by its script (no message) → FundedA, then funds
        # chain B; his funded_b message is NEVER given to Alice.
        drive_until(bob, lambda evs: any(e["action"] == "funded-a" for e in evs))
        bob.cli("fund", "--swap", sid, "--out", m_dump_b)
        h.btc.generate(1, "bob_btc")

        # Alice discovers chain B by its script and auto-redeems (revealing s);
        # she may tick once for funded-b then again for the redeem.
        drive_until(alice, lambda evs: any(e["action"] == "auto-redeem" for e in evs))
        h.btc.generate(1, "bob_btc")  # confirm Alice's chain-B redeem (reveal)

        # Bob extracts the preimage from chain B and redeems chain A.
        drive_until(bob, lambda evs: any(e["action"] == "auto-redeem" for e in evs))
        h.pocx.generate(1, "alice_pocx")

        # Completed (not refunded): both HTLCs spent and Bob received POCX.
        assert_htlc_spent(h.pocx, m_dump_a, "chain-A")
        assert_htlc_spent(h.btc, m_dump_b, "chain-B")
        after = balances(h)
        assert after["bob_pocx"] >= before["bob_pocx"] + float(GIVE_POCX) - FEE_SLACK, \
            f"bob did not receive POCX: {before} -> {after}"
        print("[e2e] chain-watched funding (no funded messages) scenario OK")
    finally:
        alice.stop()
        bob.stop()


def test_funding_fee_bump_v1(h):
    """The funding-bump nurse (v1, RBF). A funding/lock that goes out UNDER the
    market is RBF-bumped by the scheduler while it is still unconfirmed — the one
    swap tx that previously had no bump at all. The swap then completes through
    chain-watched detection, which proves three things at once:

      1. the nurse actually replaces the under-priced funding (a new txid);
      2. the rebuilt+re-signed refund and the updated outpoint are correct
         (the swap still runs to completion afterwards); and
      3. the RBF is invisible to the counterparty, who detects the lock by
         scriptPubKey, not txid (Bob is never given the funded_a message — he
         discovers the BUMPED funding by its script).

    Setup: regtest has no fee market (estimatesmartfee returns nothing → pactd's
    1 sat/vB fallback) and settxfee is gone in Core v31, so we inject the gap via
    the regtest-only `_settestfeerate` hook — fund at the 1 sat/vB fallback, then
    raise the market so the nurse sees broadcast(1) < market and RBF-bumps."""
    alice = Party("alicefb", h, h.workdir, "alice_pocx", "alice_btc").start()  # initiator
    bob = Party("bobfb", h, h.workdir, "bob_pocx", "bob_btc").start()          # participant
    try:
        before = balances(h)
        t2, t1 = regtest_timelocks(h)
        m_init = msg(h.workdir, "fb_init.json")
        m_accept = msg(h.workdir, "fb_accept.json")
        # funded_a is written but NEVER delivered to Bob (chain-watched path).
        m_dump_a = msg(h.workdir, "fb_funded_a.json")
        m_dump_b = msg(h.workdir, "fb_funded_b.json")

        # Handshake only.
        alice.cli("offer", "--give", f"btcx:{GIVE_POCX}", "--get", f"btc:{GET_BTC}",
                  "--t1", str(t1), "--t2", str(t2), "--out", m_init)
        sid = swap_id_from(m_init)
        bob.cli("accept", "--in", m_init, "--out", m_accept)
        alice.cli("recv", "--in", m_accept)

        # Alice funds chain A cheap; do NOT mine — leave it unconfirmed so the
        # nurse can act.
        alice.cli("fund", "--swap", sid, "--out", m_dump_a)
        orig_txid, _ = outpoint_from(m_dump_a)

        # Funding went out at the 1 sat/vB fallback; now raise the market so the
        # nurse sees it as under-priced and RBF-bumps it.
        alice.rpc("_settestfeerate", 10)

        # The scheduler RBF-bumps the unconfirmed, under-priced funding.
        events = drive_until(
            alice, lambda evs: any(e["action"] == "funding-fee-bump" for e in evs))
        bump = next(e for e in events if e["action"] == "funding-fee-bump")
        print(f"[e2e] funding bumped: {bump['detail']}")

        # The stored funding pointer now references a NEW txid, and the original
        # is gone from the mempool (replaced by the higher-fee version).
        new_txid = alice.rpc("getswap", sid)["htlc_a_txid"]
        assert new_txid and new_txid != orig_txid, \
            f"funding txid did not change after bump: {orig_txid} -> {new_txid}"
        assert h.pocx.rpc("gettxout", orig_txid, 0, True) is None and \
            h.pocx.rpc("gettxout", orig_txid, 1, True) is None, \
            "original (replaced) funding is still in the mempool"
        print(f"[e2e] funding RBF: {orig_txid[:12]}… replaced by {new_txid[:12]}…")

        # Confirm the bumped funding, then complete via chain-watched detection:
        # Bob never received funded_a — he finds the BUMPED lock by its script,
        # which is exactly why the RBF is safe for him.
        h.pocx.generate(1, "alice_pocx")
        drive_until(bob, lambda evs: any(e["action"] == "funded-a" for e in evs))
        bob.cli("fund", "--swap", sid, "--out", m_dump_b)
        h.btc.generate(1, "bob_btc")
        drive_until(alice, lambda evs: any(e["action"] == "auto-redeem" for e in evs))
        h.btc.generate(1, "bob_btc")  # confirm Alice's reveal on chain B
        drive_until(bob, lambda evs: any(e["action"] == "auto-redeem" for e in evs))
        h.pocx.generate(1, "alice_pocx")

        # Bob got POCX (and Alice got BTC): the bumped funding completed cleanly.
        assert_htlc_spent(h.btc, m_dump_b, "chain-B")
        after = balances(h)
        assert after["bob_pocx"] >= before["bob_pocx"] + float(GIVE_POCX) - FEE_SLACK, \
            f"bob did not receive POCX after a bumped funding: {before} -> {after}"
        assert after["alice_btc"] >= before["alice_btc"] + float(GET_BTC) - FEE_SLACK, \
            f"alice did not receive BTC after a bumped funding: {before} -> {after}"
        print("[e2e] funding-fee-bump (v1 RBF) scenario OK")
    finally:
        alice.stop()
        bob.stop()


def test_balance_validation(h):
    """An offer you can't fund is refused up front, at the point it would be
    advertised. `board post` runs the cumulative funds gate
    (engine.ensure_can_fund_new_offer) so an un-fundable offer never reaches the
    board / pollutes it. NOTE: the bare `offer` command is an offline envelope
    builder and is intentionally ungated (engine.offer, "works offline") — the
    funds gate lives only where money is actually committed: board-post, take and
    fund. So this drives `board post`, the same gated path Satchel's "Post an
    offer" uses (boardpostoffer). The other scenarios already prove a fundable
    offer is accepted, so this only checks rejection."""
    alice = Party("alicebal", h, h.workdir, "alice_pocx", "alice_btc").start()
    try:
        # alice_pocx holds ~100 POCX; advertising an offer to GIVE a million is
        # refused because the core wallet can't cover the leg we'd lock when taken.
        # The gate fires after the chains-live check but before any board is
        # contacted, so this needs no Corkboard. Default board-post timelocks
        # (12h/6h) satisfy validate_offer_offsets.
        err = expect_fail(alice, "over-balance offer",
                          "board", "post", "--give", "btcx:1000000.0", "--get", "btc:0.001")
        assert "insufficient" in err.lower(), f"expected insufficient-balance error, got: {err}"
        print("[e2e] balance-validation scenario OK")
    finally:
        alice.stop()


def test_create_import_then_swap(h):
    """Phase B: neither party is auto-initialized. Alice creates a brand-new
    seed and Bob imports a known mnemonic — both through the seed-lifecycle
    RPCs (createseed / importseed), the same path the Satchel wizard drives —
    and then they complete a normal manual swap. Proves a merchant set up via
    the wizard is fully functional."""
    # A fixed BIP39 test mnemonic for the import path (deterministic identity).
    BOB_MNEMONIC = ("legal winner thank year wave sausage worth useful legal "
                    "winner thank yellow")
    alice = Party("alice6", h, h.workdir, "alice_pocx", "alice_btc",
                  auto_init=False).start()
    bob = Party("bob6", h, h.workdir, "bob_pocx", "bob_btc",
                auto_init=False).start()
    try:
        before = balances(h)

        # First run: no seed yet, so getinfo reports no identity and the
        # wallet is in first-run state.
        assert alice.rpc("walletstatus")["seed_exists"] is False, "alice should start seedless"
        assert alice.rpc("getinfo")["identity"] is None, "no identity before seed creation"

        # Alice creates a fresh seed; Bob imports a known one (encrypted).
        created = alice.setup_seed()
        assert len(created.split()) == 12, f"unexpected mnemonic: {created!r}"
        bob.setup_seed(mnemonic=BOB_MNEMONIC, passphrase="bobpass")

        st_a = alice.rpc("walletstatus")
        assert st_a == {"seed_exists": True, "encrypted": False, "locked": False,
                        "needs_reimport": False}, st_a
        st_b = bob.rpc("walletstatus")
        assert st_b == {"seed_exists": True, "encrypted": True, "locked": False,
                        "needs_reimport": False}, st_b
        # Both now have a usable identity (Bob's is deterministic from import).
        assert alice.rpc("getinfo")["identity"], "alice has no identity after createseed"
        assert bob.rpc("getinfo")["identity"], "bob has no identity after importseed"

        # A normal manual swap on these wizard-provisioned merchants.
        sid, m_funded_a, m_funded_b = handshake_and_fund(h, alice, bob, "61")
        alice.cli("recv", "--in", m_funded_b)
        alice.cli("redeem", "--swap", sid)
        h.btc.generate(1, "bob_btc")
        bob.cli("redeem", "--swap", sid)
        h.pocx.generate(1, "alice_pocx")

        assert_htlc_spent(h.pocx, m_funded_a, "chain-A")
        assert_htlc_spent(h.btc, m_funded_b, "chain-B")
        after = balances(h)
        assert after["bob_pocx"] >= before["bob_pocx"] + float(GIVE_POCX) - FEE_SLACK, \
            f"Bob did not receive POCX: {after}"
        assert after["alice_btc"] >= before["alice_btc"] + float(GET_BTC) - FEE_SLACK, \
            f"Alice did not receive BTC: {after}"
        print("[e2e] create-import-then-swap scenario OK")
    finally:
        alice.stop()
        bob.stop()


def test_coin_setup(h):
    """Phase C: the coin-setup RPCs. listcoins reports the shipped registry +
    configured state + a live connection status; listpairs derives swap-pair
    availability from capabilities (not a curated list); validatecoin runs the
    genesis-hash check that gates saving a backend — accepting the right node
    and rejecting a cross-wired one."""
    alice = Party("alice7", h, h.workdir, "alice_pocx", "alice_btc").start()
    try:
        # listcoins: both shipped coins, both configured (the harness launches
        # pactd with --coin btcx=.../--coin btc=...), both connected to the right chain.
        info = alice.rpc("listcoins")
        assert info["network"] == "regtest", info
        by_id = {c["id"]: c for c in info["coins"]}
        assert set(by_id) == {"btcx", "btc"}, by_id
        for cid in ("btcx", "btc"):
            c = by_id[cid]
            assert c["configured"] is True, c
            assert c["status"] == "ok", c
            assert c["tip_height"] is not None, c
            assert c["capabilities"]["cltv"] and c["capabilities"]["segwit_v0"], c
        # The reported genesis is the regtest one the node actually serves.
        assert by_id["btcx"]["genesis_hash"] == h.pocx.rpc("getblockhash", 0)
        assert by_id["btc"]["genesis_hash"] == h.btc.rpc("getblockhash", 0)
        # getinfo now surfaces the configured coins too.
        assert set(alice.rpc("getinfo")["coins"]) == {"btcx", "btc"}
        print("[e2e] listcoins OK (configured + connected + genesis)")

        # listpairs: POCX<->BTC is available now (both configured) via HTLC.
        pairs = alice.rpc("listpairs")["pairs"]
        pair = next(p for p in pairs if {p["coin_a"], p["coin_b"]} == {"btcx", "btc"})
        assert pair["both_configured"] and pair["available"], pair
        assert "htlc" in pair["protocols"], pair
        assert pair["selectable"] == "htlc", pair
        print("[e2e] listpairs OK (POCX<->BTC available via HTLC)")

        # validatecoin: the genesis check that gates saving a backend.
        ok = alice.rpc("validatecoin", "btcx", alice.pocx_url)
        assert ok["ok"] and ok["genesis_hash"] == by_id["btcx"]["genesis_hash"], ok
        assert ok["tip_height"] is not None, ok
        # Cross-wire: point "btcx" at the BTC node — genesis mismatch, rejected.
        try:
            alice.rpc("validatecoin", "btcx", alice.btc_url)
            raise AssertionError("validatecoin accepted a wrong-chain backend")
        except RuntimeError as exc:
            assert "wrong chain" in str(exc).lower() or "genesis" in str(exc).lower(), exc
            print("[e2e] correctly rejected: btcx pointed at the BTC node")

        print("[e2e] coin-setup scenario OK")
    finally:
        alice.stop()


def _drive_board_swap(h, maker, taker, want_completed):
    """Post an offer, take it, and drive both daemons until each has at least
    `want_completed` completed swaps. Used by the board-reset test to run two
    swaps in a row and confirm the second one (post-reset) lands."""
    offer_id = maker.rpc(
        "boardpostoffer", f"btcx:{GIVE_POCX}", f"btc:{GET_BTC}", 4 * 3600, 2 * 3600,
        "pact-htlc-v1")["offer_id"]
    taker.rpc("boardtake", offer_id)
    ca = cb = 0
    for _ in range(18):
        for party in (maker, taker):
            party.rpc("tick")
            h.pocx.generate(1, "alice_pocx")
            h.btc.generate(1, "bob_btc")
        ca = sum(1 for s in maker.rpc("listswaps") if s["state"] == "completed")
        cb = sum(1 for s in taker.rpc("listswaps") if s["state"] == "completed")
        if ca >= want_completed and cb >= want_completed:
            return
    raise AssertionError(
        f"board swap #{want_completed} did not complete: maker={ca}, taker={cb}")


def test_board_reset_recovery(h):
    """A board wiped/redeployed under running clients must not strand them: their
    relay cursors are now ahead of the fresh board's ids, but reset hygiene
    re-serves from the start. Run a swap (advances cursors), WIPE the board DB,
    then run a second swap with the same stale-cursor parties — it must complete.
    (Without the fix the second swap's relay traffic is silently dropped.)"""
    board = Corkboard(h.workdir)
    board.start()
    maker = Party("alicerst", h, h.workdir, "alice_pocx", "alice_btc",
                  board_url=board.url, auto_fund=True).start()
    taker = Party("bobrst", h, h.workdir, "bob_pocx", "bob_btc",
                  board_url=board.url, auto_fund=True).start()
    try:
        _drive_board_swap(h, maker, taker, want_completed=1)   # advances relay cursors
        board.reset()                                          # wipe board under the clients
        _drive_board_swap(h, maker, taker, want_completed=2)   # stale cursors must self-heal
        print("[e2e] board-reset recovery scenario OK")
    finally:
        maker.stop()
        taker.stop()
        board.stop()


def test_nostr_relay_swap(h):
    """Phase 2 over a LIVE Nostr relay: maker + taker share one local relay (no
    HTTP board) and complete a full board-driven swap through it — exercising
    the real relay publish/fetch round-trip the in-process nostr test can't
    cover. Offers + mail propagate asynchronously via pactd's relay service, so
    we poll for propagation and give the round-trips a beat between passes."""
    relay = NostrRelay(h.workdir)
    relay.start()
    maker = Party("alicenos", h, h.workdir, "alice_pocx", "alice_btc",
                  nostr_relays=relay.ws_url, auto_fund=True).start()
    taker = Party("bobnos", h, h.workdir, "bob_pocx", "bob_btc",
                  nostr_relays=relay.ws_url, auto_fund=True).start()
    try:
        offer_id = maker.rpc(
            "boardpostoffer", f"btcx:{GIVE_POCX}", f"btc:{GET_BTC}", 4 * 3600, 2 * 3600,
            "pact-htlc-v1")["offer_id"]
        # Each tick runs a full relay round-trip (publish our outbox + fetch),
        # awaited inside the RPC — so tick the maker (publishes the offer) and the
        # taker (fetches it) until the offer shows up in the taker's board.
        seen = False
        for _ in range(20):
            maker.rpc("tick")
            taker.rpc("tick")
            if any(o["swap_id"] == offer_id for o in taker.rpc("boardlistoffers")["offers"]):
                seen = True
                break
        assert seen, "offer never propagated over the nostr relay to the taker"
        taker.rpc("boardtake", offer_id)

        # Drive both daemons; each tick = a relay round-trip + the engine pass,
        # so the gift-wrapped take/init/funded mail flows over the live relay.
        ca = cb = 0
        for _ in range(30):
            for party in (maker, taker):
                party.rpc("tick")
                if handshake_done(maker, taker):
                    h.pocx.generate(1, "alice_pocx")
                    h.btc.generate(1, "bob_btc")
            ca = sum(1 for s in maker.rpc("listswaps") if s["state"] == "completed")
            cb = sum(1 for s in taker.rpc("listswaps") if s["state"] == "completed")
            if ca and cb:
                print("[e2e] nostr-relay swap scenario OK (live relay round-trip)")
                break
        else:
            raise AssertionError(f"nostr swap did not complete: maker={ca}, taker={cb}")
    finally:
        maker.stop()
        taker.stop()
        relay.stop()


def test_concurrent_drain_no_double_send(h):
    """Regression for #176 / #181: an RPC `flush_nostr` (fired straight after
    boardtake) and the scheduler-tick drain must not BOTH publish the same
    still-unsent outbox row — a fresh gift-wrap mints a fresh event id, so
    event-id dedup can't collapse it and the maker would receive the `take`
    TWICE. PACT_TEST_OUTBOX_DRAIN_DELAY_MS widens the read->mark-sent window so
    the race is deterministic (in a clean env it's ~µs and never fires); the
    atomic outbox claim (store `last_attempt`) must still yield exactly one
    delivered take. Asserts the swap completes AND the maker narrates zero
    duplicate/rejected takes. Reverting the claim to a plain pending-read makes
    this go red (2+ takes -> take-duplicate)."""
    relay = NostrRelay(h.workdir)
    relay.start()
    maker = Party("alicedrain", h, h.workdir, "alice_pocx", "alice_btc",
                  nostr_relays=relay.ws_url, auto_fund=True).start()
    # Delay ONLY the taker's outbox drains — the side that sends the `take`.
    taker = Party("bobdrain", h, h.workdir, "bob_pocx", "bob_btc",
                  nostr_relays=relay.ws_url, auto_fund=True,
                  extra_env={"PACT_TEST_OUTBOX_DRAIN_DELAY_MS": "800"}).start()
    counts = {}

    def tally(resp):
        for e in (resp or {}).get("events", []):
            counts[e.get("action", "")] = counts.get(e.get("action", ""), 0) + 1

    try:
        offer_id = maker.rpc(
            "boardpostoffer", f"btcx:{GIVE_POCX}", f"btc:{GET_BTC}", 4 * 3600, 2 * 3600,
            "pact-htlc-v1")["offer_id"]
        seen = False
        for _ in range(20):
            maker.rpc("tick")
            taker.rpc("tick")
            if any(o["swap_id"] == offer_id for o in taker.rpc("boardlistoffers")["offers"]):
                seen = True
                break
        assert seen, "offer never propagated over the nostr relay to the taker"
        # boardtake fires flush_nostr (pass A, still inside its 800ms delay); an
        # immediate tick is pass B — both would drain the same unsent `take`.
        taker.rpc("boardtake", offer_id)
        taker.rpc("tick")
        ca = cb = 0
        for _ in range(30):
            tally(maker.rpc("tick"))
            if handshake_done(maker, taker):
                h.pocx.generate(1, "alice_pocx")
                h.btc.generate(1, "bob_btc")
            taker.rpc("tick")
            ca = sum(1 for s in maker.rpc("listswaps") if s["state"] == "completed")
            cb = sum(1 for s in taker.rpc("listswaps") if s["state"] == "completed")
            if ca and cb:
                break
        else:
            raise AssertionError(f"drain swap did not complete: maker={ca} taker={cb}")
        dup = counts.get("take-duplicate", 0)
        rej = counts.get("take-rejected", 0)
        assert dup == 0 and rej == 0, \
            f"concurrent-drain double-send regressed: take-duplicate={dup} take-rejected={rej}"
        assert counts.get("take->init", 0) >= 1, "maker never processed the take"
        print("[e2e] concurrent-drain no-double-send OK (0 duplicate takes under 800ms delay)")
    finally:
        maker.stop()
        taker.stop()
        relay.stop()


def test_corkboard_swap(h):
    """Phase 2 end to end: maker posts a signed offer on the Corkboard,
    taker takes it, the whole handshake travels through the blind relay,
    and both legs auto-fund and auto-redeem to completion. Zero files
    exchanged, zero manual swap commands."""
    board = Corkboard(h.workdir)
    board.start()
    maker = Party("alice5", h, h.workdir, "alice_pocx", "alice_btc",
                  board_url=board.url, auto_fund=True).start()
    taker = Party("bob5", h, h.workdir, "bob_pocx", "bob_btc",
                  board_url=board.url, auto_fund=True).start()
    carol = Party("carol5", h, h.workdir, "bob_pocx", "alice_btc",
                  board_url=board.url).start()
    try:
        # Per-scenario stacks start alice_btc (carol's btc wallet) EMPTY; under
        # the old shared harness it held redeem proceeds from earlier
        # scenarios, which carol's competing take needs to pass the take-side
        # funds gate (it must reach the MAKER and be rejected there, not die
        # client-side on "insufficient btc"). Stake her explicitly.
        h.btc.rpc("sendtoaddress",
                  h.btc.rpc("getnewaddress", wallet="alice_btc"), 0.01,
                  wallet="bob_btc")
        h.btc.generate(1, "bob_btc")

        before = balances(h)

        # Withdraw flow: post an offer, withdraw it, it's gone instantly.
        withdrawn_id = maker.rpc(
            "boardpostoffer", f"btcx:{GIVE_POCX}", f"btc:{GET_BTC}", 4 * 3600, 2 * 3600,
            "pact-htlc-v1")["offer_id"]  # force v1 HTLC over the board (PoCX↔BTC defaults to v2)
        maker.rpc("boardrevoke", withdrawn_id)
        offers = taker.rpc("boardlistoffers")["offers"]
        assert not any(o["swap_id"] == withdrawn_id for o in offers), "withdrawn offer still listed"
        print("[e2e] offer withdraw OK")

        offer_id = maker.rpc(
            "boardpostoffer", f"btcx:{GIVE_POCX}", f"btc:{GET_BTC}", 4 * 3600, 2 * 3600,
            "pact-htlc-v1")["offer_id"]  # force v1 HTLC over the board (PoCX↔BTC defaults to v2)
        offers = taker.rpc("boardlistoffers")["offers"]
        listed = next((o for o in offers if o["swap_id"] == offer_id), None)
        assert listed is not None, f"offer not listed: {offers}"
        # Phase D: the Satchel offer cards render amounts, implied rate,
        # timelocks and a "posted Nm ago" freshness from the offer body — guard
        # that contract so a body-shape change can't silently break the display.
        b = listed["body"]
        assert b["give_asset"] == "btcx" and b["get_asset"] == "btc", b
        assert b["give_amount"] > 0 and b["get_amount"] > 0, b          # implied rate
        assert b["t1_secs"] == 4 * 3600 and b["t2_secs"] == 2 * 3600, b  # safety refunds
        assert b["created"] > 0, b                                      # age / freshness
        assert listed["from"], listed   # maker identity is carried on the offer
        taker.rpc("boardtake", offer_id)

        # A second taker grabs the same offer before the maker reacts —
        # the 20-minute-live-ad problem. The maker must serve exactly one
        # and explicitly reject the other.
        carol.cli("board", "take", "--offer", offer_id)

        events = maker.rpc("tick")["events"]
        actions = [e["action"] for e in events]
        assert "take->init" in actions, f"maker did not serve the first take: {events}"
        assert "take-rejected" in actions, f"maker did not reject the second take: {events}"
        offers = taker.rpc("boardlistoffers")["offers"]
        assert not any(o["swap_id"] == offer_id for o in offers), \
            "served offer still listed (auto-delist failed)"
        carol_events = json.loads(carol.cli("board", "sync"))["events"]
        assert any(e["action"] == "take-failed" for e in carol_events), \
            f"carol never learned her take was rejected: {carol_events}"
        print("[e2e] competing-take rejection + auto-delist OK")

        # Drive both daemons; mine after each pass so confirmations land.
        sid = None
        for round_no in range(12):
            for party in (maker, taker):
                events = party.rpc("tick")["events"]
                for ev in events:
                    print(f"[e2e]   board[{party.name}]: {ev['action']} {ev['detail'][:60]}")
                h.pocx.generate(1, "alice_pocx")
                h.btc.generate(1, "bob_btc")
            swaps_a = maker.rpc("listswaps")
            swaps_b = taker.rpc("listswaps")
            if swaps_a and swaps_b:
                sid = swaps_a[0]["swap_id"]
                states = (swaps_a[0]["state"], swaps_b[0]["state"])
                if states == ("completed", "completed"):
                    print(f"[e2e] board swap {sid} completed in {round_no + 1} rounds")
                    break
        else:
            raise AssertionError(
                f"board swap did not complete: a={swaps_a}, b={swaps_b}")

        # Privacy: every relay blob on the board must be sealed ciphertext,
        # not plaintext coordination JSON (inspect the board's own db).
        import sqlite3
        with sqlite3.connect(board.db) as conn:
            blobs = [row[0] for row in conn.execute("SELECT blob FROM relay")]
        assert blobs, "no relay traffic recorded?"
        for blob in blobs:
            assert blob.startswith("PACTSEALED1:"), f"plaintext blob on the board: {blob[:60]}"
            assert "funded" not in blob and "txid" not in blob, "coordination data leaked"
        print(f"[e2e] all {len(blobs)} relay blobs are sealed (E2E encrypted)")

        after = balances(h)
        assert after["bob_pocx"] >= before["bob_pocx"] + float(GIVE_POCX) - FEE_SLACK
        assert after["alice_btc"] >= before["alice_btc"] + float(GET_BTC) - FEE_SLACK
        print("[e2e] corkboard swap scenario OK")
    finally:
        maker.stop()
        taker.stop()
        carol.stop()
        board.stop()


def test_private_offer_swap(h):
    """Private (off-market) offers, PRIVATE_OFFERS.md: the maker builds a
    signed offer with `makeprivateoffer` (NOT boardpostoffer) — it is NEVER
    listed on any board — and hands the returned `slip` string to the taker
    over an out-of-band channel (here, a Python variable). The taker calls
    `takeoffer <slip>`; the take travels through the SAME blind relay, both
    legs auto-fund and auto-redeem, and the swap completes — proving an
    off-market swap with zero board listing."""
    board = Corkboard(h.workdir)
    board.start()
    # Unique party names — data dirs are keyed by name and shared across the
    # suite's single Harness; "alice6"/"bob6" are taken (bob6 is encrypted) by
    # the create-import scenario, so reusing them brings up a locked seed.
    maker = Party("alicePO", h, h.workdir, "alice_pocx", "alice_btc",
                  board_url=board.url, auto_fund=True).start()
    taker = Party("bobPO", h, h.workdir, "bob_pocx", "bob_btc",
                  board_url=board.url, auto_fund=True).start()
    try:
        before = balances(h)

        # Maker creates a PRIVATE offer — returns a slip, posts NOTHING.
        slip = maker.rpc(
            "makeprivateoffer", f"btcx:{GIVE_POCX}", f"btc:{GET_BTC}",
            4 * 3600, 2 * 3600, "pact-htlc-v1")["slip"]  # force v1 (PoCX↔BTC defaults to v2)
        assert slip.startswith("pactoffer1:"), f"bad slip: {slip[:40]}"

        # It is tracked locally (for cancel) but NOT on the board.
        mine = maker.rpc("listprivateoffers")["offers"]
        assert len(mine) == 1 and mine[0]["give_asset"] == "btcx", mine
        private_id = mine[0]["offer_id"]
        offers = taker.rpc("boardlistoffers")["offers"]
        assert all(o["swap_id"] != private_id for o in offers), \
            "a private offer leaked onto the board"
        print("[e2e] private offer created off-board (slip handed over out of band)")

        # The friend pastes the slip and takes it — decode + verify happen in
        # pactd; the take relays straight to the maker's mailbox.
        taker.rpc("takeoffer", slip)

        # Drive both daemons to completion (same loop as the corkboard test).
        sid = None
        for round_no in range(12):
            for party in (maker, taker):
                events = party.rpc("tick")["events"]
                for ev in events:
                    print(f"[e2e]   board[{party.name}]: {ev['action']} {ev['detail'][:60]}")
                h.pocx.generate(1, "alice_pocx")
                h.btc.generate(1, "bob_btc")
            swaps_a = maker.rpc("listswaps")
            swaps_b = taker.rpc("listswaps")
            if swaps_a and swaps_b:
                sid = swaps_a[0]["swap_id"]
                states = (swaps_a[0]["state"], swaps_b[0]["state"])
                if states == ("completed", "completed"):
                    print(f"[e2e] private swap {sid} completed in {round_no + 1} rounds")
                    break
        else:
            raise AssertionError(
                f"private swap did not complete: a={swaps_a}, b={swaps_b}")

        # No board listing ever existed for this swap.
        offers = taker.rpc("boardlistoffers")["offers"]
        assert all(o["swap_id"] != sid for o in offers), "private swap appeared on the board"

        after = balances(h)
        assert after["bob_pocx"] >= before["bob_pocx"] + float(GIVE_POCX) - FEE_SLACK
        assert after["alice_btc"] >= before["alice_btc"] + float(GET_BTC) - FEE_SLACK
        print("[e2e] private-offer swap scenario OK")
    finally:
        maker.stop()
        taker.stop()
        board.stop()


class CompleteSwap(PactTestFramework):
    def run_test(self):
        test_complete_swap(self.h)


class Refund(PactTestFramework):
    def run_test(self):
        test_refund(self.h)


class DaemonAutopilotSwap(PactTestFramework):
    def run_test(self):
        test_daemon_autopilot_swap(self.h)


class DaemonAutopilotRefund(PactTestFramework):
    def run_test(self):
        test_daemon_autopilot_refund(self.h)


class ChainWatchedFunding(PactTestFramework):
    def run_test(self):
        test_chain_watched_funding(self.h)


class FundingFeeBumpV1(PactTestFramework):
    def run_test(self):
        test_funding_fee_bump_v1(self.h)


class BalanceValidation(PactTestFramework):
    def run_test(self):
        test_balance_validation(self.h)


class CreateImportThenSwap(PactTestFramework):
    def run_test(self):
        test_create_import_then_swap(self.h)


class CoinSetup(PactTestFramework):
    def run_test(self):
        test_coin_setup(self.h)


class CorkboardSwap(PactTestFramework):
    def run_test(self):
        test_corkboard_swap(self.h)


class BoardResetRecovery(PactTestFramework):
    def run_test(self):
        test_board_reset_recovery(self.h)


class NostrRelaySwap(PactTestFramework):
    def run_test(self):
        test_nostr_relay_swap(self.h)


class ConcurrentDrainNoDoubleSend(PactTestFramework):
    def run_test(self):
        test_concurrent_drain_no_double_send(self.h)


class PrivateOfferSwap(PactTestFramework):
    def run_test(self):
        test_private_offer_swap(self.h)


SCENARIOS = [
    CompleteSwap,
    Refund,
    DaemonAutopilotSwap,
    DaemonAutopilotRefund,
    ChainWatchedFunding,
    FundingFeeBumpV1,
    BalanceValidation,
    CreateImportThenSwap,
    CoinSetup,
    CorkboardSwap,
    BoardResetRecovery,
    NostrRelaySwap,
    ConcurrentDrainNoDoubleSend,
    PrivateOfferSwap,
]


if __name__ == "__main__":
    run_scenarios(SCENARIOS)
