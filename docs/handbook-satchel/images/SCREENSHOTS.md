# Screenshot capture list

The Satchel User Handbook references the screenshots below. Status: ✅ = real
capture in `images/processed/`; ⬜ = still the auto-generated placeholder;
🔁 = a raw capture exists but needs a re-shoot (see note). Capture progress
2026-07-27: 10/25 done.

Capture each on a clean **regtest** Satchel (use the playground:
`python -m play` from `pact/harness`) so no real funds or live seeds appear. Use a
throwaway seed for any phrase shown on screen. Save each as a PNG at the exact
path below, then rebuild with `./build.ps1`.

> **Warning** — Never capture a screenshot that shows a real recovery phrase, a
> real passphrase, or a mainnet balance. Use regtest and throwaway seeds only.

| ✔ File (`images/processed/…`) | Chapter | Screen | Required app state |
|---|---|---|---|
| ⬜ `ch02-architecture.png` | 2 — What Satchel Is | Conceptual diagram (the three parts) | A diagram, not a literal screenshot — may be drawn rather than captured |
| ✅ `ch04-merchant-name.png` | 4 — First Launch | First-run wizard, "Merchant name" step | Name field filled (e.g. "Main") |
| ✅ `ch04-seed-reveal.png` | 4 — First Launch | SeedForm reveal step | Recovery phrase shown, numbered grid, **12/24-word toggle** above the grid — **throwaway regtest seed only** |
| ✅ `ch05-coins-screen.png` | 5 — Setting Up Coins | First-run "Connect your coins" dialog | One coin **Live**, others showing **Set up** — captured from the onboarding flow |
| ✅ `ch05-coin-setup.png` | 5 — Setting Up Coins | CoinSetup dialog | Electrum vs own-node cards + both auth options (Cookie file / User-password) visible |
| ✅ `ch05-validate.png` | 5 — Setting Up Coins | CoinSetup dialog, post-validation | "Genesis matched" with tip height, **Save** enabled |
| ✅ `ch05-pairs.png` | 5 — Setting Up Coins | Coins screen, Trading pairs list | BTCX ↔ BTC reading "Ready to trade" |
| ✅ `ch06-overview.png` | 6 — A Tour | Full main window | Corkboard view, populated ladder, Cashrate chip left of the merchant chip |
| ✅ `ch06-status.png` | 6 — A Tour | Header status indicators | Engine green, relays + coins healthy, live-swap counter badge showing |
| ⬜ `ch06-network.png` | 6 — A Tour | Network monitor (left-nav screen), Nostr tab — screen renamed from “Relays” | A few Nostr relays connected (green dots), latency + uptime shown |
| ✅ `ch07-corkboard.png` | 7 — The Corkboard | Order-book ladder | Captured from the LIVE mainnet board (public content only) — Cashrate on; `ch07-cashrate.png` differs by its open popover |
| ⬜ `ch07-cashrate.png` | 7 — The Corkboard | Cashrate chip + popover | Corkboard on BTCX/BTC, Cashrate **enabled** with a rate set for the quote coin; header chip reading `~ 91,400.00 · BTC` with the popover **open** (toggle + rate field); ladder showing the inner ~Cash columns |
| ✅ `ch07-detail.png` | 7 — The Corkboard | Detail pane | Price level selected; offers with protocol chip, timelocks, expiry, Withdraw |
| ⬜ `ch08-offer-form.png` | 8 — Making an Offer | Post an offer form (rebuilt) | Pair selected, Sell/Buy direction, base amount, quote-per-base price + denom dropdown, swap type, timelock — balances showing |
| ⬜ `ch09-take-confirm.png` | 9 — Taking an Offer | Take-offer confirmation dialog | Open over the Corkboard |
| ⬜ `ch10-swaps.png` | 10 — Tracking Swaps | Swaps page | Both **In flight** and **History** sections populated |
| ⬜ `ch10-dock.png` | 10 — Tracking Swaps | Active-swaps dock (any page) | A live swap card with a state-gated action button visible |
| ⬜ `ch11-wallets.png` | 11 — Your Wallets | Wallets page | At least two coin cards with balances |
| ⬜ `ch11-send-dialog.png` | 11 — Your Wallets | Send dialog (Electrum coin) | Recipient + amount fields with the **Max** button visible; fee selector showing **Slow / Normal / Fast** presets priced in sat/vB plus **Custom**; fee-preview line below |
| ⬜ `ch12-create-slip.png` | 12 — Private Offers | Create slip | Generated `pactoffer1:` slip box visible |
| ⬜ `ch12-my-slips.png` | 12 — Private Offers | My slips | At least one outstanding slip with expiry countdown and **Cancel** |
| ⬜ `ch13-network-tab.png` | 13 — Transports | Settings → Network tab | Nostr relays list (6 prewired) and Corkboards list visible |
| ⬜ `ch14-settings-tabs.png` | 14 — Settings | Settings screen | All **six** tabs visible (General / Coins / Network / Fees / Notifications / About) |
| ⬜ `ch14-fees-tab.png` | 14 — Settings | Settings → Fees tab | The three fee-bumping knobs with Save and Reset to defaults |
| ⬜ `ch14-notifications-tab.png` | 14 — Settings | Settings → Notifications tab | Master switch **on**, all five event toggles visible, **Send a test notification** button |

25 screenshots total. Filenames use the `chNN-` chapter prefix so assets stay
grouped with their chapter. If a screen is recaptured with Cashrate **on**,
the offer form, take-confirm, and create-slip shots gain `~Cash` lines — either
is fine, but keep the set consistent.
