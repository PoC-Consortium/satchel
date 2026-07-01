#!/usr/bin/env python3
"""End-to-end test for v2 (pact-htlc-v2) Taproot/MuSig2 adaptor swaps on
regtest. Mirrors test_swap_e2e.py but drives the adaptor lifecycle through
pactd's JSON-RPC.

Flow (PoCX leg A = Alice funds; BTC leg B = Bob funds):
  adaptorinit (Alice) -> adaptoraccept (Bob) -> Alice recv accept
  -> both adaptorfund -> exchange funding_ready
  -> both adaptornonces -> exchange -> both adaptorsign -> exchange
  -> both adaptorassemble  (verified AdaptorSignatures, state Signed)
  -> Alice adaptorredeem leg B (reveals t) -> Bob adaptorredeem leg A
Success = both Taproot funding outputs are cooperatively spent (key-path).

Run:  python test_adaptor_swap.py
Env:  POCX_BITCOIND / BTC_BITCOIND   (node binaries, see regtest_harness.py)

NOTE: each redeem sweeps to a fresh CORE-WALLET address communicated in the
signed init/accept (alice_sweep_b / bob_sweep_a), so both parties co-sign the
identical redeem tx AND the proceeds land in a spendable wallet (not a swap-key
address the node can't spend). Success is checked by the funding outputs being
spent AND the claimers' core-wallet balances rising. If no node can mint an
address the protocol falls back to the deterministic swap-key destination.
"""

import sys

from regtest_harness import Harness
from test_swap_e2e import COINS_TOML, Corkboard, Party, build_workspace, regtest_timelocks

GIVE_POCX = "btcx:50.0"
GET_BTC = "btc:0.001"
GET_LTC = "ltc:0.5"


def _env(result):
    return result["envelope"]


def test_adaptor_swap(h):
    # auto_init=False: start seedless so setup_seed()'s createseed can run (the
    # default auto_init would create a seed on boot → createseed then conflicts).
    alice = Party("ad-alice", h, h.workdir, "alice_pocx", "alice_btc", auto_init=False).start()
    bob = Party("ad-bob", h, h.workdir, "bob_pocx", "bob_btc", auto_init=False).start()
    try:
        alice.setup_seed()
        bob.setup_seed()
        t2, t1 = regtest_timelocks(h)

        # Claim-wallet balances before the swap: the redeems must land in these
        # spendable core wallets (fresh sweep addrs), not at a swap-key address.
        alice_btc_before = float(h.btc.rpc("getbalance", wallet="alice_btc"))
        bob_pocx_before = float(h.pocx.rpc("getbalance", wallet="bob_pocx"))

        # init / accept — both sides can now rebuild identical Taproot legs.
        init = _env(alice.rpc("adaptorinit", GIVE_POCX, GET_BTC, t1, t2))
        sid = init["swap_id"]
        accept = _env(bob.rpc("adaptoraccept", init))
        alice.rpc("adaptorrecv", accept)

        # Fund both legs (Alice -> leg A on PoCX, Bob -> leg B on BTC).
        fa = _env(alice.rpc("adaptorfund", sid))
        h.pocx.generate(1, "alice_pocx")
        bob.rpc("adaptorrecv", fa)
        fb = _env(bob.rpc("adaptorfund", sid))
        h.btc.generate(1, "bob_btc")
        alice.rpc("adaptorrecv", fb)

        # Nonce exchange (both redeem sessions).
        na = _env(alice.rpc("adaptornonces", sid))
        nb = _env(bob.rpc("adaptornonces", sid))
        bob.rpc("adaptorrecv", na)
        alice.rpc("adaptorrecv", nb)

        # Partial-adaptor-sig exchange.
        pa = _env(alice.rpc("adaptorsign", sid))
        pb = _env(bob.rpc("adaptorsign", sid))
        bob.rpc("adaptorrecv", pa)
        alice.rpc("adaptorrecv", pb)

        # Assemble + verify the aggregate adaptor signatures (state -> Signed).
        ar = alice.rpc("adaptorassemble", sid)["record"]
        br = bob.rpc("adaptorassemble", sid)["record"]
        assert ar["state"] == "signed", ar["state"]
        assert br["state"] == "signed", br["state"]

        # Funding outpoints, from the funding_ready bodies.
        a_txid, a_vout = fa["body"]["txid"], fa["body"]["vout"]   # leg A (PoCX)
        b_txid, b_vout = fb["body"]["txid"], fb["body"]["vout"]   # leg B (BTC)

        # Alice redeems leg B (reveals t on chain); Bob extracts t, redeems A.
        alice.rpc("adaptorredeem", sid)
        h.btc.generate(1, "bob_btc")
        bob.rpc("adaptorredeem", sid)
        h.pocx.generate(1, "alice_pocx")

        # Both Taproot funding outputs are now cooperatively spent.
        assert h.btc.rpc("gettxout", b_txid, b_vout) is None, "leg B (BTC) not redeemed"
        assert h.pocx.rpc("gettxout", a_txid, a_vout) is None, "leg A (PoCX) not redeemed"
        print("[e2e] adaptor swap OK: both legs cooperatively key-path-redeemed")

        # The redeemed coins must land in the claimers' spendable CORE wallets
        # (the fresh sweep addresses communicated in init/accept) — this is the
        # whole point of fresh-sweep-address and would fail with a swap-key dest.
        alice_btc_after = float(h.btc.rpc("getbalance", wallet="alice_btc"))
        bob_pocx_after = float(h.pocx.rpc("getbalance", wallet="bob_pocx"))
        assert alice_btc_after > alice_btc_before, \
            f"Alice's leg-B redeem missed her core wallet: {alice_btc_before} -> {alice_btc_after}"
        assert bob_pocx_after > bob_pocx_before, \
            f"Bob's leg-A redeem missed his core wallet: {bob_pocx_before} -> {bob_pocx_after}"
        print("[e2e] redeems landed in spendable core wallets (fresh sweep addrs)")
    finally:
        alice.stop()
        bob.stop()


