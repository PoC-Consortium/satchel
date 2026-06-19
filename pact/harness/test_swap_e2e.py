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


_next_port = [PACTD_PORT]


def _alloc_port():
    p = _next_port[0]
    _next_port[0] += 1
    return p


class Party:
    """A trading party = one pactd (the engine, JSON-RPC) plus two ways to
    drive it: rpc() calls the daemon directly; cli() shells out to pact-cli
    (the bitcoin-cli-style client) against the same daemon. Mirrors the
    Bitcoin Core split: pactd / pact-cli."""

    def __init__(self, name, harness, workdir, pocx_wallet, btc_wallet,
                 duplicate_backends=False, board_url=None, auto_fund=False,
                 tick_secs=0, auto_init=True, coin_confs=None, nostr_relays=None,
                 extra_coins=None, coins_file=None):
        self.name = name
        self.auto_init = auto_init
        # Additional coins beyond the built-in btcx/btc legs, as a list of
        # (coin_id, rpc_url) — e.g. [("ltc", node.rpc_url(wallet="bob_ltc"))].
        # Requires coins_file so pactd's registry knows the file coin.
        self.extra_coins = extra_coins or []
        self.coins_file = coins_file
        # Optional per-coin confirmation-depth overrides: {"btc": 2, ...} →
        # `--coin-confs btc=2` (reorg-safety/finality gate).
        self.coin_confs = coin_confs or {}
        self.data_dir = os.path.join(workdir, f"pact-{name}")
        self.pocx_url = harness.pocx.rpc_url(wallet=pocx_wallet)
        self.btc_url = harness.btc.rpc_url(wallet=btc_wallet)
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
        assert st_a == {"seed_exists": True, "encrypted": False, "locked": False}, st_a
        st_b = bob.rpc("walletstatus")
        assert st_b == {"seed_exists": True, "encrypted": True, "locked": False}, st_b
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


def main():
    build_workspace()
    failures = 0
    tests = (test_complete_swap, test_refund,
             test_daemon_autopilot_swap, test_daemon_autopilot_refund,
             test_chain_watched_funding,
             test_create_import_then_swap, test_coin_setup, test_corkboard_swap,
             test_private_offer_swap)
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
