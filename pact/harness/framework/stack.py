"""Stack composition: the workspace build, shared data-file paths, and the
per-scenario datadir cache (TEST_FRAMEWORK_PLAN §2.3). The spec-driven stack
builder (nodes+electrs+board+relay+pactds from a spec) lands with Phase 3's
playground work.
"""

import json
import os
import shutil
import subprocess
import time

from framework import binaries

# The shipped coin-templates file (consensus params for file-added coins like
# ltc). A Pactd that trades a file coin passes this so its registry knows the
# coin's genesis + HRP.
COINS_TOML = os.path.join(binaries.REPO_DIR, "satchel", "coins.toml")

# The on-demand datadir cache (gitignored). Per-scenario isolation would
# re-mine 110 funding blocks per chain per scenario; instead the funded
# datadirs are built ONCE here and copied into each scenario's tmpdir
# (Bitcoin Core's cache mechanism). Invalidation: the node-binary fingerprint
# below; force with drop_node_cache() / the runner's --rebuild-cache.
CACHE_DIR = os.path.join(binaries.HARNESS_DIR, "cache")


def build_workspace():
    print("[e2e] building pact workspace ...")
    subprocess.run(["cargo", "build"], cwd=binaries.PACT_DIR, check=True)
    print("[e2e] building corkboard ...")
    subprocess.run(["cargo", "build"], cwd=binaries.CORKBOARD_DIR, check=True)
    for path in (binaries.pact_cli(), binaries.pactd(), binaries.corkboard()):
        assert os.path.exists(path), f"missing {path}"


def _cache_fingerprint():
    fp = {}
    for key, path in (("pocx", binaries.find_pocx_bitcoind()),
                      ("btc", binaries.find_btc_bitcoind())):
        st = os.stat(path)
        fp[key] = [os.path.abspath(path), st.st_size, int(st.st_mtime)]
    return fp


def drop_node_cache():
    shutil.rmtree(CACHE_DIR, ignore_errors=True)


def ensure_node_cache():
    """Build (once) the funded pocx+btc regtest datadirs; returns
    ({"pocx": dir, "btc": dir}, {"pocx": tip_time, "btc": tip_time}).
    Content matches the uncached Harness bringup: the standard wallet layout
    (alice_pocx funded / bob_pocx empty; bob_btc funded / alice_btc empty)
    with 110 blocks mined under mocktime. The LTC node is never cached
    (rarely used; its callers own its funding).

    The tip times matter: PoCX forging AUTO-ADVANCES the mock clock while
    mining, so the cached pocx tip sits HOURS ahead of the build wall clock —
    a plain restart trips Core's block-from-the-future init check. The
    Harness therefore restarts cached nodes with -mocktime >= tip (recorded
    here), then mines a 12-block runway at max(now, tips+1) so scenarios read
    a current tip time/MTP regardless of cache age."""
    from framework.node import (
        BTC_REGTEST_GENESIS,
        BTC_RPC_PORT,
        POCX_REGTEST_GENESIS,
        POCX_RPC_PORT,
        Node,
    )
    marker = os.path.join(CACHE_DIR, "fingerprint.json")
    dirs = {key: os.path.join(CACHE_DIR, key) for key in ("pocx", "btc")}
    want_fp = _cache_fingerprint()
    if os.path.exists(marker):
        try:
            with open(marker, encoding="utf-8") as fh:
                have = json.load(fh)
        except (OSError, ValueError):
            have = {}
        if (have.get("fingerprint") == want_fp and have.get("tips")
                and all(os.path.isdir(d) for d in dirs.values())):
            return dirs, have["tips"]
        print("[cache] node binaries changed — rebuilding the datadir cache")
    drop_node_cache()
    os.makedirs(CACHE_DIR)
    print("[cache] building funded regtest datadirs (one-time) ...")
    now = int(time.time())
    tips = {}
    specs = (
        ("pocx", binaries.find_pocx_bitcoind(), POCX_RPC_PORT,
         POCX_REGTEST_GENESIS, ("alice_pocx", "bob_pocx")),
        ("btc", binaries.find_btc_bitcoind(), BTC_RPC_PORT,
         BTC_REGTEST_GENESIS, ("bob_btc", "alice_btc")),
    )
    for key, binary, port, genesis, (funded, empty) in specs:
        node = Node(key, binary, dirs[key], port, genesis)
        node.start()
        try:
            node.set_mocktime(now)
            node.create_wallet(funded)
            node.create_wallet(empty)
            # 110 blocks: >100 for coinbase maturity, headroom for fees.
            node.generate(110, funded)
            info = node.rpc("getblockchaininfo")
            tips[key] = int(info.get("time", info["mediantime"]))
        finally:
            node.stop()
    with open(marker, "w", encoding="utf-8") as fh:
        json.dump({"fingerprint": want_fp, "tips": tips}, fh)
    print(f"[cache] datadir cache ready at {CACHE_DIR}")
    return dirs, tips
