#!/usr/bin/env python3
"""Live spike for the nodeless wallet chain source (epic #58).

First real-server exercise of `wallet_bdk.rs`: stock bdk over our raw-Electrum
calls, against a REAL electrs (PoCX-patched romanz fork) indexing a regtest
PoCX node. Everything below `sync_entry` is unit-tested; this proves the wire.
The full swap-parity matrix lives in test_nodeless_e2e.py — this is the
minimal wallet-flow smoke, kept for quick iteration on the chain source.

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
import subprocess
import sys
import tempfile
import time
import urllib.request

from framework import binaries
from regtest_harness import (
    ELECTRS_ELECTRUM_PORT,
    POCX_REGTEST_GENESIS,
    POCX_REST_RPC_PORT,
    ElectrsServer,
    Node,
    find_pocx_bitcoind,
)

PACT_DIR = binaries.PACT_DIR
PACTD_BIN = binaries.pactd()

PACTD_PORT = 19752


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
            "--coin", f"btcx=tcp://127.0.0.1:{ELECTRS_ELECTRUM_PORT}",
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
                POCX_REST_RPC_PORT, POCX_REGTEST_GENESIS,
                extra_args=["-rest=1"])
    electrs = None
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
        electrs = ElectrsServer(workdir, node)
        electrs.start()
        electrs.wait_synced(10)
        print("[spike] electrs synced to height 10")

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
        print("[spike] persistence OK: balance stable across restart")

        ok = True
    finally:
        pactd.stop()
        if electrs:
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
