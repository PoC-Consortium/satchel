#!/usr/bin/env python3
"""End-to-end Phase 1+ tests: PoCX<->BTC swaps on regtest.

Scenarios:
  1. complete swap, manual handshake, two `pact` CLIs (the Phase 1 DoD)
  2. refund path + negative safety checks (premature/late actions rejected)
  3. complete swap with Alice driven via pactd's REST API and both sides
     auto-redeeming through the scheduler (one engine, many faces)
  4. unattended refund: both sides reclaim via `pactd --once` only

Run:  python test_swap_e2e.py
Env:  POCX_BITCOIND / BTC_BITCOIND     (node binaries, see regtest_harness.py)
"""

import json
import os
import shlex
import shutil
import socket
import subprocess
import sys
import time
import urllib.request

from regtest_harness import Harness, HERE, EXE

PACT_DIR = os.path.normpath(os.path.join(HERE, ".."))
PACT_BIN = os.path.join(PACT_DIR, "target", "debug", "pact-cli" + EXE)
PACTD_BIN = os.path.join(PACT_DIR, "target", "debug", "pactd" + EXE)
CORKBOARD_DIR = os.path.normpath(os.path.join(HERE, "..", "..", "corkboard"))
CORKBOARD_BIN = os.path.join(CORKBOARD_DIR, "target", "debug", "corkboard" + EXE)
CORKBOARD_PORT = 19790
NOSTR_RELAY_PORT = 19791
# The shipped coin-templates file (consensus params for file-added coins like
# ltc). A Party that trades a file coin passes this so its pactd registry knows
# the coin's genesis + HRP.
COINS_TOML = os.path.normpath(os.path.join(HERE, "..", "..", "satchel", "coins.toml"))

GIVE_POCX = "50.0"      # Alice gives 50 POCX
GET_BTC = "0.001"       # ... for 0.001 BTC from Bob
FEE_SLACK = 0.01        # generous bound for redeem/refund fees
PACTD_PORT = 19737


def build_workspace():
    print("[e2e] building pact workspace ...")
    subprocess.run(["cargo", "build"], cwd=PACT_DIR, check=True)
    print("[e2e] building corkboard ...")
    subprocess.run(["cargo", "build"], cwd=CORKBOARD_DIR, check=True)
    assert os.path.exists(PACT_BIN), f"missing {PACT_BIN}"
    assert os.path.exists(PACTD_BIN), f"missing {PACTD_BIN}"
    assert os.path.exists(CORKBOARD_BIN), f"missing {CORKBOARD_BIN}"


