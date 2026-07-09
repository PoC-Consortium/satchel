// Shapes of the pactd JSON-RPC results and Satchel command payloads the UI
// consumes. These mirror what main.rs / pactd return; they are intentionally
// loose where pactd is the source of truth (the UI renders, never decides).

/** A merchant row from pactd's `listmerchants` (C10 — pactd owns the registry).
 *  `identity` keys the identicon; `active` = loaded in-process;
 *  `locked` = an encrypted seed still awaiting its session passphrase. */
export interface Merchant {
  id: string;
  label: string;
  /** BIP340 x-only identity pubkey (hex). Null until the seed exists. */
  identity?: string | null;
  /** Whether this is the currently loaded (active) merchant. */
  active?: boolean;
  /** Whether the active merchant's encrypted seed is still locked. */
  locked?: boolean;
  created?: number;
  encrypted?: boolean;
}

/** pactd `listmerchants` result. */
export interface MerchantList {
  merchants: Merchant[];
  active: string | null;
}

/** Per-install UI preferences (UI-1), persisted in satchel.json (not the
 *  webview's localStorage). */
export interface UiPrefs {
  theme: "dark" | "light" | "system";
  language: string;
  nav_open: boolean;
  /** One-time first-run flag: true once the user has completed or skipped the
   *  first-run coin-setup dialog, so it never shows again. */
  onboarded: boolean;
  /** Desktop-notification switches (issue #55). */
  notify: NotifyPrefs;
}

/** OS-notification toggles (issue #55): one master switch + one per event kind
 *  (the sub-toggles in Settings → Notifications). Mirrors Rust `NotifyPrefs`. */
export interface NotifyPrefs {
  enabled: boolean;
  /** A counterparty took your offer / your take was accepted. */
  swap_started: boolean;
  /** A leg's lock confirmed on-chain (ours or theirs). */
  locks: boolean;
  /** Swap finished — coins settled in the wallet. */
  completed: boolean;
  /** Swap unwound — refunded or aborted. */
  failed: boolean;
  /** Chain reorg touched a watched swap. */
  reorg: boolean;
}

/** A local-only contact's standing. `trusted` = a counterparty you vouch for
 *  (whitelist); `blocked` = one you'd rather not deal with (blacklist) — both
 *  are purely your own view/warnings, never protocol-enforced. */
export type ContactStatus = "trusted" | "neutral" | "blocked";

/** One entry in the local contact book. Keyed by — and bound to — the
 *  counterparty's BIP340 hex pubkey: the nick is a display *alias* shown
 *  ALONGSIDE the spoof-proof fingerprint, never a replacement for it. Stored
 *  only on this machine (satchel.json), never published. */
export interface Contact {
  /** BIP340 hex pubkey — the map key; immutable. */
  id: string;
  /** User-chosen alias; may be empty. */
  nick: string;
  /** Freeform notes (e.g. how to reach them, trade history). */
  note?: string;
  status: ContactStatus;
  /** epoch ms when first added. */
  added: number;
}

/** The whole local contact book: hex pubkey → Contact. */
export type ContactBook = Record<string, Contact>;

/** One configured coin connection from Satchel's satchel.json. The structured
 *  fields are absent on a pre-v2 (chain_data-only) config. */
export interface CoinConn {
  coin_id: string;
  chain_data: string;
  funding_wallet: string;
  /** Confirmation depth override (reorg-safety); null/absent = use the default. */
  confirmations?: number | null;
  rpc_host?: string | null;
  rpc_port?: number | null;
  /** "cookie" | "userpass". */
  auth_method?: string | null;
  rpc_user?: string | null;
  rpc_password?: string | null;
  datadir?: string | null;
  cookie_subpath?: string | null;
  wallet?: string | null;
  extra_backends?: string[];
}

/** Per-(coin, network) connection defaults from a coins.toml template,
 *  returned by the `list_coin_templates` Satchel command. */
export interface NetConnDefaults {
  rpc_host: string;
  rpc_port?: number | null;
  auth_method: string;
  datadir: string;
  cookie_subpath: string;
  wallet: string;
  /** Default Electrum servers for the nodeless mode (pre-fill the URL list). */
  electrum?: string[];
}

/** One coin template (connection defaults + presentation) for the picker. */
export interface CoinTemplate {
  coin_id: string;
  display_name: string;
  symbol: string;
  decimals: number;
  has_icon: boolean;
  defaults: NetConnDefaults;
}

/** `list_coin_templates` result for the current network. */
export interface CoinTemplateList {
  network: string;
  coins: CoinTemplate[];
}

/** The structured connection payload sent to `save_coin` / `compose_coin_url`
 *  (mirrors the Rust `CoinConnInput`). */
