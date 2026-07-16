#!/usr/bin/env python3
"""ONE flag-composed playground (TEST_FRAMEWORK_PLAN §2.5; closes #110).

    python -m play [--board cork|nostr] [--btcx node|nodeless] [--electrs N]
                   [--satchel one|two-observer|viewer|none]
                   [--first-run] [--relay-cmd CMD] [--persist] [--keep]
                   [--no-build] [--down]

The old scripts map to:
    playground-cork.ps1            -> --board cork                    (default)
    playground-nostr.ps1           -> --board nostr
    playground-nodeless.ps1        -> --board cork  --btcx nodeless [--electrs 4]
    playground-nostr-nodeless.ps1  -> --board nostr --btcx nodeless
    playground-observer.ps1        -> --satchel two-observer
    playground.py (headless)       -> --satchel none
    playground-viewer.ps1          -> --satchel viewer            (mainnet)
    prod-watch-viewer.ps1          -> --satchel viewer --persist
    knockdown.ps1                  -> --down

SAFETY: teardown is PID/port-only off framework.util's mainnet-safe registry —
never by process name, never 9737/9738 (the live mainnet/testnet pactd).
Close the Satchel window (the MAIN window in two-observer) to tear the whole
stack down; --keep leaves the backdrop running until Ctrl+C instead.
"""

import argparse
import os
import sys
import time

