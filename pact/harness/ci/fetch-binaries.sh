#!/usr/bin/env bash
# Provision the harness's external binaries for CI (TEST_FRAMEWORK_PLAN Phase 5).
#
# Fills pact/harness/bin/ (the binaries.py home) with the six external
# binaries the e2e suite needs, on Linux x86_64:
#
#   btc-bitcoind    Bitcoin Core (official release download, sha256-pinned)
#   litecoind       Litecoin Core (official release download, sha256-pinned)
#   electrs         PoCX-patched electrs (PoC-Consortium/electrs-btcx release)
#   btc-electrs     vanilla-bitcoin electrs (same release, electrs-bitcoin flavor)
#   nostr-rs-relay  built from git (no upstream binary releases past 0.8.x)
#   pocx-bitcoind   built from PoC-Consortium/bitcoin-pocx (no releases exist)
#
# ci.yml caches pact/harness/bin keyed on this file's hash, so bumping any
# pin below invalidates the cache and triggers a rebuild. The two source
# builds (relay ~5 min, pocx node ~40 min) only run on that cache miss.
#
# Idempotent per binary: anything already present in bin/ is skipped, so a
# partially-saved cache (e.g. from a cancelled run) self-heals instead of
# poisoning every later run — ci.yml therefore runs this unconditionally.
#
# Build deps (installed by ci.yml): build-essential cmake pkgconf libevent-dev
# libboost-dev libsqlite3-dev libssl-dev protobuf-compiler
#
# Windows devs: this script is CI/Linux-only — keep copying binaries into
# pact/harness/bin manually as per pact/harness/README.md.
set -euo pipefail

# ---- pins ------------------------------------------------------------------
BITCOIN_VER=31.0
BITCOIN_SHA256=d3e4c58a35b1d0a97a457462c94f55501ad167c660c245cb1ffa565641c65074

LITECOIN_VER=0.21.5.5
LITECOIN_SHA256=623410d4f2695a68aa71332ae0672fee19276f41c1c63a531f97e24a50edde14

ELECTRS_TAG=v0.11.1-btcx.1
ELECTRS_BTCX_SHA256=d1ae81c2564f5f4bf42bcca8b425877098d9ec0d45557ec76e19fae659ad42cb
ELECTRS_BITCOIN_SHA256=625fa69a04839f1a2328a77ca8f9e4d3392dabfaadcec1170967410b6fcf8ba6

# master @ Cargo.toml version 0.10.0 (matches the local dev binaries; the
# author stopped tagging after 0.9.0)
NOSTR_RELAY_REV=b5c1f642e4f4c3b9c54f5d18d66f4c53642076b4

# PoC-Consortium/bitcoin-pocx master (v31.0.0rc1 era)
POCX_REV=441e8527359ec6552f1627e02f595c6346af5c5d
# ----------------------------------------------------------------------------

if [[ "$(uname -s)-$(uname -m)" != "Linux-x86_64" ]]; then
    echo "fetch-binaries.sh only supports Linux x86_64 (CI)" >&2
    exit 1
fi

HARNESS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_DIR="${PACT_HARNESS_BIN:-$HARNESS_DIR/bin}"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
mkdir -p "$BIN_DIR"

fetch() { # url sha256 outfile
    curl -fsSL --retry 3 -o "$3" "$1"
    echo "$2  $3" | sha256sum -c -
}

have() { # bin/ name
    if [[ -x "$BIN_DIR/$1" ]]; then echo "== $1: already present, skipping"; return 0; fi
    return 1
}

have btc-bitcoind || {
    echo "== Bitcoin Core $BITCOIN_VER"
    fetch "https://bitcoincore.org/bin/bitcoin-core-$BITCOIN_VER/bitcoin-$BITCOIN_VER-x86_64-linux-gnu.tar.gz" \
          "$BITCOIN_SHA256" "$WORK/bitcoin.tar.gz"
    tar -xzf "$WORK/bitcoin.tar.gz" -C "$WORK" "bitcoin-$BITCOIN_VER/bin/bitcoind"
    install -m 755 "$WORK/bitcoin-$BITCOIN_VER/bin/bitcoind" "$BIN_DIR/btc-bitcoind"
}

have litecoind || {
    echo "== Litecoin Core $LITECOIN_VER"
    fetch "https://download.litecoin.org/litecoin-$LITECOIN_VER/linux/litecoin-$LITECOIN_VER-x86_64-linux-gnu.tar.gz" \
          "$LITECOIN_SHA256" "$WORK/litecoin.tar.gz"
    tar -xzf "$WORK/litecoin.tar.gz" -C "$WORK" "litecoin-$LITECOIN_VER/bin/litecoind"
    install -m 755 "$WORK/litecoin-$LITECOIN_VER/bin/litecoind" "$BIN_DIR/litecoind"
}

ELECTRS_BASE="https://github.com/PoC-Consortium/electrs-btcx/releases/download/$ELECTRS_TAG"
have electrs || {
    echo "== electrs ($ELECTRS_TAG, btcx flavor)"
    fetch "$ELECTRS_BASE/electrs-btcx-$ELECTRS_TAG-x86_64-linux-gnu.tar.gz" \
          "$ELECTRS_BTCX_SHA256" "$WORK/electrs-btcx.tar.gz"
    tar -xzf "$WORK/electrs-btcx.tar.gz" -C "$WORK" electrs
    install -m 755 "$WORK/electrs" "$BIN_DIR/electrs"
    rm "$WORK/electrs"
}

have btc-electrs || {
    echo "== electrs ($ELECTRS_TAG, vanilla-bitcoin flavor)"
    fetch "$ELECTRS_BASE/electrs-bitcoin-$ELECTRS_TAG-x86_64-linux-gnu.tar.gz" \
          "$ELECTRS_BITCOIN_SHA256" "$WORK/electrs-bitcoin.tar.gz"
    tar -xzf "$WORK/electrs-bitcoin.tar.gz" -C "$WORK" electrs
    install -m 755 "$WORK/electrs" "$BIN_DIR/btc-electrs"
}

have nostr-rs-relay || {
    echo "== nostr-rs-relay @ $NOSTR_RELAY_REV (source build)"
    cargo install --locked --git https://github.com/scsibug/nostr-rs-relay \
          --rev "$NOSTR_RELAY_REV" --root "$WORK/nrr" nostr-rs-relay
    install -m 755 "$WORK/nrr/bin/nostr-rs-relay" "$BIN_DIR/nostr-rs-relay"
}

have pocx-bitcoind || {
    echo "== pocx-bitcoind @ $POCX_REV (source build)"
    mkdir "$WORK/pocx" && cd "$WORK/pocx"
    git init -q
    git remote add origin https://github.com/PoC-Consortium/bitcoin-pocx
    git fetch -q --depth 1 origin "$POCX_REV"
    git checkout -q FETCH_HEAD
    cd bitcoin
    cmake -B build \
          -DCMAKE_BUILD_TYPE=Release \
          -DBUILD_TESTS=OFF -DBUILD_TX=OFF -DBUILD_UTIL=OFF \
          -DBUILD_WALLET_TOOL=OFF -DBUILD_CLI=OFF -DBUILD_BITCOIN_BIN=OFF \
          -DENABLE_IPC=OFF -DWITH_ZMQ=OFF -DWITH_CCACHE=OFF
    cmake --build build -j"$(nproc)" --target bitcoind
    install -m 755 build/bin/bitcoind "$BIN_DIR/pocx-bitcoind"
}

echo "== done"
ls -la "$BIN_DIR"
