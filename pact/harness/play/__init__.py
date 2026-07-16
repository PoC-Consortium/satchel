"""The flag-composed interactive playground (TEST_FRAMEWORK_PLAN §2.5).

    python -m play --board cork|nostr --btcx node|nodeless --electrs N
                   --satchel one|two-observer|viewer|none
                   [--first-run] [--relay-cmd CMD] [--persist] [--keep] [--down]

Replaces the seven tools/playground-*.ps1 + knockdown.ps1 + the four
per-variant Python drivers. Companion diagnostics: play/repro_multiswap.py,
play/observer_compare.py.
"""
