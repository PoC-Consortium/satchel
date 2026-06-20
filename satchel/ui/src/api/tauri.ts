// The UI's ONLY data paths (load-bearing invariant from SATCHEL_PLAN Phase E):
//   1. invoke('pactd_rpc', { method, params })  → all pactd JSON-RPC, INCLUDING
//      the C10 merchant registry (createmerchant/listmerchants/loadmerchant/…)
//   2. the get_/set_/save_ Satchel commands      → daemon-level config + UI prefs
// No other backend surface, and no swap logic, ever lives in the UI.

import { invoke } from "@tauri-apps/api/core";
import type {
  CoinConfig,
  CoinConnInput,
  CoinTemplateList,
  Merchant,
  MerchantList,
  PrivateOffer,
  UiPrefs,
} from "./types";

/** True when running inside the Tauri webview (so `invoke` is wired up). */
export function inTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/**
 * One call shape for every pactd JSON-RPC method. Goes through Satchel's Rust
 * `pactd_rpc` proxy, which holds the cookie. Rejects with a string message
 * (Satchel maps daemon errors to `Err(String)`), so callers `catch (e)` and
 * surface `String(e)`.
 */
export async function rpc<T = unknown>(method: string, params: unknown[] = []): Promise<T> {
  return (await invoke("pactd_rpc", { method, params })) as T;
}

// ---- Merchant registry (C10) — now pactd RPCs, not Satchel commands --------
// pactd owns merchants (the Bitcoin-Core wallet analog). These are thin wrappers
// over `pactd_rpc`; switching is in-process (no relaunch).

export const listMerchants = () => rpc<MerchantList>("listmerchants");

/** Create a new merchant + switch to it (the seed is provisioned afterwards via
 *  the createseed/importseed RPCs). Returns the new `{ id, label }`. */
export const createMerchant = (label: string) =>
  rpc<Merchant>("createmerchant", [label]);

/** Switch the active merchant in-process. pactd refuses to switch away from a
 *  merchant with a live swap (fund-safety gate) — surfaced as a thrown error. */
export const selectMerchant = (id: string) => rpc<Merchant>("loadmerchant", [id]);

// ---- Satchel-native commands (persist daemon-level config + UI prefs) -------
// Tauri converts these camelCase arg keys to the Rust snake_case params.

/** Read per-install UI prefs from satchel.json (UI-1). */
export const getUiPrefs = () => invoke("get_ui_prefs") as Promise<UiPrefs>;

/** Patch per-install UI prefs in satchel.json (UI-1). Pass only what changed. */
export const setUiPrefs = (patch: Partial<UiPrefs>) =>
  invoke("set_ui_prefs", {
    theme: patch.theme ?? null,
    language: patch.language ?? null,
    navOpen: patch.nav_open ?? null,
  }) as Promise<void>;

export const listCoinConfig = () => invoke("list_coin_config") as Promise<CoinConfig>;

/** Coin templates (connection defaults + icon availability) for the current
 *  network — the source for the setup picker. */
export const listCoinTemplates = () =>
  invoke("list_coin_templates") as Promise<CoinTemplateList>;

/** Save (upsert) a coin's structured connection. Satchel composes the backend
 *  URL (reading the cookie when auth = cookie) and relaunches pactd. */
export const saveCoin = (coinId: string, conn: CoinConnInput, confirmations: number | null) =>
  invoke("save_coin", { coinId, conn, confirmations }) as Promise<void>;

/** Preview the backend URL Satchel would compose+save for a structured
 *  connection — handed to `validatecoin` so validation hits the exact URL. */
export const composeCoinUrl = (coinId: string, conn: CoinConnInput) =>
  invoke("compose_coin_url", { coinId, conn }) as Promise<string>;

/** A coin's icon (from the file next to coins.toml) as a data: URL, or null. */
export const getCoinIcon = (coinId: string) =>
  invoke("get_coin_icon", { coinId }) as Promise<string | null>;

export const removeCoin = (coinId: string) => invoke("remove_coin", { coinId }) as Promise<void>;

export const saveBoard = (urls: string) => invoke("save_board", { urls }) as Promise<void>;

/** Save Nostr relay URLs (comma-separated; empty disables the transport). */
export const saveNostrRelays = (urls: string) =>
  invoke("save_nostr_relays", { urls }) as Promise<void>;

/** RC2: a dev-shareable diagnostics bundle for one swap (record + log lines),
 *  secrets scrubbed. Backs the per-swap "Dump logs" button. */
export const dumpSwap = (swapId: string) =>
  rpc<{ swap_id: string; pactd_version: string; record: unknown; log: string[] }>("dumpswap", [
    swapId,
  ]);

// ---- Private (off-market) offers — the Pact handbook (private offers) ------
// Thin wrappers over pactd RPCs. A private offer is built/signed/stored locally
// and handed to a friend as a `slip` string over their own chat; nothing is
// posted to a board. The friend's `takeOffer(slip)` routes into the same swap
// path as a board take.

/** Build a private offer; returns the pasteable slip string. give/want are
 *  `coin:amount`; t1/t2 are seconds (T2 < T1). `protocol` (optional) pins the
 *  swap type; omitted → the engine default. */
export const makePrivateOffer = (
  give: string,
  want: string,
  t1Secs: number,
  t2Secs: number,
  protocol?: string,
  ttlSecs?: number,
) =>
  rpc<{ slip: string }>(
    "makeprivateoffer",
    // protocol (param 4) + ttl_secs (param 5) optional; null at 4 sets the ttl
    // without forcing a protocol (opt_str ignores null).
    [give, want, t1Secs, t2Secs, protocol ?? null, ttlSecs ?? null],
  );

/** Take an offer from a pasted slip (decode + verify happen in pactd). */
export const takeOffer = (slip: string) => rpc<{ taken: boolean }>("takeoffer", [slip.trim()]);

/** The maker's outstanding private offers (local list). */
export const listPrivateOffers = () => rpc<{ offers: PrivateOffer[] }>("listprivateoffers");

/** Stop honoring a private offer's slip before its ttl lapses. */
export const cancelPrivateOffer = (offerId: string) =>
  rpc<{ cancelled: boolean }>("cancelprivateoffer", [offerId]);

// ---- app update check (GitHub releases, Phoenix pattern) -------------------
// Thin wrappers over the Satchel-native Rust commands. The check hits
// PoC-Consortium/satchel; failures (no releases yet / offline) reject and are
// treated as "no update" by the caller.

/** Update info from `check_app_update` (mirrors Rust `UpdateInfo`). */
export interface UpdateInfo {
  available: boolean;
  currentVersion: string;
  latestVersion: string | null;
  releaseUrl: string | null;
  releaseNotes: string | null;
  publishedAt: string | null;
}

/** Current app version (= satchel crate version) from the backend. */
export const getAppVersion = () => invoke("get_app_version") as Promise<string>;

/** Check GitHub for a newer release. Rejects on network/parse/no-release. */
export const checkAppUpdate = () => invoke("check_app_update") as Promise<UpdateInfo>;

/** Open an http(s) URL in the OS default browser (the webview blocks nav). */
export const openExternal = (url: string) => invoke("open_external", { url }) as Promise<void>;

/** Normalize a thrown value (string from Tauri, Error, anything) to a message. */
export function errMsg(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  return String(e);
}
