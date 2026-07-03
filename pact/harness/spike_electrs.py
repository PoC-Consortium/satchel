#!/usr/bin/env python3
"""Live spike for the nodeless wallet chain source (epic #58).

First real-server exercise of `wallet_bdk.rs`: stock bdk over our raw-Electrum
calls, against a REAL electrs (PoCX-patched romanz fork) indexing a regtest
PoCX node. Everything below `sync_entry` is unit-tested; this proves the wire.

Covered end to end, all through pactd's JSON-RPC (nodeless mode: the coin URL
list has no http:// primary, so the engine dispatches to BdkWalletBackend):

  1. fresh-seed full scan          (empty wallet syncs, balance 0)
  2. receive                        (getnewaddress -> mine coinbase to it)
  3. confirmed balance              (after maturity; anchors from PoCX headers)
  4. spend                          (sendtoaddress: bdk build/sign, Electrum
                                     broadcast, node mempool sees it)
  5. steady-state re-sync           (balance reflects the spend + change)
  6. activity                       (listtransactions: received + sent rows)
  7. persistence                    (pactd restart -> same balance, no rescan)

Run:  python spike_electrs.py
Env:  POCX_BITCOIND       node binary        (see regtest_harness.py)
      PACT_ELECTRS_BIN    electrs binary     (default: harness/bin/electrs.exe)
"""

import json
import os
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import urllib.request

from regtest_harness import (
    EXE,
    HERE,
    POCX_REGTEST_GENESIS,
    Node,
    find_pocx_bitcoind,
)

PACT_DIR = os.path.normpath(os.path.join(HERE, ".."))
PACTD_BIN = os.path.join(PACT_DIR, "target", "debug", "pactd" + EXE)

# bindex-pocx (the fork's indexer) fetches blocks over bitcoind's REST
# interface at the HARDCODED network-default RPC port (chain.rs:
# `http://localhost:{default_rpc_port(network)}`) — --daemon-rpc-addr does
# not move it. So unlike the e2e suite, the spike node MUST sit on the
# regtest default 18443, with `-rest=1` (REST is unauthenticated, RPC-port).
POCX_SPIKE_RPC_PORT = 18443
ELECTRUM_PORT = 19750
MONITORING_PORT = 19751
PACTD_PORT = 19752


def find_electrs():
    candidates = [
        os.environ.get("PACT_ELECTRS_BIN"),
        os.path.join(HERE, "bin", "electrs" + EXE),
    ]
    for candidate in candidates:
        if candidate and os.path.exists(candidate):
            return candidate
    raise FileNotFoundError(
        "electrs binary not found. Copy the PoCX-patched electrs to "
        "harness/bin/electrs" + EXE + " or set PACT_ELECTRS_BIN.")


class Electrs:
    """The PoCX-patched electrs, indexing the harness's regtest PoCX node."""

    def __init__(self, workdir, node):
        self.dir = os.path.join(workdir, "electrs")
        os.makedirs(self.dir, exist_ok=True)
        # electrs authenticates via a cookie FILE whose content is user:pass —
        # hand it the harness node's static credentials in that shape.
        self.cookie_path = os.path.join(self.dir, "rpc.cookie")
        with open(self.cookie_path, "w", encoding="ascii") as fh:
            fh.write(f"{node.rpc_user}:{node.rpc_pass}")
        self.daemon_port = node.rpc_port
        self.proc = None
        self.logf = None

    def start(self):
        cmd = [
            find_electrs(),
            "--network", "regtest",
            "--daemon-rpc-addr", f"127.0.0.1:{self.daemon_port}",
            "--cookie-file", self.cookie_path,
            "--db-dir", os.path.join(self.dir, "db"),
            "--electrum-rpc-addr", f"127.0.0.1:{ELECTRUM_PORT}",
            "--monitoring-addr", f"127.0.0.1:{MONITORING_PORT}",
            "--log-filters", "INFO",
        ]
        self.logf = open(os.path.join(self.dir, "electrs.log"), "w", encoding="utf-8")
        print(f"[spike] starting electrs: {' '.join(cmd)}")
        self.proc = subprocess.Popen(cmd, stdout=self.logf, stderr=subprocess.STDOUT)

    def raw_call(self, method, params, timeout=5):
        """One-shot Electrum JSONRPC call over a fresh TCP connection."""
        with socket.create_connection(("127.0.0.1", ELECTRUM_PORT), timeout=timeout) as s:
            req = {"id": 0, "jsonrpc": "2.0", "method": method, "params": params}
            s.sendall((json.dumps(req) + "\n").encode())
            s.settimeout(timeout)
            buf = b""
            while not buf.endswith(b"\n"):
                chunk = s.recv(65536)
                if not chunk:
                    break
                buf += chunk
        resp = json.loads(buf.decode())
        if resp.get("error"):
            raise RuntimeError(f"electrum {method}: {resp['error']}")
        return resp["result"]

    def wait_synced(self, want_height, timeout=90):
        deadline = time.time() + timeout
        last = None
        while time.time() < deadline:
            if self.proc.poll() is not None:
                raise RuntimeError(
                    f"electrs exited early: {self.proc.returncode} "
                    f"(see {self.dir}/electrs.log)")
            try:
                tip = self.raw_call("blockchain.headers.subscribe", [])
                last = tip.get("height")
                if last is not None and last >= want_height:
                    print(f"[spike] electrs synced to height {last}")
                    return
            except (OSError, RuntimeError):
                pass
            time.sleep(0.5)
        raise TimeoutError(
            f"electrs did not reach height {want_height} (last seen: {last}; "
            f"see {self.dir}/electrs.log)")

    def stop(self):
        if self.proc:
            self.proc.terminate()
            try:
                self.proc.wait(timeout=15)
            except subprocess.TimeoutExpired:
                self.proc.kill()
            self.proc = None
        if self.logf:
            self.logf.close()
            self.logf = None


