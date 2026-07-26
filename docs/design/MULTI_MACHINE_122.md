# Multi-machine safety via seed-scoped partitioning (issue #122)

Status: design agreed 2026-07-08; shipped (#134, legacy-claim fix #144;
auto-follow #163; tier-L follow evidence #171). Reconciled against master at
rc19; lives in docs/design/. §0–§5 are cited **by number** from code comments —
do not renumber. Supersedes the original "active/passive session ownership with
lease + heartbeat" framing in the issue.

## Problem

One BIP39 seed can be live on more than one machine at once (restore-on-laptop,
desktop + server, etc.). Two live drivers on one seed can:

- **double-fund / double-advertise** the same offers and swaps, and
- **reuse initiator secrets** — the catastrophic vector. The initiator swap counter
  `i` (`store.rs next_swap_index`, starts at 0 per DB) feeds `keys.rs`; the *same*
  `i` on two machines derives, for two *different* maker swaps:
  - the same **preimage / hash `H`** (`keys.rs:227`) → in v1, once swap A's redeem
    reveals the preimage on-chain, anyone can sweep swap B's HTLC (same `H`) —
    **theft**;
  - the same **adaptor secret `t`** (`keys.rs:296`) → v2 equivalent: revealing `t`
    in one swap lets a third party complete the other;
  - the same **swap key** and therefore the same **`swap_id`** → offer-coordinate +
    record clobber on the relay/DB.

  Note this is **secret reuse, not nonce reuse**: MuSig2 nonces are fresh CSPRNG,
  use-once, write-ahead-persisted (`fresh_nonce_seed`, engine.rs:203) — never
  derived from `i` — so two machines draw independent nonces. The nonce layer is
  already safe; the leak is the derived preimage/`t`/swap-key.

Same-machine double-launch is **only partly** handled: a 2nd Satchel on the *same
listen port* adopts the 1st's pactd via port + cookie + network probe
(`satchel/src/main.rs` setup). But that guard is bypassed by a custom `--listen` or a
mis-pointed second Satchel — two pactd on the **same data dir, different ports** both
read the same `machine.json` → **identical `derive_scope` → same keys/preimages** →
the very secret-reuse catastrophe above, reintroduced by a config slip (plus two
writers on one SQLite/bdk store). `derive_scope` partitions *different* data dirs; it
does **nothing** for the same one. So a **data-dir lock is required** (§0) — the two
guards are complementary.

## Model (decided)

**No passive mode, no lease, no heartbeat. Every session is active.** Cross-machine
safety comes from *partitioning* work by a per-install seed-derivation scope, not
from electing a single owner. Machines on one seed self-partition and never touch
each other's offers/swaps. If one dies, another adopts its work with a confirmed
click.

Key insight: **each machine has its own DB, so the DB *is* the partition.** A
machine acts only on records it holds. Inbound protocol messages hit the shared
nostr identity mailbox (both machines receive them), but a machine ignores any
message with no matching local offer/swap. No cross-machine id comparison needed
in the happy path.

## Components

### 0. Data-dir lock — one pactd per data dir (highest priority)
The whole scheme assumes **one daemon per data dir** ("the DB is the partition"), but
nothing enforces it: `--data-dir` and `--listen` are independent CLI args and pactd
has no lockfile/PID check. Two pactd on one data dir with different ports share
`machine.json` → identical scope → secret reuse, and two writers hit one SQLite/bdk
store.

Fix: **copy Bitcoin Core's `.lock` mechanism** — pactd already mirrors bitcoind
conventions (cookie auth, datadir, port offsets), so this is feature-parity. At
startup, before opening the store, pactd takes an **exclusive advisory OS lock** on
`<data-dir>/.lock` (pactd/src/main.rs:45; bitcoind uses boost `file_lock`, the Rust
equivalent is `fs2` → `flock` / `LockFileEx`, pure-Rust, no C dep). If held, refuse to start
with a clear "another pactd is already running on this data dir" error (bitcoind's
"Cannot obtain a lock on data directory"). The OS releases it on process exit, so a
crash never strands it (no stale-PID detection needed). This guards the same data dir;
`derive_scope` guards *different* ones — complementary, both needed. It also covers
the bdk wallet sqlite (same data dir), so the two-writers hazard is closed here too.

**But a folder *copy* to another machine defeats scope** — `machine.json` lives in
the data dir, so it travels, and it only regenerates when *absent*, so a copy keeps
the scope → two machines, one scope → collision. Free interlock: **rotate
`derive_scope` whenever the seed is (re)imported** — #120's no-passphrase seed is
sealed to a machine-bound OS-keyring key that does *not* travel, so a cross-machine
copy already fails to decrypt and forces a mnemonic re-import
(`Store::create_seed`/`import_seed`, store.rs:431/:452 — the copy-heal
`pending_scope_rotation` there, :434/:455, drives `machine::rotate_scope`);
regenerating the scope on that path **auto-heals the copy** with no
reliance on the user reading docs. Residuals (documented, not belted): (a) a
*same-machine* copy to a second folder still decrypts (keyring key is machine-global)
→ no re-import → scope not rotated; (b) Linux / passphrase seeds have no machine-bound
key so the copy decrypts anywhere. Both are narrower deliberate-misuse cases; a
best-effort belt is the boot-time relay check "a live snapshot carries *my* scope →
regenerate" (also covers the ~1e-19 random collision), but it only fires once a
snapshot exists, so it's a belt, not a guarantee.

**Rotation consequence — deliberate, don't "fix" it (review-5).** After a scope
rotation, in-flight swaps already in the local DB still carry the *old* scope →
they fail `scope == ours`, demote to **followed**, and need an explicit take-over
to re-drive. That is correct: in the copy case the original machine may be alive,
so auto-adopting on rotation would recreate the double-drive. The cost lands on
the *legit* same-machine #120 keyring-loss reconfirm (OS reinstall / keychain
reset — the very path `seed_needs_reimport`, store.rs:413, exists for): the user re-imports the mnemonic
and must then "take over" their **own** swaps behind the confirm dialog.
Surprising but safe; the UI copy for this case should say "recovered after
re-import", not "another machine". Never auto-adopt on rotation.

