#!/usr/bin/env python3
"""Framework self-checks — fast, no nodes. The proper unit test the plan
demands for the mainnet-safe teardown port registry, plus sanity on the
per-scenario pactd port allocator and the binary resolver.

Run:  python tests/framework_selftest.py
"""

import os
import sys

sys.path.insert(0, os.path.normpath(
    os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")))

from framework import binaries, daemon, util  # noqa: E402
from framework.testbase import PactTestFramework, run_scenarios  # noqa: E402


class PortRegistryMainnetSafe(PactTestFramework):
    """The 15 Jun 2026 invariant: no teardown list may carry 9737/9738."""

    uses_harness = False

    def run_test(self):
        ports = util.all_teardown_ports()
        assert ports, "registry is empty?"
        assert util.MAINNET_PACTD_PORT == 9737
        assert util.TESTNET_PACTD_PORT == 9738
        for forbidden in (9737, 9738):
            assert forbidden in util.FORBIDDEN_TEARDOWN_PORTS
            assert forbidden not in ports, \
                f"FORBIDDEN port {forbidden} in the teardown registry!"
        # No group may sneak a forbidden port in either (the aggregate check
        # in all_teardown_ports would catch it, but assert per group for a
        # pinpointed failure message).
        for group, members in util.TEARDOWN_PORT_GROUPS.items():
            hit = set(members) & util.FORBIDDEN_TEARDOWN_PORTS
            assert not hit, f"group {group!r} contains forbidden ports {hit}"
        # The documented pactd range is present and correctly bounded.
        rng = util.TEARDOWN_PORT_GROUPS["pactd_range"]
        assert rng[0] == daemon.PACTD_PORT and rng[-1] == daemon.PACTD_PORT_MAX
        print(f"[selftest] registry OK ({len(ports)} ports, 9737/9738 excluded)")


class PortAllocatorResets(PactTestFramework):
    """Per-scenario allocation restarts at the range base and is capped."""

    uses_harness = False

    def run_test(self):
        daemon.reset_port_allocator()
        first = daemon._alloc_port()
        assert daemon.PACTD_PORT <= first <= daemon.PACTD_PORT_MAX
        second = daemon._alloc_port()
        assert second > first
        daemon.reset_port_allocator()
        again = daemon._alloc_port()
        assert again == first, \
            f"reset did not restart allocation: {first} then {again}"
        # The cap: exhausting the range raises instead of walking into the
        # electrs ports.
        daemon._next_port[0] = daemon.PACTD_PORT_MAX + 1
        try:
            daemon._alloc_port()
            raise AssertionError("allocator walked past PACTD_PORT_MAX")
        except RuntimeError as exc:
            assert "range exhausted" in str(exc)
        finally:
            daemon.reset_port_allocator()
        print(f"[selftest] allocator OK (base {daemon.PACTD_PORT}, "
              f"cap {daemon.PACTD_PORT_MAX})")


class BinaryResolver(PactTestFramework):
    """Every binary the suites need resolves (fails fast with a clear message
    before an hour-long run does)."""

    uses_harness = False

    def run_test(self):
        found = {
            "pocx-bitcoind": binaries.find_pocx_bitcoind(),
            "btc-bitcoind": binaries.find_btc_bitcoind(),
            "electrs": binaries.find_electrs(),
            "btc-electrs": binaries.find_btc_electrs(),
        }
        for name, path in found.items():
            assert os.path.exists(path), f"{name}: {path}"
        # litecoind is only needed by the with_ltc cells — report, don't fail.
        try:
            found["litecoind"] = binaries.find_litecoind()
        except FileNotFoundError:
            print("[selftest] note: litecoind not found — with_ltc cells would fail")
        print(f"[selftest] resolver OK ({len(found)} binaries)")


SCENARIOS = [
    PortRegistryMainnetSafe,
    PortAllocatorResets,
    BinaryResolver,
]


if __name__ == "__main__":
    run_scenarios(SCENARIOS)
