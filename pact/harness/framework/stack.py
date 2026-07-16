"""Stack composition. Phase 1 scope: the workspace build + shared data-file
paths. The spec-driven stack builder (nodes+electrs+board+relay+pactds from a
spec) and the per-scenario datadir cache land in Phase 2
(TEST_FRAMEWORK_PLAN §2.1/§2.3).
"""

import os
import subprocess

from framework import binaries

# The shipped coin-templates file (consensus params for file-added coins like
# ltc). A Pactd that trades a file coin passes this so its registry knows the
# coin's genesis + HRP.
COINS_TOML = os.path.join(binaries.REPO_DIR, "satchel", "coins.toml")


def build_workspace():
    print("[e2e] building pact workspace ...")
    subprocess.run(["cargo", "build"], cwd=binaries.PACT_DIR, check=True)
    print("[e2e] building corkboard ...")
    subprocess.run(["cargo", "build"], cwd=binaries.CORKBOARD_DIR, check=True)
    for path in (binaries.pact_cli(), binaries.pactd(), binaries.corkboard()):
        assert os.path.exists(path), f"missing {path}"
