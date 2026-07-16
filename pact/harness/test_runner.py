#!/usr/bin/env python3
"""The e2e test runner (TEST_FRAMEWORK_PLAN §2.6) — the single entry that
runs every asserting suite under tests/.

Bitcoin-Core style: an EXPLICIT ordered list, cross-checked against the
directory — a tests/*.py that exists but is not listed is a hard error, so a
new scenario file can never be silently skipped. Each file runs as its own
subprocess (crash isolation); inside a file, every scenario class gets its
own fresh cached stack (framework/testbase.py).

SEQUENTIAL BY DESIGN: the stacks use fixed ports (see framework/util.py's
registry), so do not add a --jobs flag without port-namespacing work first.

Run:  python test_runner.py [--filter SUBSTR] [--keep] [--rebuild-cache]
                            [--skip-build]
"""

import argparse
import os
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
TESTS_DIR = os.path.join(HERE, "tests")
sys.path.insert(0, HERE)

from framework import stack  # noqa: E402

# Explicit run order: the fast sanity files first (a broken framework or a
# missing binary fails in seconds, not after an hour), then the suites,
# heaviest last.
TEST_LIST = [
    "framework_selftest.py",
    "multimachine.py",
    "swap_v1.py",
    "swap_v1_rescue.py",
    "swap_v2_adaptor.py",
    "nodeless.py",
    "follow.py",
]


def crosscheck():
    on_disk = {f for f in os.listdir(TESTS_DIR)
               if f.endswith(".py") and not f.startswith("_")}
    listed = set(TEST_LIST)
    unlisted = sorted(on_disk - listed)
    ghosts = sorted(listed - on_disk)
    if unlisted:
        sys.exit(f"test_runner: tests/ contains files missing from TEST_LIST "
                 f"(add them so they cannot be silently skipped): {unlisted}")
    if ghosts:
        sys.exit(f"test_runner: TEST_LIST names files that do not exist: {ghosts}")


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--filter", default="", metavar="SUBSTR",
                    help="only run test files whose name contains SUBSTR")
    ap.add_argument("--keep", action="store_true",
                    help="keep every scenario workdir (default: on failure only)")
    ap.add_argument("--rebuild-cache", action="store_true",
                    help="drop the funded-datadir cache before running")
    ap.add_argument("--skip-build", action="store_true",
                    help="skip the one-time cargo build (binaries already fresh)")
    args = ap.parse_args()

    crosscheck()
    if args.rebuild_cache:
        stack.drop_node_cache()
    if not args.skip_build:
        stack.build_workspace()

    selected = [f for f in TEST_LIST if args.filter in f]
    if not selected:
        sys.exit(f"test_runner: no test file matches --filter {args.filter!r}")

    results = []
    for name in selected:
        cmd = [sys.executable, os.path.join(TESTS_DIR, name), "--no-build"]
        if args.keep:
            cmd.append("--keep")
        print(f"\n================ {name} ================", flush=True)
        started = time.time()
        code = subprocess.run(cmd, cwd=HERE).returncode
        results.append((name, code, time.time() - started))

    print("\n================ summary ================")
    failed = 0
    for name, code, secs in results:
        status = "PASS" if code == 0 else f"FAIL (exit {code})"
        failed += code != 0
        print(f"  {name:<28} {status:<16} {secs:7.0f}s")
    if failed:
        print(f"\nRED: {failed}/{len(results)} test file(s) failing.", file=sys.stderr)
        sys.exit(1)
    print(f"\nGREEN: all {len(results)} test file(s) pass.")


if __name__ == "__main__":
    main()