def test_adaptor_refund(h):
    """Refund path (spec v2 §5): a party funds its leg, the counterparty never
    completes, and after the timelock each reclaims via the single-key CLTV
    tapleaf (no MuSig2). v1-parity for the M7 happy+refund bar."""
    # auto_init=False: seedless so setup_seed()'s createseed doesn't collide
    # with a boot-created seed (same fix as test_adaptor_swap).
    alice = Party("adr-alice", h, h.workdir, "alice_pocx", "alice_btc", auto_init=False).start()
    bob = Party("adr-bob", h, h.workdir, "bob_pocx", "bob_btc", auto_init=False).start()
    try:
        alice.setup_seed()
        bob.setup_seed()
        t2, t1 = regtest_timelocks(h)

        init = _env(alice.rpc("adaptorinit", GIVE_POCX, GET_BTC, t1, t2))
        sid = init["swap_id"]
        accept = _env(bob.rpc("adaptoraccept", init))
        alice.rpc("adaptorrecv", accept)

        # Both fund, then the swap stalls (no nonces/sign/redeem).
        fa = _env(alice.rpc("adaptorfund", sid))
        h.pocx.generate(1, "alice_pocx")
        bob.rpc("adaptorrecv", fa)
        fb = _env(bob.rpc("adaptorfund", sid))
        h.btc.generate(1, "bob_btc")
        a_txid, a_vout = fa["body"]["txid"], fa["body"]["vout"]   # leg A (PoCX, T1)
        b_txid, b_vout = fb["body"]["txid"], fb["body"]["vout"]   # leg B (BTC, T2)

        # Advance both clocks past T1 (the later lock) so the CLTV leaves unlock.
        h.advance_time(5 * 3600)

        # Each reclaims its own funded leg via the script-path refund.
        alice.rpc("adaptorrefund", sid)   # leg A on PoCX
        h.pocx.generate(1, "alice_pocx")
        bob.rpc("adaptorrefund", sid)     # leg B on BTC
        h.btc.generate(1, "bob_btc")

        assert h.pocx.rpc("gettxout", a_txid, a_vout) is None, "leg A (PoCX) not refunded"
        assert h.btc.rpc("gettxout", b_txid, b_vout) is None, "leg B (BTC) not refunded"
        print("[e2e] adaptor refund OK: both legs reclaimed via the CLTV tapleaf")
    finally:
        alice.stop()
        bob.stop()


