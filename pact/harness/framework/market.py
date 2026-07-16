"""Playground market simulation (TEST_FRAMEWORK_PLAN §2.1): the Bob/Carol
offer books with non-destructive topup, the nodeless-Alice faucet, and the
observer playground's auto-taker. Consolidates what the three GUI playground
drivers each carried a copy of.
"""

import os
import time

from framework.util import pactd_rpc_or_none

# Each side posts a SPREAD at varied sizes + implied rates so the book is
# rate-sorted, not one card repeated. Both sides hover ~47k–51k POCX/BTC so
# they read as a coherent market. Offer strings use the wire coin_id ("btcx",
# "btc"); the UI shows the symbols. Every offer is completable: the maker
# funds what it GIVES, the taker funds what it gives in return. Offers
# alternate v1 HTLC / v2 adaptor by slot so both routes are always live
# (unless a book pins one — the LTC books pin v1).
PROTOCOLS = ["pact-htlc-v1", "pact-htlc-v2"]

# Bob = BUY side: gives BTC, wants POCX (funded with BTC). Taker gives POCX.
BOB_OFFERS = [
    ("btc:0.0005", "btcx:24"),    # 1 BTC = 48,000 POCX  (small)
    ("btc:0.001",  "btcx:47"),    #         47,000       (cheap for taker)
    ("btc:0.001",  "btcx:50"),    #         50,000       (canonical)
    ("btc:0.0015", "btcx:72"),    #         48,000
    ("btc:0.002",  "btcx:102"),   #         51,000
    ("btc:0.003",  "btcx:153"),   #         51,000       (large)
]

# Carol = SELL side: gives POCX, wants BTC (funded with POCX). Taker gives BTC.
CAROL_OFFERS = [
    ("btcx:25",  "btc:0.0005"),   # 1 BTC = 50,000 POCX
    ("btcx:50",  "btc:0.00104"),  #         ~48,000      (asks a bit more BTC)
    ("btcx:50",  "btc:0.00098"),  #         ~51,000
    ("btcx:75",  "btc:0.00156"),  #         ~48,000
    ("btcx:100", "btc:0.00196"),  #         ~51,000
]

# Litecoin sub-book — a two-sided spread on BOTH LTC pairs so those boards
# aren't near-empty next to BTCX<->BTC. Bob BUYS ltc (gives BTC or BTCX),
# Carol SELLS ltc. LTC is a file-added coin; pinned to v1 HTLC.
BOB_LTC_OFFERS = [
    ("btc:0.0005", "ltc:1"),      # buy LTC with BTC: 1 LTC = 0.0005 BTC
    ("btc:0.001",  "ltc:2"),
    ("btc:0.0015", "ltc:3"),
    ("btcx:25", "ltc:1"),         # buy LTC with BTCX: 1 LTC = 25 BTCX
    ("btcx:50", "ltc:2"),
    ("btcx:75", "ltc:3"),
]
CAROL_LTC_OFFERS = [
    ("ltc:1", "btc:0.00052"),     # sell LTC for BTC (asks a touch above Bob)
    ("ltc:2", "btc:0.00104"),
    ("ltc:3", "btc:0.00156"),
    ("ltc:1", "btcx:26"),         # sell LTC for BTCX
    ("ltc:2", "btcx:52"),
    ("ltc:3", "btcx:78"),
]

# The observer playground's smaller btcx<->btc-only book (LTC omitted to keep
# the observer demo focused).
OBSERVER_BOB_OFFERS = [
    ("btc:0.001", "btcx:47"),
    ("btc:0.0015", "btcx:72"),
    ("btc:0.002", "btcx:102"),
]
OBSERVER_CAROL_OFFERS = [
    ("btcx:50", "btc:0.00104"),
    ("btcx:75", "btc:0.00156"),
    ("btcx:100", "btc:0.00196"),
]

DEFAULT_TIMELOCKS = (4 * 3600, 2 * 3600)  # (t1_secs, t2_secs)


class Book:
    """One party's side of the book, refreshed NON-destructively: never revoke
    a live offer (that would churn offer IDs and make a taker's click race a
    revoke — the "offer not on any configured board" failure). Only PRUNE ids
    that are already gone (taken → C5 auto-revoke, or expired) and refill back
    to the target count. Live offers keep their IDs, so a take never races a
    repost."""

    def __init__(self, party, key, offers, pin_proto=None):
        self.party = party
        self.key = key
        self.offers = list(offers)
        self.pin_proto = pin_proto
        self.posted = []

    def topup(self, tag):
        try:
            live = {o["swap_id"] for o in self.party.rpc("boardlistoffers")["offers"]}
        except Exception:  # noqa: BLE001 — party not up yet: retry next round
            return
        self.posted[:] = [oid for oid in self.posted if oid in live]
        deficit = len(self.offers) - len(self.posted)
        for i, (give, get) in enumerate(self.offers[:max(0, deficit)]):
            proto = self.pin_proto or PROTOCOLS[i % len(PROTOCOLS)]
            try:
                t1, t2 = DEFAULT_TIMELOCKS
                r = self.party.rpc("boardpostoffer", give, get, t1, t2, proto)
                self.posted.append(r["offer_id"])
            except Exception as e:  # noqa: BLE001
                print(f"[{tag}] {self.key} post failed ({give} -> {get}, {proto}): {e}")


