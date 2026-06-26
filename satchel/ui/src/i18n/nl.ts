// The Dutch (Nederlands) string bundle. Mirrors en.ts key-for-key; only values are translated.
import type { Bundle } from "./en";

export const nl: Bundle = {
  app: {
    name: "Satchel",
  },
  // In-app update notifications (sidebar version badge + dialog).
  update: {
    title: "Update beschikbaar",
    upToDate: "Je bent up-to-date",
    current: "Geïnstalleerd",
    latest: "Nieuwste",
    notesTitle: "Release-notities",
    get: "Haal de update op",
    dismiss: "Negeren",
    close: "Sluiten",
    badgeTooltip: "Update beschikbaar — klik voor details",
    versionTooltip: "Klik om te controleren op updates",
  },
  // Risk disclaimer (first-run welcome + Settings → About).
  disclaimer: {
    title: "Zelfbeheer — jouw sleutels, jouw verantwoordelijkheid",
    body: "Satchel voert non-custodial atomic swaps uit: alleen jij beheert je sleutels, en het seed van een merchant houdt hot transit-sleutels vast terwijl een swap loopt. De swap-protocollen (v1 HTLC en v2 Taproot/MuSig2) zijn beoordeeld en draaien live op mainnet. MIT-gelicentieerd en geleverd zoals het is, zonder enige garantie — maak een back-up van je herstelzin en gebruik op eigen risico.",
  },
  nav: {
    public: "Openbaar",
    corkboard: "Corkboard",
    postOffer: "Plaats een aanbod",
    private: "Privé",
    privateCreate: "Slip aanmaken",
    privateReceive: "Slip aannemen",
    privateSlips: "Mijn slips",
    swaps: "Swaps",
    relays: "Relays",
    wallets: "Wallets",
    settings: "Instellingen",
    coins: "Munten",
  },
  makeOffer: {
    title: "Plaats een aanbod",
    intro:
      "Plaats een ondertekend aanbod op de Corkboard. Er wordt niets vergrendeld — het is slechts een advertentie; trek het op elk moment terug, en een swap begint pas wanneer iemand het aanneemt en beide kanten funden.",
    give: "Jij geeft",
    want: "Jij ontvangt",
    // Canonical pair + direction: pick the pair, choose Sell/Buy the base, enter
    // the base amount and a quote-per-base price (invariant to direction).
    pair: "Paar",
    noPairs: "Geen verhandelbare paren — verbind minstens twee munten in Instellingen → Munten.",
    sell: "Verkoop {sym}",
    buy: "Koop {sym}",
    amount: "Bedrag",
    youGive: "Jij geeft",
    youGet: "Jij krijgt",
    price: "Prijs",
    priceUnit: "{unit} per {base}",
    pricePlaceholder: "eenheidsprijs",
    balance: "Saldo: {amt} {sym}",
    balanceLoading: "Saldo: …",
    noCoins: "Geen munten geconfigureerd",
    legDown: "Een van de nodes van deze munten ligt eruit — start hem (of controleer Instellingen → Munten) voordat je plaatst.",
    // Swap-protocol pin (only offered when a pair+network supports more than
    // one). v2 label reuses coins.protoPrivate ("Private (Taproot)").
    protocol: "Swap-type",
    protoStandard: "Standaard (HTLC)",
    // Titles for the review/confirm dialog shown before posting (see OfferForm).
    reviewOfferTitle: "Controleer je aanbod",
    reviewSlipTitle: "Controleer je slip",
    term: "Veiligheids-timelock",
    termShort: "Kort",
    termMedium: "Gemiddeld",
    termLong: "Lang",
    termHint: {
      short: "Kort — fondsen krijgen het snelst een auto-refund als de handel vastloopt (~12u / 6u), met de kleinste veiligheidsmarge.",
      medium: "Gemiddeld — gebalanceerd refund-venster (~24u / 12u).",
      long: "Lang (veiligst) — breedste veiligheidsmarge; auto-refund na ~36u / 18u als de handel vastloopt.",
    },
    validFor: "Geldig gedurende (minuten)",
    validForMins: "{mins} min",
    validForHint:
      "Hoe lang het aanbod genoteerd blijft. Terwijl je online bent wordt het automatisch vers gehouden; daarna verloopt het. Het sluiten van de app trekt het terug.",
    note: "Aanbod met vaste grootte — er wordt niets vergrendeld tot iemand het aanneemt. Bedragen zijn on-chain; je betaalt daarbovenop netwerkkosten en de Corkboard rekent niets. De timelock is het auto-refund-venster als een swap vastloopt.",
    post: "Aanbod plaatsen",
    makeSlip: "Slip aanmaken",
    slipTitle: "Je privé aanbod-slip",
    slipExplainer:
      "Stuur dit naar je vriend. Hij plakt het in Satchel om het aan te nemen. Er wordt niets vergrendeld; het verloopt over {ttl}.",
    copy: "Kopiëren",
    copied: "Gekopieerd",
    makeAnother: "Maak er nog een",
    myPrivateTitle: "Mijn privé aanbiedingen",
    myPrivateEmpty: "Geen openstaande privé aanbiedingen.",
    privateExpires: "verloopt {when}",
    privateExpired: "verlopen",
    cancel: "Annuleren",
    cancelTip: "Stop met het honoreren van deze slip — een vriend die hem nog heeft, kan hem niet langer aannemen.",
  },
  takeSlip: {
    intro:
      "Een vriend heeft je een privé aanbod-slip gestuurd (die begint met pactoffer1:). Plak hem hier om hem te bekijken en aan te nemen — precies zoals een aanbod van het bord.",
    placeholder: "pactoffer1:…",
    take: "Bekijken & aannemen",
    invalid: "Dat lijkt niet op een slip — hij hoort te beginnen met pactoffer1:.",
    previewLabel: "Deze slip biedt",
  },
  // PRIVATE nav group (off-market slips) — screen titles + intros. The form,
  // slip output, my-slips list and take flow reuse makeOffer.*/takeSlip.* copy.
  private: {
    createTitle: "Maak een privé aanbod aan",
    createIntro:
      "Bouw een ondertekend aanbod en geef het als slip aan een vriend via je eigen chat. Er wordt nergens iets genoteerd — en er wordt niets vergrendeld tot jullie beiden funden.",
    slipsIntro:
      "Slips die je hebt aangemaakt. Iedereen die een slip heeft, kan hem aannemen tot hij verloopt; annuleer er een om hem niet langer te honoreren.",
    slipsEmptyBody: "Maak een privé aanbod aan om een slip te krijgen die je naar een vriend kunt sturen.",
    receiveTitle: "Neem een privé aanbod aan",
    received: "Aangenomen — volg het in Swaps.",
  },
  // Shared take-confirmation dialog (board take + slip take).
  takeConfirm: {
    title: "Dit aanbod aannemen?",
    confirm: "Aanbod aannemen",
    counterparty: "Tegenpartij",
    youGive: "Jij geeft",
    youReceive: "Jij ontvangt",
    safetyRefund: "Veiligheids-refund",
    offerAge: "Leeftijd aanbod",
    makerFundsFirst:
      "De maker vergrendelt zijn {sym} eerst — jij stuurt nooit als eerste. Je kunt nog steeds annuleren voordat je je kant fundt, en de engine doet een auto-refund na de veiligheids-timelock als de swap vastloopt.",
  },
  header: {
    activeMerchant: "Actieve merchant — klik om te wisselen of te beheren",
    manageMerchants: "Merchants beheren…",
    noMerchant: "geen merchant",
    openMenu: "Menu openen",
    collapseMenu: "menu inklappen",
    settings: "Instellingen",
    language: "Taal",
    pactConnected: "Engine verbonden",
    pactUnreachable: "Engine onbereikbaar",
    liveSwapsOne: "1 swap onderweg — klik om te bekijken",
    liveSwapsMany: "{count} swaps onderweg — klik om te bekijken",
    liveSwapsNone: "Geen swaps onderweg",
    coinOk: "{name} — verbonden · tip {tip}",
    coinUnconfigured: "{name} — niet ingesteld",
    coinError: "{name} — {status}",
    relaysOk: "Nostr-relays — {up}/{total} verbonden",
    relaysDown: "Nostr-relays — geen van {total} verbonden",
  },
  network: {
    mainnet: "MainNet",
    testnet: "TestNet",
    regtest: "RegTest",
    signet: "Signet",
    notRealFunds: "Geen echte fondsen — dit is het {network}-netwerk",
  },
  // Watch-only mode: a viewer session with no coins. Browse the board and
  // withdraw your own offers, but no posting / taking / funding.
  watchOnly: {
    badge: "Alleen-bekijken",
    badgeTip:
      "Alleen-bekijken-modus — bekijk het bord en trek je eigen aanbiedingen terug, maar je kunt niet plaatsen, aannemen of funden. Stel munten in bij Instellingen om te handelen.",
    coinWizardButton: "Bekijken in alleen-bekijken-modus",
    coinWizardHint:
      "Sla muntinstelling over en bekijk gewoon het bord (alleen-lezen). Je kunt nog steeds je eigen aanbiedingen terugtrekken — handig om aanbiedingen op te halen die door een andere sessie zijn achtergelaten. Schakel het op elk moment uit in Instellingen.",
    postBlockedTitle: "Alleen-bekijken-modus",
    postBlockedBody:
      "Dit is een alleen-bekijken-sessie, dus er kunnen geen aanbiedingen worden geplaatst. Stel minstens twee munten in bij Instellingen → Munten om te handelen.",
    takeBlockedBody: "Alleen-bekijken-modus — je kunt dit aanbod bekijken, maar om het aan te nemen zijn ingestelde munten nodig.",
    takeBlockedTip: "Alleen-bekijken-modus — stel munten in bij Instellingen om aanbiedingen aan te nemen.",
  },
  merchants: {
    title: "Jouw merchants",
    intro:
      "Een merchant is één handelsidentiteit — met een eigen seed en swap-geschiedenis. Handelen onder een andere merchant houdt contexten onkoppelbaar (een wegwerpidentiteit). Je belangrijkste munten staan in je eigen wallet, niet hier.",
    // First-run welcome (empty merchant manager).
    welcomeTitle: "Welkom bij Satchel",
    welcomeIntro:
      "Satchel handelt onder een “merchant” — één handelsidentiteit met een eigen seed. Je hebt er nog geen: maak een nieuwe aan, of importeer een bestaande herstelzin om te beginnen.",
    importMerchant: "Importeer een merchant",
    none: "Nog geen merchants.",
    switch: "wisselen",
    newMerchant: "Nieuwe merchant",
    thisMerchant: "deze merchant",
    nameLabel: "Merchant-naam",
    namePlaceholder: "bijv. Hoofd",
    introFirst:
      "Stel je eerste handelsidentiteit in (een “merchant”). Hij houdt alleen hot transit-sleutels vast voor lopende swaps — je belangrijkste munten blijven in je eigen wallet.",
    introNew: "Een nieuwe merchant is een verse, aparte identiteit met een eigen seed en swap-geschiedenis.",
    // UI-5 merchant selector (phoenix wallet-selector parity).
    createNew: "Nieuwe aanmaken",
    import: "Importeren",
    load: "Merchant laden",
    loaded: "geladen",
    locked: "vergrendeld",
    lockedTip: "Versleuteld seed — ontgrendel met je wachtwoordzin wanneer je hem laadt.",
    close: "Sluiten",
    idLabel: "map",
    switching: "Merchant wisselen…",
    switchingBody: "De engine wordt opnieuw gestart tegen die map.",
  },
  // Seed create/import (SeedForm) + counterparty + status.
  seed: {
    intro: "Maak een gloednieuw seed aan, of importeer er een die je al hebt.",
    createNew: "Nieuwe aanmaken",
    createDesc: "Genereer een vers seed. Jij maakt een back-up van de herstelzin.",
    import: "Importeren",
    importDesc: "Herstel vanuit een bestaande 12/24-woordenzin.",
    recoveryLabel: "Herstelzin",
    encrypt: "Versleutelen",
    encryptDesc:
      "Een wachtwoordzin beschermt het seed in rust. Je voert hem één keer per sessie in — Satchel slaat hem nooit op. Let op: onbeheerde auto-refund pauzeert na een herstart tot je hem opnieuw invoert.",
    noPassphrase: "Geen wachtwoordzin (aanbevolen)",
    noPassphraseDesc:
      "Auto-refund blijft werken na herstarts zonder dat er iets ingevoerd hoeft te worden — dit is slechts een hot transit-seed. Kosten: bestands-/host-toegang stelt de transit-sleutels + identiteit van deze merchant bloot.",
    passphraseLabel: "Wachtwoordzin",
    passphrasePlaceholder: "kies een wachtwoordzin",
    revealTitle: "Schrijf je herstelzin op",
    revealBody:
      "Iedereen met deze woorden beheert de hot keys van deze merchant. Satchel bewaart geen kopie — bewaar hem offline. Hierna bevestig je een paar woorden.",
    ackLabel: "Ik heb mijn herstelzin opgeschreven.",
    // Multi-step onboarding (create/import -> secret -> confirm -> passphrase).
    chooseTitle: "Stel {label} in",
    enterTitle: "Importeer je herstelzin",
    enterBody:
      "Typ elk woord — ze worden automatisch aangevuld terwijl je typt — of plak de hele zin. We controleren hem voordat je verdergaat.",
    wordCount: "{n} woorden",
    wordAria: "Woord {n}",
    checkIncomplete: "Voer alle {n} woorden in.",
    checkUnknown: "Sommige woorden staan niet in de BIP39-woordenlijst — controleer de gemarkeerde.",
    checkBadChecksum: "Checksum komt niet overeen — controleer je woorden en hun volgorde opnieuw.",
    checkOk: "Herstelzin ziet er geldig uit.",
    verifyTitle: "Bevestig je back-up",
    verifyBody: "Typ de woorden op deze posities om te bevestigen dat je de zin hebt opgeschreven.",
    verifyWord: "Woord #{n}",
    verifyMismatch: "Die komen niet overeen met je zin — controleer je back-up.",
    passphraseTitle: "Bescherm het seed",
    passphraseBody:
      "Versleutel optioneel het opgeslagen seed met een wachtwoordzin. Je kunt dit overslaan — zie de afweging hieronder.",
  },
  counterparty: {
    you: "Dit ben jij",
    youShort: "jij",
    unknown: "onbekende identiteit",
    // Short fingerprint fallback (identity.ts shortId) when no pubkey is known.
    unknownShort: "onbekend",
  },
  status: {
    notConnectedTitle: "Niet verbonden met de engine",
    disconnectedBody:
      "Satchel kan de engine niet bereiken. Hij is mogelijk nog aan het opstarten, of de node-verbindingen van de actieve merchant liggen eruit. Probeer opnieuw, of wissel van merchant via de selector bovenaan.",
    openInSatchel: "Open dit in Satchel",
    noTauriBody:
      "Dit is de UI van Satchel — hij heeft de Tauri-brug nodig om de engine te bereiken. Start de desktop-app (cargo tauri dev) in plaats van een browser.",
  },
  settings: {
    title: "Instellingen",
    subtitle: "App-brede voorkeuren voor deze installatie.",
    // UI-3 Settings tabs.
    tabGeneral: "Algemeen",
    tabCoins: "Munten",
    tabNetwork: "Netwerk",
    tabAbout: "Over",
    appearance: "Weergave",
    theme: "Thema",
    themeDark: "Donker",
    themeLight: "Licht",
    themeSystem: "Systeem",
    themeHint: "Kies hoe Satchel eruitziet. Systeem volgt je OS-instelling.",
    language: "Taal",
    languageHint: "Meer talen verschijnen naarmate vertalingen worden bijgedragen.",
    mode: "Modus",
    watchOnly: "Alleen-bekijken-modus",
    watchOnlyHint:
      "Bekijk het bord zonder munten in te stellen. Je kunt nog steeds je eigen aanbiedingen terugtrekken, maar niet plaatsen, aannemen of funden. Schakel uit om te handelen (je hebt minstens twee verbonden munten nodig).",
    network: "Netwerk",
    boards: "Corkboards",
    boardsDesc:
      "Optionele zelf-gehoste HTTP-borden. Voeg er toe die je vertrouwt; laat leeg om op Nostr te vertrouwen.",
    boardsNone: "Geen geconfigureerd",
    nostrRelays: "Nostr-relays",
    nostrRelaysDesc:
      "Relays dragen het prikbord over een gedecentraliseerd netwerk — geen operator kan je aanbiedingen lezen of matchen. Voorbedraad met een standaardset; bewerk vrij.",
    nostrRelaysOff: "Uit — Nostr-transport uitgeschakeld",
    addUrl: "Toevoegen",
    removeUrl: "Verwijderen",
    relayInvalid: "Voer een ws:// of wss:// relay-URL in",
    boardInvalid: "Voer een http:// of https:// bord-URL in",
    netSave: "Opslaan & opnieuw verbinden",
    netSaving: "Opslaan & opnieuw verbinden…",
    netSaved: "Opgeslagen",
    // Fees tab — fee-bump policy (per active merchant).
    tabFees: "Kosten",
    fees: "Fee-bumping",
    feesScope: "Deze instellingen gelden voor de actieve merchant.",
    feesIntro:
      "Veiligheids-/kostenafwegingen voor fee-bumps, geen verplichte instelling. Nieuwe waarden gelden voor toekomstige bumps; reeds gefunde swaps behouden het beleid waaronder ze zijn gefund.",
    feeMax: "Max feerate (sat/vB)",
    feeMaxHint:
      "Plafond voor elke fee-bump. Standaard 500, tevens het harde systeemmaximum. Verlaag het om kosten te beperken.",
    feeReservation: "Funding-bump-reservering (×)",
    feeReservationHint:
      "Saldo dat de fondscontrole apart zet als bump-speelruimte. Hoger redt grotere fee-pieken maar zet meer saldo vast en weigert meer swaps. Standaard 3.",
    feeCommitted: "Redeem-overprovisie (×)",
    feeCommittedHint:
      "Hoeveel extra de v2-redeem-fee vooruit wordt betaald zodat hij bevestigt zelfs wanneer Satchel gesloten is. Geldt alleen voor nieuwe swaps. Standaard 2.",
    feeSave: "Opslaan",
    feeSaving: "Opslaan…",
    feeSaved: "Opgeslagen",
    feeReset: "Terug naar standaardwaarden",
    coins: "Munten & nodes",
    coinsHint: "Verbind elke munt met je eigen node. Genesis wordt gecontroleerd voordat er iets wordt opgeslagen.",
    about: "Over",
    version: "Versie {version}",
    updateUpToDate: "Up-to-date",
    updateCheckPlaceholder: "Updatecontrole komt in een latere release.",
    trustModel: "Waar je sleutels staan",
    trustModelBody:
      "Geheimen staan in de engine, nooit in Satchel. Het merchant-seed staat in de datamap van de engine (versleuteld of platte tekst — jouw keuze); Satchel slaat geen seed of wachtwoordzin op. Het seed is bewust hot (alleen transit-sleutels) — veeg aanzienlijke opbrengsten naar je eigen cold wallet.",
  },
  coins: {
    intro:
      "Verbind elke munt met je eigen node. De eerste URL is de eigen wallet van je node — die fundt je swap-legs en ontvangt de opbrengsten. Voordat er iets wordt opgeslagen, controleert Satchel het genesis-block van de node zodat fondsen nooit naar de verkeerde keten kunnen worden gestuurd. Verbindingen worden gedeeld over al je merchants.",
    networkBadge: "Configureren voor het {network}-netwerk",
    needMerchant:
      "Verbind eerst een merchant — muntinstelling vereist een draaiende engine. Gebruik de merchant-selector rechtsboven.",
    pairsTitle: "Handelsparen",
    pairsHint:
      "Paren worden afgeleid van wat elke munt kan — er is geen vaste lijst. Een paar opent zodra beide munten ervan verbonden zijn.",
    noPairs: "Geen paren beschikbaar.",
    notSetUp: "Niet ingesteld",
    connectedTip: "Verbonden · tip {tip}",
    connError: "Verbindingsfout",
    setUp: "Instellen",
    editConnection: "Verbinding bewerken",
    remove: "verwijderen",
    disconnectTip: "Verbreek deze munt",
    disconnectTitle: "{coin} verbreken?",
    disconnectBody: "Swaps die hem nodig hebben zijn niet beschikbaar tot je opnieuw verbindt.",
    ready: "Klaar om te handelen",
    connectMissing: "Verbind {coins}",
    notBuildable: "Nog niet bouwbaar",
    // Swap-protocol chips on a pair (pact-htlc-v1 HTLC vs pact-htlc-v2 adaptor).
    protoPrivate: "Privé (Taproot)",
    protoPrivateTip: "Privé swap (Taproot/MuSig2 adaptor) — ziet er on-chain uit als een gewone betaling",
    protoHtlcTip: "Klassieke HTLC-swap",
    // Coin-setup backend choices (CoinSetup).
    // CoinSetup dialog.
    setupTitle: "Verbind {coin}",
    setupIntro:
      "Wijs Satchel naar je eigen {sym}-node. Er wordt niets opgeslagen tot de node een genesis-block-controle doorstaat — je fondsen raken alleen ooit de echte {sym}-keten.",
    confirmationsLabel: "Bevestigingen voor definitief",
    confirmationsHint:
      "Hoe diep een funding of redeem op deze keten moet zijn voordat een swap erop reageert — de reorg-veiligheidsmarge. Hoger is veiliger maar trager; laat leeg voor de aanbevolen standaard ({default}).",
    validateNode: "Node valideren",
    checking: "De node controleren…",
    genesisOk: "Genesis komt overeen — dit is de juiste keten",
    genesisDetail: "tip-hoogte {tip} · genesis {hash}…",
    genesisBad: "Geweigerd — wordt niet opgeslagen",
    errorShort: "fout",
    // Structured connection form (CoinSetup v2).
    rpcHostLabel: "RPC-host",
    rpcPortLabel: "RPC-poort",
    authMethodLabel: "Authenticatie",
    authCookie: "Cookie-bestand",
    authCookieDesc: "Lees automatisch het .cookie van de node uit zijn datamap (de standaard, geen wachtwoord opgeslagen).",
    authUserPass: "Gebruiker / wachtwoord",
    authUserPassDesc: "De rpcuser / rpcpassword uit de config van je node — nodig voor een externe node.",
    rpcUserLabel: "RPC-gebruikersnaam",
    rpcPasswordLabel: "RPC-wachtwoord",
    datadirLabel: "Node-datamap",
    cookiePathNote: "De cookie wordt gelezen uit {path} onder deze map.",
    walletLabel: "Wallet-naam (optioneel)",
    walletPlaceholder: "de wallet van je node",
    needPort: "Voer eerst de RPC-poort in.",
    validateFirst: "Valideer de node voordat je opslaat.",
    savingReconnecting: "Opslaan & opnieuw verbinden…",
    connected: "{coin} verbonden",
    // Template picker (a coins.toml coin the engine version doesn't support).
    unsupportedByEngine: "Niet ondersteund",
    unsupportedByEngineTip:
      "Deze munt is gedefinieerd in coins.toml maar niet ingebouwd in deze versie van de engine, dus hij kan niet verhandeld worden.",
  },
  coinWizard: {
    title: "Verbind je munten",
    intro:
      "Kies minstens twee munten en wijs elk naar je eigen node. Een swap heeft twee ketens nodig, dus handelen ontgrendelt zodra twee nodes verbonden en live zijn. Je kunt later munten toevoegen of wijzigen in Instellingen.",
    progress: "{count} van {min} munten verbonden",
    continue: "Doorgaan",
    live: "Live",
    nodeDown: "Node ligt eruit",
  },
  wallets: {
    intro:
      "Dit zijn de wallets van je eigen nodes (degene die de engine gebruikt om swaps te funden en opbrengsten te ontvangen) — jouw sleutels, jouw machine. Satchel houdt nooit je munten vast.",
    hotSeedNudge:
      "Dit is een uitgeef-wallet op een hot seed, geen kluis — veeg aanzienlijke saldi naar je eigen cold/core-wallet.",
    notConnected: "Niet verbonden",
    notConnectedBody: "Verbind eerst een merchant — de wallet-weergave vereist een draaiende engine.",
    noCoins: "Nog geen munten ingesteld",
    noCoinsBody: "Verbind een munt in Instellingen → Munten en zijn wallet verschijnt hier.",
    goToCoins: "Ga naar Munten",
    watchOnlyTitle: "Geen wallets in alleen-bekijken-modus",
    watchOnlyBody:
      "Dit is een alleen-bekijken-sessie zonder verbonden munten, dus er zijn geen wallets om te tonen. Schakel alleen-bekijken uit in Instellingen en verbind een munt om swaps te funden.",
    walletName: "wallet · {wallet}",
    walletScopedHint: "Elke RPC voor deze munt is afgebakend tot deze node-wallet.",
    walletDefault: "standaard wallet (niet afgebakend)",
    walletDefaultHint:
      "Geen wallet ingesteld voor deze munt, dus RPC's gebruiken de standaard wallet van de node. Stel er een in bij Instellingen → Munten om elke aanroep af te bakenen tot een specifieke wallet.",
    balanceLabel: "{symbol}-saldo",
  },
  corkboard: {
    noBoardTitle: "Geen Corkboard verbonden",
    noBoardBody:
      "Een Corkboard is een gedeeld prikbord waar makers aanbiedingen vastpinnen. Het matcht nooit handels of houdt munten vast — wijs Satchel naar een die je vertrouwt om te bladeren en te plaatsen.",
    noPairs: "Geen paren beschikbaar",
    board: "Corkboard",
    boardSettings: "Configureer in Instellingen",
    filterAll: "Alle",
    filterMine: "Mijne",
    noOffers: "Geen aanbiedingen die je nu kunt aannemen",
    noOffersBody:
      "Aanbiedingen verschijnen hier zodra een maker er een plaatst voor een paar dat je hebt ingesteld. Je kunt ook je eigen aanbod plaatsen.",
    hiddenOffers:
      "{count} aanbieding(en) meer voor paren die je niet hebt verbonden. Stel beide munten in om ze te verhandelen:",
    yourOffer: "jouw aanbod",
    offerStaged: "plaatsen…",
    offerStagedTip:
      "Geplaatst vanaf dit apparaat en wacht op bevestiging terug van een relay. Het adverteert; het wordt live zodra een relay het echoot.",
    take: "Aanbod aannemen",
    legDown: "Een van de nodes van dit paar ligt eruit — start hem (of controleer Instellingen → Munten) voordat je aanneemt.",
    withdraw: "Terugtrekken",
    withdrawTip: "Trek direct terug — een aanbod vergrendelt nooit fondsen",
    safetyRefund: "veiligheids-refund",
    safetyRefundTip:
      "Als de swap vastloopt, krijgen beide kanten een auto-refund — de leg van de taker ontgrendelt als eerste, die van jou iets later. Niemand blijft vastzitten.",
    activeTitle: "Je actieve swaps",
    states: {
      takenByUs: "door jou aangenomen",
      revoked: "teruggetrokken",
      expired: "verlopen",
    },
    // Two-sided order-book view of the Corkboard. Bids = makers giving the
    // quote coin to get the base; asks = the reverse. The ladder is a way to
    // READ the board — it never matches or prioritises (load-bearing).
    book: {
      bids: "Bids",
      asks: "Asks",
      bidsHint: "wil {base} · betaalt {quote}",
      asksHint: "verkoopt {base} · voor {quote}",
      price: "Prijs",
      size: "Grootte",
      noBids: "Geen bids",
      noAsks: "Geen asks",
      spread: "Spread {pct}",
      spreadOneSided: "Eenzijdig",
      crossed: "gekruist",
      crossedTip: "Top-bid ≥ top-ask. Het bord matcht nooit automatisch, dus deze overlappende aanbiedingen blijven gewoon staan — neem een van beide kanten aan.",
      mid: "mid {price}",
      levelOffers: "{count} aanbieding(en) op deze prijs — kies er een om aan te nemen",
      depthTip: "Totaal {sym} aangeboden op deze prijs over {count} bericht(en).",
      selectLevel: "Kies hierboven een prijsniveau om de aanbiedingen daar te zien.",
      paneHeader: "{size} {base} @ {price} {unit}",
      denomTip: "Weergave-eenheid voor {coin}-bedragen",
      showMore: "Toon {count} meer",
      showLess: "Toon top {count}",
    },
  },
  relays: {
    title: "Relays",
    subtitle: "Live connectiviteit met je Nostr-relays — het netwerk waarover je aanbiedingen en aannames reizen. Voeg relays toe of verwijder ze in Instellingen → Netwerk.",
    connectedCount: "{up} / {total} verbonden",
    refresh: "Vernieuwen",
    ms: "{ms} ms",
    up: "up",
    down: "down",
    statsTip: "{success}/{attempts} geslaagde verbindingen · ↓{down} ↑{up}",
    none: "Geen relays geconfigureerd",
    noneBody: "Voeg een Nostr-relay toe in Instellingen → Netwerk om aanbiedingen over het netwerk te publiceren en te ontvangen.",
    goToNetwork: "Ga naar Instellingen",
    notConnected: "Niet verbonden",
    notConnectedBody: "De relay-weergave vereist een draaiende engine — verbind eerst een merchant.",
  },
  swaps: {
    title: "Swaps",
    hint: "Je volledige grootboek — lopende swaps bovenaan, afgeronde handels eronder. Je kunt ook live swaps bedienen vanaf de Corkboard.",
    activeTitle: "Onderweg",
    historyTitle: "Geschiedenis",
    none: "Nog geen swaps — neem een aanbod aan op de Corkboard.",
    // Active-swaps dock action buttons + the refund-time label.
    cancel: "annuleren",
    refund: "refund",
    dump: "logs dumpen",
    dumpHint: "Kopieer een geheim-vrije diagnostiek-bundel (status + logregels) voor deze swap, om naar de ontwikkelaars te plakken.",
    dumpCopied: "Diagnostiek gekopieerd — plak naar de ontwikkelaars.",
    dumpFailed: "Kon de diagnostiek-bundel niet kopiëren.",
    refundAt: "refund {when}",
    // Confirm dialogs for acting on a live swap (ActiveSwaps).
    cancelTitle: "Deze swap annuleren?",
    cancelConfirm: "Swap annuleren",
    cancelKeep: "Behouden",
    // Abort reason recorded on the swap when cancelled from Satchel.
    cancelReason: "geannuleerd in Satchel",
    cancelBody:
      "Dit verlaat de swap voordat je hebt gefund. Niets van jou is nog vergrendeld, dus je verliest niets — het aanbod komt alleen niet tot stand.",
    refundTitle: "Je fondsen terughalen?",
    refundConfirm: "Refund",
    refundBody:
      "De veiligheids-timelock is verstreken, dus je kunt de fondsen die je vergrendelde terugvorderen. Dit verstuurt je refund nu; de engine doet het ook automatisch na de deadline.",
    col: {
      swap: "swap",
      role: "rol",
      state: "status",
      amounts: "geeft → ontvangt",
      when: "wanneer",
      finalTx: "definitieve tx",
    },
    // Expandable per-leg on-chain detail (the audit trail). We show both
    // funding txs + OUR settlement; never the counterparty's settlement or the
    // swap secret.
    audit: {
      toggle: "Toon on-chain detail",
      title: "On-chain detail",
      youLocked: "jij vergrendelde",
      theyLocked: "zij vergrendelden",
      funding: "Funding",
      received: "Ontvangen",
      refunded: "Gerefund",
      pending: "nog niet on-chain",
      copy: "Transactie-id kopiëren",
      copied: "Transactie-id gekopieerd",
    },
  },
  fees: {
    title: "Voorbeeld netwerkkosten",
    estimated: "geschat",
    provisionalNote: "Deze pactd-build stelt nog geen kostenschatting beschikbaar.",
    summary: "Een swap bestaat uit 2 on-chain transacties waarvoor je betaalt: funding op de geef-keten, redeem op de ontvang-keten.",
    fallbackTip: "Een node was onbereikbaar, dus is een conservatieve standaard-feerate gebruikt — beschouw deze als een schatting.",
    ifItStalls: "(als het vastloopt)",
  },
  funds: {
    insufficient:
      "Niet genoeg {sym} om deze swap te funden — nodig ~{need} {sym} (bedrag + funding-fee), wallet heeft {have} {sym}.",
  },
  wizard: {
    back: "Terug",
    continue: "Doorgaan",
  },
  // UI-4 docked activity log.
  log: {
    title: "Activiteit",
    empty: "— activiteitenlog —",
    count: "{count} regels",
    collapse: "Log inklappen",
    expand: "Log uitklappen",
    // Activity-log lines emitted by the frontend. `{err}` carries a raw engine
    // message (itself not translated — it comes from pactd over the wire).
    noTauri: "draait niet binnen Satchel — deze UI heeft de Tauri-brug nodig",
    startupError: "opstarten: {err}",
    notConnected: "niet verbonden: {err}",
    connected: "verbonden met pactd {version} ({protocol})",
    listcoinsError: "listcoins: {err}",
    watchOnlyError: "alleen-bekijken: {err}",
    switchedMerchant: "overgeschakeld naar merchant {id}",
    switchMerchantError: "merchant wisselen: {err}",
    loadMerchantError: "merchant laden: {err}",
    merchantCreated: "merchant {id} aangemaakt",
    merchantReady: "merchant gereed",
    actionOk: "{action} {id}: ok",
    actionError: "{action} {id}: {err}",
    diagCopied: "diagnostiek voor {id} gekopieerd ({count} logregels) — plak naar de devs",
    dumpError: "dump {id}: {err}",
    coinDisconnected: "{coin} verbroken",
    removeCoinError: "munt verwijderen: {err}",
    tookOffer: "aanbod {id} aangenomen — het verschijnt nu hieronder in je actieve swaps",
    takeError: "aannemen: {err}",
    offerWithdrawn: "aanbod {id} teruggetrokken",
    withdrawError: "terugtrekken: {err}",
    postedOffer: "aanbod {id} geplaatst — trek het op elk moment terug; er wordt niets vergrendeld",
    createdSlip: "een privé aanbod-slip aangemaakt — stuur het naar je vriend",
    tookPrivateOffer: "privé aanbod {id} aangenomen — het verschijnt nu in je actieve swaps",
    cancelledPrivateOffer: "privé aanbod {id} geannuleerd",
    cancelError: "annuleren: {err}",
    noticeboardUpdated: "prikbord bijgewerkt",
    feePolicyUpdated: "kostenbeleid bijgewerkt",
  },
  // Relative-time + freshness prose from format.ts (rendered via the tr() mirror,
  // since those are pure non-component helpers). Unit letters stay inside the
  // template so a translation owns the whole phrase.
  format: {
    ageUnknown: "leeftijd onbekend",
    justNow: "zojuist",
    minutesAgo: "{n}m geleden",
    hoursAgo: "{n}u geleden",
    daysAgo: "{n}d geleden",
    expiryNow: "nu",
    expirySoon: "binnenkort",
    inMinutes: "over ~{n}m",
    inHours: "over ~{n}u",
    inDays: "over ~{n}d",
    posted: "geplaatst {age}",
    expires: "verloopt {time}",
  },
  // Plain-language swap story per (role, state) — the honest "who is exposed
  // when" framing shown on every active swap. {a}/{b} are coin tickers; {t1}/{t2}
  // are local refund times. Rendered via tr() (narrate() is a pure helper).
  narrate: {
    finalizing: "Je hebt je {got} geclaimd — laatste bevestigingen. Houd de app open tot het begraven is; je {gave} blijven tot dan beschermd.",
    initiating:
      "Aanname verstuurd — wachten tot de maker de swap start. Er is nog niets vergrendeld; het annuleert vanzelf als zij niet reageren.",
    created: "Aanbod verstuurd — wachten tot de andere kant akkoord gaat. Er is niets vastgelegd.",
    acceptedMaker: "Voorwaarden overeengekomen. Volgende: vergrendel je {a}. Tot je fundt, kun je nog vrij annuleren.",
    acceptedTaker: "Voorwaarden overeengekomen. De andere kant vergrendelt zijn {a} eerst — jij stuurt nooit als eerste.",
    noncesExchanged:
      "De privé swap opzetten — ondertekeningsmateriaal uitwisselen. Er is nog niets vergrendeld.",
    signedMaker:
      "Beide kanten hebben ondertekend. Je daemon vergrendelt de {a} en claimt vervolgens automatisch de {b}. Als er iets vastloopt, komt je {a} terug om {t1}.",
    signedTaker:
      "Beide kanten hebben ondertekend. Je daemon vergrendelt de {b} en claimt de {a} zodra de andere kant beweegt. Vangnet: refund om {t2}.",
    fundedAMaker:
      "Je {a} is vergrendeld. Wachten tot de andere kant zijn {b} vergrendelt. Als zij dat nooit doen, komt je {a} automatisch terug om {t1}.",
    fundedATaker:
      "Hun {a} is vergrendeld en geverifieerd. Volgende: vergrendel je {b}. Vangnet: automatische refund om {t2} als er iets vastloopt.",
    fundedBMaker: "Beide vergrendeld. Je daemon claimt de {b} zodra die veilig bevestigd is.",
    fundedBTaker: "Beide vergrendeld. Je daemon zal de {a} claimen zodra de andere kant zijn {b} aanneemt.",
    completed: "Swap voltooid — de {coin} staat in je wallet.",
    refunded: "De swap is niet voltooid, dus je {coin} kwam automatisch terug. Niets verloren behalve fees.",
    aborted: "Geannuleerd voordat er geld bewoog.",
  },
  progress: {
    awaitingLock: "Wachten op hun vergrendeling",
    awaitingClaim: "Wachten op hun claim",
    theirLock: "Hun vergrendeling wordt bevestigd",
    securing: "Je {coin} beveiligen",
    blocks: "+{n} blokken",
    feeBumped: "Vergoeding verhoogd",
    reorg: "Reorg gedetecteerd — opnieuw controleren",
  },
  exit: {
    // Exit-gate dialog (fund safety, C6). The engine manages alone, so "keep
    // running" detaches it (it keeps watching timelocks + servicing offers).
    liveTitle: "Een swap is onderweg",
    liveBodyOne:
      "1 swap is halverwege. Hij wordt beheerd door on-chain timelocks — de engine moet blijven draaien om te redeemen of refunden voor de deadline.",
    liveBodyMany:
      "{count} swaps zijn halverwege. Ze worden beheerd door on-chain timelocks — de engine moet blijven draaien om te redeemen of refunden voor de deadline.",
    keepRunningExplain:
      "Het venster sluiten houdt de engine op de achtergrond draaiend, zodat hij de swap headless afrondt. Je kunt Satchel op elk moment heropenen om te controleren.",
    forceQuitWarn: "Nu geforceerd afsluiten stopt de engine en kan fondsen verliezen.",
    // {word} is the confirm word below; a translation may localize both together.
    typeToConfirm: "Om toch geforceerd af te sluiten, typ {word} hieronder.",
    confirmWord: "QUIT",
    keepRunning: "Laat draaien, sluit venster",
    keepWithdraw: "Laat draaien + trek aanbiedingen terug",
    keepLeaveOffers: "Laat draaien, laat aanbiedingen staan",
    forceQuit: "Geforceerd afsluiten",
    offersTitle: "Je hebt aanbiedingen geplaatst",
    offersBodyOne:
      "1 aanbod van jou staat nog op de Corkboard. Aanbiedingen vergrendelen niets, maar het laten staan betekent dat tegenpartijen het nog kunnen aannemen terwijl Satchel gesloten is — de engine bedient de aanname.",
    offersBodyMany:
      "{count} aanbiedingen van jou staan nog op de Corkboard. Aanbiedingen vergrendelen niets, maar ze laten staan betekent dat tegenpartijen ze nog kunnen aannemen terwijl Satchel gesloten is — de engine bedient de aannames.",
    withdrawExit: "Trek alles terug & sluit af",
  },
  unlock: {
    title: "Merchant ontgrendelen",
    body:
      "Het seed van deze merchant is versleuteld. Voer de wachtwoordzin in om het voor deze sessie te ontgrendelen — Satchel houdt het alleen in het geheugen en vergeet het bij afsluiten.",
    switchMerchant: "Merchant wisselen",
    unlock: "Ontgrendelen",
  },
  common: {
    cancel: "Annuleren",
    confirm: "Bevestigen",
    save: "Opslaan",
    done: "Klaar",
    later: "Later",
    retry: "Verbinding opnieuw proberen",
  },
};