def test_adaptor_refund_feebump(h):
    """Fee-bump path (spec v2 §8, inheriting v1 §7.4 'MUST fee-bump
    aggressively'): once a single-key CLTV refund is broadcast but still
    unconfirmed, the scheduler RBF-bumps it. We drive `tick` WITHOUT mining the
    refund in, so it sits at 0 confirmations and the next tick must emit an
    `adaptor-fee-bump` (RBF, deterministic re-sign) — then mining settles it.

    The cooperative MuSig2 redeem cannot be RBF'd (its fee is sealed in the
    pre-signed adaptor signature); only this single-key refund path can, which
    is exactly what this exercises."""
    alice = Party("adfb-alice", h, h.workdir, "alice_pocx", "alice_btc", auto_init=False).start()
    bob = Party("adfb-bob", h, h.workdir, "bob_pocx", "bob_btc", auto_init=False).start()
    try:
        alice.setup_seed()
        bob.setup_seed()
        t2, t1 = regtest_timelocks(h)

        init = _env(alice.rpc("adaptorinit", GIVE_POCX, GET_BTC, t1, t2))
        sid = init["swap_id"]
        accept = _env(bob.rpc("adaptoraccept", init))
        alice.rpc("adaptorrecv", accept)

        # Alice funds leg A (PoCX), then the swap stalls.
        fa = _env(alice.rpc("adaptorfund", sid))
        h.pocx.generate(1, "alice_pocx")
        a_txid, a_vout = fa["body"]["txid"], fa["body"]["vout"]

        # Past T1 so the CLTV leaf unlocks, then refund leg A — but DON'T mine it.
        h.advance_time(5 * 3600)
        alice.rpc("adaptorrefund", sid)
        assert alice.rpc("listadaptorswaps")[0]["state"] == "refunded"

        # The refund is unconfirmed → a tick must RBF-bump (or rebroadcast) it.
        bumped = False
        for ev in alice.rpc("tick")["events"]:
            print(f"[e2e]   feebump[alice]: {ev['action']} {ev['detail'][:70]}")
            if ev["action"] in ("adaptor-fee-bump", "adaptor-rebroadcast"):
                bumped = True
        assert bumped, "stuck refund was not fee-bumped/rebroadcast"

        # Mining settles whichever replacement won; the leg ends up refunded.
        h.pocx.generate(2, "alice_pocx")
        assert h.pocx.rpc("gettxout", a_txid, a_vout) is None, "leg A refund never confirmed"
        print("[e2e] adaptor refund fee-bump OK: stuck refund RBF-escalated then confirmed")
    finally:
        alice.stop()
        bob.stop()


