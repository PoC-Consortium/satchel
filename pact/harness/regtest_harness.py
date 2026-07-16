#!/usr/bin/env python3
"""Compatibility shim — the regtest harness moved into the framework package
(Phase 2, docs/TEST_FRAMEWORK_PLAN.md): nodes/electrs/Harness live in
framework/node.py, binary resolution in framework/binaries.py. Everything is
re-exported here so the pre-Phase-3 playground drivers keep importing from
this module unchanged.

Smoke test (no Pact involved):  python regtest_harness.py --smoke
"""

import os
import sys

from framework.binaries import (  # noqa: F401
    EXE,
    find_btc_bitcoind,
    find_btc_electrs,
    find_electrs,
    find_litecoind,
    find_pocx_bitcoind,
)
from framework.node import (  # noqa: F401
    BTC_ELECTRS_ELECTRUM_PORT,
    BTC_ELECTRS_MONITORING_PORT,
    BTC_REGTEST_GENESIS,
    BTC_REST_RPC_PORT,
    BTC_RPC_PORT,
    ELECTRS_ELECTRUM_PORT,
    ELECTRS_MONITORING_PORT,
    LTC_REGTEST_GENESIS,
    LTC_RPC_PORT,
    POCX_REGTEST_GENESIS,
    POCX_REST_RPC_PORT,
    POCX_RPC_PORT,
    ElectrsServer,
    Harness,
    Node,
    RpcError,
    smoke,
)

HERE = os.path.dirname(os.path.abspath(__file__))

if __name__ == "__main__":
    if "--smoke" in sys.argv:
        smoke()
    else:
        print(__doc__)
