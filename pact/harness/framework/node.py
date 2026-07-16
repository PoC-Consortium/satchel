#!/usr/bin/env python3
"""Regtest nodes for Pact: PoCX / Bitcoin / Litecoin `Node`s, `ElectrsServer`,
and the `Harness` that composes funded two-chain stacks for the e2e suites.
Moved from regtest_harness.py (Phase 2); that module remains as a compat shim.

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
# Regtest seeds take the obfuscation wrap (#120), not a real OS-keystore key —
# keeps playground/e2e runs off the developer's Windows/macOS keychain. pactd
# subprocesses inherit this env.
os.environ.setdefault("PACT_DISABLE_KEYRING", "1")
import shutil
import subprocess
import time
import urllib.request

from framework.binaries import (
    find_btc_bitcoind,
    find_electrs,
    find_litecoind,
    find_pocx_bitcoind,
)

POCX_REGTEST_GENESIS = "2a98a52253aeff06093948b00568d380b7634621bc606403127973c9acbbfde0"
BTC_REGTEST_GENESIS = "0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e2206"
LTC_REGTEST_GENESIS = "530827f38f93b43ed12af0b3ad25a288dc02ed74d6d7857862df51fc56c416f9"

# Non-default ports so a developer's own regtest nodes are not disturbed.
POCX_RPC_PORT = 19443
BTC_RPC_PORT = 19543
# Nodeless (epic #58) exception: bindex-pocx (electrs' indexer) fetches blocks
# over bitcoind's REST interface at the HARDCODED network-default RPC port —
# regtest 18443 (`--daemon-rpc-addr` does not move the indexer). A harness that
# brings up electrs must therefore run its PoCX node on 18443 with `-rest=1`
# (`Harness(pocx_rest=True)`); REST is unauthenticated on the RPC port.
POCX_REST_RPC_PORT = 18443
ELECTRS_ELECTRUM_PORT = 19750
ELECTRS_MONITORING_PORT = 19751
# Vanilla (upstream) electrs for the BTC leg has the same bindex REST hardcode,
# so a nodeless-BTC stack parks the BTC regtest node on the TESTNET default RPC
# port and runs electrs with --network testnet (bindex asserts no genesis — the
# PoCX fork already proves a "regtest" network serving a non-bitcoin chain).
# Needs a Core v31+ node (/rest/blockpart).
BTC_REST_RPC_PORT = 18332
BTC_ELECTRS_ELECTRUM_PORT = 19760
BTC_ELECTRS_MONITORING_PORT = 19761
# Litecoin RPC port matches the `ltc` regtest connection default in
# satchel/coins.toml so the same port story holds end to end. Only used when a
# Harness is built with_ltc (the playground); the e2e suite never starts it.
LTC_RPC_PORT = 19643


class ElectrsServer:
    """The PoCX-patched electrs (romanz fork over bindex-pocx), indexing a
    regtest PoCX node that MUST be on :18443 with -rest=1 (see
    POCX_REST_RPC_PORT above). Electrum RPC on ELECTRS_ELECTRUM_PORT."""

    def __init__(self, workdir, node, electrum_port=ELECTRS_ELECTRUM_PORT,
                 monitoring_port=ELECTRS_MONITORING_PORT,
                 network="regtest", binary=None, name="electrs"):
        # `network` picks bindex's hardcoded REST port (regtest→18443,
        # testnet→18332); `binary` overrides the PoCX-patched default (the
        # vanilla upstream electrs for a BTC leg); `name` keeps two instances'
        # dirs/logs apart in one workdir.
        self.dir = os.path.join(workdir, name)
        os.makedirs(self.dir, exist_ok=True)
        self.electrum_port = electrum_port
        self.monitoring_port = monitoring_port
        self.network = network
        self.binary = binary
        self.name = name
        # electrs authenticates its RPC (non-REST) calls via a cookie FILE
        # whose content is user:pass — hand it the node's credentials.
        self.cookie_path = os.path.join(self.dir, "rpc.cookie")
        with open(self.cookie_path, "w", encoding="ascii") as fh:
            fh.write(f"{node.rpc_user}:{node.rpc_pass}")
        self.daemon_port = node.rpc_port
        self.proc = None
        self.logf = None

    @property
    def url(self):
        return f"tcp://127.0.0.1:{self.electrum_port}"

    def start(self):
        cmd = [
            self.binary or find_electrs(),
            "--network", self.network,
            "--daemon-rpc-addr", f"127.0.0.1:{self.daemon_port}",
            "--cookie-file", self.cookie_path,
            "--db-dir", os.path.join(self.dir, "db"),
            "--electrum-rpc-addr", f"127.0.0.1:{self.electrum_port}",
            "--monitoring-addr", f"127.0.0.1:{self.monitoring_port}",
            "--log-filters", "INFO",
        ]
        self.logf = open(os.path.join(self.dir, "electrs.log"), "w", encoding="utf-8")
        self.proc = subprocess.Popen(cmd, stdout=self.logf, stderr=subprocess.STDOUT)

    def raw_call(self, method, params, timeout=5):
        """One-shot Electrum JSONRPC call over a fresh TCP connection."""
        import socket
        with socket.create_connection(("127.0.0.1", self.electrum_port),
                                      timeout=timeout) as s:
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
        probed = False
        while time.time() < deadline:
            if self.proc.poll() is not None:
                raise RuntimeError(
                    f"electrs exited early: {self.proc.returncode} "
                    f"(see {self.dir}/electrs.log)")
            try:
                # Fork wart (reported upstream): headers.subscribe PANICS —
                # killing the whole server — while the initial index is still
                # empty (electrum.rs `tip_height().unwrap()`). Probe with
                # block.header(0) first: it error-returns cleanly until the
                # index holds at least genesis.
                if not probed:
                    self.raw_call("blockchain.block.header", [0])
                    probed = True
                tip = self.raw_call("blockchain.headers.subscribe", [])
                last = tip.get("height")
                if last is not None and last >= want_height:
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


class RpcError(Exception):
    def __init__(self, code, message):
        super().__init__(f"RPC error {code}: {message}")
        self.code = code


class Node:
    """One regtest node (PoCX or BTC) plus JSON-RPC access."""

    def __init__(self, name, binary, datadir, rpc_port, expected_genesis, extra_args=None):
        self.name = name
        self.binary = binary
        self.datadir = datadir
        self.rpc_port = rpc_port
        self.expected_genesis = expected_genesis
        self.rpc_user = "pactharness"
        self.rpc_pass = "pactharness"
        # Per-node launch flags (e.g. the LTC node disables MWEB — see Harness).
        self.extra_args = extra_args or []
        self.proc = None

    def start(self, runtime_args=None):
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
            # 1 sat/vB (0.00001 BTC/kvB). Regtest has no fee history so the wallet
            # can't estimate and falls back to this for funding txs; keep it at the
            # market floor so playground funding fees read ~1 sat/vB like the rest,
            # instead of an artificial 10x hump. (Mainnet nodes disable fallbackfee
            # and use live estimatesmartfee.)
            "-fallbackfee=0.00001",
            "-debug=rpc",
        ] + self.extra_args + (runtime_args or [])
        # stderr to a file: early-init failures (datadir lock, bind, settings)
        # exit BEFORE debug.log opens and used to vanish into DEVNULL.
        self._errf = open(os.path.join(self.datadir, "stderr.log"), "a", encoding="utf-8")
        self.proc = subprocess.Popen(
            args, stdout=subprocess.DEVNULL, stderr=self._errf)
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
                self._errf.close()
                with open(os.path.join(self.datadir, "stderr.log"),
                          encoding="utf-8", errors="replace") as fh:
                    err = fh.read().strip()
                raise RuntimeError(
                    f"{self.name}: node exited early with code {self.proc.returncode} "
                    f"(see {self.datadir}/regtest/debug.log)"
                    + (f"\nstderr: {err}" if err else ""))
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

    def load_wallet(self, name):
        """Load a wallet that already exists in the datadir (the cache-restore
        path — createwallet does not persist load-on-startup, so a copied
        datadir starts with its wallets unloaded)."""
        try:
            self.rpc("loadwallet", name)
        except RpcError as e:
            if e.code != -35:  # -35 = already loaded
                raise
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
        errf = getattr(self, "_errf", None)
        if errf is not None:
            try:
                errf.close()
            except Exception:  # noqa: BLE001
                pass
            self._errf = None


class Harness:
    """Both nodes + funded wallets for Alice and Bob.

    Wallet layout (one node per chain, one wallet per party per chain):
      pocx node: wallets alice_pocx (funded), bob_pocx   (empty)
      btc node:  wallets bob_btc    (funded), alice_btc  (empty)

    use_cache=True restores the pre-mined funded datadirs from the on-demand
    cache (framework/stack.py, plan §2.3) instead of mining 110 blocks per
    chain, then mines a short 12-block runway at the CURRENT mock clock so
    tip time and MTP land at "now" (cached block timestamps are from cache
    build time — without the runway, timelocks computed off chain time would
    sit in the wall-clock past). Playgrounds keep the uncached path.
    """

    def __init__(self, workdir=None, keep=False, with_ltc=False, pocx_rest=False,
                 btc_rest=False, use_cache=False):
        import tempfile
        self.workdir = workdir or tempfile.mkdtemp(prefix="pact-regtest-")
        self.keep = keep
        self.use_cache = use_cache
        # pocx_rest: nodeless/electrs stacks need the PoCX node on the regtest
        # DEFAULT RPC port with REST on (bindex hardcodes :18443 — see
        # POCX_REST_RPC_PORT). Everything else is unchanged.
        self.pocx = Node("pocx", find_pocx_bitcoind(),
                         os.path.join(self.workdir, "pocx"),
                         POCX_REST_RPC_PORT if pocx_rest else POCX_RPC_PORT,
                         POCX_REGTEST_GENESIS,
                         extra_args=["-rest=1"] if pocx_rest else None)
        # btc_rest: nodeless-BTC stacks need the BTC node on bindex's testnet
        # default (18332) with REST on — see BTC_REST_RPC_PORT above.
        self.btc = Node("btc", find_btc_bitcoind(),
                        os.path.join(self.workdir, "btc"),
                        BTC_REST_RPC_PORT if btc_rest else BTC_RPC_PORT,
                        BTC_REGTEST_GENESIS,
                        extra_args=["-rest=1"] if btc_rest else None)
        # Optional third chain (Litecoin) — a file-added coin, brought up only by
        # callers that ask for it (the playground). The e2e suite leaves it off
        # so it never depends on a litecoind binary. Wallets/funding for this
        # node are the caller's job (mirrors how carol/alice extra wallets are
        # created in the playground, not here). The LTC node is never cached:
        # only a couple of scenarios use it and its callers own its funding.
        self.ltc = None
        if with_ltc:
            # Disable MWEB on the regtest LTC node. Litecoin Core 0.21.5's MWEB
            # locks in at regtest height ~432; once active, CreateNewBlock builds
            # an MWEB/HogEx integration tx that fails TestBlockValidity with
            # bad-txns-vin-empty whenever the mempool is non-empty — so HTLC
            # funding txs never confirm and every LTC swap stalls at "accepted".
            # A far-future vbparams start keeps MWEB DEFINED (never active), so
            # block assembly stays normal. (Pact swaps don't use MWEB.)
            self.ltc = Node("ltc", find_litecoind(),
                            os.path.join(self.workdir, "ltc"), LTC_RPC_PORT,
                            LTC_REGTEST_GENESIS,
                            extra_args=["-vbparams=mweb:9999999999:9999999999"])

    def __enter__(self):
        print(f"[harness] workdir: {self.workdir}")
        runtime_args = None
        if self.use_cache:
            from framework.stack import ensure_node_cache
            cache_dirs, cache_tips = ensure_node_cache()
            for node, key in ((self.pocx, "pocx"), (self.btc, "btc")):
                if os.path.exists(node.datadir):
                    shutil.rmtree(node.datadir)
                shutil.copytree(cache_dirs[key], node.datadir)
            # PoCX forging auto-advanced the mock clock while the cache was
            # mined, so the cached tips sit AHEAD of the build wall clock — a
            # plain restart trips Core's block-from-the-future init check.
            # Boot with -mocktime past every cached tip; also covers an OLD
            # cache (tips behind wall) by re-basing at "now".
            now = max([int(time.time())] + [t + 1 for t in cache_tips.values()])
            runtime_args = [f"-mocktime={now}"]
        else:
            now = int(time.time())
        self.pocx.start(runtime_args=runtime_args)
        print(f"[harness] pocx node up (rpc :{self.pocx.rpc_port})")
        self.btc.start(runtime_args=runtime_args)
        print(f"[harness] btc node up (rpc :{self.btc.rpc_port})")
        if self.ltc:
            self.ltc.start()
            print(f"[harness] ltc node up (rpc :{self.ltc.rpc_port})")

        # PoCX regtest forging needs mocktime to mine without real delays;
        # keep all chains on the same mock clock so CLTV timelocks line up.
        self.pocx.set_mocktime(now)
        self.btc.set_mocktime(now)
        if self.ltc:
            self.ltc.set_mocktime(now)

        for node, funded, empty in ((self.pocx, "alice_pocx", "bob_pocx"),
                                    (self.btc, "bob_btc", "alice_btc")):
            if self.use_cache:
                # Wallets ride in the cached datadir; re-load them, then mine a
                # 12-block runway stamped at the CURRENT mock clock so the tip
                # time and MTP land at `now` (same trick as advance_time) —
                # scenarios derive timelocks from the tip time, which would
                # otherwise be the cache build's clock.
                node.load_wallet(funded)
                node.load_wallet(empty)
                node.generate(12, funded)
            else:
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
        if self.ltc:
            self.ltc.stop()
        if not self.keep and exc_type is None:
            shutil.rmtree(self.workdir, ignore_errors=True)
        elif exc_type is not None:
            # keep=True clean exits stay silent: the caller (testbase) owns
            # the workdir lifecycle and reports whatever it decides to keep.
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
