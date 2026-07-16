"""The harness framework package (Bitcoin Core's test_framework/ analog).

Being extracted from the ad-hoc harness per docs/TEST_FRAMEWORK_PLAN.md.
Shipped: binaries.py (the single binary resolver), daemon.py (Pactd/Party),
services.py (Corkboard + NostrRelay), stack.py (build_workspace; the full
stack builder + datadir cache come with Phase 2), util.py (cookie-RPC client,
wait_until, the mainnet-safe teardown port registry). Still to come:
node.py (Phase 2 moves Node/ElectrsServer/Harness), testbase.py, satchel.py.
"""
