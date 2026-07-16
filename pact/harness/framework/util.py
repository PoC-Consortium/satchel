"""Shared helpers: the ONE cookie-auth pactd JSON-RPC client, wait_until, and
the MAINNET-SAFE teardown port registry (TEST_FRAMEWORK_PLAN §2.4/Appendix B).
"""

import base64
import json
import time
import urllib.error
import urllib.request

# ---------------------------------------------------------------------------
# pactd JSON-RPC (cookie auth) — previously hand-rolled in 5 places.

def read_cookie(path):
    with open(path, encoding="utf-8") as fh:
        return fh.read().strip()


def pactd_rpc(url, method, *params, cookie=None, timeout=120, rpc_id="h"):
    """One JSON-RPC call to a pactd. `cookie` is the raw .cookie content
    (Basic auth); None sends no Authorization header (the 401 test path).
    Raises RuntimeError on HTTP or JSON-RPC errors."""
    body = {"jsonrpc": "2.0", "id": rpc_id, "method": method, "params": list(params)}
    req = urllib.request.Request(url, data=json.dumps(body).encode(), method="POST")
    req.add_header("Content-Type", "application/json")
    if cookie:
        req.add_header(
            "Authorization", f"Basic {base64.b64encode(cookie.encode()).decode()}")
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            data = json.loads(resp.read())
    except urllib.error.HTTPError as e:
        raise RuntimeError(f"pactd {method}: HTTP {e.code}: {e.read().decode()}") from e
    if data.get("error"):
        raise RuntimeError(f"pactd {method}: {data['error']['message']}")
    return data["result"]


def pactd_rpc_or_none(url, method, *params, cookie_path=None, timeout=10):
    """Best-effort variant for observer/playground probes: cookie read fresh
    from `cookie_path` each call (the daemon may not be up yet), any failure
    — missing cookie, connection refused, RPC error — returns None so a
    driver loop never crashes on a not-yet-up daemon."""
    try:
        cookie = read_cookie(cookie_path) if cookie_path else None
        return pactd_rpc(url, method, *params, cookie=cookie, timeout=timeout)
    except Exception:  # noqa: BLE001 — best-effort by contract
        return None


# ---------------------------------------------------------------------------
# Polling.

def wait_until(predicate, *, timeout=60, poll=0.5, what="condition"):
    """Poll `predicate` until truthy; raise TimeoutError after `timeout`s.
    Returns the predicate's (truthy) result."""
    deadline = time.time() + timeout
    while time.time() < deadline:
        result = predicate()
        if result:
            return result
        time.sleep(poll)
    raise TimeoutError(f"timed out after {timeout}s waiting for {what}")


# ---------------------------------------------------------------------------
# The mainnet-safe teardown port registry.
#
# HARD INVARIANT (memory `no-kill-nodes-by-name`, 15 Jun 2026 incident):
# teardown is PID/port-only, NEVER by process name, and the kill list must
# NEVER contain the user's live mainnet/testnet pactd ports. This registry is
# the single source of teardown targets; the import-time check below makes a
# forbidden port a crash at import, not a dead mainnet node at runtime.

MAINNET_PACTD_PORT = 9737
TESTNET_PACTD_PORT = 9738
FORBIDDEN_TEARDOWN_PORTS = frozenset({MAINNET_PACTD_PORT, TESTNET_PACTD_PORT})

TEARDOWN_PORT_GROUPS = {
    "nodes": (19443, 19543, 19643),               # pocx / btc / ltc RPC
    "nodes_rest": (18443, 18332),                 # bindex-hardcoded REST variants
    "electrs": tuple(range(19750, 19758)),        # PoCX electrs + fleet (elec/mon ×2)
    "btc_electrs": (19760, 19761),
    "pactd_range": tuple(range(19737, 19750)),    # bots + harness parties (see plan
                                                  # Appendix B: capped in Phase 2)
    "relay_e2e": (19791,),
    "relay_playground": (19788,),
    "corkboard": (19790,),
    "satchel_managed": (9739, 9740),              # regtest managed + observer
    "viewer": (9747,),                            # mainnet-isolated viewer pactd
    "vite": (5173,),
    "multimachine": (19801, 19802, 19803),
}


def all_teardown_ports():
    """Every port a full teardown may kill by. Structurally mainnet-safe."""
    ports = sorted({p for group in TEARDOWN_PORT_GROUPS.values() for p in group})
    illegal = set(ports) & FORBIDDEN_TEARDOWN_PORTS
    if illegal:
        raise AssertionError(
            f"teardown registry contains FORBIDDEN mainnet/testnet ports: {illegal}")
    return ports


# Import-time structural check: a bad edit to the registry fails every run
# immediately instead of killing a live daemon on the next teardown.
all_teardown_ports()


if __name__ == "__main__":
    ports = all_teardown_ports()
    print(f"{len(ports)} teardown ports, mainnet-safe "
          f"(9737/9738 excluded): {ports}")