def test_adaptor_redeem_cpfp(h):
    """CPFP bump of the cooperative redeem (v2+). The redeem's fee is sealed in
    the adaptor signature and cannot be RBF'd, so once it is broadcast but still
    unconfirmed the scheduler must CPFP-bump it: a self-funded child spending the
    redeem's own (wallet-owned sweep) output. We broadcast Alice's leg-B redeem,
    DON'T mine it, drive `tick`, and assert an `adaptor-cpfp` event AND that the
    redeem output is now spent by an in-mempool child — then mining settles both."""
    alice = Party("adcp-alice", h, h.workdir, "alice_pocx", "alice_btc", auto_init=False).start()
    bob = Party("adcp-bob", h, h.workdir, "bob_pocx", "bob_btc", auto_init=False).start()
    try:
        alice.setup_seed()
        bob.setup_seed()
        t2, t1 = regtest_timelocks(h)

        init = _env(alice.rpc("adaptorinit", GIVE_POCX, GET_BTC, t1, t2))
        sid = init["swap_id"]
        accept = _env(bob.rpc("adaptoraccept", init))
        alice.rpc("adaptorrecv", accept)

        # Fund both legs and drive the handshake to Signed.
        fa = _env(alice.rpc("adaptorfund", sid))
        h.pocx.generate(1, "alice_pocx")
        bob.rpc("adaptorrecv", fa)
        fb = _env(bob.rpc("adaptorfund", sid))
        h.btc.generate(1, "bob_btc")
        alice.rpc("adaptorrecv", fb)
        na = _env(alice.rpc("adaptornonces", sid))
        nb = _env(bob.rpc("adaptornonces", sid))
        bob.rpc("adaptorrecv", na)
        alice.rpc("adaptorrecv", nb)
        pa = _env(alice.rpc("adaptorsign", sid))
        pb = _env(bob.rpc("adaptorsign", sid))
        bob.rpc("adaptorrecv", pa)
        alice.rpc("adaptorrecv", pb)
        alice.rpc("adaptorassemble", sid)
        bob.rpc("adaptorassemble", sid)
        b_txid, b_vout = fb["body"]["txid"], fb["body"]["vout"]   # leg B (BTC)

        # Alice redeems leg B but we do NOT mine it — it sits unconfirmed.
        alice.rpc("adaptorredeem", sid)
        rec = alice.rpc("listadaptorswaps")[0]
        assert rec["state"] == "redeemed_b", rec["state"]
        redeem_txid = rec["final_txid_b"]
        # No child yet: the redeem output is present in the mempool.
        assert h.btc.rpc("gettxout", redeem_txid, 0) is not None, "redeem output missing pre-CPFP"

        # The redeem's fee was sealed at the 1 sat/vB fallback; raise the market
        # (regtest has none) so the nurse sees it under-priced and CPFP-bumps it.
        alice.rpc("_settestfeerate", 10)

        # The unconfirmed redeem → a tick must CPFP-bump it with a child.
        cpfp = False
        for ev in alice.rpc("tick")["events"]:
            print(f"[e2e]   cpfp[alice]: {ev['action']} {ev['detail'][:70]}")
            if ev["action"] == "adaptor-cpfp":
                cpfp = True
        assert cpfp, "stuck cooperative redeem was not CPFP-bumped"
        # The child now spends the redeem's output (so gettxout reports it spent),
        # and both parent + child sit in the mempool.
        assert h.btc.rpc("gettxout", redeem_txid, 0) is None, "redeem output not spent by a CPFP child"
        assert len(h.btc.rpc("getrawmempool")) >= 2, "expected redeem + child in the mempool"

        # Mining confirms the package; leg B is cooperatively redeemed.
        h.btc.generate(2, "bob_btc")
        assert h.btc.rpc("gettxout", b_txid, b_vout) is None, "leg B redeem never confirmed"
        print("[e2e] adaptor redeem CPFP OK: stuck redeem bumped by a self-funded child")
    finally:
        alice.stop()
        bob.stop()


