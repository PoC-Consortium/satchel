#!/usr/bin/env python3
"""Regtest harness for Pact: launches a PoCX node + a Bitcoin node and
provides RPC plumbing for the end-to-end swap tests.

Stdlib only. Binaries are located via (in order):
  PoCX node:    $POCX_BITCOIND, else harness/bin/pocx-bitcoind(.exe),
                else ../../../bitcoin-pocx/bitcoin/build/bin/bitcoind(.exe)
  Bitcoin node: $BTC_BITCOIND,  else harness/bin/btc-bitcoind(.exe),
                else `bitcoind` on PATH
(harness/bin is gitignored; copy installed daemons there for convenience)

Chain facts this harness depends on (read from bitcoin-pocx chainparams —
do not guess):
  * PoCX regtest shares Bitcoin regtest's network magic (fa bf b5 da) and
    default port 18444 -> both nodes run with -listen=0 and explicit
    -rpcport, so they can never cross-connect.
  * PoCX regtest block forging enforces a minimum inter-block delay UNLESS
    setmocktime is active (pocx/regtest/forging.cpp). The harness always
    sets mocktime on the PoCX node; forging then auto-advances mock time.
    This also makes MTP-based CLTV refund tests deterministic.
  * PoCX regtest bech32 HRP is "rpocx"; genesis
    2a98a52253aeff06093948b00568d380b7634621bc606403127973c9acbbfde0.

Smoke test (no Pact involved):  python regtest_harness.py --smoke
"""

import json
import os
import platform
import shutil
import subprocess
import sys
import tempfile
import time
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
EXE = ".exe" if platform.system() == "Windows" else ""

POCX_REGTEST_GENESIS = "2a98a52253aeff06093948b00568d380b7634621bc606403127973c9acbbfde0"
BTC_REGTEST_GENESIS = "0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e2206"

# Non-default ports so a developer's own regtest nodes are not disturbed.
POCX_RPC_PORT = 19443
BTC_RPC_PORT = 19543


def find_pocx_bitcoind():
    candidates = [
        os.environ.get("POCX_BITCOIND"),
        os.path.join(HERE, "bin", "pocx-bitcoind" + EXE),
        os.path.normpath(os.path.join(
            HERE, "..", "..", "..", "bitcoin-pocx", "bitcoin", "build", "bin", "bitcoind" + EXE)),
    ]
    for candidate in candidates:
        if candidate and os.path.exists(candidate):
            return candidate
    raise FileNotFoundError(
        "PoCX node binary not found. Copy the installed daemon to "
        "harness/bin/pocx-bitcoind" + EXE + " or set POCX_BITCOIND.")


def find_btc_bitcoind():
    candidates = [
        os.environ.get("BTC_BITCOIND"),
        os.path.join(HERE, "bin", "btc-bitcoind" + EXE),
        shutil.which("bitcoind"),
    ]
    for candidate in candidates:
        if candidate and os.path.exists(candidate):
            return candidate
    raise FileNotFoundError(
        "Bitcoin Core binary not found. Copy the installed daemon to "
        "harness/bin/btc-bitcoind" + EXE + " or set BTC_BITCOIND.")


class RpcError(Exception):
    def __init__(self, code, message):
        super().__init__(f"RPC error {code}: {message}")
        self.code = code


