#!/usr/bin/env python3
"""Multi-machine safety (issue #122) — the automatable half of the #122 test
matrix (docs/MULTI_MACHINE_122.md, Testing section), ported from the retired
tools/playground-multimachine.ps1. Two pactds on the SAME seed in SEPARATE
data dirs ("machine A" + "machine B") verify the seed-scoped partition —
no nodes, board or Electrum needed (offline derivation + the data-dir lock):

  1. Section 0 data-dir lock — a 2nd pactd on machine A's data dir is REFUSED.
  2. Section 1 machine label — A and B derive DISTINCT machine labels
     (distinct machine.json scopes).
  3. Section 1 partition — the SAME offer built on A and B yields DISTINCT
     swap_id + distinct nonzero derive_scope (so preimage, keys and relay
     coordinates never collide).

The full FAILOVER flow (A drives a live swap, B follows read-only, kill A,
take over on B, B drives to completion) is on-chain + human-confirm driven,
so it stays a MANUAL walkthrough on top of a normal playground stack:
  1. Bring up a full stack (nodes + board + a counterparty trader), point
     BOTH A and B at the SAME coins, same board/relay (shared seed wallet).
  2. On machine A, post an offer; counterparty takes it: listswaps on A
     shows source=local.
  3. On machine B, listswaps shows the SAME swap source=foreign with A's
     machine_label (Satchel: the read-only "Another machine" group). B never
     broadcasts for it.
  4. Withdraw (getnewaddress/sendtoaddress) works on BOTH A and B — always.
  5. Kill machine A; confirm it is really stopped.
  6. On B: `takeover <swap_id>` — listswaps flips to source=local and B
     drives the swap to the end.

Run:  python tests/multimachine.py
"""

import os
import subprocess
import sys
import time

sys.path.insert(0, os.path.normpath(
    os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")))

from framework import binaries  # noqa: E402
from framework.testbase import PactTestFramework, run_scenarios  # noqa: E402
from framework.util import pactd_rpc, read_cookie, wait_until  # noqa: E402

PORT_A = 19801
PORT_B = 19802
PORT_LOCK = 19803   # the refused 2nd-pactd-on-A attempt

# The standard BIP39 test mnemonic — shared by both "machines".
MNEMONIC = ("abandon abandon abandon abandon abandon abandon abandon abandon "
            "abandon abandon abandon about")


class _BarePactd:
    """A coin-less pactd (no nodes involved) on its own data dir + port."""

    def __init__(self, data_dir, port):
        self.data_dir = data_dir
        self.port = port
        self.proc = None
        self.cookie = None
        self._logf = None

    def spawn(self):
        """Launch without waiting (the lock check wants the raw process)."""
        os.makedirs(self.data_dir, exist_ok=True)
        self._logf = open(os.path.join(self.data_dir, "pactd.log"), "w",
                          encoding="utf-8")
        self.proc = subprocess.Popen(
            [binaries.pactd(), "--data-dir", self.data_dir,
             "--listen", f"127.0.0.1:{self.port}", "--network", "regtest"],
            stdout=self._logf, stderr=subprocess.STDOUT)
        return self

    def start(self):
        self.spawn()

        def up():
            if self.proc.poll() is not None:
                raise RuntimeError(
                    f"pactd (:{self.port}) exited early: {self.proc.returncode} "
                    f"(see {self.data_dir}/pactd.log)")
            try:
                self.cookie = read_cookie(os.path.join(self.data_dir, ".cookie"))
                self.rpc("getinfo")
                return True
            except Exception:  # noqa: BLE001 — not up yet
                return False

        wait_until(up, timeout=20, poll=0.4, what=f"pactd on :{self.port}")
        return self

    def rpc(self, method, *params):
        return pactd_rpc(f"http://127.0.0.1:{self.port}/", method, *params,
                         cookie=self.cookie, timeout=10)

    def stop(self):
        if self.proc:
            self.proc.terminate()
            try:
                self.proc.wait(timeout=15)
            except subprocess.TimeoutExpired:
                self.proc.kill()
            self.proc = None
        if self._logf:
            self._logf.close()
            self._logf = None


class MultiMachinePartition(PactTestFramework):
    """The three automated #122 checks on two same-seed pactds."""

    uses_harness = False

    def run_test(self):
        a = _BarePactd(os.path.join(self.workdir, "machineA"), PORT_A)
        b = _BarePactd(os.path.join(self.workdir, "machineB"), PORT_B)
        lock = None
        try:
            a.start()
            a.rpc("importseed", MNEMONIC, None)
            b.start()
            b.rpc("importseed", MNEMONIC, None)

            # 1. data-dir lock: a 2nd pactd on A's dir must exit nonzero.
            lock = _BarePactd(a.data_dir, PORT_LOCK)
            # Don't let the attempt clobber A's log.
            lock_log = os.path.join(self.workdir, "lock-attempt.log")
            lock._logf = open(lock_log, "w", encoding="utf-8")
            lock.proc = subprocess.Popen(
                [binaries.pactd(), "--data-dir", a.data_dir,
                 "--listen", f"127.0.0.1:{PORT_LOCK}", "--network", "regtest"],
                stdout=lock._logf, stderr=subprocess.STDOUT)
            try:
                code = lock.proc.wait(timeout=8)
            except subprocess.TimeoutExpired:
                lock.stop()
                raise AssertionError(
                    "a 2nd pactd on machine A's data dir was NOT refused "
                    "(still running after 8s)")
            assert code != 0, \
                f"2nd pactd on A's data dir exited 0 — the lock did not refuse it"
            print(f"[mm] data-dir lock OK (2nd pactd refused, exit {code})")

            # 2. distinct machine labels (distinct machine.json scopes).
            label_a = a.rpc("getinfo")["machine_label"]
            label_b = b.rpc("getinfo")["machine_label"]
            assert label_a and label_b and label_a != label_b, \
                f"machines must derive distinct labels: A={label_a} B={label_b}"
            print(f"[mm] distinct machine labels OK (A={label_a} B={label_b})")

            # 3. same offer on A and B => distinct swap_id + distinct nonzero
            #    derive_scope (the seed-scoped partition).
            off_a = a.rpc("offer", "btcx:100", "btc:100", 1700000002, 1700000001)["record"]
            off_b = b.rpc("offer", "btcx:100", "btc:100", 1700000002, 1700000001)["record"]
            assert off_a["swap_id"] != off_b["swap_id"], \
                f"same offer minted the SAME swap_id on both machines: {off_a['swap_id']}"
            sa, sb = off_a["derive_scope"], off_b["derive_scope"]
            assert sa and sb and sa != sb, \
                f"derive scopes must be distinct and nonzero: A={sa} B={sb}"
            print(f"[mm] partition OK (swap ids + scopes distinct: {sa} vs {sb})")
        finally:
            for p in (lock, a, b):
                if p is not None:
                    p.stop()
            time.sleep(0.2)  # let Windows release the data-dir handles


SCENARIOS = [
    MultiMachinePartition,
]


if __name__ == "__main__":
    run_scenarios(SCENARIOS)