def test_adaptor_redeem_cpfp_ltc(h):
    """Same CPFP bump as test_adaptor_redeem_cpfp, but on a btcx<->ltc adaptor
    swap so the stuck redeem + child run on LITECOIND (a Core fork). This is the
    first v2 adaptor swap on LTC (the LTC playground is v1-pinned), so it proves
    both that the Taproot/MuSig2 path works on litecoind AND that CPFP — a
    generic Core wallet op (signrawtransactionwithwallet + sendrawtransaction) —
    bumps the redeem there. Requires Harness(with_ltc=True)."""
    assert h.ltc is not None, "this test needs Harness(with_ltc=True)"
    # Litecoin wallets: Bob funds leg B (LTC), so bob_ltc holds the coins;
    # alice_ltc receives the redeem sweep (+ the CPFP child).
    h.ltc.create_wallet("alice_ltc")
    h.ltc.create_wallet("bob_ltc")
    h.ltc.generate(110, "bob_ltc")  # >100 for coinbase maturity

    alice = Party("adcpl-alice", h, h.workdir, "alice_pocx", "alice_btc", auto_init=False,
                  coins_file=COINS_TOML, extra_coins=[("ltc", h.ltc.rpc_url(wallet="alice_ltc"))]).start()
    bob = Party("adcpl-bob", h, h.workdir, "bob_pocx", "bob_btc", auto_init=False,
                coins_file=COINS_TOML, extra_coins=[("ltc", h.ltc.rpc_url(wallet="bob_ltc"))]).start()
    try:
        alice.setup_seed()
        bob.setup_seed()
        t2, t1 = regtest_timelocks(h)

        # leg A = btcx (Alice funds on PoCX); leg B = ltc (Bob funds on LTC).
        init = _env(alice.rpc("adaptorinit", GIVE_POCX, GET_LTC, t1, t2))
        sid = init["swap_id"]
        accept = _env(bob.rpc("adaptoraccept", init))
        alice.rpc("adaptorrecv", accept)

        fa = _env(alice.rpc("adaptorfund", sid))
        h.pocx.generate(1, "alice_pocx")
        bob.rpc("adaptorrecv", fa)
        fb = _env(bob.rpc("adaptorfund", sid))
        h.ltc.generate(1, "bob_ltc")          # leg B 1 conf (regtest n_b = 1)
        alice.rpc("adaptorrecv", fb)
        na = _env(alice.rpc("adaptornonces", sid))
        nb = _env(bob.rpc("adaptornonces", sid))
        bob.rpc("adaptorrecv", na)
        alice.rpc("adaptorrecv", nb)
        pa = _env(alice.rpc("adaptorsign", sid))
        pb = _env(bob.rpc("adaptorsign", sid))
        bob.rpc("adaptorrecv", pa)
        alice.rpc("adaptorrecv", pb)
        alice.rpc("adaptorassemble", sid)
        bob.rpc("adaptorassemble", sid)
        b_txid, b_vout = fb["body"]["txid"], fb["body"]["vout"]   # leg B (LTC)

        # Alice redeems leg B on LTC but we do NOT mine it.
        alice.rpc("adaptorredeem", sid)
        rec = alice.rpc("listadaptorswaps")[0]
        assert rec["state"] == "redeemed_b", rec["state"]
        redeem_txid = rec["final_txid_b"]
        assert h.ltc.rpc("gettxout", redeem_txid, 0) is not None, "LTC redeem output missing pre-CPFP"

        # The redeem's fee was sealed at 2x litecoind's 10 sat/vB floor = 20 sat/vB
        # (committed_mult=2 — the generous v2 redeem default, so the pre-signed
        # key-path fee already carries margin). Raise the market above THAT so the
        # nurse still sees it under-priced and CPFP-bumps it.
        alice.rpc("_settestfeerate", 40)

        # The unconfirmed LTC redeem → a tick must CPFP-bump it on litecoind.
        cpfp = False
        for ev in alice.rpc("tick")["events"]:
            print(f"[e2e]   cpfp-ltc[alice]: {ev['action']} {ev['detail'][:70]}")
            if ev["action"] == "adaptor-cpfp":
                cpfp = True
        assert cpfp, "stuck LTC cooperative redeem was not CPFP-bumped"
        assert h.ltc.rpc("gettxout", redeem_txid, 0) is None, "LTC redeem output not spent by a CPFP child"
        assert len(h.ltc.rpc("getrawmempool")) >= 2, "expected LTC redeem + child in the mempool"

        h.ltc.generate(2, "bob_ltc")
        assert h.ltc.rpc("gettxout", b_txid, b_vout) is None, "LTC leg B redeem never confirmed"
        print("[e2e] adaptor redeem CPFP on LTC OK: stuck litecoind redeem bumped by a child")
    finally:
        alice.stop()
        bob.stop()