class Node:
    """One regtest node (PoCX or BTC) plus JSON-RPC access."""

    def __init__(self, name, binary, datadir, rpc_port, expected_genesis):
        self.name = name
        self.binary = binary
        self.datadir = datadir
        self.rpc_port = rpc_port
        self.expected_genesis = expected_genesis
        self.rpc_user = "pactharness"
        self.rpc_pass = "pactharness"
        self.proc = None

    def start(self):
        os.makedirs(self.datadir, exist_ok=True)
        args = [
            self.binary,
            "-regtest",
            f"-datadir={self.datadir}",
            "-listen=0",          # no P2P: avoids the shared-magic/port trap entirely
            "-server=1",
            f"-rpcport={self.rpc_port}",
            f"-rpcuser={self.rpc_user}",
            f"-rpcpassword={self.rpc_pass}",
            "-fallbackfee=0.0001",
            "-debug=rpc",
        ]
        self.proc = subprocess.Popen(
            args, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        self._wait_for_rpc()
        genesis = self.rpc("getblockhash", 0)
        if genesis != self.expected_genesis:
            raise RuntimeError(
                f"{self.name}: wrong chain! genesis {genesis}, expected "
                f"{self.expected_genesis} — check which binary/network this is")

    def _wait_for_rpc(self, timeout=60):
        deadline = time.time() + timeout
        while time.time() < deadline:
            if self.proc.poll() is not None:
                raise RuntimeError(
                    f"{self.name}: node exited early with code {self.proc.returncode} "
                    f"(see {self.datadir}/regtest/debug.log)")
            try:
                self.rpc("getblockcount")
                return
            except Exception:
                time.sleep(0.25)
        raise TimeoutError(f"{self.name}: RPC not up after {timeout}s")

    def rpc(self, method, *params, wallet=None):
        url = f"http://127.0.0.1:{self.rpc_port}"
        if wallet is not None:
            url += f"/wallet/{wallet}"
        payload = json.dumps({
            "jsonrpc": "2.0", "id": "harness", "method": method, "params": list(params),
        }).encode()
        req = urllib.request.Request(url, data=payload, method="POST")
        import base64
        auth = base64.b64encode(f"{self.rpc_user}:{self.rpc_pass}".encode()).decode()
        req.add_header("Authorization", f"Basic {auth}")
        req.add_header("Content-Type", "application/json")
        try:
            with urllib.request.urlopen(req, timeout=120) as resp:
                body = json.loads(resp.read())
        except urllib.error.HTTPError as e:
            body = json.loads(e.read())
        if body.get("error"):
            raise RpcError(body["error"]["code"], body["error"]["message"])
        return body["result"]

    def rpc_url(self, wallet=None):
        suffix = f"/wallet/{wallet}" if wallet else ""
        return f"http://{self.rpc_user}:{self.rpc_pass}@127.0.0.1:{self.rpc_port}{suffix}"

    def create_wallet(self, name):
        self.rpc("createwallet", name)
        return name

    def new_address(self, wallet):
        return self.rpc("getnewaddress", wallet=wallet)

    def generate(self, nblocks, wallet):
        addr = self.new_address(wallet)
        return self.rpc("generatetoaddress", nblocks, addr)

    def set_mocktime(self, unix_time):
        self.rpc("setmocktime", int(unix_time))

    def median_time(self):
        return self.rpc("getblockchaininfo")["mediantime"]

    def stop(self):
        if self.proc is None:
            return
        try:
            self.rpc("stop")
            self.proc.wait(timeout=30)
        except Exception:
            self.proc.kill()
            self.proc.wait(timeout=10)
        self.proc = None


class Harness:
    """Both nodes + funded wallets for Alice and Bob.

    Wallet layout (one node per chain, one wallet per party per chain):
      pocx node: wallets alice_pocx (funded), bob_pocx   (empty)
      btc node:  wallets bob_btc    (funded), alice_btc  (empty)
    """

    def __init__(self, workdir=None, keep=False):
        self.workdir = workdir or tempfile.mkdtemp(prefix="pact-regtest-")
        self.keep = keep
        self.pocx = Node("pocx", find_pocx_bitcoind(),
                         os.path.join(self.workdir, "pocx"), POCX_RPC_PORT,
                         POCX_REGTEST_GENESIS)
        self.btc = Node("btc", find_btc_bitcoind(),
                        os.path.join(self.workdir, "btc"), BTC_RPC_PORT,
                        BTC_REGTEST_GENESIS)

    def __enter__(self):
        print(f"[harness] workdir: {self.workdir}")
        self.pocx.start()
        print(f"[harness] pocx node up (rpc :{self.pocx.rpc_port})")
        self.btc.start()
        print(f"[harness] btc node up (rpc :{self.btc.rpc_port})")

        # PoCX regtest forging needs mocktime to mine without real delays;
        # keep both chains on the same mock clock so CLTV tests line up.
        now = int(time.time())
        self.pocx.set_mocktime(now)
        self.btc.set_mocktime(now)

        for node, funded, empty in ((self.pocx, "alice_pocx", "bob_pocx"),
                                    (self.btc, "bob_btc", "alice_btc")):
            node.create_wallet(funded)
            node.create_wallet(empty)
            # 110 blocks: >100 for coinbase maturity, headroom for fees.
            node.generate(110, funded)
        print("[harness] wallets funded "
              f"(alice_pocx: {self.pocx.rpc('getbalance', wallet='alice_pocx')} POCX, "
              f"bob_btc: {self.btc.rpc('getbalance', wallet='bob_btc')} BTC)")
        return self

    def advance_time(self, seconds, blocks_each=12):
        """Jump both mock clocks forward and mine blocks so MTP follows.

        MTP is the median of the last 11 block timestamps, so ~12 blocks
        after the jump push the median past the new time.
        """
        target = max(self.pocx.rpc("getblockchaininfo")["time"],
                     self.btc.rpc("getblockchaininfo")["time"]) + seconds
        for node, wallet in ((self.pocx, "alice_pocx"), (self.btc, "bob_btc")):
            node.set_mocktime(target)
            node.generate(blocks_each, wallet)
        return target

    def __exit__(self, exc_type, exc, tb):
        self.pocx.stop()
        self.btc.stop()
        if not self.keep and exc_type is None:
            shutil.rmtree(self.workdir, ignore_errors=True)
        else:
            print(f"[harness] keeping workdir: {self.workdir}")
        return False


def smoke():
    """Infrastructure check without Pact: nodes start, mine, obey mocktime."""
    with Harness(keep=False) as h:
        assert h.pocx.rpc("getblockcount") == 110
        assert h.btc.rpc("getblockcount") == 110
        before = h.pocx.median_time()
        h.advance_time(3600)
        after = h.pocx.median_time()
        assert after > before, "PoCX MTP did not advance with mocktime"
        assert h.pocx.rpc("getblockcount") == 122
        print("[smoke] OK: both nodes mine and MTP advances under mocktime")


if __name__ == "__main__":
    if "--smoke" in sys.argv:
        smoke()
    else:
        print(__doc__)