### 1. `derive_scope` — the backbone
A random **62-bit** value, machine-level, owned entirely by pactd: stored in a
dedicated **`machine.json` at the pactd data-dir root**, generated by pactd on
startup if absent. It is an engine derivation input, so pactd owns it end-to-end —
Satchel does not touch it. Per-network for free (the data dir already nests by
network). 62 bits (two hardened BIP32 indices) makes the accidental
two-machine-collision probability ~1e-19 instead of ~5e-10 at 31 bits — see the
Known-limitation note.

- A dedicated root file (not per-merchant, not sqlite `meta`): machine-level ⇒
  one scope per install, shared by all merchants (different seeds already diverge
  the keys, so a shared scope never collides across merchants). Root-level, so it
  **survives a per-merchant DB loss** — that is what lets a same-machine recovery
  recognize its own swaps and drive them (see §5). Losing the whole data dir yields
  a new scope.
- **Why a random file, not a hardware GUID?** (a) There is no reliable
  cross-platform stable machine ID. (b) Losing `machine.json` is *safe anyway*:
  `derive_scope` also travels in every rescue snapshot, so seed + relay still
  recovers the in-flight scoped swaps (via the confirm-gated foreign path of §5), and
  pre-funding swaps have no committed funds. So the file is an optimization for the
  no-confirm self-rescue, not a correctness dependency.
- Injected as hardened BIP32 levels into every **initiator / counter-based**
  derivation in `keys.rs` — the **three** branches (the adaptor secret is scoped
  *for free* because it rides the branch-2 key):
  - swap key, branch 1: `m/7228'/1'/coin'/scope_hi'/scope_lo'/i'`;
  - preimage **and** adaptor secret `t`, branch 2:
    `m/7228'/2'/scope_hi'/scope_lo'/i'` (same key, different tagged-hash domain —
    keys.rs:227/296); `swap_id` derives from this, so it's scoped too;
  - refund key, branch 3: `m/7228'/3'/coin'/scope_hi'/scope_lo'/i'`.

  **Anchored (participant) derivations are untouched** — already collision-free.
  Path depth stays distinct from the 7-level anchored scheme, so the two never
  collide.
- Consequence: two machines' maker swaps get **distinct keys, distinct `swap_id`s,
  and distinct relay offer-coordinates** — records and nostr events never collide.
  This single change removes the secret-reuse vector *and* the record/relay clobber,
  and makes N-machine self-partitioning work.