def test_adaptor_funding_cpfp(h):
    """The funding-bump nurse (v2, CPFP-via-change). A v2 funding/lock that goes
    out under the market is bumped by a CHILD spending its change output. RBF is
    impossible for a v2 funding — its outpoint is committed into the MuSig2 adaptor
    signatures already exchanged, so changing the txid would invalidate them — so
    the nurse keeps the outpoint FIXED and CPFPs instead (mirrors the redeem-side
    adaptor_cpfp_bump). The swap then completes, proving the adaptor sigs over the
    unchanged outpoint are still valid.

    Reaching Signed with the funding still UNCONFIRMED is fine: signing commits the
    funding *outpoint* (txid:vout), known the moment adaptorfund broadcasts — no
    confirmation is needed until the reveal/redeem depth gate. That unconfirmed
    window is exactly where the nurse acts. As in the v1 test we fund at the
    regtest 1 sat/vB fallback, then raise the market via `_settestfeerate`."""
    alice = Party("adfc-alice", h, h.workdir, "alice_pocx", "alice_btc", auto_init=False).start()
    bob = Party("adfc-bob", h, h.workdir, "bob_pocx", "bob_btc", auto_init=False).start()
    try:
        alice.setup_seed()
        bob.setup_seed()
        t2, t1 = regtest_timelocks(h)
        alice_btc_before = float(h.btc.rpc("getbalance", wallet="alice_btc"))
        bob_pocx_before = float(h.pocx.rpc("getbalance", wallet="bob_pocx"))

        init = _env(alice.rpc("adaptorinit", GIVE_POCX, GET_BTC, t1, t2))
        sid = init["swap_id"]
        accept = _env(bob.rpc("adaptoraccept", init))
        alice.rpc("adaptorrecv", accept)

        # Alice funds leg A cheap — DO NOT mine (leave it unconfirmed for the
        # nurse). Bob funds leg B and confirms it.
        fa = _env(alice.rpc("adaptorfund", sid))
        bob.rpc("adaptorrecv", fa)
        fb = _env(bob.rpc("adaptorfund", sid))
        h.btc.generate(1, "bob_btc")
        alice.rpc("adaptorrecv", fb)
        a_txid, a_vout = fa["body"]["txid"], fa["body"]["vout"]   # leg A (PoCX)
        b_txid, b_vout = fb["body"]["txid"], fb["body"]["vout"]   # leg B (BTC)

        # Handshake to Signed (outpoints known; no confirmation needed to sign).
        na = _env(alice.rpc("adaptornonces", sid)); nb = _env(bob.rpc("adaptornonces", sid))
        bob.rpc("adaptorrecv", na); alice.rpc("adaptorrecv", nb)
        pa = _env(alice.rpc("adaptorsign", sid)); pb = _env(bob.rpc("adaptorsign", sid))
        bob.rpc("adaptorrecv", pa); alice.rpc("adaptorrecv", pb)
        alice.rpc("adaptorassemble", sid); bob.rpc("adaptorassemble", sid)
        assert alice.rpc("listadaptorswaps")[0]["state"] == "signed", "leg A should sign unconfirmed"

        # Funding went out at the 1 sat/vB fallback; raise the market so the nurse
        # sees it under-priced and CPFP-bumps it.
        alice.rpc("_settestfeerate", 10)

        # The unconfirmed, under-priced leg-A funding → a tick CPFP-bumps it
        # (the nurse runs before the reveal logic, so it fires even though leg B
        # is already fundable).
        cpfp = False
        for ev in alice.rpc("tick")["events"]:
            print(f"[e2e]   fundcpfp[alice]: {ev['action']} {ev['detail'][:70]}")
            if ev["action"] == "funding-cpfp-bump":
                cpfp = True
        assert cpfp, "stuck v2 funding was not CPFP-bumped"

        # Child + funding both sit in the mempool, and the leg-A SWAP output (the
        # adaptor-committed outpoint) is UNCHANGED — CPFP spends the change, never
        # the swap output, so the exchanged sigs stay valid.
        assert len(h.pocx.rpc("getrawmempool")) >= 2, "expected funding + child in the mempool"
        assert h.pocx.rpc("gettxout", a_txid, a_vout, True) is not None, \
            "the funding outpoint must be unchanged (CPFP spends change, not the swap output)"
        print(f"[e2e] v2 funding CPFP'd, outpoint {a_txid[:12]}…:{a_vout} intact")

        # Mine the package, then complete the swap — proving the adaptor sigs over
        # the unchanged outpoint still redeem.
        h.pocx.generate(1, "alice_pocx")
        for ev in alice.rpc("tick")["events"]:   # reveal t (redeem leg B)
            print(f"[e2e]   v2[alice]: {ev['action']} {ev['detail'][:60]}")
        assert alice.rpc("listadaptorswaps")[0]["state"] == "redeemed_b", "Alice did not reveal/redeem B"
        h.btc.generate(1, "bob_btc")
        for ev in bob.rpc("tick")["events"]:     # extract t, redeem leg A
            print(f"[e2e]   v2[bob]: {ev['action']} {ev['detail'][:60]}")
        h.pocx.generate(1, "alice_pocx")

        assert h.btc.rpc("gettxout", b_txid, b_vout) is None, "leg B (BTC) not redeemed"
        assert h.pocx.rpc("gettxout", a_txid, a_vout) is None, "leg A (PoCX) not redeemed"
        assert float(h.btc.rpc("getbalance", wallet="alice_btc")) > alice_btc_before, \
            "Alice's leg-B redeem missed her core wallet"
        assert float(h.pocx.rpc("getbalance", wallet="bob_pocx")) > bob_pocx_before, \
            "Bob's leg-A redeem missed his core wallet"
        print("[e2e] funding-cpfp-bump (v2) OK: child bumped the lock, swap completed cleanly")
    finally:
        alice.stop()
        bob.stop()


