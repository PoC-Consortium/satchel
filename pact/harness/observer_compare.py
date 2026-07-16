#!/usr/bin/env python3
"""Observer/main progress comparator (read-only oracle for the observer playground).

Both the MAIN Satchel (:9739) and the OBSERVER Satchel (:9740) run the SAME seed,
so they hold the SAME swap under the SAME swap_id -- Main drives it, the observer
follows. Post-#174/#175 the follower MIRRORS the owner: identical narrate-driving
state and identical progress line (watching / confs / needed), modulo the few
seconds the observer trails while it discovers Main's snapshot.

So a per-tick diff of the two ports is a precise automated oracle: any divergence
that PERSISTS past the discovery-lag window is a real bug -- exactly the classes
we've been eyeballing (wrong voice "their vs your lock", off-by-one depth, a row
that blinks out or lingers). This polls both ports read-only (no RPC mutates
anything), prints a MISMATCH line the moment a divergence sticks, and writes a
full side-by-side timeline to .playground/observer-compare.log for offline review.

Run it alongside a live playground:  python observer_compare.py
Ctrl-C to stop. It never touches the windows -- pure observation.
"""
import os
import sys
import time

sys.stdout.reconfigure(encoding="utf-8", line_buffering=True)

from framework.util import pactd_rpc_or_none  # noqa: E402

MAIN, OBS = 9739, 9740
POLL_SECS = 1.0
# The two daemons tick INDEPENDENTLY, so a few seconds' skew at every transition
# is inherent, not a bug. Only flag a divergence that outlives that jitter by a
# wide margin (a real structural bug persists far longer than one tick cycle).
PERSIST = 20
# confs are read from chain independently by each pactd, so a 1-block skew is
# expected; only a wider or watching/needed gap is a real divergence.
CONFS_TOL = 1
# Owner keeps a finished swap in history; the follower PURGES it. So a swap
# present on only one side is a real signal ONLY while it is still ACTIVE — a
# terminal-state straggler is the expected history-vs-purge asymmetry.
TERMINAL = {"completed", "refunded", "aborted"}

LOG = os.path.join(os.path.dirname(__file__), "..", "..", ".playground", "observer-compare.log")


def rpc(port, method, *params, timeout=10):
    base = "org.pocx.satchel" if port == MAIN else "org.pocx.satchel-observer"
    cookie_path = os.path.join(
        os.environ["LOCALAPPDATA"], base, "regtest", "pactd", ".cookie"
    )
    return pactd_rpc_or_none(f"http://127.0.0.1:{port}/", method, *params,
                             cookie_path=cookie_path, timeout=timeout)


def progress_map(port):
    """swap_id -> {watching, confs, needed} from swapprogress (map OR list shape)."""
    p = rpc(port, "swapprogress")
    out = {}
    if isinstance(p, dict):
        items = p.items()
    elif isinstance(p, list):
        items = [(x.get("swap_id"), x) for x in p if isinstance(x, dict)]
    else:
        return out
    for sid, pr in items:
        if isinstance(pr, dict):
            out[sid] = {k: pr.get(k) for k in ("watching", "confs", "needed")}
    return out


def view(port):
    """swap_id -> merged {state, role, source, kind, watching, confs, needed}."""
    prog = progress_map(port)
    out = {}
    for kind, method in (("v1", "listswaps"), ("v2", "listadaptorswaps")):
        for s in rpc(port, method) or []:
            if not isinstance(s, dict):
                continue
            sid = s.get("swap_id")
            row = {
                "state": s.get("state"),
                "role": s.get("role"),
                "source": s.get("source"),
                "kind": kind,
            }
            row.update(prog.get(sid, {"watching": None, "confs": None, "needed": None}))
            out[sid] = row
    return out


def fmt(row):
    if row is None:
        return "(absent)"
    return (
        f"{row.get('kind')}/{row.get('role')}/{row.get('state')} "
        f"[{row.get('watching')} {row.get('confs')}/{row.get('needed')}] src={row.get('source')}"
    )


def divergences(main, obs):
    """List of (swap_id, reason, main_row, obs_row) for swaps that DIVERGE now.

    source differs by design (local vs foreign) and is never a divergence. confs
    within CONFS_TOL is tolerated (independent tip reads)."""
    out = []
    for sid in set(main) | set(obs):
        m, o = main.get(sid), obs.get(sid)
        if (m is None) != (o is None):
            # A one-sided swap is a real signal only while the present side is
            # still ACTIVE (discovery lag / stuck follower). A terminal straggler
            # is the owner-keeps-history vs follower-purges asymmetry — expected.
            present = m or o
            if present.get("state") not in TERMINAL:
                out.append((sid, "present on one side only", m, o))
            continue
        # We compare the PROGRESS LINE (what the user reads), not the internal
        # state label: the owner's driving state machine and the follower's
        # chain-derived display-state legitimately differ by a phase mid-swap
        # (e.g. a taker's transient funded_a) while the progress line agrees.
        reasons = []
        if m.get("watching") != o.get("watching"):
            reasons.append("watching")
        if m.get("needed") != o.get("needed"):
            reasons.append("needed")
        mc, oc = m.get("confs"), o.get("confs")
        if isinstance(mc, int) and isinstance(oc, int) and abs(mc - oc) > CONFS_TOL:
            reasons.append("confs")
        if reasons:
            out.append((sid, "+".join(reasons), m, o))
    return out


def main():
    os.makedirs(os.path.dirname(LOG), exist_ok=True)
    log = open(LOG, "w", encoding="utf-8", buffering=1)
    print("[cmp] comparing MAIN :9739 vs OBSERVER :9740 (read-only). MISMATCH = persistent divergence.")
    log.write("# ts | swap_id | reason | MAIN | OBSERVER\n")

    streak = {}      # swap_id -> consecutive divergent polls
    flagged = set()  # swap_ids already reported as MISMATCH (report once per episode)
    last_snap = {}   # swap_id -> last printed (side, fmt) so the timeline logs only CHANGES

    while True:
        main_v, obs_v = view(MAIN), view(OBS)
        ts = time.strftime("%H:%M:%S")

        # Timeline: log any swap whose either-side view CHANGED this poll.
        for sid in sorted(set(main_v) | set(obs_v)):
            snap = (fmt(main_v.get(sid)), fmt(obs_v.get(sid)))
            if last_snap.get(sid) != snap:
                last_snap[sid] = snap
                log.write(f"{ts} | {sid[:12]} | MAIN={snap[0]} | OBS={snap[1]}\n")

        divs = {sid: (reason, m, o) for sid, reason, m, o in divergences(main_v, obs_v)}
        # Update streaks; flag when a divergence has persisted past the lag window.
        for sid, (reason, m, o) in divs.items():
            streak[sid] = streak.get(sid, 0) + 1
            if streak[sid] == PERSIST and sid not in flagged:
                flagged.add(sid)
                print(
                    f"[cmp] MISMATCH {sid[:12]} ({reason}, {streak[sid]}x)\n"
                    f"        MAIN     = {fmt(m)}\n"
                    f"        OBSERVER = {fmt(o)}"
                )
        # Clear streaks/flags for swaps that converged (or vanished).
        for sid in list(streak):
            if sid not in divs:
                if sid in flagged:
                    print(f"[cmp] resolved  {sid[:12]} (converged/purged)")
                streak.pop(sid, None)
                flagged.discard(sid)

        time.sleep(POLL_SECS)


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print("\n[cmp] stopped.")
