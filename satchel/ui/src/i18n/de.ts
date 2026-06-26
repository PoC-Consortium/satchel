// The German (Deutsch) string bundle. Mirrors en.ts key-for-key; only values are translated.
import type { Bundle } from "./en";

export const de: Bundle = {
  app: {
    name: "Satchel",
    tagline: "vertrauenslose Swaps",
  },
  // In-app update notifications (sidebar version badge + dialog).
  update: {
    title: "Update verfügbar",
    upToDate: "Du bist auf dem neuesten Stand",
    current: "Installiert",
    latest: "Neueste",
    notesTitle: "Versionshinweise",
    get: "Update holen",
    dismiss: "Verwerfen",
    close: "Schließen",
    badgeTooltip: "Update verfügbar — für Details klicken",
    versionTooltip: "Klicken, um nach Updates zu suchen",
  },
  // Risk disclaimer (first-run welcome + Settings → About).
  disclaimer: {
    title: "Selbstverwahrung — deine Schlüssel, deine Verantwortung",
    body: "Satchel führt nicht-verwahrte Atomic Swaps durch: Nur du allein hältst deine Schlüssel, und das Seed eines Merchants hält während eines laufenden Swaps heiße Transit-Schlüssel. Die Swap-Protokolle (v1 HTLC und v2 Taproot/MuSig2) sind geprüft und live im MainNet. MIT-lizenziert und ohne Gewähr bereitgestellt — sichere deine Wiederherstellungsphrase und nutze es auf eigenes Risiko.",
  },
  nav: {
    public: "Öffentlich",
    corkboard: "Corkboard",
    postOffer: "Angebot einstellen",
    private: "Privat",
    privateCreate: "Slip erstellen",
    privateReceive: "Slip annehmen",
    privateSlips: "Meine Slips",
    swaps: "Swaps",
    relays: "Relays",
    wallets: "Wallets",
    settings: "Einstellungen",
    coins: "Coins",
  },
  makeOffer: {
    title: "Angebot einstellen",
    intro:
      "Stelle ein signiertes Angebot auf das Corkboard. Nichts wird gesperrt — es ist nur eine Anzeige; du kannst es jederzeit zurückziehen, und ein Swap startet erst, wenn jemand es annimmt und beide Seiten finanzieren.",
    give: "Du gibst",
    want: "Du erhältst",
    // Canonical pair + direction: pick the pair, choose Sell/Buy the base, enter
    // the base amount and a quote-per-base price (invariant to direction).
    pair: "Paar",
    noPairs: "Keine handelbaren Paare — verbinde mindestens zwei Coins unter Einstellungen → Coins.",
    sell: "{sym} verkaufen",
    buy: "{sym} kaufen",
    amount: "Betrag",
    youGive: "Du gibst",
    youGet: "Du erhältst",
    price: "Preis",
    priceUnit: "{unit} pro {base}",
    pricePlaceholder: "Stückpreis",
    balance: "Guthaben: {amt} {sym}",
    balanceLoading: "Guthaben: …",
    noCoins: "Keine Coins konfiguriert",
    sameCoin: "Geben und Erhalten müssen unterschiedliche Coins sein.",
    legDown: "Die Node eines dieser Coins ist offline — starte sie (oder prüfe Einstellungen → Coins), bevor du einstellst.",
    // Swap-protocol pin (only offered when a pair+network supports more than
    // one). v2 label reuses coins.protoPrivate ("Private (Taproot)").
    protocol: "Swap-Typ",
    protoStandard: "Standard (HTLC)",
    // Titles for the review/confirm dialog shown before posting (see OfferForm).
    reviewOfferTitle: "Angebot prüfen",
    reviewSlipTitle: "Slip prüfen",
    term: "Sicherheits-Timelock",
    termShort: "Kurz",
    termMedium: "Mittel",
    termLong: "Lang",
    termHint: {
      short: "Kurz — Mittel werden am schnellsten automatisch erstattet, wenn der Handel stockt (~12 Std. / 6 Std.), mit dem kleinsten Sicherheitspuffer.",
      medium: "Mittel — ausgewogenes Erstattungsfenster (~24 Std. / 12 Std.).",
      long: "Lang (am sichersten) — größter Sicherheitspuffer; automatische Erstattung nach ~36 Std. / 18 Std., wenn der Handel stockt.",
    },
    validFor: "Gültig für (Minuten)",
    validForMins: "{mins} Min.",
    validForHint:
      "Wie lange das Angebot gelistet bleibt. Solange du online bist, wird es automatisch frisch gehalten; danach läuft es ab. Das Schließen der App zieht es zurück.",
    note: "Angebot mit fester Größe — nichts wird gesperrt, bis jemand es annimmt. Beträge sind On-Chain; du zahlst die Netzwerkgebühren obendrauf, und das Corkboard berechnet nichts. Der Timelock ist das automatische Erstattungsfenster, falls ein Swap stockt.",
    post: "Angebot einstellen",
    makeSlip: "Slip erstellen",
    slipTitle: "Dein privater Angebots-Slip",
    slipExplainer:
      "Schicke das an deinen Freund. Er fügt es in Satchel ein, um es anzunehmen. Nichts wird gesperrt; es läuft in {ttl} ab.",
    copy: "Kopieren",
    copied: "Kopiert",
    makeAnother: "Weiteres erstellen",
    myPrivateTitle: "Meine privaten Angebote",
    myPrivateEmpty: "Keine ausstehenden privaten Angebote.",
    privateExpires: "läuft ab {when}",
    privateExpired: "abgelaufen",
    cancel: "Abbrechen",
    cancelTip: "Diesen Slip nicht mehr einlösen — ein Freund, der ihn noch hält, kann ihn nicht mehr annehmen.",
  },
  takeSlip: {
    open: "Slip einfügen",
    title: "Privates Angebot annehmen",
    intro:
      "Ein Freund hat dir einen privaten Angebots-Slip geschickt (er beginnt mit pactoffer1:). Füge ihn hier ein, um ihn zu prüfen und anzunehmen — genau wie ein Angebot vom Board.",
    placeholder: "pactoffer1:…",
    take: "Prüfen & annehmen",
    invalid: "Das sieht nicht nach einem Slip aus — er sollte mit pactoffer1: beginnen.",
    previewLabel: "Dieser Slip bietet",
  },
  // PRIVATE nav group (off-market slips) — screen titles + intros. The form,
  // slip output, my-slips list and take flow reuse makeOffer.*/takeSlip.* copy.
  private: {
    createTitle: "Privates Angebot erstellen",
    createIntro:
      "Erstelle ein signiertes Angebot und übergib es einem Freund als Slip über deinen eigenen Chat. Nichts wird irgendwo gelistet — und nichts wird gesperrt, bis ihr beide finanziert.",
    slipsIntro:
      "Slips, die du erstellt hast. Jeder, der einen Slip hält, kann ihn annehmen, bis er abläuft; brich einen ab, um ihn vorher nicht mehr einzulösen.",
    slipsEmptyBody: "Erstelle ein privates Angebot, um einen Slip zu erhalten, den du einem Freund schicken kannst.",
    receiveTitle: "Privates Angebot annehmen",
    received: "Angenommen — verfolge es unter Swaps.",
  },
  // Shared take-confirmation dialog (board take + slip take).
  takeConfirm: {
    title: "Dieses Angebot annehmen?",
    confirm: "Angebot annehmen",
    counterparty: "Gegenpartei",
    youGive: "Du gibst",
    youReceive: "Du erhältst",
    safetyRefund: "Sicherheits-Erstattung",
    offerAge: "Alter des Angebots",
    makerFundsFirst:
      "Der Maker sperrt seine {sym} zuerst — du sendest nie zuerst. Du kannst weiterhin abbrechen, bevor du deine Seite finanzierst, und die Engine erstattet nach dem Sicherheits-Timelock automatisch, falls der Swap stockt.",
  },
  header: {
    activeMerchant: "Aktiver Merchant — klicken, um zu wechseln oder zu verwalten",
    manageMerchants: "Merchants verwalten…",
    noMerchant: "kein Merchant",
    openMenu: "Menü öffnen",
    collapseMenu: "Menü einklappen",
    settings: "Einstellungen",
    language: "Sprache",
    pactConnected: "Engine verbunden",
    pactUnreachable: "Engine nicht erreichbar",
    liveSwapsOne: "1 laufender Swap — zum Ansehen klicken",
    liveSwapsMany: "{count} laufende Swaps — zum Ansehen klicken",
    liveSwapsNone: "Keine laufenden Swaps",
    coinOk: "{name} — verbunden · Tip {tip}",
    coinUnconfigured: "{name} — nicht eingerichtet",
    coinError: "{name} — {status}",
    relaysOk: "Nostr-Relays — {up}/{total} verbunden",
    relaysDown: "Nostr-Relays — keines von {total} verbunden",
  },
  network: {
    mainnet: "MainNet",
    testnet: "TestNet",
    regtest: "RegTest",
    signet: "Signet",
    notRealFunds: "Keine echten Mittel — dies ist das {network}-Netzwerk",
  },
  // Watch-only mode: a viewer session with no coins. Browse the board and
  // withdraw your own offers, but no posting / taking / funding.
  watchOnly: {
    badge: "Nur Beobachten",
    badgeTip:
      "Nur-Beobachten-Modus — durchstöbere das Board und ziehe deine eigenen Angebote zurück, aber du kannst nicht einstellen, annehmen oder finanzieren. Richte Coins in den Einstellungen ein, um zu handeln.",
    coinWizardButton: "Im Nur-Beobachten-Modus durchstöbern",
    coinWizardHint:
      "Überspringe die Coin-Einrichtung und durchstöbere einfach das Board (schreibgeschützt). Du kannst trotzdem deine eigenen Angebote zurückziehen — praktisch, um Angebote zu entfernen, die eine andere Sitzung hinterlassen hat. Du kannst es jederzeit in den Einstellungen ausschalten.",
    postBlockedTitle: "Nur-Beobachten-Modus",
    postBlockedBody:
      "Dies ist eine Nur-Beobachten-Sitzung, daher können keine Angebote eingestellt werden. Richte mindestens zwei Coins unter Einstellungen → Coins ein, um zu handeln.",
    takeBlockedBody: "Nur-Beobachten-Modus — du kannst dieses Angebot prüfen, aber zum Annehmen müssen Coins eingerichtet sein.",
    takeBlockedTip: "Nur-Beobachten-Modus — richte Coins in den Einstellungen ein, um Angebote anzunehmen.",
  },
  merchants: {
    title: "Deine Merchants",
    intro:
      "Ein Merchant ist eine Handelsidentität — mit eigenem Seed und eigener Swap-Historie. Unter einem anderen Merchant zu handeln hält Kontexte unverknüpfbar (eine Wegwerf-Identität). Deine Haupt-Coins liegen in deiner eigenen Wallet, nicht hier.",
    // First-run welcome (empty merchant manager).
    welcomeTitle: "Willkommen bei Satchel",
    welcomeIntro:
      "Satchel handelt unter einem „Merchant“ — einer Handelsidentität mit eigenem Seed. Du hast noch keinen: erstelle einen neuen oder importiere eine vorhandene Wiederherstellungsphrase, um loszulegen.",
    importMerchant: "Merchant importieren",
    none: "Noch keine Merchants.",
    active: "aktiv",
    switch: "wechseln",
    newMerchant: "Neuer Merchant",
    thisMerchant: "dieser Merchant",
    nameLabel: "Merchant-Name",
    namePlaceholder: "z. B. Haupt",
    introFirst:
      "Richte deine erste Handelsidentität (einen „Merchant“) ein. Sie hält nur heiße Transit-Schlüssel für laufende Swaps — deine Haupt-Coins bleiben in deiner eigenen Wallet.",
    introNew: "Ein neuer Merchant ist eine frische, separate Identität mit eigenem Seed und eigener Swap-Historie.",
    // UI-5 merchant selector (phoenix wallet-selector parity).
    createNew: "Neu erstellen",
    import: "Importieren",
    load: "Merchant laden",
    loaded: "geladen",
    locked: "gesperrt",
    lockedTip: "Verschlüsseltes Seed — entsperre es mit deiner Passphrase beim Laden.",
    close: "Schließen",
    idLabel: "Ordner",
    switching: "Merchant wird gewechselt…",
    switchingBody: "Die Engine wird gegen diesen Ordner neu gestartet.",
  },
  // Seed create/import (SeedForm) + counterparty + status.
  seed: {
    intro: "Erstelle ein brandneues Seed oder importiere eines, das du bereits hast.",
    createNew: "Neu erstellen",
    createDesc: "Generiere ein frisches Seed. Du sicherst die Wiederherstellungsphrase.",
    import: "Importieren",
    importDesc: "Aus einer vorhandenen 12-/24-Wort-Phrase wiederherstellen.",
    recoveryLabel: "Wiederherstellungsphrase",
    importPlaceholder: "Wort1 Wort2 Wort3 …",
    encrypt: "Verschlüsseln",
    encryptDesc:
      "Eine Passphrase schützt das Seed im Ruhezustand. Du gibst sie einmal pro Sitzung ein — Satchel speichert sie nie. Hinweis: Die unbeaufsichtigte automatische Erstattung pausiert nach einem Neustart, bis du sie erneut eingibst.",
    noPassphrase: "Keine Passphrase (empfohlen)",
    noPassphraseDesc:
      "Die automatische Erstattung funktioniert über Neustarts hinweg ohne Eingabe — dies ist nur ein heißes Transit-Seed. Preis: Datei-/Host-Zugriff legt die Transit-Schlüssel und Identität dieses Merchants offen.",
    passphraseLabel: "Passphrase",
    passphrasePlaceholder: "Passphrase wählen",
    createTitle: "Seed erstellen",
    importTitle: "Seed importieren",
    secureTitle: "{label} sichern",
    revealTitle: "Schreibe deine Wiederherstellungsphrase auf",
    revealBody:
      "Jeder mit diesen Wörtern kontrolliert die heißen Schlüssel dieses Merchants. Satchel behält keine Kopie — bewahre sie offline auf. Als Nächstes bestätigst du ein paar Wörter.",
    ackLabel: "Ich habe meine Wiederherstellungsphrase aufgeschrieben.",
    // Multi-step onboarding (create/import -> secret -> confirm -> passphrase).
    chooseTitle: "{label} einrichten",
    enterTitle: "Importiere deine Wiederherstellungsphrase",
    enterBody:
      "Tippe jedes Wort — sie vervollständigen sich automatisch — oder füge die ganze Phrase ein. Wir prüfen sie, bevor du fortfährst.",
    wordCount: "{n} Wörter",
    wordAria: "Wort {n}",
    checkIncomplete: "Gib alle {n} Wörter ein.",
    checkUnknown: "Einige Wörter sind nicht in der BIP39-Wortliste — prüfe die markierten.",
    checkBadChecksum: "Die Prüfsumme stimmt nicht — überprüfe deine Wörter und ihre Reihenfolge.",
    checkOk: "Die Wiederherstellungsphrase sieht gültig aus.",
    verifyTitle: "Bestätige dein Backup",
    verifyBody: "Tippe die Wörter an diesen Positionen ein, um zu bestätigen, dass du die Phrase aufgeschrieben hast.",
    verifyWord: "Wort #{n}",
    verifyMismatch: "Diese stimmen nicht mit deiner Phrase überein — prüfe dein Backup.",
    passphraseTitle: "Das Seed schützen",
    passphraseBody:
      "Verschlüssle das gespeicherte Seed optional mit einer Passphrase. Du kannst dies überspringen — siehe die Abwägung unten.",
  },
  counterparty: {
    you: "Das bist du",
    youShort: "du",
    unknown: "unbekannte Identität",
    // Short fingerprint fallback (identity.ts shortId) when no pubkey is known.
    unknownShort: "unbekannt",
  },
  status: {
    notConnectedTitle: "Nicht mit der Engine verbunden",
    disconnectedBody:
      "Satchel kann die Engine nicht erreichen. Sie startet vielleicht noch, oder die Node-Verbindungen des aktiven Merchants sind unterbrochen. Versuche es erneut oder wechsle oben über den Selektor den Merchant.",
    openInSatchel: "Dies in Satchel öffnen",
    noTauriBody:
      "Dies ist die Oberfläche von Satchel — sie benötigt die Tauri-Brücke, um die Engine zu erreichen. Starte die Desktop-App (cargo tauri dev) statt eines Browsers.",
  },
  settings: {
    title: "Einstellungen",
    subtitle: "App-weite Einstellungen für diese Installation.",
    // UI-3 Settings tabs.
    tabGeneral: "Allgemein",
    tabCoins: "Coins",
    tabNetwork: "Netzwerk",
    tabAbout: "Über",
    appearance: "Erscheinungsbild",
    theme: "Design",
    themeDark: "Dunkel",
    themeLight: "Hell",
    themeSystem: "System",
    themeHint: "Wähle, wie Satchel aussieht. System folgt deiner Betriebssystem-Einstellung.",
    language: "Sprache",
    languageHint: "Weitere Sprachen kommen hinzu, sobald Übersetzungen beigetragen werden.",
    mode: "Modus",
    watchOnly: "Nur-Beobachten-Modus",
    watchOnlyHint:
      "Durchstöbere das Board, ohne Coins einzurichten. Du kannst weiterhin deine eigenen Angebote zurückziehen, aber nicht einstellen, annehmen oder finanzieren. Schalte es aus, um zu handeln (du brauchst mindestens zwei verbundene Coins).",
    network: "Netzwerk",
    boards: "Corkboards",
    boardsDesc:
      "Optionale selbst gehostete HTTP-Boards. Füge welche hinzu, denen du vertraust; lass es leer, um auf Nostr zu setzen.",
    boardsNone: "Keine konfiguriert",
    nostrRelays: "Nostr-Relays",
    nostrRelaysDesc:
      "Relays übertragen das Schwarze Brett über ein dezentrales Netzwerk — kein Betreiber kann deine Angebote lesen oder zusammenführen. Mit einem Standardsatz vorkonfiguriert; frei bearbeitbar.",
    nostrRelaysOff: "Aus — Nostr-Transport deaktiviert",
    addUrl: "Hinzufügen",
    removeUrl: "Entfernen",
    relayInvalid: "Gib eine ws:// oder wss:// Relay-URL ein",
    boardInvalid: "Gib eine http:// oder https:// Board-URL ein",
    netSave: "Speichern & neu verbinden",
    netSaving: "Speichern & neu verbinden…",
    netSaved: "Gespeichert",
    // Fees tab — fee-bump policy (per active merchant).
    tabFees: "Gebühren",
    fees: "Gebühren-Bumping",
    feesScope: "Diese Einstellungen gelten für den aktiven Merchant.",
    feesIntro:
      "Sicherheits-/Kosten-Abwägungen für Gebühren-Bumps, keine erforderliche Einrichtung. Neue Werte gelten für künftige Bumps; bereits finanzierte Swaps behalten die Richtlinie, unter der sie finanziert wurden.",
    feeMax: "Maximale Feerate (sat/vB)",
    feeMaxHint:
      "Obergrenze für jeden Gebühren-Bump. Standard 500, zugleich das harte Systemmaximum. Senke ihn, um Kosten zu deckeln.",
    feeReservation: "Reserve für Finanzierungs-Bump (×)",
    feeReservationHint:
      "Anteil des Guthabens, den die Mittelprüfung als Bump-Puffer zurücklegt. Höher rettet größere Gebührenspitzen, bindet aber mehr Guthaben und lehnt mehr Swaps ab. Standard 3.",
    feeCommitted: "Redeem-Überprovisionierung (×)",
    feeCommittedHint:
      "Wie viel zusätzlich die v2-Redeem-Gebühr vorausbezahlt wird, damit sie auch bei geschlossenem Satchel bestätigt. Gilt nur für neue Swaps. Standard 2.",
    feeSave: "Speichern",
    feeSaving: "Speichern…",
    feeSaved: "Gespeichert",
    feeReset: "Auf Standard zurücksetzen",
    coins: "Coins & Nodes",
    coinsHint: "Verbinde jeden Coin mit deiner eigenen Node. Der Genesis-Block wird geprüft, bevor irgendetwas gespeichert wird.",
    about: "Über",
    version: "Version {version}",
    updateUpToDate: "Aktuell",
    updateCheckPlaceholder: "Die Update-Prüfung kommt in einer späteren Version.",
    trustModel: "Wo deine Schlüssel liegen",
    trustModelBody:
      "Geheimnisse liegen in der Engine, nie in Satchel. Das Merchant-Seed liegt im Datenordner der Engine (verschlüsselt oder im Klartext — deine Wahl); Satchel speichert weder Seed noch Passphrase. Das Seed ist von Natur aus heiß (nur Transit-Schlüssel) — überweise nennenswerte Erlöse auf deine eigene Cold-Wallet.",
  },
  coins: {
    intro:
      "Verbinde jeden Coin mit deiner eigenen Node. Die erste URL ist die eigene Wallet deiner Node — sie finanziert deine Swap-Legs und empfängt die Erlöse. Bevor irgendetwas gespeichert wird, prüft Satchel den Genesis-Block der Node, damit Mittel nie an die falsche Chain gesendet werden können. Verbindungen werden über alle deine Merchants hinweg geteilt.",
    networkBadge: "Konfiguration für das {network}-Netzwerk",
    needMerchant:
      "Verbinde zuerst einen Merchant — die Coin-Einrichtung braucht eine laufende Engine. Nutze den Merchant-Selektor oben rechts.",
    pairsTitle: "Handelspaare",
    pairsHint:
      "Paare leiten sich daraus ab, was jeder Coin kann — es gibt keine feste Liste. Ein Paar öffnet sich, sobald beide seiner Coins verbunden sind.",
    noPairs: "Keine Paare verfügbar.",
    notSetUp: "Nicht eingerichtet",
    connectedTip: "Verbunden · Tip {tip}",
    connError: "Verbindungsfehler",
    setUp: "Einrichten",
    editConnection: "Verbindung bearbeiten",
    remove: "entfernen",
    disconnectTip: "Diesen Coin trennen",
    disconnectTitle: "{coin} trennen?",
    disconnectBody: "Swaps, die ihn brauchen, sind nicht verfügbar, bis du wieder verbindest.",
    ready: "Handelsbereit",
    connectMissing: "{coins} verbinden",
    notBuildable: "Noch nicht baubar",
    // Swap-protocol chips on a pair (pact-htlc-v1 HTLC vs pact-htlc-v2 adaptor).
    protoPrivate: "Privat (Taproot)",
    protoPrivateTip: "Privater Swap (Taproot/MuSig2-Adaptor) — sieht On-Chain wie eine gewöhnliche Zahlung aus",
    protoHtlcTip: "Klassischer HTLC-Swap",
    // Coin-setup backend choices (CoinSetup).
    backendCoreTitle: "Core-RPC-Wallet",
    backendCoreDesc: "Die eigene Wallet deiner Node finanziert den Swap und empfängt die Erlöse.",
    backendHardwareTitle: "Hardware",
    backendHardwareDesc: "Ledger-/PSBT-Signierung für das Finanzierungs-Leg.",
    backendLater: "später",
    // CoinSetup dialog.
    setupTitle: "{coin} verbinden",
    setupIntro:
      "Richte Satchel auf deine eigene {sym}-Node. Nichts wird gespeichert, bis die Node eine Genesis-Block-Prüfung besteht — deine Mittel berühren nur jemals die echte {sym}-Chain.",
    backendUrlLabel: "Node-Backend-URL(s)",
    backendUrlHint:
      "Erste URL = die eigene Wallet deiner Node (finanziert Swaps, empfängt Erlöse). Füge nach Kommas Electrum-Server (tcp://host:port) für zusätzliche, unabhängige Chain-Sichten hinzu.",
    fundingWallet: "Finanzierungs-Wallet",
    confirmationsLabel: "Bestätigungen bis final",
    confirmationsHint:
      "Wie tief eine Finanzierung oder ein Redeem auf dieser Chain sein muss, bevor ein Swap darauf reagiert — der Reorg-Sicherheitspuffer. Höher ist sicherer, aber langsamer; lass es leer für den empfohlenen Standard ({default}).",
    validateNode: "Node validieren",
    checking: "Node wird geprüft…",
    genesisOk: "Genesis stimmt überein — dies ist die richtige Chain",
    genesisDetail: "Tip-Höhe {tip} · Genesis {hash}…",
    genesisBad: "Abgelehnt — wird nicht gespeichert",
    errorShort: "Fehler",
    // Structured connection form (CoinSetup v2).
    rpcHostLabel: "RPC-Host",
    rpcPortLabel: "RPC-Port",
    authMethodLabel: "Authentifizierung",
    authCookie: "Cookie-Datei",
    authCookieDesc: "Liest die .cookie der Node automatisch aus ihrem Datenverzeichnis (der Standard, kein Passwort gespeichert).",
    authUserPass: "Benutzer / Passwort",
    authUserPassDesc: "rpcuser / rpcpassword aus der Konfiguration deiner Node — nötig für eine entfernte Node.",
    rpcUserLabel: "RPC-Benutzername",
    rpcPasswordLabel: "RPC-Passwort",
    datadirLabel: "Datenverzeichnis der Node",
    cookiePathNote: "Das Cookie wird aus {path} unter diesem Verzeichnis gelesen.",
    walletLabel: "Wallet-Name (optional)",
    walletPlaceholder: "die Wallet deiner Node",
    needPort: "Gib zuerst den RPC-Port ein.",
    validateFirst: "Validiere die Node vor dem Speichern.",
    savingReconnecting: "Speichern & neu verbinden…",
    connected: "{coin} verbunden",
    // Template picker (a coins.toml coin the engine version doesn't support).
    unsupportedByEngine: "Nicht unterstützt",
    unsupportedByEngineTip:
      "Dieser Coin ist in coins.toml definiert, aber nicht in diese Version der Engine eingebaut, daher kann er nicht gehandelt werden.",
  },
  coinWizard: {
    title: "Verbinde deine Coins",
    intro:
      "Wähle mindestens zwei Coins und richte jeden auf deine eigene Node. Ein Swap braucht zwei Chains, daher wird der Handel freigeschaltet, sobald zwei Nodes verbunden und live sind. Du kannst Coins später in den Einstellungen hinzufügen oder ändern.",
    progress: "{count} von {min} Coins verbunden",
    continue: "Weiter",
    live: "Live",
    nodeDown: "Node offline",
  },
  wallets: {
    intro:
      "Dies sind die Wallets deiner eigenen Nodes (die, die die Engine nutzt, um Swaps zu finanzieren und Erlöse zu empfangen) — deine Schlüssel, deine Maschine. Satchel hält nie deine Coins.",
    hotSeedNudge:
      "Dies ist eine Ausgabe-Wallet auf einem heißen Seed, kein Tresor — überweise nennenswerte Guthaben auf deine eigene Cold-/Core-Wallet.",
    notConnected: "Nicht verbunden",
    notConnectedBody: "Verbinde zuerst einen Merchant — die Wallet-Ansicht braucht eine laufende Engine.",
    noCoins: "Noch keine Coins eingerichtet",
    noCoinsBody: "Verbinde einen Coin unter Einstellungen → Coins, und seine Wallet erscheint hier.",
    goToCoins: "Zu Coins",
    watchOnlyTitle: "Keine Wallets im Nur-Beobachten-Modus",
    watchOnlyBody:
      "Dies ist eine Nur-Beobachten-Sitzung ohne verbundene Coins, daher gibt es keine Wallets anzuzeigen. Schalte Nur-Beobachten in den Einstellungen aus und verbinde einen Coin, um Swaps zu finanzieren.",
    walletName: "Wallet · {wallet}",
    walletScopedHint: "Jeder RPC für diesen Coin ist auf diese Node-Wallet beschränkt.",
    walletDefault: "Standard-Wallet (nicht beschränkt)",
    walletDefaultHint:
      "Für diesen Coin ist keine Wallet gesetzt, daher nutzen RPCs die Standard-Wallet der Node. Setze eine unter Einstellungen → Coins, um jeden Aufruf auf eine bestimmte Wallet zu beschränken.",
    balanceLabel: "{symbol}-Guthaben",
    receive: "Empfangen",
    send: "Senden",
    sendTo: "An Adresse senden",
    amount: "Betrag",
    sendTitle: "{amount} {sym} senden?",
    sendConfirmBody: "An {to}\n\nDies gibt aus der eigenen Wallet deiner Node aus und kann nicht rückgängig gemacht werden.",
  },
  corkboard: {
    noBoardTitle: "Kein Corkboard verbunden",
    noBoardBody:
      "Ein Corkboard ist ein gemeinsames Schwarzes Brett, an das Maker Angebote pinnen. Es führt nie Trades zusammen und hält keine Coins — richte Satchel auf eins, dem du vertraust, um zu stöbern und einzustellen.",
    noPairs: "Keine Paare verfügbar",
    board: "Corkboard",
    boardSettings: "In den Einstellungen konfigurieren",
    filterAll: "Alle",
    filterMine: "Meine",
    offered: "{symbol} angeboten",
    noOffers: "Derzeit keine Angebote, die du annehmen kannst",
    noOffersBody:
      "Angebote erscheinen hier, sobald ein Maker eins für ein von dir eingerichtetes Paar einstellt. Du kannst auch eigene einstellen.",
    hiddenOffers:
      "{count} weitere(s) Angebot(e) für Paare, die du nicht verbunden hast. Richte beide Coins ein, um sie zu handeln:",
    yourOffer: "dein Angebot",
    offerStaged: "wird eingestellt…",
    offerStagedTip:
      "Von diesem Gerät eingestellt und wartet auf Rückbestätigung von einem Relay. Es wirbt; es wird live, sobald ein Relay es zurückspielt.",
    take: "Angebot annehmen",
    legDown: "Die Node eines dieser Paare ist offline — starte sie (oder prüfe Einstellungen → Coins), bevor du annimmst.",
    withdraw: "Zurückziehen",
    withdrawTip: "Sofort zurückziehen — ein Angebot sperrt nie Mittel",
    safetyRefund: "Sicherheits-Erstattung",
    safetyRefundTip:
      "Wenn der Swap stockt, erstatten beide Seiten automatisch — das Leg des Takers entsperrt sich zuerst, deins etwas später. Niemand bleibt stecken.",
    activeTitle: "Deine aktiven Swaps",
    states: {
      open: "offen",
      takenByUs: "von dir angenommen",
      revoked: "zurückgezogen",
      expired: "abgelaufen",
    },
    // Two-sided order-book view of the Corkboard. Bids = makers giving the
    // quote coin to get the base; asks = the reverse. The ladder is a way to
    // READ the board — it never matches or prioritises (load-bearing).
    book: {
      bids: "Gebote",
      asks: "Briefe",
      bidsHint: "wollen {base} · zahlen {quote}",
      asksHint: "verkaufen {base} · für {quote}",
      price: "Preis",
      size: "Größe",
      noBids: "Keine Gebote",
      noAsks: "Keine Briefe",
      spread: "Spread {pct}",
      spreadOneSided: "Einseitig",
      crossed: "gekreuzt",
      crossedTip: "Höchstes Gebot ≥ niedrigster Brief. Das Board führt nie automatisch zusammen, daher liegen diese sich überschneidenden Angebote einfach da — nimm eine der beiden Seiten an.",
      mid: "Mitte {price}",
      levelOffers: "{count} Angebot(e) zu diesem Preis — wähle eines zum Annehmen",
      depthTip: "Insgesamt {sym} im Angebot zu diesem Preis über {count} Anzeige(n).",
      takerNote: "Wenn du es annimmst, gibst du {give} und erhältst {get}.",
      selectLevel: "Wähle oben ein Preisniveau, um die Angebote dort zu sehen.",
      paneHeader: "{size} {base} @ {price} {unit}",
      denomTip: "Anzeigeeinheit für {coin}-Beträge",
      showMore: "{count} weitere anzeigen",
      showLess: "Top {count} anzeigen",
    },
  },
  relays: {
    title: "Relays",
    subtitle: "Live-Konnektivität zu deinen Nostr-Relays — das Netzwerk, über das deine Angebote und Annahmen laufen. Füge Relays unter Einstellungen → Netzwerk hinzu oder entferne sie.",
    connectedCount: "{up} / {total} verbunden",
    refresh: "Aktualisieren",
    ms: "{ms} ms",
    up: "online",
    down: "offline",
    statsTip: "{success}/{attempts} erfolgreiche Verbindungen · ↓{down} ↑{up}",
    none: "Keine Relays konfiguriert",
    noneBody: "Füge unter Einstellungen → Netzwerk ein Nostr-Relay hinzu, um Angebote über das Netzwerk zu veröffentlichen und zu empfangen.",
    goToNetwork: "Zu den Einstellungen",
    notConnected: "Nicht verbunden",
    notConnectedBody: "Die Relay-Ansicht braucht eine laufende Engine — verbinde zuerst einen Merchant.",
  },
  swaps: {
    title: "Swaps",
    hint: "Dein vollständiges Verzeichnis — laufende Swaps oben, abgeschlossene Trades darunter. Du kannst auch von der Corkboard aus auf laufende Swaps einwirken.",
    activeTitle: "Laufend",
    historyTitle: "Historie",
    none: "Noch keine Swaps — nimm ein Angebot auf der Corkboard an.",
    // Active-swaps dock action buttons + the refund-time label.
    cancel: "abbrechen",
    refund: "erstatten",
    dump: "Logs ausgeben",
    dumpHint: "Kopiere ein geheimnisfreies Diagnose-Paket (Status + Log-Zeilen) für diesen Swap, um es den Entwicklern zu schicken.",
    dumpCopied: "Diagnose kopiert — an die Entwickler schicken.",
    dumpFailed: "Das Diagnose-Paket konnte nicht kopiert werden.",
    refundAt: "Erstattung {when}",
    // Confirm dialogs for acting on a live swap (ActiveSwaps).
    cancelTitle: "Diesen Swap abbrechen?",
    cancelConfirm: "Swap abbrechen",
    cancelKeep: "Behalten",
    // Abort reason recorded on the swap when cancelled from Satchel.
    cancelReason: "in Satchel abgebrochen",
    cancelBody:
      "Dies bricht den Swap ab, bevor du finanziert hast. Noch ist nichts von dir gesperrt, du verlierst also nichts — das Angebot wird nur nicht abgeschlossen.",
    refundTitle: "Deine Mittel zurückholen?",
    refundConfirm: "Erstatten",
    refundBody:
      "Der Sicherheits-Timelock ist abgelaufen, du kannst also die von dir gesperrten Mittel zurückfordern. Dies sendet deine Erstattung jetzt; die Engine tut dies nach der Frist auch automatisch.",
    col: {
      swap: "Swap",
      role: "Rolle",
      state: "Status",
      amounts: "gibt → erhält",
      when: "wann",
      finalTx: "finale Tx",
    },
    // Expandable per-leg on-chain detail (the audit trail). We show both
    // funding txs + OUR settlement; never the counterparty's settlement or the
    // swap secret.
    audit: {
      toggle: "On-Chain-Details anzeigen",
      title: "On-Chain-Details",
      youLocked: "du hast gesperrt",
      theyLocked: "sie haben gesperrt",
      funding: "Finanzierung",
      received: "Empfangen",
      refunded: "Erstattet",
      pending: "noch nicht On-Chain",
      copy: "Transaktions-ID kopieren",
      copied: "Transaktions-ID kopiert",
    },
  },
  fees: {
    title: "Vorschau der Netzwerkkosten",
    estimated: "geschätzt",
    provisionalNote: "Dieser pactd-Build stellt noch keine Gebührenschätzung bereit.",
    summary: "Ein Swap besteht aus 2 On-Chain-Transaktionen, die du bezahlst: Finanzierung auf der Geben-Chain, Redeem auf der Erhalten-Chain.",
    fallbackTip: "Eine Node war nicht erreichbar, daher wurde eine konservative Standard-Feerate verwendet — betrachte diese als Schätzung.",
    ifItStalls: "(falls er stockt)",
  },
  funds: {
    insufficient:
      "Nicht genug {sym}, um diesen Swap zu finanzieren — benötigt ~{need} {sym} (Betrag + Finanzierungsgebühr), Wallet hat {have} {sym}.",
  },
  wizard: {
    welcome: "Willkommen bei Satchel",
    connectTitle: "Die Pact-Engine verbinden",
    connectIntro:
      "Satchel ist ein schlanker Client der Pact-Engine — dem Kern, der deine Schlüssel hält und die Swaps ausführt. Wähle, wie du sie erreichst.",
    managed: "Die eingebaute Pact-Engine starten",
    managedDesc: "Satchel startet und überwacht seine eigene Pact-Engine. Empfohlen.",
    external: "Mit einer externen Pact-Engine verbinden",
    externalDesc: "Richte sie auf eine bereits laufende Pact-Engine (setze SATCHEL_PACTD_URL + Cookie vor dem Start).",
    externalNote:
      "Der externe Modus wird über Umgebungsvariablen vor dem Start von Satchel ausgewählt. Starte mit gesetztem SATCHEL_PACTD_URL neu, um ihn zu nutzen.",
    coinsTitle: "Füge deine Coins hinzu",
    coinsIntro:
      "Nachdem dein Merchant erstellt ist, verbinde jeden Coin unter Einstellungen → Coins mit deiner eigenen Node. Wähle einen Coin und ein Backend (öffentliches Electrum für null Einrichtung oder deine eigene Node); der Genesis wird vor dem Speichern gegen dieses Netzwerk geprüft.",
    coinsTemplatesSoon: "Ein-Klick-Coin-Vorlagen kommen hier in einer späteren Version.",
    back: "Zurück",
    continue: "Weiter",
    finish: "Einrichtung abschließen",
  },
  // UI-4 docked activity log.
  log: {
    title: "Aktivität",
    empty: "— Aktivitätsprotokoll —",
    count: "{count} Zeilen",
    collapse: "Protokoll einklappen",
    expand: "Protokoll ausklappen",
    // Activity-log lines emitted by the frontend. `{err}` carries a raw engine
    // message (itself not translated — it comes from pactd over the wire).
    noTauri: "läuft nicht innerhalb von Satchel — diese Oberfläche braucht die Tauri-Brücke",
    startupError: "Start: {err}",
    notConnected: "nicht verbunden: {err}",
    connected: "verbunden mit pactd {version} ({protocol})",
    listcoinsError: "listcoins: {err}",
    watchOnlyError: "Nur-Beobachten: {err}",
    switchedMerchant: "zu Merchant {id} gewechselt",
    switchMerchantError: "Merchant wechseln: {err}",
    loadMerchantError: "Merchant laden: {err}",
    merchantCreated: "Merchant {id} erstellt",
    merchantReady: "Merchant bereit",
    actionOk: "{action} {id}: ok",
    actionError: "{action} {id}: {err}",
    diagCopied: "Diagnose für {id} kopiert ({count} Log-Zeilen) — an die Entwickler schicken",
    dumpError: "Dump {id}: {err}",
    coinDisconnected: "{coin} getrennt",
    removeCoinError: "Coin entfernen: {err}",
    tookOffer: "Angebot {id} angenommen — es erscheint nun unten in deinen aktiven Swaps",
    takeError: "Annahme: {err}",
    offerWithdrawn: "Angebot {id} zurückgezogen",
    withdrawError: "Zurückziehen: {err}",
    postedOffer: "Angebot {id} eingestellt — jederzeit zurückziehbar; nichts ist gesperrt",
    createdSlip: "einen privaten Angebots-Slip erstellt — schicke ihn deinem Freund",
    tookPrivateOffer: "privates Angebot {id} angenommen — es erscheint nun in deinen aktiven Swaps",
    cancelledPrivateOffer: "privates Angebot {id} abgebrochen",
    cancelError: "Abbruch: {err}",
    noticeboardUpdated: "Schwarzes Brett aktualisiert",
    feePolicyUpdated: "Gebührenrichtlinie aktualisiert",
  },
  // Relative-time + freshness prose from format.ts (rendered via the tr() mirror,
  // since those are pure non-component helpers). Unit letters stay inside the
  // template so a translation owns the whole phrase.
  format: {
    ageUnknown: "Alter unbekannt",
    justNow: "gerade eben",
    minutesAgo: "vor {n} Min.",
    hoursAgo: "vor {n} Std.",
    daysAgo: "vor {n} Tg.",
    expiryNow: "jetzt",
    expirySoon: "bald",
    inMinutes: "in ~{n} Min.",
    inHours: "in ~{n} Std.",
    inDays: "in ~{n} Tg.",
    posted: "eingestellt {age}",
    expires: "läuft ab {time}",
  },
  // Plain-language swap story per (role, state) — the honest "who is exposed
  // when" framing shown on every active swap. {a}/{b} are coin tickers; {t1}/{t2}
  // are local refund times. Rendered via tr() (narrate() is a pure helper).
  narrate: {
    finalizing: "Du hast deine {got} beansprucht — finale Bestätigungen. Lass die App offen, bis es vergraben ist; deine {gave} bleiben bis dahin geschützt.",
    initiating:
      "Annahme gesendet — Warten darauf, dass der Maker den Swap startet. Noch ist nichts gesperrt; er bricht von selbst ab, wenn er nicht antwortet.",
    created: "Angebot gesendet — Warten darauf, dass die andere Seite zustimmt. Nichts ist festgelegt.",
    acceptedMaker: "Bedingungen vereinbart. Als Nächstes: sperre deine {a}. Bis du finanzierst, kannst du frei abbrechen.",
    acceptedTaker: "Bedingungen vereinbart. Die andere Seite sperrt ihre {a} zuerst — du sendest nie zuerst.",
    noncesExchanged:
      "Der private Swap wird eingerichtet — Signiermaterial wird ausgetauscht. Noch ist nichts gesperrt.",
    signedMaker:
      "Beide Seiten haben signiert. Dein Daemon sperrt die {a} und beansprucht dann automatisch die {b}. Falls etwas stockt, kommen deine {a} um {t1} zurück.",
    signedTaker:
      "Beide Seiten haben signiert. Dein Daemon sperrt die {b} und beansprucht die {a}, sobald die andere Seite handelt. Sicherheitsnetz: Erstattung um {t2}.",
    fundedAMaker:
      "Deine {a} sind gesperrt. Warten darauf, dass die andere Seite ihre {b} sperrt. Falls sie es nie tut, kommen deine {a} automatisch um {t1} zurück.",
    fundedATaker:
      "Ihre {a} sind gesperrt und verifiziert. Als Nächstes: sperre deine {b}. Sicherheitsnetz: automatische Erstattung um {t2}, falls etwas stockt.",
    fundedBMaker: "Beide gesperrt. Dein Daemon beansprucht die {b}, sobald sie sicher bestätigt ist.",
    fundedBTaker: "Beide gesperrt. Dein Daemon beansprucht die {a}, sobald die andere Seite ihre {b} nimmt.",
    redeemedB:
      "Du hast die {b} beansprucht — Warten auf die Bestätigung. Deine gesperrten {a} bleiben geschützt, bis dies final ist.",
    completed: "Swap abgeschlossen — die {coin} sind in deiner Wallet.",
    refunded: "Der Swap wurde nicht abgeschlossen, daher kamen deine {coin} automatisch zurück. Außer Gebühren nichts verloren.",
    aborted: "Abgebrochen, bevor Geld geflossen ist.",
  },
  progress: {
    awaitingLock: "Warten auf deren Sperre",
    awaitingClaim: "Warten auf deren Einlösung",
    theirLock: "Deren Sperre wird bestätigt",
    securing: "Sichere deine {coin}",
    blocks: "+{n} Blöcke",
    feeBumped: "Gebühr erhöht",
    reorg: "Reorg erkannt — wird erneut geprüft",
  },
  exit: {
    // Exit-gate dialog (fund safety, C6). The engine manages alone, so "keep
    // running" detaches it (it keeps watching timelocks + servicing offers).
    liveTitle: "Ein Swap läuft",
    liveBodyOne:
      "1 Swap läuft gerade. Er wird durch On-Chain-Timelocks gesteuert — die Engine muss weiterlaufen, um vor der Frist zu redeemen oder zu erstatten.",
    liveBodyMany:
      "{count} Swaps laufen gerade. Sie werden durch On-Chain-Timelocks gesteuert — die Engine muss weiterlaufen, um vor der Frist zu redeemen oder zu erstatten.",
    keepRunningExplain:
      "Das Schließen des Fensters lässt die Engine im Hintergrund weiterlaufen, sodass sie den Swap headless abschließt. Du kannst Satchel jederzeit wieder öffnen, um nachzusehen.",
    forceQuitWarn: "Ein erzwungenes Beenden jetzt stoppt die Engine und kann Mittel verlieren.",
    // {word} is the confirm word below; a translation may localize both together.
    typeToConfirm: "Um trotzdem zu erzwingen, tippe unten {word} ein.",
    confirmWord: "QUIT",
    keepRunning: "Weiterlaufen lassen, Fenster schließen",
    keepWithdraw: "Weiterlaufen lassen + Angebote zurückziehen",
    keepLeaveOffers: "Weiterlaufen lassen, Angebote stehen lassen",
    forceQuit: "Beenden erzwingen",
    offersTitle: "Du hast Angebote eingestellt",
    offersBodyOne:
      "1 Angebot von dir steht noch auf dem Corkboard. Angebote sperren nichts, aber wenn du es stehen lässt, können Gegenparteien es weiterhin annehmen, während Satchel geschlossen ist — die Engine bedient die Annahme.",
    offersBodyMany:
      "{count} Angebote von dir stehen noch auf dem Corkboard. Angebote sperren nichts, aber wenn du sie stehen lässt, können Gegenparteien sie weiterhin annehmen, während Satchel geschlossen ist — die Engine bedient die Annahmen.",
    withdrawExit: "Alle zurückziehen & beenden",
  },
  unlock: {
    title: "Merchant entsperren",
    body:
      "Das Seed dieses Merchants ist verschlüsselt. Gib seine Passphrase ein, um ihn für diese Sitzung zu entsperren — Satchel hält sie nur im Speicher und vergisst sie beim Beenden.",
    switchMerchant: "Merchant wechseln",
    unlock: "Entsperren",
  },
  common: {
    cancel: "Abbrechen",
    confirm: "Bestätigen",
    save: "Speichern",
    done: "Fertig",
    later: "Später",
    retry: "Verbindung erneut versuchen",
  },
};