def test_adaptor_depth_gate(h):
    """Reveal depth gate (spec v2 §8 / v1 §9.5): with `--coin-confs btc=2`, the
    initiator must NOT publish `t` (redeem leg B) until Bob's leg-B funding is 2
    confirmations deep — a shallow funding could reorg out from under the
    reveal. At 1 conf the auto-redeem tick is a no-op; at 2 it fires."""
    alice = Party("adg-alice", h, h.workdir, "alice_pocx", "alice_btc",
                  auto_init=False, coin_confs={"btc": 2}).start()
    bob = Party("adg-bob", h, h.workdir, "bob_pocx", "bob_btc", auto_init=False).start()
    try:
        alice.setup_seed()
        bob.setup_seed()
        t2, t1 = regtest_timelocks(h)

        init = _env(alice.rpc("adaptorinit", GIVE_POCX, GET_BTC, t1, t2))
        sid = init["swap_id"]
        accept = _env(bob.rpc("adaptoraccept", init))
        alice.rpc("adaptorrecv", accept)

        fa = _env(alice.rpc("adaptorfund", sid))
        h.pocx.generate(1, "alice_pocx")
        bob.rpc("adaptorrecv", fa)
        fb = _env(bob.rpc("adaptorfund", sid))
        # Leg B (BTC) gets exactly ONE confirmation — below the configured 2.
        h.btc.generate(1, "bob_btc")
        alice.rpc("adaptorrecv", fb)
        b_txid, b_vout = fb["body"]["txid"], fb["body"]["vout"]

        # Run the full handshake to Signed.
        na = _env(alice.rpc("adaptornonces", sid)); nb = _env(bob.rpc("adaptornonces", sid))
        bob.rpc("adaptorrecv", na); alice.rpc("adaptorrecv", nb)
        pa = _env(alice.rpc("adaptorsign", sid)); pb = _env(bob.rpc("adaptorsign", sid))
        bob.rpc("adaptorrecv", pa); alice.rpc("adaptorrecv", pb)
        alice.rpc("adaptorassemble", sid); bob.rpc("adaptorassemble", sid)

        # At 1 conf (< n_b=2) the reveal gate holds: tick does NOT redeem.
        for ev in alice.rpc("tick")["events"]:
            assert ev["action"] != "adaptor-redeem-b", "revealed t against a shallow (1-conf) funding!"
        assert alice.rpc("listadaptorswaps")[0]["state"] == "signed", "should still be waiting on depth"
        assert h.btc.rpc("gettxout", b_txid, b_vout) is not None, "leg B should be unspent (no reveal yet)"

        # Second confirmation reaches n_b=2 → the next tick reveals + redeems.
        h.btc.generate(1, "bob_btc")
        revealed = any(ev["action"] == "adaptor-redeem-b" for ev in alice.rpc("tick")["events"])
        assert revealed, "reveal did not fire once funding reached n_b confirmations"
        print("[e2e] adaptor depth gate OK: reveal withheld at 1 conf, fired at 2")
    finally:
        alice.stop()
        bob.stop()


