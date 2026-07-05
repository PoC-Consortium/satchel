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
    title: "Self-custody — your keys, your responsibility",
    body: "Satchel performs non-custodial atomic swaps: you alone hold your keys, and a merchant's seed holds hot transit keys while a swap is in flight. The swap protocols (v1 HTLC and v2 Taproot/MuSig2) are reviewed and live on mainnet. MIT-licensed and provided as-is, with no warranty — back up your recovery phrase and use at your own risk.",
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
    relays: "Relays",
    wallets: "Wallets",
    contacts: "Contacts",
    settings: "Settings",
    coins: "Coins",
  },
  makeOffer: {
    title: "Post an offer",
    intro:
      "Post a signed offer to the Corkboard. Nothing is locked — it's just an advert; withdraw any time, and a swap only starts when someone takes it and both sides fund.",
    give: "You give",
    want: "You receive",
    // Canonical pair + direction: pick the pair, choose Sell/Buy the base, enter
    // the base amount and a quote-per-base price (invariant to direction).
    pair: "Pair",
    noPairs: "No tradable pairs — connect at least two coins in Settings → Coins.",
    sell: "Sell {sym}",
    buy: "Buy {sym}",
    amount: "Amount",
    youGive: "You give",
    youGet: "You get",
    price: "Price",
    priceUnit: "{unit} per {base}",
    pricePlaceholder: "unit price",
    balance: "Balance: {amt} {sym}",
    balanceLoading: "Balance: …",
    noCoins: "No coins configured",
    legDown: "One of these coins' nodes is down — start it (or check Settings → Coins) before posting.",
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
      short: "Short — funds auto-refund fastest if the trade stalls (~12h / 6h), with the smallest safety margin.",
      medium: "Medium — balanced refund window (~24h / 12h).",
      long: "Long (safest) — widest safety margin; auto-refund after ~36h / 18h if the trade stalls.",
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
    intro:
      "A friend sent you a private offer slip (it starts with pactoffer1:). Paste it here to review and take it — exactly like an offer from the board.",
    placeholder: "pactoffer1:…",
    take: "Review & take",
    invalid: "That doesn't look like a slip — it should start with pactoffer1:.",
    previewLabel: "This slip offers",
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
  // Watch-only mode: a viewer session with no coins. Browse the board and
  // withdraw your own offers, but no posting / taking / funding.
  watchOnly: {
    badge: "Watch only",
    badgeTip:
      "Watch-only mode — browse the board and withdraw your own offers, but you can't post, take, or fund. Set up coins in Settings to trade.",
    coinWizardButton: "Browse in watch-only mode",
    coinWizardHint:
      "Skip coin setup and just browse the board (read-only). You can still withdraw your own offers — handy for pulling offers left up by another session. Switch it off any time in Settings.",
    postBlockedTitle: "Watch-only mode",
    postBlockedBody:
      "This is a watch-only session, so it can't post offers. Set up at least two coins in Settings → Coins to trade.",
    takeBlockedBody: "Watch-only mode — you can review this offer, but taking it needs coins set up.",
    takeBlockedTip: "Watch-only mode — set up coins in Settings to take offers.",
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
    switch: "switch",
    newMerchant: "New merchant",
    thisMerchant: "this merchant",
    nameLabel: "Merchant name",
    namePlaceholder: "e.g. Main",
    rename: "Rename",
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
    encrypt: "Encrypt",
    encryptDesc:
      "A passphrase protects the seed at rest. You enter it once per session — Satchel never stores it. Note: unattended auto-refund pauses after a restart until you re-enter it.",
    noPassphrase: "No passphrase (recommended)",
    noPassphraseDesc:
      "Auto-refund keeps working through reboots with nothing to enter — this is only a hot transit seed. Cost: file/host access exposes this merchant's transit keys + identity.",
    passphraseLabel: "Passphrase",
    passphrasePlaceholder: "choose a passphrase",
    revealTitle: "Write down your recovery phrase",
    revealBody:
      "Anyone with these words controls this merchant's hot keys. Satchel keeps no copy — store it offline. You'll confirm a few words next.",
    ackLabel: "I have written down my recovery phrase.",
    // Multi-step onboarding (create/import -> secret -> confirm -> passphrase).
    chooseTitle: "Set up {label}",
    enterTitle: "Import your recovery phrase",
    enterBody:
      "Type each word — they autocomplete as you go — or paste the whole phrase. We check it before you continue.",
    wordCount: "{n} words",
    wordCountHint:
      "12 words is plenty — this is a hot transit wallet, not cold storage. Pick 24 if you prefer the longer phrase.",
    wordAria: "Word {n}",
    checkIncomplete: "Enter all {n} words.",
    checkUnknown: "Some words aren't in the BIP39 wordlist — check the highlighted ones.",
    checkBadChecksum: "Checksum doesn't match — re-check your words and their order.",
    checkOk: "Recovery phrase looks valid.",
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
    // Short fingerprint fallback (identity.ts shortId) when no pubkey is known.
    unknownShort: "unknown",
  },
  contacts: {
    // Left-nav tab + screen.
    title: "Contacts",
    subtitle: "Your private nicknames for the people you trade with.",
    privacyNote:
      "Contacts are stored only on this device and are never shared, published, or sent to a relay. A nickname is your label — the identicon and fingerprint remain the real identity.",
    searchPlaceholder: "Search nick, note, or key",
    empty: "No contacts yet. Click a counterparty's identicon anywhere to add one.",
    emptyFiltered: "No contacts match this filter.",
    count: "{n} contacts",
    // Columns.
    colWho: "Identity",
    colNick: "Nickname",
    colNote: "Notes",
    colStatus: "Standing",
    colAdded: "Added",
    colActions: "",
    // Filter chips.
    filterAll: "All",
    filterTrusted: "Trusted",
    filterBlocked: "Blocked",
    // Corkboard toggle: drop blocked makers' offers from the ladder.
    hideBlocked: "Hide blocked offers",
    // Standing values.
    statusTrusted: "Trusted",
    statusNeutral: "Neutral",
    statusBlocked: "Blocked",
    // Click-menu (on a counterparty identicon/tag).
    menuAdd: "Add to contacts…",
    menuEdit: "Edit contact…",
    menuMarkTrusted: "Mark as trusted",
    menuMarkNeutral: "Mark as neutral",
    menuMarkBlocked: "Block",
    menuCopyKey: "Copy public key",
    menuOpen: "Open in Contacts",
    keyCopied: "Public key copied",
    // Edit dialog.
    editTitle: "Edit contact",
    addTitle: "Add contact",
    nickLabel: "Nickname",
    nickPlaceholder: "e.g. Alice from the meetup",
    noteLabel: "Notes",
    notePlaceholder: "Anything you want to remember — how to reach them, past trades…",
    save: "Save",
    cancel: "Cancel",
    remove: "Remove contact",
    removeConfirmTitle: "Remove contact?",
    removeConfirmBody: "This deletes your local nickname and notes for {who}. It can't be undone.",
    // Take-confirm warning when the counterparty is blocked.
    blockedWarning: "You blocked this counterparty",
    blockedWarningBody:
      "You marked this person as blocked. Blocking is only a personal reminder — it does not stop the trade. Continue only if you mean to.",
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
    mode: "Mode",
    watchOnly: "Watch-only mode",
    watchOnlyHint:
      "Browse the board without setting up coins. You can still withdraw your own offers, but can't post, take, or fund. Turn off to trade (you'll need at least two coins connected).",
    network: "Network",
    boards: "Corkboards",
    boardsDesc:
      "Optional self-hosted HTTP boards. Add any you trust; leave empty to rely on Nostr.",
    boardsNone: "None configured",
    nostrRelays: "Nostr relays",
    nostrRelaysDesc:
      "Relays carry the noticeboard over a decentralized network — no operator can read or match your offers. Prewired with a default set; edit freely.",
    nostrRelaysOff: "Off — Nostr transport disabled",
    addUrl: "Add",
    removeUrl: "Remove",
    relayInvalid: "Enter a ws:// or wss:// relay URL",
    boardInvalid: "Enter an http:// or https:// board URL",
    netSave: "Save & reconnect",
    netSaving: "Saving & reconnecting…",
    netSaved: "Saved",
    // Fees tab — fee-bump policy (per active merchant).
    tabFees: "Fees",
    fees: "Fee bumping",
    feesScope: "These settings apply to the active merchant.",
    feesIntro:
      "Safety/cost trade-offs for fee bumps, not required setup. New values apply to future bumps; swaps already funded keep the policy they were funded under.",
    feeMax: "Max feerate (sat/vB)",
    feeMaxHint:
      "Ceiling for every fee bump. Default 500, also the hard system maximum. Lower it to cap costs.",
    feeReservation: "Funding bump reservation (×)",
    feeReservationHint:
      "Balance the funds check sets aside as bump headroom. Higher rescues bigger fee spikes but ties up more balance and rejects more swaps. Default 3.",
    feeCommitted: "Redeem over-provision (×)",
    feeCommittedHint:
      "How much extra the v2 redeem fee is pre-paid so it confirms even when Satchel is closed. Applies to new swaps only. Default 2.",
    feeSave: "Save",
    feeSaving: "Saving…",
    feeSaved: "Saved",
    feeReset: "Reset to defaults",
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
    // CoinSetup dialog.
    setupTitle: "Connect {coin}",
    setupIntro:
      "Point Satchel at your own {sym} node. Nothing is saved until the node passes a genesis-block check — your funds only ever touch the real {sym} chain.",
    confirmationsLabel: "Confirmations before final",
    confirmationsHint:
      "How deep a funding or redeem on this chain must be before a swap acts on it — the reorg-safety margin. Higher is safer but slower; leave blank for the recommended default ({default}).",
    validateNode: "Validate node",
    checking: "Checking the node…",
    genesisOk: "Genesis matched — this is the right chain",
    genesisDetail: "tip height {tip} · genesis {hash}…",
    genesisBad: "Rejected — not saving",
    errorShort: "error",
    // Structured connection form (CoinSetup v2).
    rpcHostLabel: "RPC host",
    rpcPortLabel: "RPC port",
    authMethodLabel: "Authentication",
    authCookie: "Cookie file",
    authCookieDesc: "Auto-read the node's .cookie from its data directory (the default, no password stored).",
    authUserPass: "User / password",
    authUserPassDesc: "The rpcuser / rpcpassword from your node's config — needed for a remote node.",
    rpcUserLabel: "RPC username",
    rpcPasswordLabel: "RPC password",
    datadirLabel: "Node data directory",
    cookiePathNote: "The cookie is read from {path} under this directory.",
    walletLabel: "Wallet name (optional)",
    walletPlaceholder: "your node's wallet",
    needPort: "Enter the RPC port first.",
    validateFirst: "Validate the node before saving.",
    savingReconnecting: "Saving & reconnecting…",
    connected: "{coin} connected",
    // Electrum connection mode (epic #58) — "nodeless" is internal wording,
    // the UI says RPC vs Electrum (user decision 2026-07-04).
    modeLabel: "Connection type",
    modeNode: "Your own node",
    modeNodeDesc: "Core RPC — the node's wallet funds swaps. Maximum sovereignty.",
    modeNodeless: "Electrum",
    modeNodelessDesc:
      "No node needed: chain data comes from Electrum servers and the wallet lives on your Pact seed.",
    // Connection-kind chip on the coin card: transport + locality.
    connRpcLocal: "RPC (local)",
    connRpcRemote: "RPC (remote)",
    connElectrumLocal: "Electrum (local)",
    connElectrumRemote: "Electrum (remote)",
    connRpcTip:
      "This coin talks to a Bitcoin-Core-style node over RPC; the node's wallet funds swaps.",
    connElectrumTip:
      "This coin connects to Electrum servers — no node. The wallet lives on your Pact seed.",
    switchHidesTitle: "This hides your pact-seed wallet",
    switchHidesBody:
      "Your pact-seed wallet on this coin still holds {balance} {sym}. Switching to a node connection hides it — the coins stay safe on your seed and reappear the moment you switch back to Electrum, but until then they won't show up or fund swaps. Consider sending them somewhere first.",
    switchHidesConfirm: "Switch anyway",
    electrumUrlsLabel: "Electrum servers",
    electrumUrlsHelp:
      "One per line: tcp://host:port or ssl://host:port. Mainnet requires at least two independent servers as cross-checking chain views.",
    electrumNeedUrl: "Enter at least one Electrum server URL (tcp:// or ssl://).",
    electrumBadUrl: "Electrum URLs must start with tcp:// or ssl:// — got: {url}",
    validateServers: "Validate servers",
    // Template picker (a coins.toml coin the engine version doesn't support).
    unsupportedByEngine: "Unsupported",
    unsupportedByEngineTip:
      "This coin is defined in coins.toml but not built into this version of the engine, so it can't be traded.",
  },
  coinWizard: {
    title: "Connect your coins",
    intro:
      "Pick at least two coins and point each at your own node. A swap needs two chains, so trading unlocks once two nodes are connected and live. You can add or change coins later in Settings.",
    progress: "{count} of {min} coins connected",
    continue: "Continue",
    live: "Live",
    nodeDown: "Node down",
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
    watchOnlyTitle: "No wallets in watch-only mode",
    watchOnlyBody:
      "This is a watch-only session with no coins connected, so there are no wallets to show. Turn off watch-only in Settings and connect a coin to fund swaps.",
    walletName: "wallet · {wallet}",
    walletScopedHint: "Every RPC for this coin is scoped to this node wallet.",
    walletDefault: "default wallet (not scoped)",
    walletDefaultHint:
      "No wallet set for this coin, so RPCs use the node's default wallet. Set one in Settings → Coins to scope every call to a specific wallet.",
    balanceLabel: "{symbol} balance",
    // ---- nodeless (pact-seed bdk) wallet: send / receive / activity --------
    pactSeed: "pact seed wallet",
    pactSeedHint:
      "This coin runs nodeless: its wallet lives on your Pact seed, synced from Electrum servers — no node required. Send, receive and history live right here.",
    receive: "Receive",
    send: "Send",
    activity: "Activity",
    copy: "Copy",
    copied: "Copied",
    close: "Close",
    refresh: "Refresh",
    receiveTitle: "Receive {sym}",
    receiveIntro:
      "A fresh address from your pact-seed wallet. Coins sent here appear in the balance once confirmed.",
    receiveIntroRpc:
      "A fresh address from your node's wallet. Coins sent here appear in the balance once confirmed.",
    receiveFreshNote:
      "Every time you open this dialog you get a fresh address. Old addresses keep working — fresh ones are just better for privacy.",
    sendTitle: "Send {sym}",
    sendIntro: "Spendable: {balance} {sym}.",
    sendAddressLabel: "Recipient {sym} address",
    sendAmountLabel: "Amount",
    sendMax: "Max",
    sendAllNote: "Sending everything — the network fee comes out of this amount.",
    sendNeedAddress: "Enter the recipient address.",
    sendNeedAmount: "Enter an amount.",
    sendNeedFee: "Pick a fee rate.",
    sendOverBalance: "More than the spendable balance (including the estimated fee).",
    feeLabel: "Network fee — added on top of the amount",
    fee_slow: "Slow",
    fee_normal: "Normal",
    fee_fast: "Fast",
    fee_custom: "Custom",
    feeRate: "{rate} sat/vB",
    feeNoEstimate: "no data",
    feeNoEstimatesNote:
      "No live fee estimates right now — the fee market may be empty. Set a custom rate.",
    feeCustomLabel: "Custom rate (sat/vB)",
    feeCustomMin: "Minimum {min} sat/vB.",
    sendFeePreview: "Estimated network fee: ~{fee} {sym} for a typical transaction.",
    sendReview: "Review",
    sendBack: "Back",
    sendConfirmTitle: "Confirm send ({sym})",
    sendConfirmRecipient: "Recipient",
    sendConfirmAmount: "Amount",
    sendConfirmFee: "Network fee (estimated)",
    sendConfirmFeeValue: "~{fee} {sym} ({rate} sat/vB)",
    sendConfirmTotal: "Total (approx.)",
    sendIrreversible:
      "Transactions are irreversible. Double-check the address — coins sent to a wrong one are gone.",
    sendBroadcast: "Sent — {txid}… is on its way ({sym}).",
    sendConfirm: "Send",
    activityTitle: "{sym} activity",
    activityEmpty: "Nothing yet — receive coins or complete a swap and it shows up here.",
    activityWhen: "When",
    activityDirection: "Direction",
    activityAmount: "Amount ({sym})",
    activityFee: "Fee",
    activityConfs: "Confs",
    activityTxid: "Transaction",
    activityPending: "pending",
    activitySent: "Sent",
    activityReceived: "Received",
    bump: "Bump",
    bumpHint: "Pay a higher fee so this pending transaction confirms sooner (RBF).",
    bumpTitle: "Bump fee ({sym})",
    bumpIntro:
      "Replace this pending transaction with one paying a higher fee (RBF). It pays ~{rate} sat/vB now.",
    bumpNeedHigher: "Pick a rate above the current ~{rate} sat/vB.",
    bumpBroadcast: "Fee bumped — replacement {txid}… is on its way ({sym}).",
    bumpConfirm: "Bump fee",
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
    allPairs: "All pairs",
    allPairsTip:
      "Browse every pair on the board, including coins you haven't set up — those offers are view-only until you connect the coin.",
    noOffers: "No offers you can take right now",
    noOffersBody:
      "Offers show up here as soon as a maker posts one for a pair you've set up. You can also post your own.",
    yourOffer: "your offer",
    offerStaged: "posting…",
    offerStagedTip:
      "Posted from this device and waiting to be confirmed back from a relay. It's advertising; it becomes live once a relay echoes it.",
    take: "Take offer",
    legDown: "One of this pair's nodes is down — start it (or check Settings → Coins) before taking.",
    withdraw: "Withdraw",
    withdrawTip: "Withdraw instantly — an offer never locks funds",
    safetyRefund: "safety refund",
    safetyRefundTip:
      "If the swap stalls, both sides auto-refund — the taker's leg unlocks first, yours a little later. Nobody ends up stuck.",
    activeTitle: "Your active swaps",
    states: {
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
      selectLevel: "Pick a price level above to see the offers there.",
      paneHeader: "{size} {base} @ {price} {unit}",
      denomTip: "Display unit for {coin} amounts",
      showMore: "Show {count} more",
      showLess: "Show top {count}",
    },
  },
  relays: {
    title: "Relays",
    subtitle: "Live connectivity to your Nostr relays — the network your offers and takes travel over. Add or remove relays in Settings → Network.",
    connectedCount: "{up} / {total} connected",
    refresh: "Refresh",
    ms: "{ms} ms",
    up: "up",
    down: "down",
    statsTip: "{success}/{attempts} successful connects · ↓{down} ↑{up}",
    none: "No relays configured",
    noneBody: "Add a Nostr relay in Settings → Network to publish and receive offers over the network.",
    goToNetwork: "Go to Settings",
    notConnected: "Not connected",
    notConnectedBody: "The relay view needs the engine running — connect a merchant first.",
  },
  swaps: {
    title: "Swaps",
    // The two market roles (Maker / Taker columns + the dock's maker↔taker
    // arrow). Kept as the English trading terms by default — common loanwords on
    // crypto venues — but localizable per language.
    maker: "Maker",
    taker: "Taker",
    hint: "Your full ledger — in-flight swaps on top, finished trades below. You can also act on live swaps from the Corkboard.",
    activeTitle: "In flight",
    historyTitle: "History",
    none: "No swaps yet — take an offer on the Corkboard.",
    // Active-swaps dock action buttons + the refund-time label.
    cancel: "cancel",
    dump: "dump logs",
    dumpHint: "Copy a secret-free diagnostics bundle (state + log lines) for this swap, to paste to the developers.",
    dumpCopied: "Diagnostics copied — paste to the developers.",
    dumpFailed: "Could not copy the diagnostics bundle.",
    refundAt: "refund {when}",
    // Confirm dialogs for acting on a live swap (ActiveSwaps).
    cancelTitle: "Cancel this swap?",
    cancelConfirm: "Cancel swap",
    cancelKeep: "Keep it",
    // Abort reason recorded on the swap when cancelled from Satchel.
    cancelReason: "cancelled in Satchel",
    cancelBody:
      "This abandons the swap before you've funded. Nothing of yours is locked yet, so you lose nothing — the offer just won't complete.",
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
    ifItStalls: "(if it stalls)",
  },
  funds: {
    insufficient:
      "Not enough {sym} to fund this swap — need ~{need} {sym} (amount + funding fee), wallet has {have} {sym}.",
  },
  wizard: {
    back: "Back",
    continue: "Continue",
  },
  // UI-4 docked activity log.
  log: {
    title: "Activity",
    empty: "— activity log —",
    count: "{count} lines",
    collapse: "Collapse log",
    expand: "Expand log",
    // Activity-log lines emitted by the frontend. `{err}` carries a raw engine
    // message (itself not translated — it comes from pactd over the wire).
    noTauri: "not running inside Satchel — this UI needs the Tauri bridge",
    startupError: "startup: {err}",
    notConnected: "not connected: {err}",
    connected: "connected to pactd {version} ({protocol})",
    listcoinsError: "listcoins: {err}",
    watchOnlyError: "watch-only: {err}",
    switchedMerchant: "switched to merchant {id}",
    renamedMerchant: "renamed merchant to {name}",
    renameMerchantError: "rename merchant: {err}",
    switchMerchantError: "switch merchant: {err}",
    loadMerchantError: "load merchant: {err}",
    merchantCreated: "merchant {id} created",
    merchantReady: "merchant ready",
    actionOk: "{action} {id}: ok",
    actionError: "{action} {id}: {err}",
    diagCopied: "diagnostics for {id} copied ({count} log lines) — paste to the devs",
    dumpError: "dump {id}: {err}",
    coinDisconnected: "{coin} disconnected",
    removeCoinError: "remove coin: {err}",
    tookOffer: "took offer {id} — it now appears in your active swaps below",
    takeError: "take: {err}",
    offerWithdrawn: "offer {id} withdrawn",
    withdrawError: "withdraw: {err}",
    postedOffer: "posted offer {id} — withdraw any time; nothing is locked",
    createdSlip: "created a private offer slip — send it to your friend",
    tookPrivateOffer: "took private offer {id} — it now appears in your active swaps",
    cancelledPrivateOffer: "cancelled private offer {id}",
    cancelError: "cancel: {err}",
    noticeboardUpdated: "noticeboard updated",
    feePolicyUpdated: "fee policy updated",
  },
  // Relative-time + freshness prose from format.ts (rendered via the tr() mirror,
  // since those are pure non-component helpers). Unit letters stay inside the
  // template so a translation owns the whole phrase.
  format: {
    ageUnknown: "age unknown",
    justNow: "just now",
    minutesAgo: "{n}m ago",
    hoursAgo: "{n}h ago",
    daysAgo: "{n}d ago",
    expiryNow: "now",
    expirySoon: "soon",
    inMinutes: "in ~{n}m",
    inHours: "in ~{n}h",
    inDays: "in ~{n}d",
    posted: "posted {age}",
    expires: "expires {time}",
  },
  // Plain-language swap story per (role, state) — the honest "who is exposed
  // when" framing shown on every active swap. {a}/{b} are coin tickers; {t1}/{t2}
  // are local refund times. Rendered via tr() (narrate() is a pure helper).
  narrate: {
    initiating:
      "Take sent — waiting for the maker to start the swap. Nothing is locked yet; it cancels on its own if they don't respond.",
    created: "Offer sent — waiting for the other side to agree. Nothing is committed.",
    acceptedMaker: "Terms agreed. Next: lock your {a}. Until you fund, you can still cancel freely.",
    acceptedTaker: "Terms agreed. The other side locks their {a} first — you never send first.",
    noncesExchanged:
      "Setting up the private swap — exchanging signing material. Nothing is locked yet.",
    signedMaker:
      "Both sides signed and your {a} is locked. Your daemon claims the {b} automatically once the other side locks and confirms it. If anything stalls, your {a} returns at {t1}.",
    signedTaker:
      "Both sides signed. Once their {a} is confirmed, your daemon locks your {b}, then claims the {a} automatically. Once your {b} is locked, it returns at {t2} if anything stalls.",
    fundedAMaker:
      "Your {a} is locked. Waiting for the other side to lock their {b}. If they never do, your {a} returns automatically at {t1}.",
    fundedATaker:
      "Their {a} is locked and verified. Next: lock your {b}. Safety net: automatic refund at {t2} if anything stalls.",
    fundedBMaker: "Both locked. Your daemon claims the {b} as soon as it is safely confirmed.",
    fundedBTaker: "Both locked. Your daemon will claim the {a} the moment the other side takes their {b}.",
    finalizing:
      "You claimed your {got} — final confirmations. Keep the app open until it buries; your {gave} stays protected until then.",
    completed: "Swap complete — the {coin} is in your wallet.",
    refunded: "The swap did not complete, so your {coin} came back automatically. Nothing lost but fees.",
    aborted: "Cancelled before any money moved.",
  },
  // Live active-swap progress line (observability). Only these labels are
  // translatable; counts, feerate and the "+N blocks" number are data.
  progress: {
    awaitingLock: "Awaiting their lock",
    awaitingClaim: "Awaiting their claim",
    theirLock: "Their lock confirming",
    ourLock: "Your lock confirming",
    securing: "Securing your {coin}",
    funding: "Locking your {coin} — unlock wallet if stalled",
    blocks: "+{n} blocks",
    feeBumped: "Fee-bumped",
    reorg: "Reorg detected — re-checking",
  },
  // Desktop notifications + tray (issue #55). Notification bodies reuse the
  // narrate.* story lines; these are the titles, the Settings toggles, and the
  // tray tooltip/menu labels (pushed to Rust, which owns no copy of its own).
  notify: {
    tab: "Notifications",
    section: "Desktop notifications",
    intro:
      "Swaps take a while and run on their own — get an OS notification when one hits a milestone while Satchel is in the background. Nothing fires while you're looking at the window.",
    master: "Enable notifications",
    masterHint: "Master switch — turns every notification below on or off.",
    evStarted: "Swap started",
    evStartedHint: "Someone took your offer, or a maker accepted your take.",
    evLocks: "Locks confirmed",
    evLocksHint: "A leg's lock confirmed on-chain — yours, theirs, then both locked.",
    evCompleted: "Swap completed",
    evCompletedHint: "The swap finished and the coins are settled in your wallet.",
    evFailed: "Swap refunded or aborted",
    evFailedHint: "A swap unwound — refunded after a stall, or cancelled.",
    evReorg: "Reorg warnings",
    evReorgHint: "A chain reorganization touched a swap you're in — being re-checked.",
    test: "Send a test notification",
    testTitle: "Satchel",
    testBody: "Notifications are working.",
    denied:
      "The OS is blocking notifications — allow Satchel in your system notification settings.",
    testSent:
      "Handed to the OS. If no toast appeared: development (unpackaged) builds are often suppressed — Windows only reliably shows toasts for installed apps. The installed Satchel notifies normally; also check Do Not Disturb / notification settings.",
    titleStarted: "Swap started",
    titleLocks: "Swap update",
    titleCompleted: "Swap completed",
    titleFailed: "Swap not completed",
    titleReorg: "Reorg warning",
    reorgBody: "{coin}: chain reorganization detected — confirmations are being re-checked.",
    trayNone: "Satchel — no swaps in flight",
    trayOne: "Satchel — 1 swap in flight",
    trayMany: "Satchel — {count} swaps in flight",
    trayOpen: "Open Satchel",
    trayQuit: "Quit",
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
    // {word} is the confirm word below; a translation may localize both together.
    typeToConfirm: "To force-quit anyway, type {word} below.",
    confirmWord: "QUIT",
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
  unlock: {
    title: "Unlock merchant",
    body:
      "This merchant's seed is encrypted. Enter its passphrase to unlock it for this session — Satchel holds it in memory only and forgets it on exit.",
    switchMerchant: "Switch merchant",
    unlock: "Unlock",
  },
  // Manual Cashrate (issue #56) — display-only "~Cash" equivalents derived from
  // user-entered per-coin anchors. Currency-NEUTRAL on purpose: the user thinks
  // in whatever money they think in (EUR, USD, RMB, …) and Satchel never names
  // it. Deliberately manual: BTCX is unlisted so no feed could price it, and
  // Satchel makes no external calls. The rate entry lives in the sidebar.
  fx: {
    cashrate: "Cashrate ({sym})",
    cashrateTip:
      "What you call 1 {sym} in your own money — EUR, USD, RMB, whatever you think in. Every ~Cash figure derives from your rates, remembered per coin. Display-only — Satchel never fetches prices.",
    cashrateNoContext:
      "Greyed here — the rate binds to the pair you're looking at. Open the Corkboard or an offer form to set the rate for its quote coin.",
    cashUnit: "~Cash",
    refTip:
      "At your own Cashrate — your reference, not a market price. Both legs of an offer are worth the same at its own price. Set rates via the header chip.",
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

// `progress.funding` (#3) and the nodeless-wallet keys (epic #58) are OPTIONAL
// in Bundle so new copy can ship in en.ts without re-translating all 26 bundles
// at once — a locale missing a key falls back to English at runtime (see the
// i18n index `t`). Translators fill them in later.
type EnBundle = typeof en;

/** Namespace with the given keys made optional (English-fallback at runtime). */
type WithOptional<NS, K extends keyof NS> = Omit<NS, K> & Partial<Pick<NS, K>>;

/** Nodeless-wallet copy shipped 2026-07 (epic #58) — optional until the next
 *  full translation pass. */
type NewWalletKeys =
  | "pactSeed"
  | "pactSeedHint"
  | "receive"
  | "send"
  | "activity"
  | "copy"
  | "copied"
  | "close"
  | "refresh"
  | "receiveTitle"
  | "receiveIntro"
  | "receiveIntroRpc"
  | "receiveFreshNote"
  | "sendTitle"
  | "sendIntro"
  | "sendAddressLabel"
  | "sendAmountLabel"
  | "sendMax"
  | "sendAllNote"
  | "sendNeedAddress"
  | "sendNeedAmount"
  | "sendNeedFee"
  | "sendOverBalance"
  | "feeLabel"
  | "fee_slow"
  | "fee_normal"
  | "fee_fast"
  | "fee_custom"
  | "feeRate"
  | "feeNoEstimate"
  | "feeNoEstimatesNote"
  | "feeCustomLabel"
  | "feeCustomMin"
  | "sendFeePreview"
  | "sendReview"
  | "sendBack"
  | "sendConfirmTitle"
  | "sendConfirmRecipient"
  | "sendConfirmAmount"
  | "sendConfirmFee"
  | "sendConfirmFeeValue"
  | "sendConfirmTotal"
  | "sendIrreversible"
  | "sendBroadcast"
  | "sendConfirm"
  | "activityTitle"
  | "activityEmpty"
  | "activityWhen"
  | "activityDirection"
  | "activityAmount"
  | "activityFee"
  | "activityConfs"
  | "activityTxid"
  | "activityPending"
  | "activitySent"
  | "activityReceived"
  | "bump"
  | "bumpHint"
  | "bumpTitle"
  | "bumpIntro"
  | "bumpNeedHigher"
  | "bumpBroadcast"
  | "bumpConfirm";
type NewCoinKeys =
  | "modeLabel"
  | "modeNode"
  | "modeNodeDesc"
  | "modeNodeless"
  | "modeNodelessDesc"
  | "electrumUrlsLabel"
  | "electrumUrlsHelp"
  | "electrumNeedUrl"
  | "electrumBadUrl"
  | "validateServers"
  | "connRpcLocal"
  | "connRpcRemote"
  | "connElectrumLocal"
  | "connElectrumRemote"
  | "connRpcTip"
  | "connElectrumTip"
  | "switchHidesTitle"
  | "switchHidesBody"
  | "switchHidesConfirm";

export type Bundle = Omit<
  EnBundle,
  "progress" | "wallets" | "coins" | "seed" | "corkboard" | "fx" | "notify"
> & {
  progress: WithOptional<EnBundle["progress"], "funding">;
  wallets: WithOptional<EnBundle["wallets"], NewWalletKeys>;
  coins: WithOptional<EnBundle["coins"], NewCoinKeys>;
  /** rc10 review copy (All-pairs browse toggle) — optional until the next
   *  full translation pass. */
  corkboard: WithOptional<EnBundle["corkboard"], "allPairs" | "allPairsTip">;
  seed: WithOptional<EnBundle["seed"], "wordCountHint">;
  /** USD-reference copy shipped 2026-07 (issue #56) — optional until the next
   *  full translation pass. */
  fx?: EnBundle["fx"];
  /** Desktop notifications + tray (issue #55) — a whole namespace shipped
   *  2026-07, optional until the next full translation pass. */
  notify?: EnBundle["notify"];
};
