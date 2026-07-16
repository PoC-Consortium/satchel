"""The harness framework package (Bitcoin Core's test_framework/ analog).

Being extracted from the ad-hoc harness per docs/TEST_FRAMEWORK_PLAN.md.
binaries.py — the single binary resolver;  node.py — Node/ElectrsServer/
Harness;  daemon.py — Pactd (a.k.a. Party);  services.py — Corkboard +
NostrRelay;  stack.py — build_workspace + the funded-datadir cache;
testbase.py — PactTestFramework (one scenario = one fresh stack);
util.py — cookie-RPC client, wait_until, the mainnet-safe port registry;
clock.py — the playground mining/mocktime model;  market.py — offer books,
faucet, auto-taker;  satchel.py — the Satchel GUI seam + teardown machinery.
"""
