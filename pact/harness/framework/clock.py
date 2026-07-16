"""The playground mining/mocktime model (TEST_FRAMEWORK_PLAN §2.1) — ONE home
for the monotonic-clock rule that was copy-pasted across five driver loops.

Rules encoded here (learned the hard way):
  * The mock clock must NEVER move backwards: PoCX forging auto-advances it
    beyond wall pace on its own, so every tick re-reads the tips and clamps.
  * Per-tick mining is BEST-EFFORT: a transient node error (e.g. a momentary
    `bad-txns-vin-empty` on CreateNewBlock) must not crash the driver — that
    would unwind the Harness and tear every node down under a live Satchel
    (the spurious coin-setup gate). Failures are logged and retried next tick.
  * Per-coin cadence: chains mine on their own intervals (mainnet ratios,
    ~20x) while every chain's clock advances every tick, so timelocks keep
    moving and there are several scheduler ticks per block — mainnet-like,
    not instant finality.
"""

import time


def chain_time(node):
    """Tip block time; litecoind (an older Core fork) lacks the "time" field
    in getblockchaininfo, so fall back to mediantime."""
    info = node.rpc("getblockchaininfo")
    return int(info.get("time", info["mediantime"]))


class PacedMiner:
    """Advance mocktime with wall time and mine each chain on its own cadence.

    legs: [(node, mining_wallet, coin_id)]; block_secs: {coin_id: seconds};
    base_secs: the tick granularity (= the fastest chain's interval). The
    caller owns the sleep loop: sleep(base_secs) then tick()."""

    def __init__(self, legs, block_secs, base_secs, tag="pg"):
        self.legs = list(legs)
        self.block_secs = dict(block_secs)
        self.base_secs = base_secs
        self.tag = tag
        self.elapsed = 0
        self.start_wall = time.time()
        self.base = max(self._tip(node) for node, _, _ in self.legs)

    def _tip(self, node):
        try:
            return chain_time(node)
        except Exception:  # noqa: BLE001 — best-effort by design
            return 0

    def tick(self):
        """One pass: clamp the clock monotonic, advance every chain's
        mocktime, mine the chains whose cadence is due."""
        self.elapsed += self.base_secs
        tip = self.base
        for node, _, _ in self.legs:
            tip = max(tip, self._tip(node))
        now = max(tip, self.base + int(time.time() - self.start_wall)) + 1
        for node, wallet, coin in self.legs:
            try:
                node.set_mocktime(now)
                if self.elapsed % self.block_secs[coin] == 0:
                    node.generate(1, wallet)
            except Exception as e:  # noqa: BLE001 — log + retry next tick
                print(f"[{self.tag}] mine skipped ({wallet}): {e}")
        return now