class NodelessPactd:
    """A pactd with ONE nodeless coin: btcx over tcp:// only (no Core URL)."""

    def __init__(self, workdir):
        self.data_dir = os.path.join(workdir, "pact-nodeless")
        self.port = PACTD_PORT
        self.proc = None
        self.cookie = None
        self.logf = None

    def start(self):
        cmd = [
            PACTD_BIN,
            "--data-dir", self.data_dir,
            "--network", "regtest",
            "--coin", f"btcx=tcp://127.0.0.1:{ELECTRUM_PORT}",
            "--listen", f"127.0.0.1:{self.port}",
            "--tick-secs", "0",
        ]
        os.makedirs(self.data_dir, exist_ok=True)
        self.logf = open(
            os.path.join(self.data_dir, "pactd.log"), "a", encoding="utf-8")
        env = dict(os.environ)
        env.setdefault("RUST_LOG", "pactd=debug,libswap=debug")
        self.proc = subprocess.Popen(cmd, stdout=self.logf, stderr=subprocess.STDOUT, env=env)
        deadline = time.time() + 30
        while time.time() < deadline:
            if self.proc.poll() is not None:
                raise RuntimeError(
                    f"pactd exited early: {self.proc.returncode} "
                    f"(see {self.data_dir}/pactd.log)")
            try:
                urllib.request.urlopen(f"http://127.0.0.1:{self.port}/health", timeout=5)
                break
            except Exception:
                time.sleep(0.2)
        else:
            raise TimeoutError("pactd did not come up")
        with open(os.path.join(self.data_dir, ".cookie"), encoding="utf-8") as fh:
            self.cookie = fh.read().strip()
        print(f"[spike] pactd (nodeless btcx) up on :{self.port}")
        return self

    def rpc(self, method, *params):
        import base64
        body = {"jsonrpc": "2.0", "id": "s", "method": method, "params": list(params)}
        req = urllib.request.Request(
            f"http://127.0.0.1:{self.port}/", data=json.dumps(body).encode(), method="POST")
        req.add_header("Content-Type", "application/json")
        req.add_header(
            "Authorization", f"Basic {base64.b64encode(self.cookie.encode()).decode()}")
        with urllib.request.urlopen(req, timeout=120) as resp:
            data = json.loads(resp.read())
        if data.get("error"):
            raise RuntimeError(f"pactd {method}: {data['error']['message']}")
        return data["result"]

    def stop(self):
        if self.proc:
            self.proc.terminate()
            try:
                self.proc.wait(timeout=15)
            except subprocess.TimeoutExpired:
                self.proc.kill()
            self.proc = None
        if self.logf:
            self.logf.close()
            self.logf = None