export interface CoinConnInput {
  rpc_host?: string;
  rpc_port?: number;
  /** Absent for the nodeless (pact-seed) mode — nothing to recompose. */
  auth_method?: string;
  rpc_user?: string;
  rpc_password?: string;
  datadir?: string;
  cookie_subpath?: string;
  wallet?: string;
  extra_backends?: string[];
  funding_wallet?: string;
  /** Expert/legacy escape hatch: raw URL string overrides composition. */
  chain_data?: string;
}

export interface CoinConfig {
  coins: CoinConn[];
  network: string;
  board_urls: string[];
  /** Nostr relay URLs for the decentralized transport (prewired by default;
   *  an explicit empty list = transport off). */
  nostr_relays?: string[];
}

/** One relay's connectivity from pactd `boardstatus` (Nostr transport).
 *  Empty list ⇒ Nostr not configured (the header hides the indicator). */
export interface RelayStatus {
  url: string;
  connected: boolean;
  /** Pool status token: connected | connecting | pending | disconnected |
   *  terminated | banned | sleeping | initialized. */
  status?: string;
  /** Round-trip latency in ms, when measured. */
  latency_ms?: number | null;
  /** Unix seconds since the current connection was established (uptime base). */
  connected_since?: number | null;
  attempts?: number;
  success?: number;
  bytes_sent?: number;
  bytes_received?: number;
}

/** `getinfo` — extended in Phase B/C with seed + identity state. */
export interface Info {
  version?: string;
  protocol?: string;
  network?: string;
  identity?: string | null;
  seed_exists?: boolean;
  locked?: boolean;
  /** Wire-compatibility epoch this pactd speaks per protocol family (rc10),
   *  e.g. { "pact-htlc-v1": 1, "pact-htlc-v2": 2 }. Offers whose signed
   *  `wire` differs are badged un-takeable. */
  wire_epochs?: Record<string, number>;
  /** Short one-way label of THIS install's multi-machine scope (#122), e.g.
   *  "M-7f3a" — shown in Settings so a user running the same seed on more than
   *  one machine can tell them apart. */
  machine_label?: string;
}

/** A swap leg's chain ref (older builds used `asset` instead of `coin_id`). */
export interface ChainRef {
  coin_id?: string;
  asset?: string;
  network?: string;
}

export type SwapState =
  // UI-only pre-swap: a take has been sent but the maker hasn't started the
  // swap yet, so no record exists in the engine (sourced from listpendingtakes).
  | "initiating"
  | "created"
  | "accepted"
  // v2-only handshake states (AdaptorState): MuSig2 nonces traded, then both
  // adaptor signatures aggregated + verified. Both are pre-redeem / non-terminal.
  | "nonces_exchanged"
  | "signed"
  | "funded_a"
  | "funded_b"
  | "redeemed_b"
  | "completed"
  | "refunded"
  | "aborted";

export interface Swap {
  swap_id: string;
  role: "initiator" | "participant";
  state: SwapState;
  chain_a?: ChainRef;
  chain_b?: ChainRef;
  amount_a: number;
  amount_b: number;
  t1: number;
  t2: number;
  /** OUR settlement txid (the leg we redeemed, or our refund). Never the
   *  counterparty's settlement — we don't track or show that. */
  final_txid?: string | null;
  /** Per-leg funding txids — what was locked on each chain. Both legs are
   *  surfaced (the on-chain audit trail); normalized from v1 `htlc_*_txid` /
   *  v2 `funding_*_txid`. */
  fund_a_txid?: string | null;
  fund_b_txid?: string | null;
  /** unix seconds (C2) — served by pactd's listswaps/getswap. Old records that
   *  predate the field default to 0; history falls back to list order for those. */
  created_at?: number;
  /** Which swap protocol produced this record. Absent ⇒ v1 (`pact-htlc-v1`,
   *  from `listswaps`); `pact-htlc-v2` is set when we fold in `listadaptorswaps`
   *  so the ledger can badge the Taproot/MuSig2 ones. */
  protocol?: string;
  /** Live progress snapshot (observability), folded in from the `swapprogress`
   *  RPC by `swap_id`. Present only for active swaps with something watchable. */
  progress?: SwapProgress;
  /** BIP340 pubkey (hex) of the other party — who we're swapping with. From the
   *  swap record (set at init/accept); shown via CounterpartyTag. Absent on a
   *  pre-record pending take until the maker is known. */
  counterparty_identity?: string | null;
  /** Multi-machine ownership (issue #122): `"local"` = driven by this machine,
   *  `"foreign"` = another machine on the same seed drives it and we only follow
   *  it read-only (take it over to drive it here). Computed by pactd; absent ⇒
   *  treat as local (a pre-#122 daemon, or a pending take). */
  source?: "local" | "foreign";
  /** Short one-way label of the machine that owns this swap (e.g. "M-7f3a"),
   *  used to GROUP foreign swaps per machine in the dock. From pactd. */
  machine_label?: string;
}

