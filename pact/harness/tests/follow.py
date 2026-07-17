#!/usr/bin/env python3
"""Dormant-observer state reconstruction e2e (docs/STATE_RECONSTRUCTION.md),
from the former test_follow_e2e.py (bodies verbatim; cell rationale in the
docstrings). Each cell runs on a fresh cached stack with BOTH legs served by
a Core-RPC + electrs multi-backend.

Run:  python tests/follow.py [--filter SUBSTR] [--keep] [--no-build]
"""

import time
import os
import sys

sys.path.insert(0, os.path.normpath(
    os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")))

from framework.binaries import find_btc_electrs  # noqa: E402
from framework.daemon import Party  # noqa: E402
from framework.node import (  # noqa: E402
    BTC_ELECTRS_ELECTRUM_PORT,
    BTC_ELECTRS_MONITORING_PORT,
    ElectrsServer,
)
from framework.services import NostrRelay  # noqa: E402
from framework.testbase import PactTestFramework, run_scenarios  # noqa: E402
from framework.util import GET_BTC, GIVE_POCX, handshake_done  # noqa: E402


# Standard BIP39 English test vectors (checksum-valid, NOT for real funds),
# distinct from the rescue suite's so the relay scans never cross-pollinate.
FOLLOW_MNEMONIC_S1 = ("scheme spot photo card baby mountain device kick "
                      "cradle pact join borrow")
FOLLOW_MNEMONIC_S2 = ("cat swing flag economy stadium alone churn speed "
                      "unique patch report train")
FOLLOW_MNEMONIC_S3 = ("vessel ladder alter error federal sibling chat "
                      "ability sun glass valve picture")

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
        if handshake_done(maker, taker):
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


def scenario_wallet_assisted_core_only(h, ep, eb):
    """Cell 3 (#171): the observer's btcx backend is CORE-ONLY (no electrs) —
    the exact mainnet shape of the 2026-07-12 ghost. The backup-session
    contract shares the node wallet, so the observer must still resolve a
    followed swap that settles: the taker's claim of the btcx leg is a
    wallet receive (witness-script probed), the btc leg classifies from
    electrs — the record purges promptly via `followed-purged`, NOT the
    24h age-out."""
    relay = NostrRelay(h.workdir)
    mk_bx, mk_bt = multi_urls(h, ep, eb, "alice_pocx", "alice_btc")
    tk_bx, tk_bt = multi_urls(h, ep, eb, "bob_pocx", "bob_btc")
    maker = Party("fmk3", h, h.workdir, "alice_pocx", "alice_btc",
                  nostr_relays=relay.ws_url, auto_fund=True,
                  pocx_url=mk_bx, btc_url=mk_bt)
    taker = Party("ftk3", h, h.workdir, "bob_pocx", "bob_btc",
                  nostr_relays=relay.ws_url, auto_fund=True, auto_init=False,
                  pocx_url=tk_bx, btc_url=tk_bt)
    obs = None
    try:
        relay.start()
        maker.start()
        taker.start()
        taker.setup_seed(mnemonic=FOLLOW_MNEMONIC_S3)

        # Drive mid-flight (both legs committed), then activate the observer:
        # bob's SAME node wallet for btcx, but Core-RPC ONLY — tier L.
        def both_funded():
            m, t = swap_of(maker), swap_of(taker)
            return (m is not None and m.get("htlc_a_txid")
                    and t is not None and t.get("htlc_b_txid"))

        sid = drive_swap_over_relay(h, ep, eb, maker, taker, both_funded)
        obs = Party("fob3", h, h.workdir, "bob_pocx", "bob_btc",
                    nostr_relays=relay.ws_url, auto_fund=True, auto_init=False,
                    pocx_url=h.pocx.rpc_url(wallet="bob_pocx"),  # Core-only!
                    btc_url=tk_bt)
        obs.start()
        obs.setup_seed(mnemonic=FOLLOW_MNEMONIC_S3)
        rec = None
        for _ in range(30):
            tick_all("import3", obs)
            rec = swap_of(obs, sid)
            if rec is not None:
                break
            time.sleep(0.5)
        assert rec is not None, "cell3: snapshot never imported as followed"
        assert rec.get("source") == "foreign"

        # The primary (taker) completes the swap with the maker — the
        # observer only watches.
        for _ in range(40):
            tick_all("settle3", maker, taker)
            m, t = swap_of(maker, sid), swap_of(taker, sid)
            if m and t and m["state"] == "completed" and t["state"] == "completed":
                break
            mine_and_sync(h, ep, eb)
        else:
            raise AssertionError("cell3: swap never completed")
        mine_and_sync(h, ep, eb, n=2)  # bury the claims past needed depth

        # The observer must resolve the followed record from the SHARED
        # WALLET (btcx leg: the taker's claim is a bob_pocx receive) + electrs
        # (btc leg) — a prompt followed-purged, never the age-out. The purge
        # is an ENGINE tick event (rides the tick RPC), unlike the async
        # import-scan events that go to the log.
        seen = []
        for _ in range(30):
            seen += tick_all("resolve3", obs)
            if swap_of(obs, sid) is None:
                break
            mine_and_sync(h, ep, eb)
            time.sleep(0.3)
        assert swap_of(obs, sid) is None, \
            "cell3: followed record never resolved on a Core-only btcx backend"
        assert "followed-purged" in seen, \
            f"cell3: expected a wallet-evidence purge, tick events: {seen}"
        assert "followed-aged-out" not in seen, \
            "cell3: resolution must come from wallet evidence, not the age-out"
        print("[follow-e2e] cell3 OK: Core-only observer resolved the settled "
              "swap via the shared wallet")
    finally:
        for p in (maker, taker, obs):
            if p is not None:
                try:
                    p.stop()
                except Exception:  # noqa: BLE001
                    pass
        relay.stop()


class _FollowScenario(PactTestFramework):
    """Both nodes REST-enabled with an electrs each (the production mixed
    multi-backend shape the reconstruction needs)."""

    pocx_rest = True
    btc_rest = True
    scenario = None

    def run_test(self):
        ep = ElectrsServer(self.h.workdir, self.h.pocx)
        eb = ElectrsServer(self.h.workdir, self.h.btc,
                           electrum_port=BTC_ELECTRS_ELECTRUM_PORT,
                           monitoring_port=BTC_ELECTRS_MONITORING_PORT,
                           network="testnet", binary=find_btc_electrs(),
                           name="btc-electrs")
        try:
            ep.start()
            eb.start()
            ep.wait_synced(self.h.pocx.rpc("getblockcount"))
            eb.wait_synced(self.h.btc.rpc("getblockcount"))
            print("[follow-e2e] both electrs synced")
            type(self).scenario(self.h, ep, eb)
        finally:
            ep.stop()
            eb.stop()


class ObserveAfterCompletion(_FollowScenario):
    scenario = staticmethod(scenario_observe_after_completion)


class DormantObserverTakeover(_FollowScenario):
    scenario = staticmethod(scenario_dormant_observer_takeover)


class WalletAssistedCoreOnly(_FollowScenario):
    scenario = staticmethod(scenario_wallet_assisted_core_only)


SCENARIOS = [
    ObserveAfterCompletion,
    DormantObserverTakeover,
    WalletAssistedCoreOnly,
]


if __name__ == "__main__":
    run_scenarios(SCENARIOS)