class Corkboard:
    """The noticeboard server."""

    def __init__(self, workdir, port=CORKBOARD_PORT):
        self.port = port
        self.db = os.path.join(workdir, "corkboard.sqlite")
        self.url = f"http://127.0.0.1:{port}"
        self.proc = None

    def start(self):
        self.proc = subprocess.Popen(
            [CORKBOARD_BIN, "--listen", f"127.0.0.1:{self.port}", "--db", self.db],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        deadline = time.time() + 30
        while time.time() < deadline:
            if self.proc.poll() is not None:
                raise RuntimeError(f"corkboard exited early: {self.proc.returncode}")
            try:
                urllib.request.urlopen(f"{self.url}/health", timeout=5)
                print(f"[e2e] corkboard up on :{self.port}")
                return
            except Exception:
                time.sleep(0.2)
        raise TimeoutError("corkboard did not come up")

    def stop(self):
        if self.proc:
            self.proc.terminate()
            try:
                self.proc.wait(timeout=15)
            except subprocess.TimeoutExpired:
                self.proc.kill()
            self.proc = None

    def reset(self):
        """Bring the board back up on the same URL/port but backed by a FRESH,
        empty DB — equivalent to an operator wipe / redeploy, while clients keep
        their (now ahead-of-fresh-board) relay cursors. We switch to a new file
        rather than unlink the old one, which Windows may still hold briefly."""
        self.stop()
        self._gen = getattr(self, "_gen", 0) + 1
        self.db = os.path.join(os.path.dirname(self.db), f"corkboard-reset-{self._gen}.sqlite")
        self.start()


class NostrRelay:
    """A local Nostr relay (bundled nostr-rs-relay) for the relays-only swap
    test. Ephemeral: config + db live under the temp workspace. Override the
    binary with PACT_NOSTR_RELAY_BIN or the whole command with
    PACT_NOSTR_RELAY_CMD ({port}/{dir} substituted)."""

    def __init__(self, workdir, port=NOSTR_RELAY_PORT):
        self.port = port
        self.host = "127.0.0.1"
        self.ws_url = f"ws://{self.host}:{port}"
        self.dir = os.path.join(workdir, "nostr-relay")
        os.makedirs(self.dir, exist_ok=True)
        self.proc = None

    def _build_cmd(self):
        tmpl = os.environ.get("PACT_NOSTR_RELAY_CMD")
        if tmpl:
            return shlex.split(
                tmpl.replace("{port}", str(self.port)).replace("{dir}", self.dir))
        relay_bin = os.environ.get("PACT_NOSTR_RELAY_BIN") or \
            os.path.join(HERE, "bin", "nostr-rs-relay" + EXE)
        if not os.path.exists(relay_bin):
            raise RuntimeError(
                f"nostr-rs-relay not found at {relay_bin}. Set PACT_NOSTR_RELAY_BIN "
                "or PACT_NOSTR_RELAY_CMD.")
        cfg = os.path.join(self.dir, "config.toml")
        db = self.dir.replace(os.sep, "/")
        with open(cfg, "w", encoding="utf-8") as fh:
            fh.write(
                f'[info]\nrelay_url = "{self.ws_url}/"\nname = "pact-e2e"\n\n'
                f'[network]\naddress = "{self.host}"\nport = {self.port}\n\n'
                f'[database]\ndata_directory = "{db}"\n')
        return [relay_bin, "--config", cfg, "--db", self.dir]

    def start(self):
        # The port is fixed: a relay leaked by an earlier (crashed) run would
        # keep listening, this start would "succeed" against it, and its STALE
        # event DB would poison the scenario (same test npubs). Fail loudly.
        with socket.socket() as probe:
            if probe.connect_ex((self.host, self.port)) == 0:
                raise RuntimeError(
                    f"port {self.port} already in use — leaked relay from a "
                    "previous run? Kill it (by port, never by name) first.")
        cmd = self._build_cmd()
        self.proc = subprocess.Popen(
            cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        deadline = time.time() + 30
        while time.time() < deadline:
            if self.proc.poll() is not None:
                raise RuntimeError(f"nostr relay exited early: {self.proc.returncode}")
            try:
                with socket.create_connection((self.host, self.port), timeout=2):
                    print(f"[e2e] nostr relay up on :{self.port} ({self.ws_url})")
                    return self
            except OSError:
                time.sleep(0.2)
        raise TimeoutError("nostr relay did not come up")

    def stop(self):
        if self.proc:
            self.proc.terminate()
            try:
                self.proc.wait(timeout=15)
            except subprocess.TimeoutExpired:
                self.proc.kill()
            self.proc = None


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


class Party:
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
        cmd = [PACTD_BIN, "--data-dir", self.data_dir, "--network", "regtest",
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
        body = {"jsonrpc": "2.0", "id": "h", "method": method, "params": list(params)}
        req = urllib.request.Request(
            f"http://127.0.0.1:{self.port}/", data=json.dumps(body).encode(), method="POST")
        req.add_header("Content-Type", "application/json")
        if auth:
            import base64
            req.add_header("Authorization", f"Basic {base64.b64encode(self.cookie.encode()).decode()}")
        try:
            with urllib.request.urlopen(req, timeout=120) as resp:
                data = json.loads(resp.read())
        except urllib.error.HTTPError as e:
            raise RuntimeError(f"pactd {method}: HTTP {e.code}: {e.read().decode()}") from e
        if data.get("error"):
            raise RuntimeError(f"pactd {method}: {data['error']['message']}")
        return data["result"]

    def cli(self, *args):
        cmd = [PACT_BIN, "--rpc", f"http://127.0.0.1:{self.port}",
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


def swap_id_from(message_file):
    with open(message_file, encoding="utf-8") as fh:
        return json.load(fh)["swap_id"]


def load_msg(message_file):
    with open(message_file, encoding="utf-8") as fh:
        return json.load(fh)


def save_msg(message_file, envelope):
    with open(message_file, "w", encoding="utf-8") as fh:
        json.dump(envelope, fh, indent=2)


def outpoint_from(funded_message_file):
    body = load_msg(funded_message_file)["body"]
    return body["txid"], body["vout"]


def assert_htlc_spent(node, funded_message_file, what):
    txid, vout = outpoint_from(funded_message_file)
    utxo = node.rpc("gettxout", txid, vout, True)
    assert utxo is None, f"{what} HTLC {txid}:{vout} is still unspent"


def expect_fail(party, what, *args):
    try:
        party.cli(*args)
    except RuntimeError as exc:
        print(f"[e2e] correctly rejected: {what}")
        return str(exc)
    raise AssertionError(f"{what} should have been rejected but succeeded")


def balances(h):
    return {
        "alice_pocx": float(h.pocx.rpc("getbalance", wallet="alice_pocx")),
        "bob_pocx": float(h.pocx.rpc("getbalance", wallet="bob_pocx")),
        "alice_btc": float(h.btc.rpc("getbalance", wallet="alice_btc")),
        "bob_btc": float(h.btc.rpc("getbalance", wallet="bob_btc")),
    }


def msg(workdir, name):
    return os.path.join(workdir, name)


def regtest_timelocks(h):
    now = int(h.pocx.rpc("getblockchaininfo")["time"])
    return now + 2 * 3600, now + 4 * 3600   # (t2, t1)


def drive_until(party, cond, tries=20):
    """Tick `party` repeatedly until cond(events) holds; returns those events.
    Mining is the caller's job — this only drives the scheduler."""
    for _ in range(tries):
        events = party.tick()
        if cond(events):
            return events
    raise AssertionError(f"{party.name}: condition not met after {tries} ticks")


def handshake_and_fund(h, alice, bob, prefix):
    """Shared plumbing: offer/accept handshake, fund both legs. Returns
    (swap_id, funded_a_file, funded_b_file)."""
    t2, t1 = regtest_timelocks(h)
    m_init = msg(h.workdir, f"{prefix}_init.json")
    m_accept = msg(h.workdir, f"{prefix}_accept.json")
    m_funded_a = msg(h.workdir, f"{prefix}_funded_a.json")
    m_funded_b = msg(h.workdir, f"{prefix}_funded_b.json")

    alice.cli("offer", "--give", f"btcx:{GIVE_POCX}", "--get", f"btc:{GET_BTC}",
              "--t1", str(t1), "--t2", str(t2), "--out", m_init)
    sid = swap_id_from(m_init)
    bob.cli("accept", "--in", m_init, "--out", m_accept)
    alice.cli("recv", "--in", m_accept)
    alice.cli("fund", "--swap", sid, "--out", m_funded_a)
    h.pocx.generate(1, "alice_pocx")
    bob.cli("recv", "--in", m_funded_a)
    bob.cli("fund", "--swap", sid, "--out", m_funded_b)
    h.btc.generate(1, "bob_btc")
    return sid, m_funded_a, m_funded_b


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


def _rescue_scenario(h, protocol, tag, mnemonic, victim="taker",
                     stage="committed", refund=False):
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
    """v1: wipe the taker at ACCEPT, before anything of its is on chain — the
    earliest snapshot; the rescued (anchored-key) taker funds and completes."""
    _rescue_scenario(h, "pact-htlc-v1", "rct1", RESCUE_MNEMONIC_T1,
                     victim="taker", stage="accepted")


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


def test_rescue_v1_maker_refund(h):
    """v1 refund variant: the taker never funds leg B; the maker is wiped at
    funded_a and, once past the timelocks, the RESCUED maker must time out and
    reclaim leg A (C8 abort re-driven from a rescued record)."""
    _rescue_scenario(h, "pact-htlc-v1", "rcr1", RESCUE_MNEMONIC_R1,
                     victim="maker", stage="funded_a", refund=True)


def main():
    build_workspace()
    failures = 0
    tests = (test_complete_swap, test_refund,
             test_daemon_autopilot_swap, test_daemon_autopilot_refund,
             test_chain_watched_funding, test_funding_fee_bump_v1,
             test_balance_validation,
             test_create_import_then_swap, test_coin_setup, test_corkboard_swap,
             test_board_reset_recovery, test_nostr_relay_swap,
             test_concurrent_drain_no_double_send, test_private_offer_swap,
             test_swap_rescue_v1, test_swap_rescue_v2,
             test_rescue_v1_maker_funded_a, test_rescue_v1_taker_accepted,
             test_rescue_v1_taker_post_reveal, test_rescue_v2_maker_committed,
             test_rescue_v1_maker_refund)
    with Harness(keep=True) as h:
        for test in tests:
            try:
                test(h)
            except Exception as exc:  # noqa: BLE001 — report and continue
                failures += 1
                print(f"[e2e] FAIL {test.__name__}: {exc}", file=sys.stderr)
    if failures:
        print(f"\n[e2e] RED: {failures}/{len(tests)} scenario(s) failing.", file=sys.stderr)
        sys.exit(1)
    print(f"\n[e2e] GREEN: all {len(tests)} scenarios pass.")


if __name__ == "__main__":
    main()