/** Live per-swap progress from pactd `swapprogress` (rebuilt each scheduler
 *  tick, served from memory). Observability only — never ledger truth. */
export interface SwapProgress {
  swap_id: string;
  /** The current wait + how to show it:
   *  - `awaiting_lock` / `awaiting_claim` — waiting on the counterparty (no
   *    target); show `blocks_elapsed`.
   *  - `their_lock` — their lock burying (our gate); show `confs/needed`.
   *  - `our_lock` — our own lock burying toward the depth the taker requires
   *    before they lock (v1 maker only); show `confs/needed`.
   *  - `settlement` — our own claim burying ("Securing your {coin}"). */
  watching:
    | "awaiting_lock"
    | "awaiting_claim"
    | "their_lock"
    | "our_lock"
    | "settlement"
    | "funding";
  /** Display symbol of the watched leg (e.g. "BTC"). */
  coin: string;
  /** Confirmations so far (0 for the awaiting phases). */
  confs: number;
  /** Required depth for this leg, n_a / n_b (0 for the awaiting phases). */
  needed: number;
  /** Blocks elapsed in the current awaiting phase (liveness cue, no deadline). */
  blocks_elapsed?: number;
  /** Current feerate of our settlement tx (sat/vB); settlement phase only. */
  feerate_sat_vb?: number;
  /** The latest scheduler action for this swap (e.g. "fee-bump", "auto-redeem"). */
  last_action?: string;
  last_detail?: string;
  /** unix seconds when the snapshot was taken — used to grey out stale data. */
  updated_at: number;
}

/** Raw `listswaps` record (libswap `SwapRecord`): the audit txid fields that
 *  aren't on the normalized `Swap`. Mapped via `v1ToSwap`. */
export interface V1SwapRecord extends Swap {
  htlc_a_txid?: string | null;
  htlc_b_txid?: string | null;
}

/** One v2 (Taproot/MuSig2 adaptor) swap from pactd `listadaptorswaps`. Mirrors
 *  libswap's `AdaptorSwapRecord`; only the fields the ledger renders are typed
 *  (pactd is the source of truth). Folded into `Swap` via `adaptorToSwap`. */
export interface AdaptorSwapRecord {
  swap_id: string;
  role: "initiator" | "participant";
  state: SwapState;
  created_at: number;
  chain_a?: ChainRef;
  chain_b?: ChainRef;
  amount_a: number;
  amount_b: number;
  t1: number;
  t2: number;
  /** Taproot funding outpoints, one per leg (set when each leg is funded). */
  funding_a_txid?: string | null;
  funding_b_txid?: string | null;
  /** Cooperative key-path redeem txids, one per leg (set as each is broadcast).
   *  `_a` is the participant's claim, `_b` the initiator's — we only ever
   *  surface OUR own (see `adaptorToSwap`). */
  final_txid_a?: string | null;
  final_txid_b?: string | null;
  /** BIP340 pubkey (hex) of the counterparty. */
  counterparty_identity?: string | null;
  /** Multi-machine ownership (issue #122) — see `Swap.source` / `machine_label`.
   *  Stamped by pactd's `listadaptorswaps`; propagated by `adaptorToSwap`. */
  source?: "local" | "foreign";
  machine_label?: string;
}

export interface TickEvent {
  action: string;
  swap_id: string;
  detail: string;
}

/** A pending take from pactd `listpendingtakes`: a take we've sent that's
 *  awaiting the maker's init (no swap record exists yet). `offer_id` equals the
 *  eventual swap's `swap_id`. Folded into the swaps list as "initiating". */
export interface PendingTake {
  offer_id: string;
  from: string;
  body: OfferBody;
  created_at: number;
}

/** A board offer envelope: signed body + provenance. */
export interface OfferBody {
  give_asset: string;
  get_asset: string;
  give_amount: number;
  get_amount: number;
  created?: number;
  ttl_secs?: number;
  t1_secs: number;
  t2_secs: number;
  /** "pact-htlc-v1" (HTLC) or "pact-htlc-v2" (Taproot/MuSig2 adaptor). */
  protocol?: string;
  /** Wire-compatibility epoch of `protocol` (rc10); absent = 1 (pre-rc10
   *  maker). Compared against getinfo's `wire_epochs` — a mismatch means
   *  the offer is view-only (the engine refuses the take anyway). */
  wire?: number;
}

export interface Offer {
  swap_id: string;
  from: string;
  body: OfferBody;
  /** Surfaced by the board when a maker withdraws their notice. */
  revoked?: boolean;
}

