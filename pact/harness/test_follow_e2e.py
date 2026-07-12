#!/usr/bin/env python3
"""Dormant-observer state reconstruction e2e (docs/STATE_RECONSTRUCTION.md).

Two focused scenarios over a live Nostr relay, with BOTH legs served by a
Core-RPC + electrs multi-backend (the production mixed shape, and the
history-capable tier the reconstruction needs):

  1. observe-after-completion — the 2026-07-12 field ghost: a same-seed
     OBSERVER machine that first sees a swap AFTER it settled must classify
     that from chain history (the snapshot lingers because only the owner
     tombstones it, and the owner died at `redeemed_b`). The swap must never
     surface as a followed ghost — and the observer must never fund anything.

  2. dormant-observer takeover — the observer imports MID-SWAP (both legs
     funded), reconstructs the funding pointers from chain (the v1 accept
     snapshot carries none), and after the owner machine is STOPPED takes the
     swap over and drives it to completion with the live taker — adopting,
     never re-funding, the owner's leg-A funding (txid must be identical).

Deliberately NOT a full matrix (v2 witness classification and the tier-L
age-out are covered by Rust unit tests in libswap::reconstruct and the
follow evaluator); these two cells exercise the end-to-end seams: snapshot
import, chain reconstruction, purge, takeover fast-forward, no-double-fund.
"""

import os
import sys
import time

from regtest_harness import (
    BTC_ELECTRS_ELECTRUM_PORT,
    BTC_ELECTRS_MONITORING_PORT,
    ElectrsServer,
    Harness,
    find_btc_electrs,
)
from test_swap_e2e import GET_BTC, GIVE_POCX, NostrRelay, Party, build_workspace

# Standard BIP39 English test vectors (checksum-valid, NOT for real funds),
# distinct from the rescue suite's so the relay scans never cross-pollinate.
FOLLOW_MNEMONIC_S1 = ("scheme spot photo card baby mountain device kick "
                      "cradle pact join borrow")
FOLLOW_MNEMONIC_S2 = ("cat swing flag economy stadium alone churn speed "
                      "unique patch report train")

TERMINAL_FOLLOW_EVENTS = ("followed-skipped-terminal", "followed-purged",
                          "followed-aged-out")


def swap_of(p, sid=None):
    sw = (p.rpc("listswaps") or []) + (p.rpc("listadaptorswaps") or [])
    if sid is not None:
        sw = [s for s in sw if s["swap_id"] == sid]
    return sw[0] if sw else None


def follow_log_events(party):
    """The followed-import scan reports through pactd's tracing (it runs
    async off the tick RPC), so its events are read from the party's log."""
    path = os.path.join(party.data_dir, "pactd.log")
    try:
        with open(path, encoding="utf-8", errors="replace") as fh:
            text = fh.read()
    except OSError:
        return []
    return [ev for ev in TERMINAL_FOLLOW_EVENTS + ("followed-imported",)
            if ev in text]


def mine_and_sync(h, ep, eb, n=1):
    h.pocx.generate(n, "alice_pocx")
    h.btc.generate(n, "bob_btc")
    ep.wait_synced(h.pocx.rpc("getblockcount"))
    eb.wait_synced(h.btc.rpc("getblockcount"))


def multi_urls(h, ep, eb, pocx_wallet, btc_wallet):
    """Core-RPC primary (wallet) + electrs view — per coin."""
    return (f"{h.pocx.rpc_url(wallet=pocx_wallet)},{ep.url}",
            f"{h.btc.rpc_url(wallet=btc_wallet)},{eb.url}")


def tick_all(tag, *parties):
    events = []
    for p in parties:
        if p is None or p.proc is None:
            continue
        for ev in p.rpc("tick")["events"]:
            print(f"[follow-e2e]   {tag}[{p.name}]: {ev['action']} {ev['detail'][:70]}")
            events.append(ev["action"])
    return events


def drive_swap_over_relay(h, ep, eb, maker, taker, stop_when, rounds=60):
    """Post a v1 offer, take it, tick both until `stop_when()` — breaking the
    INSTANT it holds so the caller lands mid-flight. Returns the swap id."""
    offer_id = maker.rpc("boardpostoffer", f"btcx:{GIVE_POCX}", f"btc:{GET_BTC}",
                         4 * 3600, 2 * 3600, "pact-htlc-v1")["offer_id"]
    for _ in range(25):
        maker.rpc("tick")
        taker.rpc("tick")
        if any(o["swap_id"] == offer_id
               for o in taker.rpc("boardlistoffers")["offers"]):
            break
    else:
        raise AssertionError("offer never propagated to the taker over the relay")
    taker.rpc("boardtake", offer_id)
    for _ in range(rounds):
        tick_all("drive", maker, taker)
        if stop_when():
            # The SWAP id is minted in the take→init handshake (initiator
            # counter, spec §4.2) — it is NOT the board offer id.
            rec = swap_of(maker) or swap_of(taker)
            return rec["swap_id"]
        mine_and_sync(h, ep, eb)
    raise AssertionError("drive condition never reached")


