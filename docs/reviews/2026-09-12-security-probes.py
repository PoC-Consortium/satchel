"""Read-only source review probes against isolated regtest nodes; no production edits.
Run from repository root: python -X utf8 docs/reviews/2026-09-12-security-probes.py
"""
import inspect
import json
import sys
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "pact/harness"))
from framework.testbase import PactTestFramework
from framework.daemon import Party
from framework.services import Corkboard
from tests import swap_v1, swap_v2_adaptor


def losing_after_accepted(h, alice, bob, sid, v2):
    method = "adaptorredeem" if v2 else "redeem"
    if v2:
        bob.rpc(method, sid)
    else:
        bob.cli(method, "--swap", sid)
    rpc = "listadaptorswaps" if v2 else "listswaps"
    rec = bob.rpc(rpc)[0]
    claim_id = rec["final_txid_a" if v2 else "final_txid"]
    assert claim_id in h.pocx.rpc("getrawmempool")
    # Model accepted-but-evicted unconfirmed claims using a real Core restart.
    # Disable wallet rebroadcast so the test controls which conflict is mined.
    restart_time = h.pocx.rpc("getblockheader", h.pocx.rpc("getbestblockhash"))["time"] + 1
    h.pocx.stop()
    h.pocx.start(runtime_args=["-persistmempool=0", "-walletbroadcast=0", f"-mocktime={restart_time}"])
    for wallet in ["alice_btcx", "bob_btcx"]:
        h.pocx.load_wallet(wallet)
    assert claim_id not in h.pocx.rpc("getrawmempool")
    h.advance_time(8 * 3600)
    if v2:
        alice.rpc("adaptorrefund", sid)
    else:
        alice.cli("refund", "--swap", sid)
    h.pocx.generate(2, "alice_btcx")
    events = []
    for _ in range(3):
        events.extend(bob.tick())
    rec = bob.rpc(rpc)[0]
    print("REVIEW accepted-then-lost", json.dumps({
        "v2": v2, "state": rec["state"], "settled": rec["settled"],
        "settlement_loss": rec["settlement_loss"], "actions": events,
        "retains_dead_claim": rec.get("final_txid_a" if v2 else "final_txid") == claim_id,
    }))
    assert rec["state"] == "completed" and not rec["settled"]
    assert not rec["settlement_loss"]


def initiator_stalled(h, alice, bob, sid, v2):
    rpc = "listadaptorswaps" if v2 else "listswaps"
    rec = alice.rpc(rpc)[0]
    claim_id = rec["final_txid_b" if v2 else "final_txid"]
    assert claim_id in h.btc.rpc("getrawmempool")
    restart_time = h.btc.rpc("getblockheader", h.btc.rpc("getbestblockhash"))["time"] + 1
    h.btc.stop()
    h.btc.start(runtime_args=["-persistmempool=0", "-walletbroadcast=0", f"-mocktime={restart_time}"])
    for wallet in ["alice_btc", "bob_btc"]:
        h.btc.load_wallet(wallet)
    assert claim_id not in h.btc.rpc("getrawmempool")
    h.advance_time(8 * 3600)
    if v2:
        bob.rpc("adaptorrefund", sid)
    else:
        bob.cli("refund", "--swap", sid)
    h.btc.generate(2, "bob_btc")
    a_txid = rec["funding_a_txid" if v2 else "htlc_a_txid"]
    a_vout = rec["funding_a_vout" if v2 else "htlc_a_vout"]
    events = []
    for _ in range(3):
        events.extend(alice.tick())
    current = alice.rpc(rpc)[0]
    assert h.pocx.rpc("gettxout", a_txid, a_vout) is not None
    assert current["state"] == "redeemed_b"
    print("REVIEW initiator-refund-starved", json.dumps({"v2": v2,
        "state": current["state"], "leg_a_still_unspent": True,
        "mtp_a": h.pocx.median_time(), "t1": current["t1"], "actions": events}))
    # Prove the refund is actually available now, not merely inferred from clocks.
    if v2:
        alice.rpc("adaptorrefund", sid)
    else:
        alice.cli("refund", "--swap", sid)
    h.pocx.generate(1, "alice_btcx")
    assert h.pocx.rpc("gettxout", a_txid, a_vout) is None
    print("REVIEW manual-refund-succeeded", json.dumps({"v2": v2}))


def transformed(module, function_name, v2, initiator=False):
    source = inspect.getsource(getattr(module, function_name))
    a = source.index("        if late_refund:\n")
    b = source.index("            return\n", a) + len("            return\n")
    before = source[:a]
    if initiator:
        line = '        h.btc.generate(1, "bob_btc")\n'
        pos = before.rindex(line)
        before = before[:pos] + before[pos + len(line):]
    callback = "initiator_stalled" if initiator else "losing_after_accepted"
    source = before + ("        if late_refund:\n"
        f"            {callback}(h, alice, bob, sid, {v2})\n"
        "            return\n") + source[b:]
    namespace = dict(vars(module), losing_after_accepted=losing_after_accepted,
                     initiator_stalled=initiator_stalled)
    exec(compile(source, str(Path(module.__file__)), "exec"), namespace)
    return namespace[function_name]


class AcceptedClaimLostV1(PactTestFramework):
    def run_test(self):
        transformed(swap_v1, "test_complete_swap", False)(self.h, late_refund=True)


class AcceptedClaimLostV2(PactTestFramework):
    def run_test(self):
        transformed(swap_v2_adaptor, "test_adaptor_swap", True)(self.h, late_refund=True)


class InitiatorRefundStarvedV1(PactTestFramework):
    def run_test(self):
        transformed(swap_v1, "test_complete_swap", False, True)(self.h, late_refund=True)


class InitiatorRefundStarvedV2(PactTestFramework):
    def run_test(self):
        transformed(swap_v2_adaptor, "test_adaptor_swap", True, True)(self.h, late_refund=True)


class RevokedOfferReplay(PactTestFramework):
    def run_test(self):
        board = Corkboard(self.workdir)
        board.start()
        maker = Party("replay-maker", self.h, self.workdir, "alice_btcx", "alice_btc",
                      board_url=board.url).start()
        def offers():
            with urllib.request.urlopen(board.url + "/v1/offers") as response:
                return json.load(response)["offers"]
        try:
            offer_id = maker.rpc("boardpostoffer", "btcx:50", "btc:0.001", 14400, 7200,
                                 "pact-htlc-v1")["offer_id"]
            captured = next(e for e in offers() if e["swap_id"] == offer_id)
            maker.rpc("boardrevoke", offer_id)
            assert not offers()
            # No private key needed: replay the previously public signed bytes.
            request = urllib.request.Request(board.url + "/v1/offers",
                data=json.dumps(captured).encode(), headers={"Content-Type": "application/json"})
            with urllib.request.urlopen(request) as response:
                response.read()
            revived = any(e["swap_id"] == offer_id for e in offers())
            print("REVIEW revoked-offer-replay", json.dumps({"revived": revived}))
            assert revived
        finally:
            maker.stop()
            board.stop()


if __name__ == "__main__":
    for scenario in [AcceptedClaimLostV1, AcceptedClaimLostV2,
                     InitiatorRefundStarvedV1, InitiatorRefundStarvedV2, RevokedOfferReplay]:
        scenario().run()
