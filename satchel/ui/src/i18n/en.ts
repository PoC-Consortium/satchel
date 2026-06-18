// The English string bundle — the single source of UI-chrome copy. Adding a
// language later is just another bundle of the same shape. Two kinds of text are
// out of scope for this bundle: pactd `narrate()` lines (backend-generated,
// shown verbatim) and a few technical placeholders (URLs, the "QUIT" confirm
// word). Everything a user reads should otherwise live here.
//
// Keys are dot-addressed (`t("nav.corkboard")`). `{name}`-style placeholders are
// filled by the `vars` argument to `t()`.
//
// Naming note: the daemon is presented to users as "the engine" (its real name,
// pactd, only appears in the advanced external-connect path where it's typed).
export const en = {
  app: {
    name: "Satchel",
    tagline: "trustless swaps",
  },
  // In-app update notifications (sidebar version badge + dialog).
  update: {
    title: "Update available",
    upToDate: "You're up to date",
    current: "Installed",
    latest: "Latest",
    notesTitle: "Release notes",
    get: "Get the update",
    dismiss: "Dismiss",
    close: "Close",
    badgeTooltip: "Update available — click for details",
    versionTooltip: "Click to check for updates",
  },
  // Risk disclaimer (first-run welcome + Settings → About).
  disclaimer: {
    title: "Experimental — tech demo",
    body: "Satchel is early alpha software for a regtest tech demo — do not use it with real funds. Swaps are non-custodial but unaudited (especially the v2 Taproot/MuSig2 path), and a merchant's seed holds hot transit keys. MIT-licensed and provided as-is, with no warranty: use at your own risk.",
  },
  nav: {
    public: "Public",
    corkboard: "Corkboard",
    postOffer: "Post an offer",
    private: "Private",
    privateCreate: "Create slip",
    privateReceive: "Take a slip",
    privateSlips: "My slips",
    swaps: "Swaps",
    wallets: "Wallets",
    settings: "Settings",
    coins: "Coins",
  },
  makeOffer: {
    title: "Post an offer",
    intro:
      "Post a signed offer to the Corkboard. Nothing is locked — it's just an advert; withdraw any time, and a swap only starts when someone takes it and both sides fund.",
    give: "You give",
    want: "You receive",
    // Price-assisted entry: price + either amount fills the other (bidirectional).
    price: "Price",
    priceUnit: "{quote} per {give}",
    pricePlaceholder: "unit price",
    balance: "Balance: {amt} {sym}",
    balanceLoading: "Balance: …",
    noCoins: "No coins configured",
    sameCoin: "Give and receive must be different coins.",
    // Swap-protocol pin (only offered when a pair+network supports more than
    // one). v2 label reuses coins.protoPrivate ("Private (Taproot)").
    protocol: "Swap type",
    protoStandard: "Standard (HTLC)",
    // Titles for the review/confirm dialog shown before posting (see OfferForm).
    reviewOfferTitle: "Review your offer",
    reviewSlipTitle: "Review your slip",
    term: "Safety timelock",
    termShort: "Short",
    termMedium: "Medium",
    termLong: "Long",
    termHint: {
      short: "Short — funds auto-refund fastest if the trade stalls (~2h / 1h), but the smallest safety margin.",
      medium: "Medium — balanced refund window (~8h / 4h).",
      long: "Long (safest) — widest safety margin; auto-refund after ~24h / 12h if the trade stalls.",
    },
    validFor: "Valid for (minutes)",
    validForMins: "{mins} min",
    validForHint:
      "How long the offer stays listed. While you're online it's kept fresh automatically; after this it expires. Closing the app withdraws it.",
    note: "Fixed-size offer — nothing's locked until someone takes it. Amounts are on-chain; you pay network fees on top and the Corkboard charges nothing. The timelock is the auto-refund window if a swap stalls.",
    post: "Post offer",
    makeSlip: "Create slip",
    slipTitle: "Your private offer slip",
    slipExplainer:
      "Send this to your friend. They paste it into Satchel to take it. Nothing is locked; it expires in {ttl}.",
    copy: "Copy",
    copied: "Copied",
    makeAnother: "Make another",
    myPrivateTitle: "My private offers",
    myPrivateEmpty: "No outstanding private offers.",
    privateExpires: "expires {when}",
    privateExpired: "expired",
    cancel: "Cancel",
    cancelTip: "Stop honoring this slip — a friend who still holds it can no longer take it.",
  },
  takeSlip: {
    open: "Paste a slip",
    title: "Take a private offer",
    intro:
      "A friend sent you a private offer slip (it starts with pactoffer1:). Paste it here to review and take it — exactly like an offer from the board.",
    placeholder: "pactoffer1:…",
    take: "Review & take",
    invalid: "That doesn't look like a slip — it should start with pactoffer1:.",
  },
  // PRIVATE nav group (off-market slips) — screen titles + intros. The form,
  // slip output, my-slips list and take flow reuse makeOffer.*/takeSlip.* copy.
  private: {
    createTitle: "Create a private offer",
    createIntro:
      "Build a signed offer and hand it to a friend as a slip over your own chat. Nothing is listed anywhere — and nothing is locked until both of you fund.",
    slipsIntro:
      "Slips you've created. Anyone holding a slip can take it until it expires; cancel one to stop honoring it before then.",
    slipsEmptyBody: "Create a private offer to get a slip you can send to a friend.",
    receiveTitle: "Take a private offer",
    received: "Taken — follow it in Swaps.",
  },
  // Shared take-confirmation dialog (board take + slip take).
  takeConfirm: {
    title: "Take this offer?",
    confirm: "Take offer",
    counterparty: "Counterparty",
    youGive: "You give",
    youReceive: "You receive",
    safetyRefund: "Safety refund",
    offerAge: "Offer age",
    makerFundsFirst:
      "The maker locks their {sym} first — you never send first. You can still cancel before you fund your side, and the engine auto-refunds after the safety timelock if the swap stalls.",
  },
  header: {
    activeMerchant: "Active merchant — click to switch or manage",
    manageMerchants: "Manage Merchants…",
    noMerchant: "no merchant",
    openMenu: "Open menu",
    collapseMenu: "collapse menu",
    settings: "Settings",
    language: "Language",
    pactConnected: "Engine connected",
    pactUnreachable: "Engine unreachable",
    liveSwapsOne: "1 swap in flight — click to view",
    liveSwapsMany: "{count} swaps in flight — click to view",
    liveSwapsNone: "No swaps in flight",
    coinOk: "{name} — connected · tip {tip}",
    coinUnconfigured: "{name} — not set up",
    coinError: "{name} — {status}",
    relaysOk: "Nostr relays — {up}/{total} connected",
    relaysDown: "Nostr relays — none of {total} connected",
  },
  network: {
    mainnet: "MainNet",
    testnet: "TestNet",
    regtest: "RegTest",
    signet: "Signet",
    notRealFunds: "Not real funds — this is the {network} network",
  },
  merchants: {
    title: "Your merchants",
    intro:
      "A merchant is one trading identity — its own seed and swap history. Trading under a different merchant keeps contexts unlinkable (a burner identity). Your main coins live in your own wallet, not here.",
    // First-run welcome (empty merchant manager).
    welcomeTitle: "Welcome to Satchel",
    welcomeIntro:
      "Satchel trades under a “merchant” — one trading identity with its own seed. You have none yet: create a fresh one, or import an existing recovery phrase to get started.",
    importMerchant: "Import a merchant",
    none: "No merchants yet.",
    active: "active",
    switch: "switch",
    newMerchant: "New merchant",
    thisMerchant: "this merchant",
    nameLabel: "Merchant name",
    namePlaceholder: "e.g. Main",
    introFirst:
      "Set up your first trading identity (a “merchant”). It holds only hot transit keys for in-flight swaps — your main coins stay in your own wallet.",
    introNew: "A new merchant is a fresh, separate identity with its own seed and swap history.",
    // UI-5 merchant selector (phoenix wallet-selector parity).
    createNew: "Create new",
    import: "Import",
    load: "Load Merchant",
    loaded: "loaded",
    locked: "locked",
    lockedTip: "Encrypted seed — unlock with your passphrase when you load it.",
    close: "Close",
    idLabel: "folder",
    switching: "Switching merchant…",
    switchingBody: "Relaunching the engine against that folder.",
  },
  // Seed create/import (SeedForm) + counterparty + status.
  seed: {
    intro: "Create a brand-new seed, or import one you already have.",
    createNew: "Create new",
    createDesc: "Generate a fresh seed. You back up the recovery phrase.",
    import: "Import",
    importDesc: "Restore from an existing 12/24-word phrase.",
    recoveryLabel: "Recovery phrase",
    importPlaceholder: "word1 word2 word3 …",
    encrypt: "Encrypt",
    encryptDesc:
      "A passphrase protects the seed at rest. You enter it once per session — Satchel never stores it. Note: unattended auto-refund pauses after a restart until you re-enter it.",
    noPassphrase: "No passphrase (recommended)",
    noPassphraseDesc:
      "Auto-refund keeps working through reboots with nothing to enter — this is only a hot transit seed. Cost: file/host access exposes this merchant's transit keys + identity.",
    passphraseLabel: "Passphrase",
    passphrasePlaceholder: "choose a passphrase",
    createTitle: "Create seed",
    importTitle: "Import seed",
    secureTitle: "Secure {label}",
    revealTitle: "Write down your recovery phrase",
    revealBody:
      "Anyone with these words controls this merchant's hot keys. Satchel keeps no copy — store it offline. You'll confirm a few words next.",
    ackLabel: "I have written down my recovery phrase.",
    // Multi-step onboarding (create/import -> secret -> confirm -> passphrase).
    chooseTitle: "Set up {label}",
    enterTitle: "Import your recovery phrase",
    enterBody: "Paste the 12 or 24-word BIP39 phrase for the seed you're restoring.",
    verifyTitle: "Confirm your backup",
    verifyBody: "Type the words at these positions to confirm you wrote the phrase down.",
    verifyWord: "Word #{n}",
    verifyMismatch: "Those don't match your phrase — check your backup.",
    passphraseTitle: "Protect the seed",
    passphraseBody:
      "Optionally encrypt the stored seed with a passphrase. You can skip this — see the trade-off below.",
  },
  counterparty: {
    you: "This is you",
    youShort: "you",
    unknown: "unknown identity",
  },
  status: {
    notConnectedTitle: "Not connected to the engine",
    disconnectedBody:
      "Satchel can't reach the engine. It may still be starting, or the active merchant's node connections may be down. Retry, or switch merchant from the selector up top.",
    openInSatchel: "Open this in Satchel",
    noTauriBody:
      "This is Satchel's UI — it needs the Tauri bridge to reach the engine. Launch the desktop app (cargo tauri dev) rather than a browser.",
  },
  settings: {
    title: "Settings",
    subtitle: "App-wide preferences for this install.",
    // UI-3 Settings tabs.
    tabGeneral: "General",
    tabCoins: "Coins",
    tabNetwork: "Network",
    tabAbout: "About",
    appearance: "Appearance",
    theme: "Theme",
    themeDark: "Dark",
    themeLight: "Light",
    themeSystem: "System",
    themeHint: "Choose how Satchel looks. System follows your OS setting.",
    language: "Language",
    languageHint: "More languages land as translations are contributed.",
    network: "Network",
    networkHint:
      "One coherent mode for this client — every coin runs on this network. Chosen when the merchant was created; mainnet is gated.",
    boards: "Corkboards",
    boardsNone: "None configured",
    boardsConfigure: "Configure",
    nostrRelays: "Nostr relays",
    nostrRelaysOff: "Off — using corkboard only",
    coins: "Coins & nodes",
    coinsHint: "Connect each coin to your own node. Genesis is checked before anything is saved.",
    about: "About",
    version: "Version {version}",
    updateUpToDate: "Up to date",
    updateCheckPlaceholder: "Update check arrives in a later release.",
    trustModel: "Where your keys live",
    trustModelBody:
      "Secrets live in the engine, never in Satchel. The merchant seed sits in the engine's data folder (encrypted or plaintext — your choice); Satchel stores no seed or passphrase. The seed is hot by design (transit keys only) — sweep sizable proceeds to your own cold wallet.",
  },
  coins: {
    intro:
      "Connect each coin to your own node. The first URL is your node's own wallet — it funds your swap legs and receives the proceeds. Before anything is saved, Satchel checks the node's genesis block so funds can never be sent to the wrong chain. Connections are shared across all your merchants.",
    networkBadge: "Configuring for the {network} network",
    needMerchant:
      "Connect a merchant first — coin setup needs the engine running. Use the merchant selector at the top right.",
    pairsTitle: "Trading pairs",
    pairsHint:
      "Pairs are derived from what each coin can do — there is no fixed list. A pair opens once both of its coins are connected.",
    noPairs: "No pairs available.",
    notSetUp: "Not set up",
    connectedTip: "Connected · tip {tip}",
    connError: "Connection error",
    setUp: "Set up",
    editConnection: "Edit connection",
    remove: "remove",
    disconnectTip: "Disconnect this coin",
    disconnectTitle: "Disconnect {coin}?",
    disconnectBody: "Swaps needing it won't be available until you reconnect.",
    ready: "Ready to trade",
    connectMissing: "Connect {coins}",
    notBuildable: "Not buildable yet",
    // Swap-protocol chips on a pair (pact-htlc-v1 HTLC vs pact-htlc-v2 adaptor).
    protoPrivate: "Private (Taproot)",
    protoPrivateTip: "Private swap (Taproot/MuSig2 adaptor) — looks like an ordinary payment on-chain",
    protoHtlcTip: "Classic HTLC swap",
    // Coin-setup backend choices (CoinSetup).
    backendCoreTitle: "Core RPC wallet",
    backendCoreDesc: "Your node's own wallet funds the swap and receives the proceeds.",
    backendHardwareTitle: "Hardware",
    backendHardwareDesc: "Ledger / PSBT signing for the funding leg.",
    backendLater: "later",
    // CoinSetup dialog.
    setupTitle: "Connect {coin}",
    setupIntro:
      "Point Satchel at your own {sym} node. Nothing is saved until the node passes a genesis-block check — your funds only ever touch the real {sym} chain.",
    backendUrlLabel: "Node backend URL(s)",
    backendUrlHint:
      "First URL = your node's own wallet (funds swaps, receives proceeds). Add Electrum servers (tcp://host:port) after commas for extra, independent chain views.",
    fundingWallet: "Funding wallet",
    confirmationsLabel: "Confirmations before final",
    confirmationsHint:
      "How deep a funding or redeem on this chain must be before a swap acts on it — the reorg-safety margin. Higher is safer but slower; leave blank for the recommended default ({default}).",
    validateNode: "Validate node",
    checking: "Checking the node…",
    genesisOk: "Genesis matched — this is the right chain",
    genesisDetail: "tip height {tip} · genesis {hash}…",
    genesisBad: "Rejected — not saving",
    errorShort: "error",
  },
  wallets: {
    intro:
      "These are the wallets of your own nodes (the ones the engine uses to fund swaps and receive proceeds) — your keys, your machine. Satchel never holds your coins.",
    hotSeedNudge:
      "This is a spending wallet on a hot seed, not a vault — sweep sizable balances to your own cold/core wallet.",
    notConnected: "Not connected",
    notConnectedBody: "Connect a merchant first — the wallet view needs the engine running.",
    noCoins: "No coins set up yet",
    noCoinsBody: "Connect a coin in Settings → Coins and its wallet appears here.",
    goToCoins: "Go to Coins",
    balanceLabel: "{symbol} balance",
    receive: "Receive",
    send: "Send",
    sendTo: "Send to address",
    amount: "Amount",
    sendTitle: "Send {amount} {sym}?",
    sendConfirmBody: "To {to}\n\nThis spends from your own node's wallet and cannot be undone.",
  },
  corkboard: {
    noBoardTitle: "No Corkboard connected",
    noBoardBody:
      "A Corkboard is a shared bulletin board where makers pin offers. It never matches trades or holds coins — point Satchel at one you trust to browse and post.",
    noPairs: "No pairs available",
    board: "Corkboard",
    boardSettings: "Configure in Settings",
    filterAll: "All",
    filterMine: "Mine",
    mine: {
      emptyTitle: "You haven't posted any offers",
      emptyBody: "Offers you post appear here to manage and withdraw. Post one to get started.",
      expiry: "Live — refreshes automatically; drops in ~{cur} if you go offline · ends in {fin}",
      state: {
        live: "Live",
        taken: "Taken",
        revoked: "Withdrawn",
        expired: "Expired",
      },
    },
    offered: "{symbol} offered",
    noOffers: "No offers you can take right now",
    noOffersBody:
      "Offers show up here as soon as a maker posts one for a pair you've set up. You can also post your own.",
    hiddenOffers:
      "{count} more offer(s) for pairs you haven't connected. Set up both coins to trade them:",
    yourOffer: "your offer",
    take: "Take offer",
    withdraw: "Withdraw",
    withdrawTip: "Withdraw instantly — an offer never locks funds",
    safetyRefund: "safety refund",
    safetyRefundTip:
      "If the swap stalls, both sides auto-refund — the taker's leg unlocks first, yours a little later. Nobody ends up stuck.",
    activeTitle: "Your active swaps",
    states: {
      open: "open",
      takenByUs: "taken by you",
      revoked: "withdrawn",
      expired: "expired",
    },
    // Two-sided order-book view of the Corkboard. Bids = makers giving the
    // quote coin to get the base; asks = the reverse. The ladder is a way to
    // READ the board — it never matches or prioritises (load-bearing).
    book: {
      bids: "Bids",
      asks: "Asks",
      bidsHint: "want {base} · paying {quote}",
      asksHint: "selling {base} · for {quote}",
      price: "Price",
      size: "Size",
      noBids: "No bids",
      noAsks: "No asks",
      spread: "Spread {pct}",
      spreadOneSided: "One-sided",
      crossed: "crossed",
      crossedTip: "Top bid ≥ top ask. The board never auto-matches, so these overlapping offers just sit there — take either side.",
      mid: "mid {price}",
      levelOffers: "{count} offer(s) at this price — pick one to take",
      depthTip: "Total {sym} on offer at this price across {count} notice(s).",
      takerNote: "Taking it, you give {give} and receive {get}.",
      selectLevel: "Pick a price level above to see the offers there.",
      paneHeader: "{size} {base} @ {price} {unit}",
      denomTip: "Display unit for {coin} amounts",
      showMore: "Show {count} more",
      showLess: "Show top {count}",
    },
  },
  swaps: {
    title: "Swaps",
    hint: "Your full ledger — in-flight swaps on top, finished trades below. You can also act on live swaps from the Corkboard.",
    activeTitle: "In flight",
    historyTitle: "History",
    none: "No swaps yet — take an offer on the Corkboard.",
    // Active-swaps dock action buttons + the refund-time label.
    cancel: "cancel",
    refund: "refund",
    refundAt: "refund {when}",
    // Confirm dialogs for acting on a live swap (ActiveSwaps).
    cancelTitle: "Cancel this swap?",
    cancelConfirm: "Cancel swap",
    cancelKeep: "Keep it",
    cancelBody:
      "This abandons the swap before you've funded. Nothing of yours is locked yet, so you lose nothing — the offer just won't complete.",
    refundTitle: "Pull your funds back?",
    refundConfirm: "Refund",
    refundBody:
      "The safety timelock has passed, so you can reclaim the funds you locked. This broadcasts your refund now; the engine also does it automatically after the deadline.",
    col: {
      swap: "swap",
      role: "role",
      state: "state",
      amounts: "gives → receives",
      when: "when",
      finalTx: "final tx",
    },
    // Expandable per-leg on-chain detail (the audit trail). We show both
    // funding txs + OUR settlement; never the counterparty's settlement or the
    // swap secret.
    audit: {
      toggle: "Show on-chain detail",
      title: "On-chain detail",
      youLocked: "you locked",
      theyLocked: "they locked",
      funding: "Funding",
      received: "Received",
      refunded: "Refunded",
      pending: "not yet on-chain",
      copy: "Copy transaction id",
      copied: "Transaction id copied",
    },
  },
  fees: {
    title: "Network cost preview",
    estimated: "estimated",
    provisionalNote: "This pactd build doesn't expose fee estimation yet.",
    summary: "A swap is 2 on-chain transactions you pay for: funding on the give-chain, redeem on the receive-chain.",
    fallbackTip: "A node was unreachable, so a conservative default fee rate was used — treat these as a guess.",
  },
  wizard: {
    welcome: "Welcome to Satchel",
    connectTitle: "Connect the Pact engine",
    connectIntro:
      "Satchel is a thin client of the Pact engine — the core that holds your keys and runs the swaps. Choose how to reach it.",
    managed: "Run the built-in Pact engine",
    managedDesc: "Satchel launches and supervises its own Pact engine. Recommended.",
    external: "Connect to an external Pact engine",
    externalDesc: "Point at a Pact engine you already run (set SATCHEL_PACTD_URL + cookie before launch).",
    externalNote:
      "External mode is selected via environment variables before launching Satchel. Relaunch with SATCHEL_PACTD_URL set to use it.",
    coinsTitle: "Add your coins",
    coinsIntro:
      "After your merchant is created, connect each coin to your own node in Settings → Coins. Pick a coin and a backend (public Electrum for zero-setup, or your own node); genesis is checked against this network before anything saves.",
    coinsTemplatesSoon: "One-click coin templates land here in a later release.",
    back: "Back",
    continue: "Continue",
    finish: "Finish setup",
  },
  // UI-4 docked activity log.
  log: {
    title: "Activity",
    empty: "— activity log —",
    count: "{count} lines",
    collapse: "Collapse log",
    expand: "Expand log",
  },
  exit: {
    // Exit-gate dialog (fund safety, C6). The engine manages alone, so "keep
    // running" detaches it (it keeps watching timelocks + servicing offers).
    liveTitle: "A swap is in flight",
    liveBodyOne:
      "1 swap is mid-flight. It's governed by on-chain timelocks — the engine must keep running to redeem or refund before the deadline.",
    liveBodyMany:
      "{count} swaps are mid-flight. They're governed by on-chain timelocks — the engine must keep running to redeem or refund before the deadline.",
    keepRunningExplain:
      "Closing the window keeps the engine running in the background, so it finishes the swap headless. You can reopen Satchel any time to check on it.",
    forceQuitWarn: "Force-quitting now stops the engine and can lose funds.",
    typeToConfirm: "To force-quit anyway, type QUIT below.",
    keepRunning: "Keep running, close window",
    keepWithdraw: "Keep running + withdraw offers",
    keepLeaveOffers: "Keep running, leave offers up",
    forceQuit: "Force-quit",
    offersTitle: "You have offers posted",
    offersBodyOne:
      "1 offer of yours is still on the Corkboard. Offers lock nothing, but leaving it up means counterparties can still take it while Satchel is closed — the engine will service the take.",
    offersBodyMany:
      "{count} offers of yours are still on the Corkboard. Offers lock nothing, but leaving them up means counterparties can still take them while Satchel is closed — the engine will service the takes.",
    withdrawExit: "Withdraw all & exit",
  },
  boards: {
    configTitle: "Your Corkboard",
    configIntro:
      "A Corkboard is a shared bulletin board for offers. Satchel reads and posts here and relays the coordination — it never matches trades or holds funds. Use any Corkboard you trust; comma-separate several for redundancy.",
    urlLabel: "Board URL(s)",
    save: "Save & reconnect",
    nostrHeading: "Nostr relays (optional)",
    nostrIntro:
      "Nostr relays run the same noticeboard over a decentralized network instead of a single server — offers and the sealed coordination ride public relays, with no operator able to read or match them. Leave empty to keep the transport off.",
    nostrLabel: "Nostr relay URL(s)",
    nostrRecommend: "Use recommended relays",
  },
  unlock: {
    title: "Unlock merchant",
    body:
      "This merchant's seed is encrypted. Enter its passphrase to unlock it for this session — Satchel holds it in memory only and forgets it on exit.",
    switchMerchant: "Switch merchant",
    unlock: "Unlock",
  },
  common: {
    cancel: "Cancel",
    confirm: "Confirm",
    save: "Save",
    done: "Done",
    later: "Later",
    retry: "Retry connection",
  },
};

export type Bundle = typeof en;
