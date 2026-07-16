"""Pactd — one pact daemon under test (Core's TestNode analog), plus the two
ways to drive it: rpc() straight to the daemon, cli() through pact-cli.
Extracted verbatim from test_swap_e2e.py (Phase 1); `Party` stays as an alias
until the Phase 2 split renames call sites.
"""

import os
import socket
import subprocess
import time
import urllib.request

from framework import binaries
from framework.util import pactd_rpc

# Base of the party port-allocation range. Appendix B of the plan caps the
# range at 19749 once Phase 2's per-scenario allocator reset lands; the
# shared-Harness suites still burn one port per Party for a whole run, so the
# allocator stays unbounded (it skips live listeners) until then.
PACTD_PORT = 19737

_next_port = [PACTD_PORT]


def _alloc_port():
    # Skip ports something is already listening on — a pactd leaked by an
    # earlier crashed run would otherwise steal the port and the fresh pactd
    # dies at bind (no .cookie ever appears).
    while True:
        p = _next_port[0]
        _next_port[0] += 1
        with socket.socket() as probe:
            if probe.connect_ex(("127.0.0.1", p)) != 0:
                return p
        print(f"[e2e] port {p} already in use (leaked process?) — skipping")


class Pactd:
    """A trading party = one pactd (the engine, JSON-RPC) plus two ways to
    drive it: rpc() calls the daemon directly; cli() shells out to pact-cli
    (the bitcoin-cli-style client) against the same daemon. Mirrors the
    Bitcoin Core split: pactd / pact-cli."""

    def __init__(self, name, harness, workdir, pocx_wallet, btc_wallet,
                 duplicate_backends=False, board_url=None, auto_fund=False,
                 tick_secs=0, auto_init=True, coin_confs=None, nostr_relays=None,
                 extra_coins=None, coins_file=None,
                 pocx_url=None, btc_url=None, extra_env=None):
        self.name = name
        self.auto_init = auto_init
        # Extra process env for this party's pactd (e.g. a PACT_TEST_* hook).
        # Merged over the inherited env at start(), so it never leaks to the
        # shared harness process or the other parties.
        self.extra_env = extra_env or {}
        # Additional coins beyond the built-in btcx/btc legs, as a list of
        # (coin_id, rpc_url) — e.g. [("ltc", node.rpc_url(wallet="bob_ltc"))].
        # Requires coins_file so pactd's registry knows the file coin.
        self.extra_coins = extra_coins or []
        self.coins_file = coins_file
        # Optional per-coin confirmation-depth overrides: {"btc": 2, ...} →
        # `--coin-confs btc=2` (reorg-safety/finality gate).
        self.coin_confs = coin_confs or {}
        self.data_dir = os.path.join(workdir, f"pact-{name}")
        # Explicit URL overrides win (the nodeless parity suite hands a
        # tcp:// Electrum URL for a coin instead of the node's Core RPC).
        self.pocx_url = pocx_url or harness.pocx.rpc_url(wallet=pocx_wallet)
        self.btc_url = btc_url or harness.btc.rpc_url(wallet=btc_wallet)
        if duplicate_backends:
            # Exercises the spec §10 multi-backend path (agreement checks,
            # conservative clocks). Same node twice — plumbing test only.
            self.pocx_url = f"{self.pocx_url},{self.pocx_url}"
            self.btc_url = f"{self.btc_url},{self.btc_url}"
        self.board_url = board_url
        # Comma-separated wss/ws relay URLs → pactd `--nostr-relay`. When set
        # (and board_url is None), this party trades over Nostr only.
        self.nostr_relays = nostr_relays
        self.auto_fund = auto_fund
        self.tick_secs = tick_secs   # 0 = tick only on demand (tests)
        self.port = _alloc_port()
        self.proc = None
        self.cookie = None

    def start(self):
        # Every coin is attached the same generic way (`--coin id=url`); there
        # are no per-coin aliases. btcx/btc are the built-in legs; extra_coins
        # carries any file-added coin (ltc), which also needs --coins-file.
        cmd = [binaries.pactd(), "--data-dir", self.data_dir, "--network", "regtest",
               "--coin", f"btcx={self.pocx_url}", "--coin", f"btc={self.btc_url}",
               "--listen", f"127.0.0.1:{self.port}",
               "--tick-secs", str(self.tick_secs)]
        if self.coins_file:
            cmd += ["--coins-file", self.coins_file]
        for coin_id, url in self.extra_coins:
            cmd += ["--coin", f"{coin_id}={url}"]
        for coin_id, n in self.coin_confs.items():
            cmd += ["--coin-confs", f"{coin_id}={n}"]
        if self.auto_init:
            cmd += ["--auto-init"]
        if self.board_url:
            cmd += ["--board-url", self.board_url]
        if self.nostr_relays:
            cmd += ["--nostr-relay", self.nostr_relays]
        if self.auto_fund:
            cmd += ["--auto-fund"]
        # pactd stderr -> a per-party log file (was DEVNULL). Cheap, and it makes
        # a misbehaving daemon debuggable without re-plumbing the harness.
        os.makedirs(self.data_dir, exist_ok=True)
        self._logf = open(os.path.join(self.data_dir, "pactd.log"), "w", encoding="utf-8")
        env = dict(os.environ)
        env.setdefault("RUST_LOG", "pactd=debug,libswap=debug")
        env.update(self.extra_env)
        # pactd's tracing goes to STDOUT; merge stderr into the same log file.
        self.proc = subprocess.Popen(cmd, stdout=self._logf, stderr=subprocess.STDOUT, env=env)
        deadline = time.time() + 30
        while time.time() < deadline:
            if self.proc.poll() is not None:
                raise RuntimeError(f"pactd ({self.name}) exited early: {self.proc.returncode}")
            try:
                urllib.request.urlopen(f"http://127.0.0.1:{self.port}/health", timeout=5)
                break
            except Exception:
                time.sleep(0.2)
        else:
            raise TimeoutError("pactd did not come up")
        with open(os.path.join(self.data_dir, ".cookie"), encoding="utf-8") as fh:
            self.cookie = fh.read().strip()
        print(f"[e2e] pactd up for {self.name} on :{self.port}")
        return self

    def rpc(self, method, *params, auth=True):
        return pactd_rpc(f"http://127.0.0.1:{self.port}/", method, *params,
                         cookie=self.cookie if auth else None)

    def cli(self, *args):
        cmd = [binaries.pact_cli(), "--rpc", f"http://127.0.0.1:{self.port}",
               "--data-dir", self.data_dir] + list(args)
        print(f"[e2e] {self.name}: pact-cli {' '.join(args)}")
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=300)
        if result.returncode != 0:
            raise RuntimeError(
                f"pact-cli ({self.name}) failed: {' '.join(args)}\n"
                f"stdout: {result.stdout.strip()}\nstderr: {result.stderr.strip()}")
        return result.stdout

    def setup_seed(self, mnemonic=None, passphrase=None):
        """Drive the Phase-B seed lifecycle over JSON-RPC (the path the
        Satchel first-run wizard uses). Create a fresh seed, or import the
        given mnemonic. Returns the resulting/normalized mnemonic."""
        if mnemonic is None:
            return self.rpc("createseed", passphrase)["mnemonic"]
        return self.rpc("importseed", mnemonic, passphrase)["mnemonic"]

    def tick(self):
        """One scheduler + board pass; prints and returns the events."""
        events = self.rpc("tick")["events"]
        for ev in events:
            print(f"[e2e]   scheduler[{self.name}]: {ev['action']} {ev['detail']}")
        return events

    def stop(self):
        if self.proc:
            self.proc.terminate()
            try:
                self.proc.wait(timeout=15)
            except subprocess.TimeoutExpired:
                self.proc.kill()
            self.proc = None
        # Release the log file handle so the data dir can be removed (Windows
        # holds a lock on any open file — a rescue test wipes the dir after stop).
        logf = getattr(self, "_logf", None)
        if logf is not None:
            try:
                logf.close()
            except Exception:  # noqa: BLE001
                pass
            self._logf = None


# Legacy name, used by every pre-split suite/playground.
Party = Pactd
