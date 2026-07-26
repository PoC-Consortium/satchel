# Local Contacts — design note

**Status:** design only. Post-rc6 (do **not** touch the current `rc6-swap-funding-fixes` branch).
**Scope:** Satchel-only. **Zero** changes to `pactd` / the Pact engine — this is personal UI metadata, not protocol state.

## 1. What it is

A private, local-only address book that maps a counterparty's cryptographic identity (the BIP340 hex
pubkey already shown everywhere as a `CounterpartyTag`) to a user-chosen **nickname**, an optional
**note**, and a **status** (`trusted` / `neutral` / `blocked`). It lets the user recognise repeat
counterparties and tag them — giving back the *soft, human* reputation signal that was deliberately
removed from the protocol (trust = atomicity only), without ever putting a trust claim on-chain or on a
relay.

### Non-goals (explicit)
- **Not protocol enforcement.** `blocked` only changes *your local view and warnings*. It cannot stop
  anyone from trading and the engine never sees it. We say so honestly in the UI.
- **Not synced / not published.** Stays on this machine. Never written to a relay, never signed, never
  attested. (One-line UI disclaimer: *"Contacts are stored locally on this device and never shared."*)
- **Not in `pactd`.** No new engine RPC. The engine stays protocol-pure.
- **Not messaging.** "Contact them if something goes wrong" is a *future* Nostr-DM feature, out of scope
  here. A note field is the only freeform text for now.

### Security invariant (carried from the codebase)
The deterministic identicon + `shortId()` fingerprint is the **spoof-proof** identity
(`CounterpartyTag.tsx:6-10`, `identity.ts:1-3`). A nickname is an *alias displayed alongside* it — it
**never replaces** the identicon/fingerprint. Otherwise a hostile counterparty could pick a pubkey you've
already nicknamed "alice" and impersonate her. Nick is decoration; the hash is identity.

## 2. Data model

A contact is keyed by the hex pubkey that already flows through `Offer.from` (`api/types.ts:289`) and
`Swap.counterparty_identity` (`api/types.ts:194`, `:256`).

```ts
// api/types.ts
export type ContactStatus = "trusted" | "neutral" | "blocked";

export interface Contact {
  id: string;            // BIP340 hex pubkey — the map key, never editable
  nick: string;          // user alias, may be ""
  note?: string;         // freeform; future "how to reach them"
  status: ContactStatus; // whitelist / neutral / blacklist
  added: number;         // epoch ms (stamped Satchel-side)
}

export type ContactBook = Record<string /* hex id */, Contact>;
```

## 3. Persistence — Satchel-side `satchel.json`, **not** localStorage

Two existing patterns (per UI survey):
- **`localStorage`** (`denom.tsx` shape) — fragile, wiped with webview data, view-prefs only.
- **`satchel.json`** (`prefs.tsx` + `get_ui_prefs`/`set_ui_prefs` in `satchel/src/main.rs:564,572`) —
  durable file in `<localdata>/satchel.json`, owned by the **Satchel** Tauri host (still **not** the Pact
  engine).

**Decision: use the `satchel.json` path.** A contact book is durable user data, not a toggle — it should
survive a webview cache clear. Mirror the prefs commands:

- Rust (`satchel/src/main.rs`): add `get_contacts` / `set_contacts` Tauri commands next to
  `get_ui_prefs`/`set_ui_prefs`, persisting a `contacts` key inside the same `satchel.json`.
- TS: a `ContactsProvider` mirroring `prefs.tsx` — loads on mount via `get_contacts`, writes through on
  every mutation. Expose `useContacts()`:

```ts
useContacts(): {
  book: ContactBook;
  get(id: string): Contact | undefined;
  setNick(id: string, nick: string, note?: string): void;
  setStatus(id: string, status: ContactStatus): void;
  remove(id: string): void;
}
```

> Trade-off being accepted (from [[funding-spike-hardening]] precedent): feepolicy/board config are
> *engine-store-owned*; contacts deliberately are **not**, so they live with the Satchel install on this
> machine rather than travelling with the seed. That's the right call for personal, single-device notes.
> If contacts ever need to follow the seed, that's the moment to promote them to an engine store — not now.

## 4. UI surfaces

### 4a. Click-on-counterparty menu (the primary entry point)
Wrap `components/CounterpartyTag.tsx` with a new `components/CounterpartyMenu.tsx` using the established
**MUI `Menu`** recipe (`Header.tsx:145-187` / `LanguageMenu.tsx:71-97` — `useRef` anchor + `useState`
open flag; there is no `onContextMenu`/`Popover` usage in the tree, so a left-click menu fits the codebase).