sys.path.insert(0, os.path.normpath(
    os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")))

from framework import binaries, satchel, stack  # noqa: E402
from framework.clock import PacedMiner  # noqa: E402
from framework.daemon import Party  # noqa: E402
from framework.market import (  # noqa: E402
    BOB_LTC_OFFERS,
    BOB_OFFERS,
    CAROL_LTC_OFFERS,
    CAROL_OFFERS,
    OBSERVER_BOB_OFFERS,
    OBSERVER_CAROL_OFFERS,
    AutoTaker,
    Book,
    Faucet,
    Market,
    managed_rpc,
)
from framework.node import (  # noqa: E402
    BTC_ELECTRS_ELECTRUM_PORT,
    BTC_ELECTRS_MONITORING_PORT,
    BTC_RPC_PORT,
    ELECTRS_ELECTRUM_PORT,
    ELECTRS_MONITORING_PORT,
    LTC_RPC_PORT,
    POCX_RPC_PORT,
    ElectrsServer,
    Harness,
)
from framework.services import (  # noqa: E402
    PLAYGROUND_NOSTR_RELAY_PORT,
    Corkboard,
    NostrRelay,
)
from framework.stack import COINS_TOML  # noqa: E402

# The multi-electrs drill's DEAD endpoint: nothing ever listens here, proving
# a down server stays a cold standby (never dialed into a stall).
DEAD_ELECTRS = "tcp://127.0.0.1:19999"

# The observer pair's shared Pact seed — a CHECKSUM-VALID BIP39 test vector so
# both wizards import the SAME identity with no copy-paste. NOT for funds.
FOLLOW_MNEMONIC = (
    "legal winner thank year wave sausage worth useful legal winner thank yellow"
)


def parse_args():
    ap = argparse.ArgumentParser(prog="python -m play", description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--board", choices=("cork", "nostr"), default="cork",
                    help="offer transport: HTTP corkboard (default) or a local Nostr relay")
    ap.add_argument("--btcx", choices=("node", "nodeless"), default="node",
                    help="Alice's BTCX leg: node-backed (default) or pact-seed bdk over electrs")
    ap.add_argument("--electrs", type=int, default=1, metavar="N",
                    help="nodeless only: N independent electrs over the PoCX node "
                         "(N>1 = the active-set failover fleet + a dead endpoint)")
    ap.add_argument("--satchel", choices=("one", "two-observer", "viewer", "none"),
                    default="one",
                    help="one GUI (default), the main+observer pair, the mainnet "
                         "board viewer (no backdrop), or headless (no GUI)")
    ap.add_argument("--first-run", action="store_true",
                    help="ship Satchel with NO coins pre-wired (exercise onboarding + coin setup)")
    ap.add_argument("--relay-cmd", metavar="CMD",
                    help="nostr relay launch command override ({port}/{dir} substituted)")
    ap.add_argument("--persist", action="store_true",
                    help="viewer only: persistent config dir (your imported seed survives runs)")
    ap.add_argument("--keep", action="store_true",
                    help="when the Satchel window closes, keep the backdrop running until Ctrl+C")
    ap.add_argument("--no-build", action="store_true",
                    help="skip cargo builds (binaries already fresh)")
    ap.add_argument("--down", action="store_true",
                    help="force-tear a stale run (full mainnet-safe port registry) and exit")
    return ap.parse_args()


def compose_alice_coins(args):
    """The satchel.json coins block per mode (shapes lifted verbatim from the
    retired ps1 wrappers)."""
    if args.first_run:
        return []
    if args.satchel == "two-observer":
        # Both legs Electrum (pact-seed) so the observer history-classifies
        # whichever side redeems; mainnet-like confs matching the bots.
        return [
            satchel.electrum_coin("btcx", [f"tcp://127.0.0.1:{ELECTRS_ELECTRUM_PORT}"], 6),
            satchel.electrum_coin("btc", [f"tcp://127.0.0.1:{BTC_ELECTRS_ELECTRUM_PORT}"], 4),
        ]
    nodeless = args.btcx == "nodeless"
    if nodeless:
        # Active-set seniority: [0]=wallet home, [1..]=views, then standbys;
        # with a fleet the DEAD endpoint rides along as a cold standby.
        servers = [f"tcp://127.0.0.1:{ELECTRS_ELECTRUM_PORT + 2 * i}"
                   for i in range(max(1, args.electrs))]
        if args.electrs > 1:
            servers.append(DEAD_ELECTRS)
        btcx = satchel.electrum_coin("btcx", servers, 10)
        if args.board == "nostr":
            btc = satchel.electrum_coin(
                "btc", [f"tcp://127.0.0.1:{BTC_ELECTRS_ELECTRUM_PORT}"], 6)
        else:
            btc = satchel.core_rpc_coin("btc", BTC_RPC_PORT, "alice_btc", 6)
    else:
        btcx = satchel.core_rpc_coin("btcx", POCX_RPC_PORT, "alice_pocx", 10)
        btc = satchel.core_rpc_coin("btc", BTC_RPC_PORT, "alice_btc", 6)
    ltc = satchel.core_rpc_coin("ltc", LTC_RPC_PORT, "alice_ltc", 6)
    return [btcx, btc, ltc]


def print_banner(args, board, relay, electrs_list, tag):
    bar = "=" * 70
    lines = [bar, "  SATCHEL PLAYGROUND IS UP" if args.satchel != "none"
             else "  HEADLESS PLAYGROUND IS UP   (Ctrl+C to stop)", ""]
    if board:
        lines.append(f"  Board: corkboard {board.url}")
    if relay:
        lines.append(f"  Board: local Nostr relay {relay.ws_url} (relays-only)")
    if electrs_list:
        lines.append("  electrs: " + ", ".join(e.url for e in electrs_list)
                     + ("  (+ DEAD :19999 in Alice's fleet)" if args.electrs > 1 else ""))
    if args.satchel == "two-observer":
        lines += [
            "", "  Two Satchel windows, SAME seed, over one local relay:",
            "    MAIN 'Alice'  (pactd :9739) — you drive make/take here.",
            "    OBSERVER      (pactd :9740) — follows Alice read-only.",
            "  SEED BOTH WINDOWS with the SAME phrase (create merchant -> IMPORT):",
            f"    {FOLLOW_MNEMONIC}",
            "  The driver auto-funds Alice once seeded; a bot auto-takes her offers.",
            "  CLOSE THE MAIN (Alice) WINDOW to tear the whole stack down.",
        ]
    elif args.first_run:
        lines += [
            "", "  FIRST-RUN: no coins pre-wired -> step through onboarding + coin",
            "  setup. Playground node details (auth = user/pass, NOT cookie):",
            "    BTCX : 127.0.0.1:19443 (node)  or NODELESS tcp://127.0.0.1:19750",
            "    BTC  : 127.0.0.1:19543   user/pass  pactharness / pactharness  wallet alice_btc",
            "    LTC  : 127.0.0.1:19643   user/pass  pactharness / pactharness  wallet alice_ltc",
            "    (for realistic timing set confirmations BTCX 10 / BTC 6 / LTC 6)",
        ]
    elif args.satchel == "one":
        lines += [
            "", "  In the window: wizard -> create merchant; Corkboard -> take any",
            "  side (incl. LTC); Swaps tab walks to 'completed'.",
            "  CLOSE THE SATCHEL WINDOW to tear the whole stack down.",
        ]
    lines += [f"  Logs: {satchel.LOG_DIR}", bar]
    print("\n".join(lines), flush=True)


def run_viewer(args):
    """Mainnet board viewer — no backdrop, isolated config dir, pactd :9747.
    Ephemeral by default; --persist keeps the dir (your imported prod seed
    survives restarts, only pactd_path is refreshed)."""
    ports = [satchel.VIEWER_PORT, satchel.VITE_PORT]
    print("[viewer] stopping any prior viewer run (PID + port only) ...")
    satchel.teardown(ports=ports, hunt_orphans=False, tag="viewer")
    base_name = "org.pocx.satchel-viewer" if args.persist else "org.pocx.satchel-watchpg"
    view_dir = satchel.config_base(base_name)
    if not args.persist and os.path.exists(view_dir):
        import shutil
        shutil.rmtree(view_dir)
    os.makedirs(view_dir, exist_ok=True)
    if not args.no_build:
        print("[viewer] building pactd + pact-cli (debug) ...")
        import subprocess
        subprocess.run(["cargo", "build", "--manifest-path",
                        os.path.join(binaries.PACT_DIR, "Cargo.toml"),
                        "-p", "pactd", "-p", "pact-cli"], check=True)
    cfg = os.path.join(view_dir, "satchel.json")
    if args.persist and os.path.exists(cfg):
        satchel.refresh_pactd_path(view_dir)
        print("[viewer] reusing existing satchel.json (your setup persists).")
    else:
        # No coins (pure board viewer); OMITTING nostr_relays lets the serde
        # default fill the six RECOMMENDED_NOSTR_RELAYS your prod uses.
        satchel.write_satchel_json(view_dir, coins=[], board_urls=[],
                                   nostr_relays=None, listen_port=satchel.VIEWER_PORT,
                                   tick_secs=30, auto_fund=False,
                                   omit_relays_key=True)
        print(f"[viewer] wrote fresh satchel.json (:{satchel.VIEWER_PORT}, no coins).")
    satchel.stage_sidecars()
    print("[viewer] launching Satchel (mainnet, isolated, :9747) ...")
    sat = satchel.launch_tauri_dev("mainnet", data_dir=view_dir,
                                   log_prefix="viewer-satchel")
    bar = "=" * 70
    print(f"""
{bar}
  SATCHEL VIEWER IS UP  (mainnet, isolated, board viewer)

  Config dir: {view_dir}  ({"PERSISTS" if args.persist else "ephemeral"};
  your prod install is untouched). pactd :9747 | no nodes, no coins |
  six default Nostr relays.

  First run: wizard -> create merchant -> IMPORT your prod mnemonic.
  WARNING: this viewer shares your PROD identity — on close, if the exit
  gate lists your prod offers, choose Keep running / Cancel, NEVER
  "Withdraw & exit" (it would revoke your REAL offers).

  CLOSE THE SATCHEL WINDOW to tear the viewer down (PID/port only).
  Logs: {satchel.LOG_DIR}
{bar}
""", flush=True)
    try:
        sat.wait()
    finally:
        print("[viewer] Satchel closed — tearing down (PID + port only) ...")
        satchel.teardown(ports=ports, hunt_orphans=False, tag="viewer")
        print("[viewer] down.")


def run_regtest(args):
    tag = "play"
    observer = args.satchel == "two-observer"
    nodeless = args.btcx == "nodeless"
    both_electrum = observer or (nodeless and args.board == "nostr")
    with_relay = observer or args.board == "nostr"
    with_ltc = not observer

    if args.relay_cmd:
        os.environ["PACT_NOSTR_RELAY_CMD"] = args.relay_cmd
    if with_relay and not os.environ.get("PACT_NOSTR_RELAY_CMD") \
            and not os.path.exists(binaries.nostr_relay_default()):
        sys.exit(f"[{tag}] no Nostr relay: {binaries.nostr_relay_default()} missing "
                 "— build/copy it, or pass --relay-cmd '<cmd with {port}/{dir}>'")

    print(f"[{tag}] cleaning up any prior run (PID + port only) ...")
    satchel.teardown(tag=tag)
    if not args.no_build:
        stack.build_workspace()

    board = relay = None
    electrs_list = []
    btc_electrs = None
    bots = []
    with Harness(keep=False, with_ltc=with_ltc,
                 pocx_rest=nodeless or observer, btc_rest=both_electrum) as h:
        try:
            # --- services ------------------------------------------------
            if with_relay:
                relay = NostrRelay(h.workdir, port=PLAYGROUND_NOSTR_RELAY_PORT,
                                   name="pact-playground")
                relay.start()
            if args.board == "cork" and not observer:
                board = Corkboard(h.workdir)
                board.start()
            if nodeless or observer:
                want = h.pocx.rpc("getblockcount")
                for i in range(max(1, args.electrs if nodeless else 1)):
                    e = ElectrsServer(
                        h.workdir, h.pocx,
                        electrum_port=ELECTRS_ELECTRUM_PORT + 2 * i,
                        monitoring_port=ELECTRS_MONITORING_PORT + 2 * i,
                        name="electrs" if i == 0 else f"electrs{i + 1}")
                    e.start()
                    e.wait_synced(want)
                    electrs_list.append(e)
            if both_electrum:
                btc_electrs = ElectrsServer(
                    h.workdir, h.btc,
                    electrum_port=BTC_ELECTRS_ELECTRUM_PORT,
                    monitoring_port=BTC_ELECTRS_MONITORING_PORT,
                    network="testnet", binary=binaries.find_btc_electrs(),
                    name="btc-electrs")
                btc_electrs.start()
                btc_electrs.wait_synced(h.btc.rpc("getblockcount"))
                electrs_list.append(btc_electrs)

            # --- extra wallets for the two-sided book ---------------------
            h.pocx.create_wallet("carol_pocx")
            h.btc.create_wallet("carol_btc")
            h.pocx.generate(110, "carol_pocx")
            h.pocx.generate(110, "bob_pocx")
            h.btc.generate(110, "alice_btc")
            if h.ltc:
                for w in ("alice_ltc", "bob_ltc", "carol_ltc"):
                    h.ltc.create_wallet(w)
                h.ltc.generate(110, "alice_ltc")
                h.ltc.generate(110, "carol_ltc")

            # --- bots ------------------------------------------------------
            confs = ({"btcx": 6, "btc": 4} if observer
                     else {"btcx": 10, "btc": 6, "ltc": 6})
            conn = ({"nostr_relays": relay.ws_url} if relay
                    else {"board_url": board.url})
            bob_extra = [("ltc", h.ltc.rpc_url(wallet="bob_ltc"))] if h.ltc else []
            carol_extra = [("ltc", h.ltc.rpc_url(wallet="carol_ltc"))] if h.ltc else []
            bob = Party("bob", h, h.workdir, "bob_pocx", "bob_btc",
                        auto_fund=True, tick_secs=2, coins_file=COINS_TOML,
                        coin_confs=confs, extra_coins=bob_extra, **conn).start()
            carol = Party("carol", h, h.workdir, "carol_pocx", "carol_btc",
                          auto_fund=True, tick_secs=2, coins_file=COINS_TOML,
                          coin_confs=confs, extra_coins=carol_extra, **conn).start()
            bots = [bob, carol]

            # --- the book ---------------------------------------------------
            if observer:
                books = [Book(bob, "buy (Bob)", OBSERVER_BOB_OFFERS),
                         Book(carol, "sell (Carol)", OBSERVER_CAROL_OFFERS)]
            else:
                books = [Book(bob, "buy (Bob)", BOB_OFFERS),
                         Book(carol, "sell (Carol)", CAROL_OFFERS)]
                if h.ltc:
                    books += [Book(bob, "ltc-buy", BOB_LTC_OFFERS, pin_proto="pact-htlc-v1"),
                              Book(carol, "ltc-sell", CAROL_LTC_OFFERS, pin_proto="pact-htlc-v1")]
            market = Market(books, tag=tag)
            market.post_now()

            # --- Satchel ----------------------------------------------------
            faucet = autotaker = None
            main_proc = None
            if args.satchel != "none":
                net = os.path.join(satchel.config_base(), "regtest")
                satchel.wipe_pactd_state(net)
                tick = 2 if (args.board == "nostr" and not observer) else 5
                satchel.write_satchel_json(
                    net, compose_alice_coins(args),
                    board_urls=[board.url] if board else [],
                    nostr_relays=[relay.ws_url] if relay else [],
                    listen_port=satchel.MANAGED_PORT, tick_secs=tick,
                    ui_extra={"onboarded": True} if observer else None)
                satchel.copy_coin_templates(net)
                satchel.stage_sidecars()
                if observer:
                    obs_base = satchel.config_base("org.pocx.satchel-observer")
                    obs_net = os.path.join(obs_base, "regtest")
                    satchel.wipe_pactd_state(obs_net)
                    satchel.write_satchel_json(
                        obs_net, compose_alice_coins(args), board_urls=[],
                        nostr_relays=[relay.ws_url],
                        listen_port=satchel.OBSERVER_PORT, tick_secs=5,
                        ui_extra={"onboarded": True})
                    exe = (satchel.build_satchel_exe() if not args.no_build else
                           os.path.join(binaries.REPO_DIR, "satchel", "target",
                                        "debug", "satchel" + binaries.EXE))
                    # Staggered: each window spawns ITS OWN managed pactd —
                    # wait for each backend before the next so they never race.
                    main_proc = satchel.launch_built(
                        exe, "regtest", webview_dir=os.path.join(net, "webview2"),
                        log_prefix="satchel-alice")
                    if not satchel.wait_health(satchel.MANAGED_PORT):
                        raise RuntimeError("Alice's pactd never came up on :9739 "
                                           f"(see {satchel.LOG_DIR}/satchel-alice.err.log)")
                    obs_proc = satchel.launch_built(
                        exe, "regtest", data_dir=obs_base,
                        webview_dir=os.path.join(obs_base, "webview2"),
                        log_prefix="satchel-observer")
                    if not satchel.wait_health(satchel.OBSERVER_PORT):
                        raise RuntimeError("the observer's pactd never came up on :9740 "
                                           f"(see {satchel.LOG_DIR}/satchel-observer.err.log)")
                else:
                    main_proc = satchel.launch_tauri_dev("regtest")

            # faucet / auto-take per mode
            if observer:
                faucet = Faucet([("btcx", h.pocx, "carol_pocx", 200.0),
                                 ("btc", h.btc, "bob_btc", 0.05)], tag=tag)
                autotaker = AutoTaker(carol, tag=tag)
            elif nodeless:
                targets = [("btcx", h.pocx, "alice_pocx", 100.0)]
                if args.board == "nostr":
                    targets.append(("btc", h.btc, "bob_btc", 0.05))
                faucet = Faucet(targets, tag=tag)

            print_banner(args, board, relay, electrs_list, tag)

            # --- the drive loop --------------------------------------------
            if observer:
                legs = [(h.pocx, "carol_pocx", "btcx"), (h.btc, "bob_btc", "btc")]
                miner = PacedMiner(legs, {"btcx": 8, "btc": 12}, 4, tag=tag)
            else:
                legs = [(h.pocx, "alice_pocx", "btcx"), (h.btc, "bob_btc", "btc")]
                block_secs = {"btcx": 6, "btc": 12}
                if h.ltc:
                    legs.append((h.ltc, "alice_ltc", "ltc"))
                    block_secs["ltc"] = 12
                miner = PacedMiner(legs, block_secs, 6, tag=tag)

            seeded = set()
            try:
                while True:
                    time.sleep(miner.base_secs)
                    miner.tick()
                    if observer:
                        for label, port in (("Alice (MAIN :9739)", satchel.MANAGED_PORT),
                                            ("observer (:9740)", satchel.OBSERVER_PORT)):
                            if port not in seeded:
                                base = ("org.pocx.satchel-observer"
                                        if port == satchel.OBSERVER_PORT
                                        else "org.pocx.satchel")
                                info = managed_rpc(port, "getinfo", base=base)
                                if info and info.get("identity"):
                                    seeded.add(port)
                                    print(f"[{tag}] {label} seeded by the wizard")
                    if faucet and (not observer or satchel.MANAGED_PORT in seeded):
                        faucet.run_once()
                    if autotaker:
                        autotaker.poll()
                    market.maintain()
                    if main_proc is not None and main_proc.poll() is not None:
                        if args.keep:
                            print(f"[{tag}] Satchel closed — --keep set, backdrop "
                                  "stays up (Ctrl+C to stop)")
                            main_proc = None
                            continue
                        print(f"\n[{tag}] Satchel closed — shutting down ...")
                        break
            except KeyboardInterrupt:
                print(f"\n[{tag}] shutting down ...")
        finally:
            for p in bots:
                try:
                    p.stop()
                except Exception:  # noqa: BLE001
                    pass
            # btc_electrs is already in electrs_list.
            for svc in [relay, board] + electrs_list:
                if svc is not None:
                    try:
                        svc.stop()
                    except Exception:  # noqa: BLE001
                        pass
    # Outside the Harness (nodes already stopped): sweep the remnants —
    # Satchel/cargo/vite trees and anything a crash left on a registry port.
    satchel.teardown(tag=tag)
    print(f"[{tag}] down.")


def main():
    args = parse_args()
    if args.down:
        print("[play] tearing down (PID + port only, full registry) ...")
        satchel.teardown(tag="play")
        print("[play] down.")
        return
    if args.satchel == "viewer":
        run_viewer(args)
        return
    run_regtest(args)


if __name__ == "__main__":
    main()