- Persisted **per-swap, immutable** in the v1 + v2 records and in the private #54
  rescue **snapshot** (which already carries the swap-index counter). It is the
  salt the swap's keys were derived under, so it can **never** change — an adopted
  swap keeps the *originating* machine's scope forever, and re-derives its keys
  from that. On a **participant** record it's only a machine tag, not a derivation
  input — participant keys are **anchored** (derived from the swap's public anchor:
  `swap_secret_key_anchored`, keys.rs:211, over the four-level `anchor_levels`
  hash, keys.rs:86). NB this means **participant partitioning is weaker than initiator
  partitioning**: two machines that *deliberately take the same offer* derive
  *identical* anchored keys (same anchor) — see Known limitations. Initiator scope
  is a hard partition; participant safety leans on the maker's first-take-wins + the
  user not double-taking.
- **Invariant — never snapshot the secnonces.** The rescue snapshot carries the
  record + `next_index` only; the private MuSig2 `secnonce`s (`nonce_sessions`) are
  deliberately **excluded** (only the counterparty's *public* nonces ride the
  record). Snapshotting them would be the one way to reintroduce nonce reuse (two
  adopters signing with the same secnonce → key leak). Keep it an explicit invariant
  so nobody "helpfully" adds them later.
- **Invariant — `adopted` is local-only; it must never travel (review-5).**
  Snapshots serialize the *whole record* (`snapshot_v1`/`snapshot_v2` do
  `serde_json::to_value(rec)`, engine.rs:8857/:8872), and an **adopter re-publishes
  snapshots as it drives** (e.g. at Signed) — so its snapshots would carry
  `adopted = true`. If an importer honored that field, a third machine would
  satisfy `drive = … || adopted` **with no confirm** — the exact double-drive the
  confirm gate exists to prevent. So `adopted` is **forced false on every
  snapshot import**: `RescuedRecord::with_adopted_cleared` (engine.rs:823),
  applied on both import paths — `rescue_from_blobs` (:8948) and
  `follow_foreign_from_blobs` (:9183). The DB blob may keep serializing it; the
  import path resets it. Same invariant class as never-snapshot-the-secnonces.
- **`derive_scope` ≠ ownership.** Whether *this* machine drives a swap is a
  separate, **mutable** `adopted` flag. The drive rule is:
  `drive = (derive_scope == our machine scope) || adopted`. Takeover sets
  `adopted = true`; it never rewrites `derive_scope` (see §4/§5). This distinction
  is load-bearing: an adopted swap is foreign-scope **and** driven.