def scenario_observe_after_completion(h, ep, eb):
    """Cell 1: the observer first sees the swap AFTER it settled → no ghost."""
    relay = NostrRelay(h.workdir)
    bx, bt = multi_urls(h, ep, eb, "alice_pocx", "alice_btc")
    tx_bx, tx_bt = multi_urls(h, ep, eb, "bob_pocx", "bob_btc")
    maker = Party("fmk1", h, h.workdir, "alice_pocx", "alice_btc",
                  nostr_relays=relay.ws_url, auto_fund=True, auto_init=False,
                  pocx_url=bx, btc_url=bt)
    taker = Party("ftk1", h, h.workdir, "bob_pocx", "bob_btc",
                  nostr_relays=relay.ws_url, auto_fund=True,
                  pocx_url=tx_bx, btc_url=tx_bt)
    obs = None
    try:
        relay.start()
        maker.start()
        taker.start()
        maker.setup_seed(mnemonic=FOLLOW_MNEMONIC_S1)

        # Owner dies at `redeemed_b` — AFTER revealing s, BEFORE the
        # Completed transition that would tombstone its relay snapshot. The
        # lingering snapshot is exactly the field shape.
        sid = drive_swap_over_relay(
            h, ep, eb, maker, taker,
            lambda: (swap_of(maker) or {}).get("state") == "redeemed_b")
        print(f"[follow-e2e] cell1: maker at redeemed_b — stopping (snapshot lingers)")
        maker.stop()

        # The live taker finishes the swap (extracts s, claims leg A)…
        for _ in range(30):
            tick_all("settle", taker)
            s = swap_of(taker, sid)
            if s and s["state"] == "completed":
                break
            mine_and_sync(h, ep, eb)
        else:
            raise AssertionError("taker never completed after the maker died")
        # …and everything gets buried past the finality depth.
        mine_and_sync(h, ep, eb, n=3)

        # A fresh same-seed machine boots long after the fact (new data dir →
        # new machine scope → the snapshot reads as another machine's swap).
        obs = Party("fob1", h, h.workdir, "alice_pocx", "alice_btc",
                    nostr_relays=relay.ws_url, auto_fund=True, auto_init=False,
                    pocx_url=bx, btc_url=bt)
        obs.start()
        obs.setup_seed(mnemonic=FOLLOW_MNEMONIC_S1)
        seen = []
        terminal = []
        for _ in range(30):
            seen += tick_all("observe", obs)
            terminal = [e for e in follow_log_events(obs)
                        if e in TERMINAL_FOLLOW_EVENTS]
            if terminal and swap_of(obs, sid) is None:
                break
            time.sleep(0.5)
        assert terminal, \
            f"observer never classified the settled swap as terminal " \
            f"(log events: {follow_log_events(obs)}, tick events: {seen})"
        assert swap_of(obs, sid) is None, \
            "settled swap ghosted on the observer (the 2026-07-12 field bug)"
        # The observer must never have committed funds for the dead swap.
        for forbidden in ("auto-fund", "funded-a", "adaptor-fund-b"):
            assert forbidden not in seen, f"observer funded a settled swap: {seen}"
        # The purged memo also shields the confirm-gated rescue path.
        st = obs.rpc("rescuestatus")
        assert st["pending"] == 0, \
            f"rescuestatus still offers the settled swap: {st}"
        print("[follow-e2e] cell1 OK: settled swap never ghosted, nothing funded")
    finally:
        for p in (maker, taker, obs):
            if p is not None:
                try:
                    p.stop()
                except Exception:  # noqa: BLE001
                    pass
        relay.stop()