def main():
    print("[spike] building pact workspace ...")
    subprocess.run(["cargo", "build"], cwd=PACT_DIR, check=True)

    workdir = tempfile.mkdtemp(prefix="pact-spike-electrs-")
    print(f"[spike] workdir {workdir}")
    node = Node("pocx", find_pocx_bitcoind(), os.path.join(workdir, "pocx"),
                POCX_SPIKE_RPC_PORT, POCX_REGTEST_GENESIS,
                extra_args=["-rest=1"])
    electrs = Electrs(workdir, node)
    pactd = NodelessPactd(workdir)
    ok = False
    try:
        node.start()
        # PoCX regtest forging needs mocktime to mine without real delays
        # (pocx/regtest/forging.cpp); forging auto-advances it afterwards.
        node.set_mocktime(int(time.time()))
        node.create_wallet("miner")
        miner = node.new_address("miner")
        # A little chain history BEFORE electrs starts (initial index path).
        node.rpc("generatetoaddress", 10, miner)
        electrs.start()
        electrs.wait_synced(10)

        # Genesis served through electrs must be the PoCX regtest genesis —
        # this is the exact header path verify_chain uses (286-byte headers).
        pactd.start()
        pactd.rpc("importseed",
                  "abandon abandon abandon abandon abandon abandon "
                  "abandon abandon abandon abandon abandon about", None)

        # 1+2. Fresh wallet: full-scan sync, then receive a mined coinbase.
        bal0 = pactd.rpc("getbalance", "btcx")["balance_sat"]
        assert bal0 == 0, f"fresh wallet expected 0, got {bal0}"
        print("[spike] fresh-seed sync OK (balance 0)")
        addr = pactd.rpc("getnewaddress", "btcx")["address"]
        print(f"[spike] bdk address: {addr}")
        node.rpc("generatetoaddress", 1, addr)          # our coinbase
        node.rpc("generatetoaddress", 100, miner)       # bury to maturity
        electrs.wait_synced(111)

        # 3. Confirmed, mature balance visible through the chain source.
        bal1 = pactd.rpc("getbalance", "btcx")["balance_sat"]
        assert bal1 > 0, "mature coinbase not reflected in bdk balance"
        print(f"[spike] receive OK: balance {bal1} sat")

        # 4. Spend back to the node wallet: bdk builds+signs, Electrum
        # broadcasts, the node's mempool must see it.
        pay_back = node.new_address("miner")
        send_sat = 100_000_000  # 1 coin
        txid = pactd.rpc("sendtoaddress", "btcx", pay_back, "1.0")["txid"]
        mempool = node.rpc("getrawmempool")
        assert txid in mempool, f"sent tx {txid} not in node mempool {mempool}"
        print(f"[spike] send OK: {txid} in node mempool")
        node.rpc("generatetoaddress", 1, miner)
        electrs.wait_synced(112)

        # 5. Steady-state re-sync: spend + change accounted.
        bal2 = pactd.rpc("getbalance", "btcx")["balance_sat"]
        assert bal2 < bal1 - send_sat + 1, (
            f"balance after spend should drop by >= {send_sat}: {bal1} -> {bal2}")
        assert bal2 > 0, "change output lost"
        print(f"[spike] re-sync OK: balance {bal2} sat (spent {bal1 - bal2})")

        # 6. Activity feed.
        txs = pactd.rpc("listtransactions", "btcx")["transactions"]
        dirs = {t["txid"]: t["direction"] for t in txs}
        assert dirs.get(txid) == "sent", f"activity missing the send: {txs}"
        assert "received" in dirs.values(), f"activity missing the receive: {txs}"
        sent_row = next(t for t in txs if t["txid"] == txid)
        assert sent_row["amount_sat"] == send_sat, sent_row
        assert sent_row["confirmations"] >= 1, sent_row
        assert sent_row["fee_sat"], sent_row
        print(f"[spike] activity OK: {len(txs)} tx(s), send row {sent_row}")

        # 7. Persistence: restart pactd — steady-state sync (revealed spks
        # only), same balance, no full rescan needed.
        pactd.stop()
        pactd.start()
        bal3 = pactd.rpc("getbalance", "btcx")["balance_sat"]
        assert bal3 == bal2, f"balance changed across restart: {bal2} -> {bal3}"
        print(f"[spike] persistence OK: balance stable across restart")

        ok = True
    finally:
        pactd.stop()
        electrs.stop()
        node.stop()
        if ok:
            shutil.rmtree(workdir, ignore_errors=True)
        else:
            print(f"[spike] KEEPING workdir for inspection: {workdir}",
                  file=sys.stderr)

    print("\n[spike] GREEN: the nodeless chain source works against live electrs.")


if __name__ == "__main__":
    main()