/** A locally-stored private (off-market) offer from pactd `listprivateoffers`
 *  (the Pact handbook, private offers). Never posted to a board; tracked so the maker can
 *  cancel its slip before the ttl lapses. */
export interface PrivateOffer {
  offer_id: string;
  give_asset: string;
  give_amount: number;
  get_asset: string;
  get_amount: number;
  t1_secs: number;
  t2_secs: number;
  /** unix seconds from the signed offer body. */
  created: number;
  /** unix expiry (created + ttl); 0 when no expiry. */
  expiry: number;
  /** whether the ttl has already lapsed (slip no longer takeable). */
  expired: boolean;
}

// --- C3 (fee preview) — served by pactd `estimateswapfees` ------------------
// `platform_fee_sat` is ALWAYS 0 (Corkboard charges nothing). Legs are the ones
// THIS user pays (give = fund + refund alt; get = redeem). `fee_rate_is_fallback`
// is true when a node was down and a conservative default rate was used — the UI
// flags those numbers as a guess rather than presenting them as live.
export interface FeeLeg {
  name: string; // "fund" | "redeem" | "refund"
  vbytes: number;
  fee_sat: number;
}
export interface FeeSide {
  coin_id: string;
  fee_rate_sat_per_vb: number;
  fee_rate_is_fallback?: boolean;
  legs: FeeLeg[];
}
export interface SwapFees {
  platform_fee_sat: 0;
  give: FeeSide;
  get: FeeSide;
}

export interface Capabilities {
  cltv?: boolean;
  segwit_v0?: boolean;
  taproot?: boolean;
}

/** A `listcoins` entry: shipped registry + whether it's configured + live status. */
export interface CoinInfo {
  id: string;
  display_name: string;
  symbol: string;
  configured?: boolean;
  status?: string; // "ok" or an error string
  tip_height?: number;
  capabilities?: Capabilities;
  /** Effective confirmation depth in force for this coin (reorg-safety). */
  confirmations?: number;
  /** The network/spacing default depth, shown as the field placeholder. */
  default_confirmations?: number;
  /** Minimum allowed confirmation depth on this network (2 on mainnet/testnet, 1 on regtest). */
  min_confirmations?: number;
  /** Core wallet this coin's RPC is scoped to (from the configured URL); null
   *  when none is set — the node's default wallet (not explicitly scoped). */
  wallet?: string | null;
  /** Nodeless coin (Electrum-only backends, epic #58): the wallet is the bdk
   *  one derived from the Pact seed — send/receive/activity apply. */
  nodeless?: boolean;
  /** Electrum fleet health (issue #98): configured / currently-healthy /
   *  in-failure-backoff counts, from the passive health registry. */
  servers_total?: number;
  servers_healthy?: number;
  servers_down?: number;
  /** State of the ELECTED wallet-home server (#99): "healthy" | "down" |
   *  "untested". Only set for nodeless coins. */
  wallet_server_state?: string | null;
  /** Seconds since the nodeless wallet cache was last confirmed against its
   *  server — the "balance as of" staleness signal (#99). */
  wallet_synced_secs_ago?: number | null;
}

/** One Electrum server's passive health (`serverstatus`, issues #98/#100).
 *  Display data only — the backend never dials a server to answer this. */
export interface ServerStatus {
  coin_id: string;
  url: string;
  /** "healthy" | "down" | "untested" (a standby never touched this run). */
  state: string;
  /** "wallet" (elected home) | "view" (active) | "standby"; absent until
   *  the coin has routed once this run. */
  role?: string;
  /** When down: seconds until the failure-backoff window expires. */
  retry_in_secs?: number;
  /** Smoothed request latency (EWMA), ms. */
  latency_ms?: number | null;
  last_ok_secs_ago?: number;
  last_error?: string;
  last_error_secs_ago?: number;
  requests: number;
  failures: number;
}

/** One row of the nodeless wallet's activity feed (`listtransactions`). */
export interface WalletTx {
  txid: string;
  direction: "sent" | "received";
  amount_sat: number;
  fee_sat?: number | null;
  /** Virtual size in vB — with fee_sat this yields the effective feerate an
   *  RBF bump has to beat. */
  vsize: number;
  confirmations: number;
  timestamp?: number | null;
}

/** Fee preview for the send form (`estimatesendfee`): raw estimator answers
 *  for the Slow/Normal/Fast presets (144/6/1 blocks), null where the
 *  estimator has no data, plus the coin's feerate floor. */
export interface SendFeeEstimates {
  min_sat_per_vb: number;
  fast?: number | null;
  normal?: number | null;
  slow?: number | null;
}

export interface Pair {
  coin_a: string;
  coin_b: string;
  available: boolean;
  both_configured?: boolean;
  protocols?: string[];
  selectable?: string;
}
