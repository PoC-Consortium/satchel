"""Post-acceptance settlement conflicts on real, isolated regtest nodes."""
import os
import sqlite3


def evict(node, wallets):
    """Drop the mempool on restart without changing the confirmed chain."""
    clock = node.rpc("getblockheader", node.rpc("getbestblockhash"))["time"] + 1
    node.stop()
    node.start(runtime_args=["-persistmempool=0", "-walletbroadcast=0", f"-mocktime={clock}"])
    for wallet in wallets:
        node.load_wallet(wallet)


def settlement_case(h, alice, bob, sid, v2, case):
    listing = "listadaptorswaps" if v2 else "listswaps"
    def record(party):
        return next(r for r in party.rpc(listing) if r["swap_id"] == sid)
    def redeem(party):
        if v2:
            return party.rpc("adaptorredeem", sid)["record"]
        return party.rpc("redeem", sid)["record"]
    def refund(party):
        return party.rpc("adaptorrefund" if v2 else "refund", sid)["record"]
    def meta(party, key):
        with sqlite3.connect(os.path.join(party.data_dir, "pact.sqlite")) as db:
            row = db.execute("SELECT value FROM meta WHERE key=?", (key,)).fetchone()
            return row[0] if row else None
    def ticks(party, count=4):
        events = []
        for _ in range(count):
            events.extend(party.tick())
        assert not any(e["action"] == "error" for e in events), events
        return events

    initial = record(alice)
    original_reveal = initial["final_tx_b_hex" if v2 else "final_tx_hex"]
    reveal_id = initial["final_txid_b" if v2 else "final_txid"]
    a_id = initial["funding_a_txid" if v2 else "htlc_a_txid"]
    a_vout = initial["funding_a_vout" if v2 else "htlc_a_vout"]
    assert reveal_id in h.btc.rpc("getrawmempool"), "first reveal must have been accepted"

    if case == "participant_conflict":
        h.btc.generate(initial["n_b"], "bob_btc")
        claimed = redeem(bob)
        dead_claim = claimed["final_txid_a" if v2 else "final_txid"]
        assert dead_claim in h.pocx.rpc("getrawmempool")
        assert meta(bob, f"claim_pending:{sid}") is None
        bob.tick()  # exercise a conflict AFTER the initial reconcile pass
        evict(h.pocx, ["alice_btcx", "bob_btcx"])
        h.advance_time(8 * 3600)
        refund(alice)
        h.pocx.generate(2, "alice_btcx")
        ticks(bob)
        lost = record(bob)
        assert lost["state"] == "refunded" and lost["settled"] and lost["settlement_loss"], lost
        fields = ["final_txid_a", "final_tx_a_hex", "final_txid_b", "final_tx_b_hex"] if v2 else ["final_txid", "final_tx_hex"]
        assert all(lost.get(field) is None for field in fields), lost
        assert meta(bob, f"claim_pending:{sid}") is None
        assert not bob.tick(), "settled loss must retire the nurse"
    elif case == "lost_refund":
        h.btc.generate(initial["n_b"], "bob_btc")
        h.advance_time(8 * 3600)
        refunded = refund(alice)
        dead_refund = refunded["final_txid_a" if v2 else "final_txid"]
        assert dead_refund in h.pocx.rpc("getrawmempool")
        evict(h.pocx, ["alice_btcx", "bob_btcx"])
        redeem(bob)
        h.pocx.generate(2, "alice_btcx")
        ticks(alice)
        won = record(alice)
        assert won["state"] == "completed" and won["settled"] and not won["settlement_loss"], won
        assert won["final_txid_b" if v2 else "final_txid"] == reveal_id, won
        assert not alice.tick()
    elif case == "final":
        h.btc.generate(initial["n_b"], "bob_btc")
        h.advance_time(8 * 3600)
        events = ticks(alice)
        assert record(alice)["state"] == "completed" and record(alice)["settled"]
        assert h.pocx.rpc("gettxout", a_id, a_vout) is not None
        assert not any("refund" in e["action"] for e in events), events
    else:
        if case == "shallow":
            assert initial["n_b"] > 1
            h.btc.generate(1, "bob_btc")
            h.pocx.set_mocktime(initial["t1"] + 86400)
            h.pocx.generate(11, "alice_btcx")
        else:
            evict(h.btc, ["alice_btc", "bob_btc"])
            h.advance_time(8 * 3600)
            if case in ("initiator_conflict", "outage"):
                refund(bob)
                h.btc.generate(2, "bob_btc")
            if case == "outage":
                # Also cover a retained write-ahead marker failing before the
                # normal RedeemedB arm could run.
                with sqlite3.connect(os.path.join(alice.data_dir, "pact.sqlite")) as db:
                    db.execute("INSERT OR REPLACE INTO meta(key,value) VALUES (?, 'claim')", (f"claim_pending:{sid}",))
                h.btc.stop()
        assert h.pocx.median_time() >= initial["t1"]
        events = ticks(alice, 1)
        assert any("refund" in e["action"] for e in events), events
        refunded = record(alice)
        assert refunded["state"] == "refunded" and not refunded["settlement_loss"], refunded
        assert h.pocx.rpc("gettxout", a_id, a_vout) is None, "refund must broadcast automatically"
        if v2:
            assert refunded["final_tx_b_hex"] == original_reveal
        else:
            assert meta(alice, f"reveal_tx:{sid}:b") == original_reveal
        h.pocx.generate(2, "alice_btcx")
        ticks(alice)
        assert record(alice)["settled"], record(alice)
        assert meta(alice, f"claim_pending:{sid}") is None
    print(f"[e2e] settlement recovery v{2 if v2 else 1} {case} passed")