- **Migration is a serde default, not a pass.** Records are single JSON blobs
  (`store.rs` swaps/adaptor_swaps) — no `ALTER TABLE` / `user_version` framework.
  The new `derive_scope` / `adopted` fields ride `#[serde(default)]` (the
  established `last_action_height` pattern), so a pre-existing record simply reads
  back as `derive_scope = 0` = the **legacy marker** ("derive the old way, no scope
  level"). The one active requirement: **swap-creation code must always write the
  real machine scope** — never let a *new* swap fall to the `0` default. At upgrade,
  pactd still generates a real nonzero machine scope in `machine.json`, so all new
  swaps partition safely. Because `0` is only ever a per-swap marker and **never a
  machine's own scope**, a legacy swap is *foreign to every machine* → its recovery
  is always confirm-gated (it can never be self-rescued without the confirm). Pre-1.0
  there are likely no in-flight swaps to protect anyway.

### 2. Inbound-path hardening — one gate, not a broad audit
The only inbound path that **auto-instantiates** a driven swap is the maker **`take`**
arm (`handle_relay_envelope`, engine.rs:10282; the arm :10326). Everything else
already rejects an unknown `swap_id`: `recv`/`recv_adaptor` fail on
`store.get`/`get_adaptor` (engine.rs:4074/2440), and the taker init is gated by
`match_pending_take` (engine.rs:10016). So "harden the inbound path" reduces to
**one change**.

**The take arm.** It reconstructs the offer from the *envelope* and serves it, gated
only by the `offer_served` / `offer_revoked` **meta flags in its own DB**;
`offer_from_take` only checks `offer.from == our identity` (board.rs:210), which
**both** machines pass (shared identity key). So without a gate both machines would
serve the same take → double-init. Fix: **serve a take only if we hold the
originating offer at all — a `my_offers` row for that `offer.swap_id` OR a local
`private_offer:<id>` record (private slips never hit `my_offers`) — keyed on
ownership *existence*, not liveness** (engine.rs:10345; a get-by-id ignoring
`state`, NOT `my_offers_live()` — the row flips to taken/revoked after a legit
serve). A foreign
machine has no such row (its offers are scope-distinct, §1) → it drops the take
silently (no reject — the real owner answers).
Existence, not "live", is deliberate: after a legit serve the row flips to
taken/revoked, so a *liveness* gate would wrongly refuse the real owner's retry —
idempotency stays with the existing `offer_served` / staleness checks. (So §2 depends
on §1.)

A *followed* record (foreign + not adopted) likewise ignores inbound protocol
messages — a follower advances by chain only (§5), never by the handshake. This is
belted explicitly at dispatch (engine.rs:10304): `abort` / `funding_ready` /
`nonces` / `partial_sigs` / `accept` / `funded` / `redeemed` for any
`followed_locally` swap are dropped-and-consumed (each machine keeps its own relay
cursor, so ignoring never starves the owner). Load-bearing with the #163
auto-follow — a standby holds followed records for swaps whose protocol messages
it *also* receives; worst case unbelted was `auto_fund` on a replayed v1 `funded`
re-funding a leg the owning machine already funded (caught live by the e2e rescue
harness).

### 3. Withdraw / receive — never gated
`getnewaddress`, `sendtoaddress`, `estimatesendfee`, `bumpfee`, `listtransactions`
stay fully ungated on all machines (a hard product requirement). They are
wallet-level, not swap-level. **But concurrent cross-machine wallet use is not
"benign"** — the shared bdk wallet has real failure modes (input-race *errors*,
address collision, stale standby balance) documented in Known limitations §"Shared
wallet". Same-data-dir double-writing is closed by §0; the cross-data-dir residuals
are inherent to one on-chain UTXO set behind two independent wallet stores.

### 4. Takeover (adopt a dead machine's work)
User-triggered, on any live machine. The reconstruction half is **automatic**
(#163): the relay scan auto-imports every foreign-scope snapshot (encrypted to the
shared seed identity, readable by any machine holding it) as a **followed** record,
ungated — `follow_foreign_from_blobs` (engine.rs:9121) forces `adopted = false`,
never raises our swap counter, and skips own-scope / legacy / terminal /
already-purged ids. Takeover is then `take_over_swap` (engine.rs:8992) flipping
`adopted = true` on the **already-local** record — no re-import, no reconstruction
at takeover time; the keys still re-derive **under the swap's original (foreign)
`derive_scope`**, which is never changed. The #54 rescue path
(`rescuestatus` / `restorefromrelay`) remains the separate, confirm-gated
**own-scope** recovery. Snapshots are distinguishable by `derive_scope`, so one
specific machine's work can be adopted without touching another's.

Two shipped behaviors ride the takeover itself:

- **Fast-forward before the first driving tick.** `take_over_swap` runs
  `fast_forward_v1`/`_v2` (engine.rs:8999/:9029; defs :7057/:7175) to align the
  snapshot-frozen state with chain truth (STATE_RECONSTRUCTION.md §5.2): an
  already-settled swap becomes history — never re-driven, never re-funded — and
  an in-flight one resumes at the right arm. The same fast-forward runs on the
  #54 self-rescue import (`rescue_from_blobs`, :8950/:8956).
- **Refund-only verdict.** A v2 cooperative redeem is pinned to the sweep address
  fixed in the MuSig2 sighash at init. If this machine cannot **prove** it owns
  that address (`v2_owns_redeem_payout`, engine.rs:9061 — an inconclusive
  `wallet_owns_address` probe counts as "no", fail-closed), adoption still
  succeeds but returns `refund_only = true` (set :9019, returned :9033), surfaced
  through the `takeover` RPC and the UI log: the swap rides to its **refund**
  (which pays a fresh current-wallet address we always own) instead of completing
  to the dead machine's wallet. The gate is re-evaluated per tick in
  `adaptor_tick_one`, so attaching the owning wallet later re-enables completion
  without another takeover. v1 pins no payout destination — never gated.

**Swaps only — offers can't be adopted.** A dead machine's resting (untaken) offer
has no snapshot and its private `swap_index`/preimage isn't in the public offer
event, so a survivor **cannot re-serve** it. It *can* **cancel** it, though — a
NIP-09 kind-5 deletion needs only the public offer coordinate
(`31510:npub:swap_id`) signed by the identity key (same seed) — and the ordinary
revoke path (`revoke_board_offer`, engine.rs:9440) needs no local row, so a
survivor withdraws the dead machine's stale offers as cleanup; to keep offering,
the user re-posts fresh offers on the survivor. Untaken offers otherwise just
expire via relay NIP-40 TTL.

Gated behind an explicit **"confirm the other machine is stopped"** dialog — this
is the whole safety model. The partition keeps *live* machines safe automatically;
takeover is the one deliberate door through the wall, and the confirmation is the
lock. No nostr notice, no live handshake.

**Nostr is NOT a status transport.** A snapshot is a *one-time reconstruction
seed* (parameters + keys-via-scope + last-known state), not a progress feed —
and a dead machine publishes nothing anyway. Chain state is the source of truth;
the engine advances a swap by monitoring confirmations/timelocks/redeems. So a
foreign swap is never "live-followed" over nostr.

### 5. Recovery + following foreign swaps — read-only chain evaluator

**Two classes of recovery info, split by `derive_scope`.** We can open every
snapshot (all sealed to the shared seed identity) and read the `derive_scope` its
keys were derived under, then compare to *this machine's* scope:

- **Scope == ours → "for our machine."** A swap we started and lost (DB loss /
  crash). No other machine holds our (nonzero) scope, so nobody else is driving it
  → **restore and drive, no stopped-machine confirm.** Classic #54 self-rescue —
  still behind the explicit `rescuestatus` warning + `restorefromrelay`, and
  deliberately *not* auto-imported: an own-scope auto-import would instantly
  auto-**drive** (`drives()` = true), so the auto-follow path skips own-scope
  snapshots (engine.rs:9157). (A legacy `scope = 0` swap never matches — it's
  always treated as foreign below.)
- **Scope != ours → "another machine's."** → **auto-imported and followed
  (monitor-only)** by the relay scan (§4, `follow_foreign_from_blobs`), with a
  **Take over** button. Takeover = confirm the other machine is stopped →
  set `adopted = true` (scope stays foreign) → drive.

So the "confirm the other machine is stopped" gate is **scope-conditional**:
skipped for our own recovered swaps, required only for foreign ones.

Foreign swaps are **followed with full chain monitoring, just never driven**.
Status comes from the chain (the source of truth), not from nostr — the snapshot
is only the one-time seed that imports the record so we know a swap exists and
where its HTLCs are.

**Why not reuse `tick_one`.** The driver loop (`tick_one`, engine.rs:7975) is organized
around *"what should I, the driver, do next"* — each `(role, state)` arm does
`observe → advance → broadcast`, and its arms wait on *our own* actions. Stubbing
out the broadcasts would leave a follower stuck. A follower asks a different
question: *"what has happened on both chains?"*

**The follow path = a read-only chain evaluator.** Given the imported record's
HTLC outpoints/scripts, query both chains and derive each leg's status — funded /
redeemed (preimage revealed in the spend) / refunded + confirmations — writing
only the followed status, **never a transaction**. `tick` (engine.rs:4807) routes
by the **drive** decision: driven records (`scope == ours || adopted`) →
`tick_one`; followed records (foreign **and not** adopted) → `follow_one`
(routed :4891, def :7690). Per leg, `follow_eval_leg` (engine.rs:6159) answers
"what happened on this chain" in escalating tiers: live-pointer `get_txout` fast
path while the leg sits locked (:6199) → script-history reconstruction
`classify_leg` (reconstruct.rs:144) → **tier-L** wallet evidence
`wallet_leg_evidence` (#171, engine.rs:6642) → block-scan fallback
`scan_upgrade_leg`/`classify_spent_by_scan` over `ChainBackend::find_spend_tx`
(engine.rs:6749, reconstruct.rs:255; Core-RPC block scan chain.rs:890) — so a
counterparty-settled leg classifies even on a node-RPC-only backend with no
script index. The verdicts feed the same `SwapProgress` machinery as an own swap
(`swap_progress_followed`, engine.rs:5066), so a followed swap shows the same
live progress line.

**Display-state ratchet (STATE_RECONSTRUCTION.md §5.1).**
`followed_display_state_v1`/`_v2` (engine.rs:5255/:5333) advance the followed
record's *displayed* state monotonically to chain reality — rank-gated, never
backwards — persisted as a status-only write in `follow_one` (:7754, write
:7768): never a tx, and routing gates on `drives()`, not state. The ratchet is
load-bearing for takeover (§5.2 there): a participant record ratcheted to
`RedeemedB` off the maker's public reveal, then taken over, must still claim its
leg — the v1 redeem arm admits `FundedB | RedeemedB | Completed`
(engine.rs:4520-4533; the preimage is public either way), with the v2 twin in
`adaptor_tick_one`.

**Safety belt at the choke point, not at tick entry.** Broadcasting happens in ~12
scattered sites — not just redeem/refund/bump, but the funding-retry arms, the
fee-bump nurses, and rescue self-heal. A per-arm belt would miss one. So **every**
swap-tx broadcast funnels through a single engine-level wrapper,
`broadcast_swap_tx` (engine.rs:899), which refuses when the record isn't driven
(followed = foreign **and not** adopted — an *adopted* swap is foreign-scope yet
must broadcast). One belt, lowest layer, keyed on the drive decision — every path
inherits it, so no routing bug can make a follower broadcast. (Funding pays via
`wallet_send`, gated by the same `drives()` check at its own two sites —
engine.rs:4250 / :2560.)

**Invariant: snapshot-before-funding.** No machine ever broadcasts funds before that
swap's snapshot is on the relay — v1 snapshots at accept (engine.rs:1773, :4234),
v2 at accept (:2100, :2547) *and* Signed (`adaptor_assemble`, :2434), all
pre-funding. This is a first-class, **tested** invariant (asserted in the e2e
harness for both protocols), because the whole recovery story rests on it.
Consequence: **no invisible mid-funding window** — a crash right after
broadcasting funds is recoverable, because the follower reconstructs *parameters*
from the snapshot and the follow evaluator **chain-scans** for the funding by
script (outpoints aren't in the accept snapshot, but the HTLC scripts are
derivable). "Follow **from funding onward**" is therefore a **UI choice**
(hide zero-risk pre-funding foreign rows), not a technical limit.

**v2 boundary worth naming.** A v2 swap has funds only *after* Signed (funding
follows the assembled-signature step). A funded v2 swap is therefore always adopted
from the **Signed** snapshot, which carries the assembled redeem/refund txs → the
adopter can complete it. A swap still in the accept→Signed window has *no* committed
funds and no shared nonce state (nonces are per-machine, use-once) — so it can't be
*completed* by a survivor, only left to refund at timelock, which is fine because
nothing is locked.

**Lifecycle** (the drive flag is the discriminator; `derive_scope` stays put):
1. **Followed** — auto-imported to the DB on detection, advanced by the read-only
   evaluator, shown in the `ActiveSwaps` dock grouped per machine (foreign scope,
   not adopted), **read-only, no drive buttons**, excluded from the Swaps ledger
   (native history only). The frontend `Swap` type carries
   `source: "local" | "foreign"` + `machine_label` (stamped by pactd's
   `listswaps`), so the dock groups per machine.
2. **Taken over** — per-machine **"Take over"** on the group header (one confirm
   asserts "that machine is stopped", covering all its swaps; resting offers are
   cancelled via the ordinary revoke, §4) runs the `takeover` RPC
   (`take_over_swap`) per record: `adopted = true` — scope unchanged — so it
   graduates from the evaluator to full `tick_one` driving in place. No
   re-import.
3. **Terminal** — a still-followed (never-adopted) record is **purged from the DB**
   once its swap ends. The shipped rule (`follow_settle` / `follow_purge_ok` /
   `purge_followed_if_deep`, engine.rs:6525/:6494/:7650): **our settlement leg —
   the one this record's role receives on a redeem, or funded on a refund — is
   spent deep at its OWN per-coin confirmation target**, and the co-leg is spent
   (its depth is not ours to wait on). Reorg-safe: the decision that deletes
   re-verifies fresh from the chain, never from the spend cache (:6594) — and a
   follower never tombstones the shared relay snapshot; the real owner does, or
   it harmlessly lingers. The purge writes a local **already-purged memo**
   (`purged_foreign:` meta, engine.rs:313; honored by the auto-import and by
   rescue): the lingering snapshot would otherwise re-import on the next relay
   scan → evaluate → purge → churn, with the row flickering back into the dock.
   An adopted record follows native rules and stays as ledger history.
   Either way, still-foreign records never pollute the ledger; the durable trace of
   any rescue is the on-chain outcome + wallet balance/tx.

## Known limitations (accepted residuals)

- **Concurrency is *safe*, not *coherent*.** Intended use is **failover / standby /
  recovery**, not concurrent multi-active market-making from one seed. Same seed =
  same wallet = same UTXOs. The scope + partition remove the *catastrophic* modes
  (double-drive of one swap, secret/key reuse), but two machines *actively trading
  at once* still draw on one shared balance and **double-post offers**. N machines
  are N interchangeable drivers, **not** N× the liquidity.
- **Shared wallet — the races are not all "benign" (Gap 3, verified).** Two machines
  = two independent bdk stores over one on-chain UTXO set, zero coordination:
  - **Input-race errors, not graceful loss.** A funding that double-spends an input
    the other machine already used returns an **`Err`** from broadcast
    (`is_already_broadcast` tolerates only "already known"/-27, chain.rs:97); the
    funding-retry arms are idempotent by chain-adoption (`locate_funding`), they do
    **not** reselect a fresh UTXO — so the swap can **strand/error**, not "one wins".
  - **Address & change reuse.** Both stores hand out the same External descriptor's
    sequence from index 0 — and the extracted wallet layer (`wallet-btcx`)
    *recycles the lowest unused* address before revealing a new one
    (`next_unused_address`/`reveal_next_address`, wallet-btcx lib.rs:808-819) —
    → same address sequence on both machines → privacy loss.
  - **Standby balance silently stale.** The per-coin background `SyncWorker`
    (#87, extracted to `electrum-btcx`) subscribes and syncs only *that machine's*
    revealed spks (`revealed_spks`, electrum-btcx sync.rs:101); the STOP_GAP=25
    gap scan (sync.rs:39) runs only from a **never-revealed (empty) store**
    (`snapshot_spks`, sync.rs:63-93). A standby that never revealed the index
    another machine funded **won't show those funds** — and nothing shipped heals
    that short of a fresh wallet store (which triggers the from-empty gap scan);
    there is no periodic-rescan mode. This directly dents the "run a watcher on
    another box" value prop.

  Mitigation (documented, not shipped): a non-primary/standby machine would need a
  **periodic gap re-scan** for its balance to stay correct (address reuse we
  accept). Swap *following* is unaffected on script-indexed views — the follow
  evaluator queries legs **by script** against the chain backend
  (`follow_eval_leg`: `get_txout` / Electrum script history), independent of the
  wallet's revealed-spk set; only the tier-L node-RPC path leans on the shared
  node **wallet's** history (`wallet_leg_evidence`) + block scan. On "**lock the
  wallet too**": §0's data-dir lock already makes the wallet single-writer for the
  *same* data dir; the *cross*-machine case can't be locked without the
  cross-machine lease this design deliberately rejects — so it's documented, not
  enforced.
- **Scope collision** (~1e-19 at 62 bits, was ~5e-10 at 31): two machines drawing
  the *same* random scope would silently revert to shared keys/preimage — a
  fund-loss-class failure, and the single assumption the whole design rests on. The
  62-bit width makes it negligible; the residual is documented rather than belted.
- **Multi-survivor adoption (N ≥ 3) is the user's responsibility** — same bucket as
  resurrection below. Two survivors can each "Take over" the same dead machine's
  swaps → both `adopted`. **The cost is state-dependent, not uniformly benign:**
  - post-funding (`RedeemedB` / refund): fully-signed txs → re-broadcast is
    **idempotent** (one wins, no loss) — this is the benign case;
  - at v2 **`Signed`** (the funding trigger — funding follows Signed): both adopters
    **fund leg A** from the shared wallet with independent coin selection → **two
    funding txs / double-funded leg A**. Refundable, but a real **fee loss + T1
    refund wait**, not "one wins." Signed is the costliest split-brain outcome.
  - pre-`Signed`: no committed funds, and adoption is refund-only anyway (the
    counterparty's nonces are consumed — a maker restored to `accepted` cannot
    re-handshake, engine.rs:8880) — safe.
  Coordinating who takes over is on the user, not the design.
- **Self secret-reuse after DB loss if the user trades before rescuing
  (review-5).** A lost per-merchant DB with a surviving `machine.json` keeps the
  scope but resets `next_swap_index` to 0 — a *new* swap then re-derives a lost
  in-flight swap's secrets (same scope, same `i`): the intra-machine twin of the
  cross-machine collision. Pre-existing #54-era hazard, unchanged by scope; the
  self-rescue path already restores the counter (snapshots carry `next_index` →
  `set_next_swap_index_at_least`), but only if rescue runs *before* the first new
  swap. Cheap belt while in here: on boot with an empty swap table but existing
  own-scope relay snapshots, raise the counter from those snapshots before
  allowing a new swap (or at least surface "you have rescuable swaps" first).
- **Participant partitioning is soft.** Initiator secrets are scope-partitioned
  (hard); participant keys are anchored, so two machines *deliberately taking the
  same offer* derive identical keys and could collide on-chain. Mostly mitigated by
  the maker's first-take-wins (only one taker is served), but the guarantee rests on
  the user not double-taking — same "shared wallet, concurrency-safe-not-coherent"
  class as double-posting.

## Explicitly OUT of scope

**Resurrection reconciliation.** If a machine is taken over and then resurrected,
both may drive the same swaps. Making this safe would require the live sync this
design deliberately avoids. Decision: **ensuring dead-is-dead is the user's
responsibility** (the takeover confirm states exactly that). If it bites in
practice, we revisit. No handoff markers, no boot-scan supersession detection, no
resume-gating in this pass.

## Delivery

**One PR** (decision kept — the review flagged splitting the backbone out first;
we're keeping it bundled). Shipped as #134; the legacy-claim fix (#144), the
auto-follow (#163) and the tier-L follow evidence (#171) followed. Order of work
inside it:

0. **Data-dir lock** (§0): exclusive `pactd.lock` at startup via `fs2`/`fs4`; refuse
   a 2nd pactd on the same data dir. Do this first — it's the guard the whole
   partition rests on.
1. `derive_scope`: nonzero **62-bit** machine scope in `machine.json` (two hardened
   BIP32 levels); thread into the **three** initiator branches in `keys.rs` — swap
   key (b1), preimage/adaptor-secret (b2), refund (b3) — and thus `swap_id`; add
   `derive_scope` (immutable) + `adopted` (mutable) to records + snapshot via
   `#[serde(default)]` (absent → `0` = legacy; **new-swap creation must always write
   the real scope**). **Rotate the scope on the #120 reconfirm-with-mnemonic path**
   (§0/Gap-2) so a cross-machine folder copy self-heals. Backbone — also the
   standalone secret-reuse fix.
2. Inbound-path hardening — **one gate**: serve a take only when a `my_offers` row
   for its `swap_id` **exists** (ownership, not liveness — the maker-take arm,
   engine.rs:10326). Other inbound paths already reject unknown `swap_id`s.
3. Follow engine path: read-only chain evaluator for followed records + `tick`
   routing by the drive rule (`scope == ours || adopted`) + broadcast-refusal belt at
   **one engine-level `broadcast_swap_tx` wrapper** that every v1/v2 broadcast site
   funnels through (a *followed* record can never sign/broadcast). Import foreign
   swaps to DB from snapshots — **force `adopted = false` on import** (§1
   invariant) and keep a local already-purged memo so terminal foreign snapshots
   don't re-import (§5). (Under scope, the `set_next_swap_index_at_least`
   high-water-mark is now redundant for foreign adoptions — don't raise our own
   counter from a foreign index; still raise it from *own-scope* snapshots.)
4. Foreign-swap UI + takeover: `ActiveSwaps` dock gets a per-machine "Another
   machine" group (live read-only progress from the evaluator) with a per-machine
   "Take over" behind the "other machine is stopped" confirm (sets `adopted`; scope
   unchanged) + optional "cancel its resting offers"; followed records stay out of
   the Swaps ledger and purge on deep terminal.
5. Polish: Settings shows a short machine label; docs on multi-machine =
   restore-from-seed. Surface the standby-wallet caveat (Known limitations) in
   the UI.

## Testing

Extend the e2e harness (`python -m play`) to run **two pactd on one seed,
separate data dirs** (two
simulated machines). Assert: a 2nd pactd on the **same** data dir is **refused** by
the lock (§0); distinct **preimage/`H` and adaptor `t`**, distinct swap keys/`swap_id`s,
and distinct offer coordinates (1); the **snapshot-before-funding** invariant for
both v1 and v2; autonomous drive + inbound dispatch stay partitioned and a *followed*
record never broadcasts (2); a followed swap's status tracks the chain without driving
(3); takeover sets `adopted` and re-drives in place, and the survivor can cancel the
dead machine's resting offers (4); a snapshot published by an *adopter* does **not**
confer drive on a third importer (`adopted` reset on import, §1 invariant).
Withdraw works on both throughout.
