# Screenshot capture list

The Satchel User Handbook references the screenshots below. They are **not yet
captured** — each chapter carries a placeholder image reference, and the build
will show a missing-image box until the file exists in `images/processed/`.

Capture each on a clean **regtest** Satchel (use the playground:
`./tools/playground-cork.ps1`) so no real funds or live seeds appear. Use a
throwaway seed for any phrase shown on screen. Save each as a PNG at the exact
path below, then rebuild with `./build.ps1`.

> **Warning** — Never capture a screenshot that shows a real recovery phrase, a
> real passphrase, or a mainnet balance. Use regtest and throwaway seeds only.

| File (`images/processed/…`) | Chapter | Screen | Required app state |
|---|---|---|---|
| `ch02-architecture.png` | 2 — What Satchel Is | Conceptual diagram (the three parts) | A diagram, not a literal screenshot — may be drawn rather than captured |
| `ch04-merchant-name.png` | 4 — First Launch | First-run wizard, "Merchant name" step | Name field filled (e.g. "Main") |
| `ch04-seed-reveal.png` | 4 — First Launch | SeedForm reveal step | Recovery phrase shown, numbered grid — **throwaway regtest seed only** |
| `ch05-coins-screen.png` | 5 — Setting Up Coins | Settings → Coins / coin wizard | One coin **Connected** (tip shown), one **Not set up** |
| `ch05-coin-setup.png` | 5 — Setting Up Coins | CoinSetup dialog | **Cookie file** auth selected, Node data directory field visible |
| `ch05-validate.png` | 5 — Setting Up Coins | CoinSetup dialog, post-validation | "Genesis matched" with tip height, **Save** enabled |
| `ch05-pairs.png` | 5 — Setting Up Coins | Coins screen, Trading pairs list | BTCX ↔ BTC reading "Ready to trade" |
| `ch06-overview.png` | 6 — A Tour | Full main window | Corkboard view, left nav + header visible |
| `ch06-status.png` | 6 — A Tour | Header status indicators | Engine green, relays + coins healthy, ≥1 live swap so the counter badge shows |
| `ch07-corkboard.png` | 7 — The Corkboard | Order-book ladder | BTCX/BTC pair with bids and asks populated |
| `ch07-detail.png` | 7 — The Corkboard | Detail pane | A price level selected, offers listed with both protocol chips and refund times |
| `ch08-offer-form.png` | 8 — Making an Offer | Post an offer form | Coins / price / swap-type / timelock filled, balances showing |
| `ch09-take-confirm.png` | 9 — Taking an Offer | Take-offer confirmation dialog | Open over the Corkboard |
| `ch10-swaps.png` | 10 — Tracking Swaps | Swaps page | Both **In flight** and **History** sections populated |
| `ch10-dock.png` | 10 — Tracking Swaps | Active-swaps dock (any page) | A live swap card with a state-gated action button visible |
| `ch11-wallets.png` | 11 — Your Wallets | Wallets page | At least two coin cards with balances |
| `ch12-create-slip.png` | 12 — Private Offers | Create slip | Generated `pactoffer1:` slip box visible |
| `ch12-my-slips.png` | 12 — Private Offers | My slips | At least one outstanding slip with expiry countdown and **Cancel** |
| `ch13-network-tab.png` | 13 — Transports | Settings → Network tab | Nostr relays list (6 prewired) and Corkboards list visible |
| `ch14-settings-tabs.png` | 14 — Settings | Settings screen | All four tabs visible (General / Coins / Network / About) |

20 screenshots total. Filenames use the `chNN-` chapter prefix so assets stay
grouped with their chapter.
