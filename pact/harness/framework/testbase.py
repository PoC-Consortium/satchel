"""PactTestFramework — the Bitcoin-Core-style scenario base
(TEST_FRAMEWORK_PLAN §2.0/§2.3).

One instance = one scenario = one FRESH stack in its own tmpdir: the
per-scenario isolation decision. The Harness restores the pre-mined funded
datadirs from the on-demand cache (framework/stack.py) so a fresh stack costs
node startup + a datadir copy, not 110 mined blocks per chain.

A test file under tests/ holds one or more subclasses and ends with

    if __name__ == "__main__":
        run_scenarios(SCENARIOS)

which gives every file the same CLI (--filter/--keep/--no-build/
--rebuild-cache) whether it is invoked standalone or by test_runner.py.
"""

import argparse
import shutil
import sys
import tempfile
import time
import traceback

from framework import stack
from framework.daemon import reset_port_allocator
from framework.node import Harness


class PactTestFramework:
    """Subclass per scenario: override run_test() (required) and, if the
    stack shape differs from the plain two-node default, set the class attrs
    below or override set_test_params(). self.h is the live Harness inside
    run_test(); scenario-local services (Corkboard/NostrRelay/electrs) are
    started inside run_test() with try/finally, exactly like the pre-split
    suites did."""

    uses_harness = True   # False = no nodes at all (e.g. multimachine)
    with_ltc = False
    pocx_rest = False
    btc_rest = False

    @property
    def name(self):
        return type(self).__name__

    def set_test_params(self):
        """Optional hook to tweak the class attrs per instance."""

    def run_test(self):
        raise NotImplementedError

    def run(self, keep=False):
        """Run the scenario on a fresh stack. Raises on failure; the workdir
        is kept (and its path printed) whenever the scenario fails or
        keep=True."""
        self.set_test_params()
        self.workdir = tempfile.mkdtemp(prefix=f"pact-{self.name}-")
        # Per-scenario allocator reset (plan Appendix B): parties start back
        # at the base of the 19737–19749 range every scenario.
        reset_port_allocator()
        ok = False
        try:
            if self.uses_harness:
                # keep=True: workdir lifecycle belongs to THIS class, not the
                # Harness (which would otherwise also try to delete it).
                with Harness(workdir=self.workdir, keep=True, use_cache=True,
                             with_ltc=self.with_ltc, pocx_rest=self.pocx_rest,
                             btc_rest=self.btc_rest) as h:
                    self.h = h
                    self.run_test()
            else:
                self.run_test()
            ok = True
        finally:
            if ok and not keep:
                shutil.rmtree(self.workdir, ignore_errors=True)
            else:
                print(f"[testbase] keeping workdir: {self.workdir}")


def run_scenarios(scenarios, argv=None):
    """Module entry for a tests/*.py file: run each scenario class on its own
    fresh stack, continue past failures, exit nonzero if any failed."""
    ap = argparse.ArgumentParser()
    ap.add_argument("--filter", default="", metavar="SUBSTR",
                    help="only run scenario classes whose name contains SUBSTR")
    ap.add_argument("--keep", action="store_true",
                    help="keep every scenario workdir (default: keep on failure only)")
    ap.add_argument("--no-build", action="store_true",
                    help="skip cargo build (the runner builds once up front)")
    ap.add_argument("--rebuild-cache", action="store_true",
                    help="drop the funded-datadir cache before running")
    args = ap.parse_args(argv)

    if args.rebuild_cache:
        stack.drop_node_cache()
    if not args.no_build:
        stack.build_workspace()

    selected = [cls for cls in scenarios if args.filter in cls.__name__]
    if not selected:
        print(f"no scenario matches --filter {args.filter!r}", file=sys.stderr)
        sys.exit(2)

    failures = []
    for cls in selected:
        started = time.time()
        print(f"\n===== {cls.__name__} =====")
        try:
            cls().run(keep=args.keep)
            print(f"----- {cls.__name__} PASSED ({time.time() - started:.0f}s)")
        except Exception:  # noqa: BLE001 — report and continue, like the old suites
            failures.append(cls.__name__)
            traceback.print_exc()
            print(f"----- {cls.__name__} FAILED ({time.time() - started:.0f}s)",
                  file=sys.stderr)

    if failures:
        print(f"\nRED: {len(failures)}/{len(selected)} scenario(s) failing: "
              f"{', '.join(failures)}", file=sys.stderr)
        sys.exit(1)
    print(f"\nGREEN: all {len(selected)} scenario(s) pass.")