def scenario_dormant_observer_takeover(h, ep, eb):
    """Cell 2: observer activates mid-swap, sees the reconstructed state,
    takes over after the owner stops, and completes — without re-funding."""
    relay = NostrRelay(h.workdir)
    bx, bt = multi_urls(h, ep, eb, "alice_pocx", "alice_btc")
    tx_bx, tx_bt = multi_urls(h, ep, eb, "bob_pocx", "bob_btc")
    maker = Party("fmk2", h, h.workdir, "alice_pocx", "alice_btc",
                  nostr_relays=relay.ws_url, auto_fund=True, auto_init=False,
                  pocx_url=bx, btc_url=bt)
    taker = Party("ftk2", h, h.workdir, "bob_pocx", "bob_btc",
                  nostr_relays=relay.ws_url, auto_fund=True,
                  pocx_url=tx_bx, btc_url=tx_bt)
    obs = None
    try:
        relay.start()
        maker.start()
        taker.start()
        maker.setup_seed(mnemonic=FOLLOW_MNEMONIC_S2)

        # Drive until BOTH legs are committed (leg B possibly still
        # unconfirmed — deliberately mid-flight), then close the main session.
        def both_funded():
            m, t = swap_of(maker), swap_of(taker)
            return (m is not None and m.get("htlc_a_txid")
                    and t is not None and t.get("htlc_b_txid"))

        sid = drive_swap_over_relay(h, ep, eb, maker, taker, both_funded)
        pre = swap_of(maker, sid)
        pre_a = pre["htlc_a_txid"]
        # Two more ticks so the accept snapshot is ON the relay for sure.
        for _ in range(2):
            maker.rpc("tick")
            time.sleep(1)
        print(f"[follow-e2e] cell2: both legs funded (A {pre_a[:16]}) — "
              "closing the main session")
        maker.stop()

        # The dormant observer activates mid-swap.
        obs = Party("fob2", h, h.workdir, "alice_pocx", "alice_btc",
                    nostr_relays=relay.ws_url, auto_fund=True, auto_init=False,
                    pocx_url=bx, btc_url=bt)
        obs.start()
        obs.setup_seed(mnemonic=FOLLOW_MNEMONIC_S2)
        rec = None
        for _ in range(30):
            tick_all("import", obs)
            rec = swap_of(obs, sid)
            if rec is not None:
                break
            time.sleep(0.5)
        assert rec is not None, "foreign snapshot never auto-imported as followed"
        assert rec.get("source") == "foreign", f"import must be read-only: {rec}"

        # "Sees the state right": the v1 snapshot was taken at ACCEPT and
        # carries NO funding pointers — the ones on the followed record can
        # only come from the observer's own chain reconstruction.
        for _ in range(20):
            rec = swap_of(obs, sid)
            if rec.get("htlc_a_txid") and rec.get("htlc_b_txid"):
                break
            tick_all("reconstruct", obs)
            time.sleep(0.3)
        assert rec.get("htlc_a_txid") == pre_a, \
            f"leg-A pointer not reconstructed from chain: {rec}"
        assert rec.get("htlc_b_txid"), \
            f"leg-B pointer not reconstructed from chain: {rec}"

        # The observer's dock line must COUNT like the owner's (the "locking…
        # with no block count" observer gap): the followed swap gets a
        # chain-derived SwapProgress entry — a burying lock (their_lock,
        # confs/needed) or, once a leg is deep, an awaiting liveness count.
        line = None
        for _ in range(10):
            tick_all("progress", obs)
            line = next((q for q in obs.rpc("swapprogress")
                         if q["swap_id"] == sid), None)
            if line is not None:
                break
            time.sleep(0.3)
        assert line is not None, "no progress line for the followed swap"
        assert line["watching"] in ("their_lock", "awaiting_claim",
                                    "awaiting_lock"), \
            f"unexpected followed progress phase: {line}"
        assert line["watching"] != "their_lock" or line["needed"] > 0, \
            f"followed their_lock line has no counting target: {line}"

        # Take over (the owner is provably stopped) and drive to completion
        # with the live taker.
        obs.rpc("takeover", sid)
        rec = swap_of(obs, sid)
        assert rec.get("source") != "foreign", f"takeover did not adopt: {rec}"
        for _ in range(40):
            tick_all("finish", obs, taker)
            o, t = swap_of(obs, sid), swap_of(taker, sid)
            if o and t and o["state"] == "completed" and t["state"] == "completed":
                break
            mine_and_sync(h, ep, eb)
        else:
            raise AssertionError(
                f"takeover never completed: obs={swap_of(obs, sid)} "
                f"taker={swap_of(taker, sid)}")

        # The no-double-fund invariant: the adopted swap kept the ORIGINAL
        # leg-A funding, it never broadcast a second one.
        assert swap_of(obs, sid)["htlc_a_txid"] == pre_a, \
            "takeover re-funded leg A (double-fund!)"
        print("[follow-e2e] cell2 OK: mid-swap takeover completed on the "
              "original funding")
    finally:
        for p in (maker, taker, obs):
            if p is not None:
                try:
                    p.stop()
                except Exception:  # noqa: BLE001
                    pass
        relay.stop()


def main():
    build_workspace()
    keep = "--keep" in sys.argv
    with Harness(keep=keep, pocx_rest=True, btc_rest=True) as h:
        ep = ElectrsServer(h.workdir, h.pocx)
        eb = ElectrsServer(h.workdir, h.btc,
                           electrum_port=BTC_ELECTRS_ELECTRUM_PORT,
                           monitoring_port=BTC_ELECTRS_MONITORING_PORT,
                           network="testnet", binary=find_btc_electrs(),
                           name="btc-electrs")
        try:
            ep.start()
            eb.start()
            ep.wait_synced(h.pocx.rpc("getblockcount"))
            eb.wait_synced(h.btc.rpc("getblockcount"))
            print("[follow-e2e] both electrs synced")
            scenario_observe_after_completion(h, ep, eb)
            scenario_dormant_observer_takeover(h, ep, eb)
            print("[follow-e2e] ALL CELLS GREEN")
        finally:
            ep.stop()
            eb.stop()


if __name__ == "__main__":
    main()