class Market:
    """All books + the repost timer. Call maintain() every miner tick."""

    def __init__(self, books, repost_every=30, tag="pg"):
        self.books = list(books)
        self.repost_every = repost_every
        self.tag = tag
        self._last = 0.0

    def post_now(self):
        for book in self.books:
            book.topup(self.tag)
        live = " + ".join(f"{len(b.posted)} {b.key}" for b in self.books)
        print(f"[{self.tag}] offers live: {live}")
        self._last = time.time()

    def maintain(self):
        if time.time() - self._last > self.repost_every:
            try:
                self.post_now()
            except Exception as e:  # noqa: BLE001
                print(f"[{self.tag}] post_offers skipped: {e}")
                self._last = time.time()


def managed_cookie_path(base="org.pocx.satchel", network="regtest"):
    """The managed Satchel pactd's cookie file for a config-dir base name.
    Windows: %LOCALAPPDATA%; macOS: ~/Library/Application Support;
    Linux: ~/.config (the Tauri app-config convention)."""
    local = os.environ.get("LOCALAPPDATA")
    if local:
        root = os.path.join(local, base)
    elif os.path.isdir(os.path.expanduser("~/Library/Application Support")):
        root = os.path.expanduser(f"~/Library/Application Support/{base}")
    else:
        root = os.path.expanduser(f"~/.config/{base}")
    return os.path.join(root, network, "pactd", ".cookie")


def managed_rpc(port, method, *params, base="org.pocx.satchel",
                network="regtest", timeout=10):
    """Best-effort RPC to a managed Satchel pactd (None until it is up)."""
    return pactd_rpc_or_none(f"http://127.0.0.1:{port}/", method, *params,
                             cookie_path=managed_cookie_path(base, network),
                             timeout=timeout)


class Faucet:
    """Starter balances for the managed Alice's NODELESS (pact-seed) wallets —
    they live on the seed she creates in the wizard, so they can't be
    pre-funded like node wallets. Poll until her pactd serves a wallet, then
    send once per coin. targets: [(coin_id, node, funding_wallet, amount)]."""

    def __init__(self, targets, port=9739, base="org.pocx.satchel", tag="pg"):
        self.targets = list(targets)
        self.port = port
        self.base = base
        self.tag = tag
        self.done = False

    def run_once(self):
        if self.done:
            return True
        all_done = True
        for coin, node, wallet, amount in self.targets:
            bal = managed_rpc(self.port, "getbalance", coin, base=self.base)
            if bal is None:
                all_done = False   # not up yet / wizard pending: retry
                continue
            if bal.get("balance_sat", 0) > 0:
                continue
            addr = managed_rpc(self.port, "getnewaddress", coin, base=self.base)
            if addr is None:
                all_done = False
                continue
            try:
                node.rpc("sendtoaddress", addr["address"], amount, wallet=wallet)
                print(f"[{self.tag}] faucet: {amount} {coin.upper()} -> Alice's "
                      f"nodeless wallet ({addr['address'][:24]}…) — confirms next block")
            except Exception as e:  # noqa: BLE001
                print(f"[{self.tag}] faucet skipped ({coin}): {e}")
                all_done = False
        self.done = all_done
        return self.done


class AutoTaker:
    """Observer playground: a bot takes each of Alice's own offers exactly
    once, so the MAKE path works (the bot funds the counterparty leg)."""

    def __init__(self, taker_party, main_port=9739, tag="obs-pg"):
        self.taker = taker_party
        self.main_port = main_port
        self.tag = tag
        self.alice_id = None
        self.taken = set()

    def poll(self):
        if self.alice_id is None:
            info = managed_rpc(self.main_port, "getinfo")
            self.alice_id = info.get("identity") if info else None
            if self.alice_id is None:
                return
        try:
            board = self.taker.rpc("boardlistoffers")["offers"]
        except Exception:  # noqa: BLE001
            return
        for o in board:
            if o.get("from") == self.alice_id and o["swap_id"] not in self.taken:
                self.taken.add(o["swap_id"])
                try:
                    self.taker.rpc("boardtake", o["swap_id"])
                    print(f"[{self.tag}] {self.taker.name} auto-took Alice's "
                          f"offer {o['swap_id'][:12]}")
                except Exception as e:  # noqa: BLE001
                    print(f"[{self.tag}] auto-take failed: {e}")
