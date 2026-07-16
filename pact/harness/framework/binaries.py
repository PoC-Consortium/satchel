"""The ONE resolver for every binary the harness runs (TEST_FRAMEWORK_PLAN §2.1).

Two categories, one home:

* External binaries (nodes, electrs, the Nostr relay) live in the single
  shared bin dir `pact/harness/bin` (gitignored; override the dir itself with
  PACT_HARNESS_BIN). Each also keeps its dedicated env-var override and the
  legacy fallbacks it always had, so nothing existing changes behavior:

    PoCX node      POCX_BITCOIND         bin/pocx-bitcoind   ../../../bitcoin-pocx build
    Bitcoin node   BTC_BITCOIND          bin/btc-bitcoind    `bitcoind` on PATH
    Litecoin node  LITECOIND             bin/litecoind or bin/ltc-bitcoind
                                                             `litecoind` on PATH
    PoCX electrs   PACT_ELECTRS_BIN      bin/electrs
    BTC electrs    PACT_BTC_ELECTRS_BIN  bin/btc-electrs
    Nostr relay    PACT_NOSTR_RELAY_BIN  bin/nostr-rs-relay  (default path only —
                                         callers keep their own exists check +
                                         PACT_NOSTR_RELAY_CMD escape hatch)

* Workspace-built binaries (pactd, pact-cli, corkboard) resolve to their cargo
  debug outputs; `build_workspace()`-style callers build them first.
"""

import os
import platform
import shutil

EXE = ".exe" if platform.system() == "Windows" else ""

# pact/harness/framework -> pact/harness -> pact -> repo root
HARNESS_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PACT_DIR = os.path.normpath(os.path.join(HARNESS_DIR, ".."))
REPO_DIR = os.path.normpath(os.path.join(PACT_DIR, ".."))
CORKBOARD_DIR = os.path.join(REPO_DIR, "corkboard")


def bin_dir():
    """The single shared external-binaries dir (PACT_HARNESS_BIN overrides)."""
    return os.environ.get("PACT_HARNESS_BIN") or os.path.join(HARNESS_DIR, "bin")


def _find(env_var, names, fallbacks, missing_msg):
    candidates = [os.environ.get(env_var)]
    candidates += [os.path.join(bin_dir(), name + EXE) for name in names]
    candidates += list(fallbacks)
    for candidate in candidates:
        if candidate and os.path.exists(candidate):
            return candidate
    raise FileNotFoundError(missing_msg)


def find_pocx_bitcoind():
    return _find(
        "POCX_BITCOIND", ["pocx-bitcoind"],
        [os.path.normpath(os.path.join(
            HARNESS_DIR, "..", "..", "..",
            "bitcoin-pocx", "bitcoin", "build", "bin", "bitcoind" + EXE))],
        "PoCX node binary not found. Copy the installed daemon to "
        "harness/bin/pocx-bitcoind" + EXE + " or set POCX_BITCOIND.")


def find_btc_bitcoind():
    return _find(
        "BTC_BITCOIND", ["btc-bitcoind"], [shutil.which("bitcoind")],
        "Bitcoin Core binary not found. Copy the installed daemon to "
        "harness/bin/btc-bitcoind" + EXE + " or set BTC_BITCOIND.")


def find_litecoind():
    return _find(
        "LITECOIND", ["litecoind", "ltc-bitcoind"], [shutil.which("litecoind")],
        "Litecoin Core binary not found. Copy the installed daemon to "
        "harness/bin/litecoind" + EXE + " or set LITECOIND.")


def find_electrs():
    return _find(
        "PACT_ELECTRS_BIN", ["electrs"], [],
        "electrs binary not found. Copy the PoCX-patched electrs to "
        "harness/bin/electrs" + EXE + " or set PACT_ELECTRS_BIN.")


def find_btc_electrs():
    return _find(
        "PACT_BTC_ELECTRS_BIN", ["btc-electrs"], [],
        "vanilla (upstream) electrs binary not found. Copy it to "
        "harness/bin/btc-electrs" + EXE + " or set PACT_BTC_ELECTRS_BIN.")


def nostr_relay_default():
    """Default nostr-rs-relay path (env override or bin dir). No exists check:
    the NostrRelay callers keep their own check + error text, and the
    PACT_NOSTR_RELAY_CMD template bypasses the binary entirely."""
    return os.environ.get("PACT_NOSTR_RELAY_BIN") or \
        os.path.join(bin_dir(), "nostr-rs-relay" + EXE)


def pactd():
    return os.path.join(PACT_DIR, "target", "debug", "pactd" + EXE)


def pact_cli():
    return os.path.join(PACT_DIR, "target", "debug", "pact-cli" + EXE)


def corkboard():
    return os.path.join(CORKBOARD_DIR, "target", "debug", "corkboard" + EXE)