def test_adaptor_corkboard_swap(h):
    """Board-driven v2 (the M6 headline): maker posts a PoCX↔BTC offer pinned to
    pact-htlc-v2 (the suite defaults to v1 HTLC; v2 is opt-in via the protocol
    param), taker takes it, and the whole adaptor handshake runs through the
    blind relay, both legs auto-fund, and the swap auto-completes. Drives both
    daemons via tick() like the v1 board test."""
    board = Corkboard(h.workdir)
    board.start()
    # auto_init default (seed created on boot); auto_fund on; no setup_seed —
    # mirrors test_corkboard_swap.
    maker = Party("adcb-maker", h, h.workdir, "alice_pocx", "alice_btc",
                  board_url=board.url, auto_fund=True).start()
    taker = Party("adcb-taker", h, h.workdir, "bob_pocx", "bob_btc",
                  board_url=board.url, auto_fund=True).start()
    try:
        # Pin v2 explicitly (param 4) — it's opt-in now that the default is HTLC.
        offer_id = maker.rpc(
            "boardpostoffer", GIVE_POCX, GET_BTC, 4 * 3600, 2 * 3600, "pact-htlc-v2"
        )["offer_id"]
        listed = next(o for o in taker.rpc("boardlistoffers")["offers"] if o["swap_id"] == offer_id)
        assert listed["body"].get("protocol") == "pact-htlc-v2", listed["body"]
        taker.rpc("boardtake", offer_id)

        # Drive both daemons; mine after each pass so confirmations + MTP advance.
        a = b = []
        for _ in range(15):
            for party in (maker, taker):
                for ev in party.rpc("tick")["events"]:
                    print(f"[e2e]   v2board[{party.name}]: {ev['action']} {ev['detail'][:60]}")
                h.pocx.generate(1, "alice_pocx")
                h.btc.generate(1, "bob_btc")
            a = maker.rpc("listadaptorswaps")
            b = taker.rpc("listadaptorswaps")
            if a and b:
                # maker (initiator) ends at redeemed_b (got its coin); taker
                # (participant) extracts t and reaches completed.
                sa, sb = a[0]["state"], b[0]["state"]
                if sa in ("redeemed_b", "completed") and sb == "completed":
                    print(f"[e2e] board v2 swap completed (maker={sa}, taker={sb})")
                    break
        else:
            raise AssertionError(f"board v2 swap did not complete: a={a}, b={b}")
    finally:
        maker.stop()
        taker.stop()
        board.stop()


def main():
    build_workspace()
    with Harness() as h:
        test_adaptor_swap(h)
        test_adaptor_refund(h)
        test_adaptor_refund_feebump(h)
        test_adaptor_redeem_cpfp(h)
        test_adaptor_funding_cpfp(h)
        test_adaptor_depth_gate(h)
        test_adaptor_corkboard_swap(h)
    # LTC leg in its own harness (brings up litecoind) so the core suite stays
    # litecoind-free: first v2 adaptor swap on LTC + CPFP on litecoind.
    with Harness(with_ltc=True) as h:
        test_adaptor_redeem_cpfp_ltc(h)
    print("[e2e] adaptor-swap suite passed "
          "(happy + refund + fee-bump + redeem-cpfp + funding-cpfp + depth-gate + board + ltc-cpfp)")


if __name__ == "__main__":
    sys.exit(main())
