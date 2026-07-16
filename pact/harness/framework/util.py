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


# ---------------------------------------------------------------------------
# Shared swap-flow helpers + canonical scenario amounts (moved verbatim from
# test_swap_e2e.py in the Phase 2 split; used by the v1/rescue/follow suites).

import os  # noqa: E402

GIVE_POCX = "50.0"      # Alice gives 50 POCX
GET_BTC = "0.001"       # ... for 0.001 BTC from Bob
FEE_SLACK = 0.01        # generous bound for redeem/refund fees


def swap_id_from(message_file):
    with open(message_file, encoding="utf-8") as fh:
        return json.load(fh)["swap_id"]


def load_msg(message_file):
    with open(message_file, encoding="utf-8") as fh:
        return json.load(fh)


def save_msg(message_file, envelope):
    with open(message_file, "w", encoding="utf-8") as fh:
        json.dump(envelope, fh, indent=2)


def outpoint_from(funded_message_file):
    body = load_msg(funded_message_file)["body"]
    return body["txid"], body["vout"]


def assert_htlc_spent(node, funded_message_file, what):
    txid, vout = outpoint_from(funded_message_file)
    utxo = node.rpc("gettxout", txid, vout, True)
    assert utxo is None, f"{what} HTLC {txid}:{vout} is still unspent"


def expect_fail(party, what, *args):
    try:
        party.cli(*args)
    except RuntimeError as exc:
        print(f"[e2e] correctly rejected: {what}")
        return str(exc)
    raise AssertionError(f"{what} should have been rejected but succeeded")


def balances(h):
    return {
        "alice_pocx": float(h.pocx.rpc("getbalance", wallet="alice_pocx")),
        "bob_pocx": float(h.pocx.rpc("getbalance", wallet="bob_pocx")),
        "alice_btc": float(h.btc.rpc("getbalance", wallet="alice_btc")),
        "bob_btc": float(h.btc.rpc("getbalance", wallet="bob_btc")),
    }


def msg(workdir, name):
    return os.path.join(workdir, name)


def regtest_timelocks(h):
    now = int(h.pocx.rpc("getblockchaininfo")["time"])
    return now + 2 * 3600, now + 4 * 3600   # (t2, t1)


def drive_until(party, cond, tries=20):
    """Tick `party` repeatedly until cond(events) holds; returns those events.
    Mining is the caller's job — this only drives the scheduler."""
    for _ in range(tries):
        events = party.tick()
        if cond(events):
            return events
    raise AssertionError(f"{party.name}: condition not met after {tries} ticks")


def handshake_and_fund(h, alice, bob, prefix):
    """Shared plumbing: offer/accept handshake, fund both legs. Returns
    (swap_id, funded_a_file, funded_b_file)."""
    t2, t1 = regtest_timelocks(h)
    m_init = msg(h.workdir, f"{prefix}_init.json")
    m_accept = msg(h.workdir, f"{prefix}_accept.json")
    m_funded_a = msg(h.workdir, f"{prefix}_funded_a.json")
    m_funded_b = msg(h.workdir, f"{prefix}_funded_b.json")

    alice.cli("offer", "--give", f"btcx:{GIVE_POCX}", "--get", f"btc:{GET_BTC}",
              "--t1", str(t1), "--t2", str(t2), "--out", m_init)
    sid = swap_id_from(m_init)
    bob.cli("accept", "--in", m_init, "--out", m_accept)
    alice.cli("recv", "--in", m_accept)
    alice.cli("fund", "--swap", sid, "--out", m_funded_a)
    h.pocx.generate(1, "alice_pocx")
    bob.cli("recv", "--in", m_funded_a)
    bob.cli("fund", "--swap", sid, "--out", m_funded_b)
    h.btc.generate(1, "bob_btc")
    return sid, m_funded_a, m_funded_b




if __name__ == "__main__":
    _ports = all_teardown_ports()
    print(f"{len(_ports)} teardown ports, mainnet-safe "
          f"(9737/9738 excluded): {_ports}")