Menu items (all new `contacts:` i18n keys):
- **Add / Edit nickname…** → small inline dialog (nick + note). Uses `useApp().showToast` on save.
- **Mark trusted** / **Mark neutral** / **Block** (radio-style, reflects current `status`).
- **Copy full public key**.
- **Open in Contacts** → `navigate("contacts")` via `useNavigate()` (`ui/nav.tsx`).

`CounterpartyTag` itself gains optional rendering (purely additive, fingerprint untouched):
- nickname shown **after** the `shortId` fingerprint when a contact exists;
- a small status adornment — `trusted` = star/check, `blocked` = slash/danger tint.

Attach the menu at the existing render sites — they already pass the hex id:
- Corkboard cards `CorkboardScreen.tsx:888`
- Swaps ledger `SwapsScreen.tsx:170,173`
- Active-swaps dock `ActiveSwaps.tsx:176,180`
- Take-confirm `hooks/useTakeConfirm.tsx`

### 4b. Contacts tab (management screen)
- New `Route` `"contacts"` (`Sidebar.tsx:45`), a `NavDef` in the `ACTIVITY` section (`Sidebar.tsx:76-80`)
  with an MUI contacts icon + `nav.contacts` key, and a `case "contacts": return <ContactsScreen/>` in
  `App.tsx:70-89`.
- `screens/ContactsScreen.tsx`: searchable table — identicon + fingerprint + nick + note + status chip +
  added date; inline edit; delete via the existing `useConfirm()` helper (`ui/ConfirmProvider.tsx`).
  Filter chips: All / Trusted / Blocked. Top-of-screen the local-only privacy disclaimer.

### 4c. Blacklist / whitelist behaviour (display + soft-warn only)
- **Corkboard** (`CorkboardScreen.tsx`): a **"Hide blocked"** toggle (default on). When off, blocked
  makers' offers render dimmed/collapsed with a "blocked" tag rather than disappearing (auditability).
  `trusted` makers get a subtle badge so good repeat counterparties stand out in the ladder.
- **Take-confirm** (`useTakeConfirm.tsx`): if the counterparty is `blocked`, show a red warning banner
  ("You blocked this counterparty") and require an extra confirm. It does **not** hard-block — honest about
  the fact that atomicity, not this list, is what protects the trade.

## 5. i18n
Add a `contacts:` block to `i18n/en.ts` (source of truth) plus `nav.contacts`. New user-visible copy must
be keys, not literals (`npm run lint` enforces `i18next/no-literal-string`). The 26 sibling locale files
are typed for completeness — they'll need the same keys (can ship English fallback first).

## 6. Suggested phasing (all post-rc6)
1. **Nicknames** — model + `ContactsProvider` + `get/set_contacts` commands + `CounterpartyMenu` (add/edit
   nick) + nick rendering in `CounterpartyTag`. *This is the cheap, high-value core — ship this alone if
   nothing else.*
2. **Contacts tab** — `ContactsScreen` + nav wiring.
3. **Whitelist/blacklist** — `status` field surfacing: corkboard dim/hide + trusted badge + take-confirm
   warning.

## 7. Files touched (summary)
| File | Change |
|---|---|
| `satchel/src/main.rs` | `get_contacts` / `set_contacts` commands; `contacts` key in `satchel.json` |
| `ui/src/api/types.ts` | `Contact`, `ContactStatus`, `ContactBook` |
| `ui/src/contacts.tsx` *(new)* | `ContactsProvider` + `useContacts()` (mirror `prefs.tsx`) |
| `ui/src/components/CounterpartyTag.tsx` | additive nick + status adornment |
| `ui/src/components/CounterpartyMenu.tsx` *(new)* | click menu |
| `ui/src/screens/ContactsScreen.tsx` *(new)* | management tab |
| `ui/src/components/Sidebar.tsx`, `App.tsx` | route + nav wiring |
| `ui/src/screens/CorkboardScreen.tsx`, `components/ActiveSwaps.tsx`, `screens/SwapsScreen.tsx`, `hooks/useTakeConfirm.tsx` | attach menu / status display |
| `ui/src/i18n/en.ts` (+ 26 locales) | `contacts:` + `nav.contacts` keys |

**Engine / `pactd`: untouched.**
