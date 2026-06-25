// The Slovak (Slovenčina) string bundle. Mirrors en.ts key-for-key; only values are translated.
import type { Bundle } from "./en";

export const sk: Bundle = {
  app: {
    name: "Satchel",
    tagline: "swapy bez dôvery v tretiu stranu",
  },
  // In-app update notifications (sidebar version badge + dialog).
  update: {
    title: "Dostupná aktualizácia",
    upToDate: "Máte najnovšiu verziu",
    current: "Nainštalované",
    latest: "Najnovšie",
    notesTitle: "Poznámky k vydaniu",
    get: "Získať aktualizáciu",
    dismiss: "Zavrieť",
    close: "Zavrieť",
    badgeTooltip: "Dostupná aktualizácia — kliknite pre podrobnosti",
    versionTooltip: "Kliknutím skontrolujete aktualizácie",
  },
  // Risk disclaimer (first-run welcome + Settings → About).
  disclaimer: {
    title: "Vlastná úschova — vaše kľúče, vaša zodpovednosť",
    body: "Satchel vykonáva nekustodiálne atomické swapy: kľúče držíte iba vy a seed obchodníka drží horúce tranzitné kľúče počas prebiehajúceho swapu. Swapové protokoly (v1 HTLC a v2 Taproot/MuSig2) sú zrevidované a v prevádzke na mainnete. Pod licenciou MIT a poskytované tak, ako sú, bez akejkoľvek záruky — zálohujte si obnovovaciu frázu a používajte na vlastné riziko.",
  },
  nav: {
    public: "Verejné",
    corkboard: "Corkboard",
    postOffer: "Zverejniť ponuku",
    private: "Súkromné",
    privateCreate: "Vytvoriť lístok",
    privateReceive: "Prijať lístok",
    privateSlips: "Moje lístky",
    swaps: "Swapy",
    relays: "Relays",
    wallets: "Peňaženky",
    settings: "Nastavenia",
    coins: "Mince",
  },
  makeOffer: {
    title: "Zverejniť ponuku",
    intro:
      "Zverejnite podpísanú ponuku na Corkboard. Nič sa nezamyká — je to len inzerát; kedykoľvek ho stiahnite a swap sa začne až vtedy, keď ho niekto prijme a obe strany ho podfinancujú.",
    give: "Vy dávate",
    want: "Vy dostávate",
    // Canonical pair + direction: pick the pair, choose Sell/Buy the base, enter
    // the base amount and a quote-per-base price (invariant to direction).
    pair: "Pár",
    noPairs: "Žiadne obchodovateľné páry — pripojte aspoň dve mince v Nastavenia → Mince.",
    sell: "Predať {sym}",
    buy: "Kúpiť {sym}",
    amount: "Množstvo",
    youGive: "Vy dávate",
    youGet: "Vy dostávate",
    price: "Cena",
    priceUnit: "{unit} za {base}",
    pricePlaceholder: "jednotková cena",
    balance: "Zostatok: {amt} {sym}",
    balanceLoading: "Zostatok: …",
    noCoins: "Žiadne nakonfigurované mince",
    sameCoin: "Mince, ktoré dávate a dostávate, musia byť rozdielne.",
    legDown: "Uzol jednej z týchto mincí je mimo prevádzky — spustite ho (alebo skontrolujte Nastavenia → Mince) pred zverejnením.",
    // Swap-protocol pin (only offered when a pair+network supports more than
    // one). v2 label reuses coins.protoPrivate ("Private (Taproot)").
    protocol: "Typ swapu",
    protoStandard: "Štandardný (HTLC)",
    // Titles for the review/confirm dialog shown before posting (see OfferForm).
    reviewOfferTitle: "Skontrolujte svoju ponuku",
    reviewSlipTitle: "Skontrolujte svoj lístok",
    term: "Bezpečnostný timelock",
    termShort: "Krátky",
    termMedium: "Stredný",
    termLong: "Dlhý",
    termHint: {
      short: "Krátky — pri zaseknutí obchodu sa prostriedky vrátia automaticky najrýchlejšie (~12 h / 6 h), s najmenšou bezpečnostnou rezervou.",
      medium: "Stredný — vyvážené okno na vrátenie (~24 h / 12 h).",
      long: "Dlhý (najbezpečnejší) — najširšia bezpečnostná rezerva; automatické vrátenie po ~36 h / 18 h, ak sa obchod zasekne.",
    },
    validFor: "Platí (minúty)",
    validForMins: "{mins} min",
    validForHint:
      "Ako dlho ponuka zostane vypísaná. Kým ste online, udržiava sa automaticky aktuálna; potom vyprší. Zatvorením aplikácie ju stiahnete.",
    note: "Ponuka s pevnou veľkosťou — nič sa nezamkne, kým ju niekto neprijme. Sumy sú on-chain; navyše platíte sieťové poplatky a Corkboard si neúčtuje nič. Timelock je okno na automatické vrátenie, ak sa swap zasekne.",
    post: "Zverejniť ponuku",
    makeSlip: "Vytvoriť lístok",
    slipTitle: "Váš súkromný ponukový lístok",
    slipExplainer:
      "Pošlite ho svojmu priateľovi. Vloží ho do Satchel, aby ho prijal. Nič sa nezamyká; vyprší za {ttl}.",
    copy: "Kopírovať",
    copied: "Skopírované",
    makeAnother: "Vytvoriť ďalší",
    myPrivateTitle: "Moje súkromné ponuky",
    myPrivateEmpty: "Žiadne nevybavené súkromné ponuky.",
    privateExpires: "vyprší {when}",
    privateExpired: "vypršalo",
    cancel: "Zrušiť",
    cancelTip: "Prestať ctiť tento lístok — priateľ, ktorý ho stále má, ho už nemôže prijať.",
  },
  takeSlip: {
    open: "Vložiť lístok",
    title: "Prijať súkromnú ponuku",
    intro:
      "Priateľ vám poslal súkromný ponukový lístok (začína na pactoffer1:). Vložte ho sem, aby ste si ho prezreli a prijali — presne ako ponuku z nástenky.",
    placeholder: "pactoffer1:…",
    take: "Skontrolovať a prijať",
    invalid: "Toto nevyzerá ako lístok — mal by začínať na pactoffer1:.",
    previewLabel: "Tento lístok ponúka",
  },
  // PRIVATE nav group (off-market slips) — screen titles + intros. The form,
  // slip output, my-slips list and take flow reuse makeOffer.*/takeSlip.* copy.
  private: {
    createTitle: "Vytvoriť súkromnú ponuku",
    createIntro:
      "Vytvorte podpísanú ponuku a odovzdajte ju priateľovi ako lístok cez váš vlastný chat. Nikde sa nevypisuje — a nič sa nezamkne, kým obaja nepodfinancujete.",
    slipsIntro:
      "Lístky, ktoré ste vytvorili. Ktokoľvek, kto má lístok, ho môže prijať, kým nevyprší; jeden z nich zrušte, ak ho chcete prestať ctiť skôr.",
    slipsEmptyBody: "Vytvorte súkromnú ponuku, aby ste získali lístok, ktorý môžete poslať priateľovi.",
    receiveTitle: "Prijať súkromnú ponuku",
    received: "Prijaté — sledujte v Swapoch.",
  },
  // Shared take-confirmation dialog (board take + slip take).
  takeConfirm: {
    title: "Prijať túto ponuku?",
    confirm: "Prijať ponuku",
    counterparty: "Protistrana",
    youGive: "Vy dávate",
    youReceive: "Vy dostávate",
    safetyRefund: "Bezpečnostné vrátenie",
    offerAge: "Vek ponuky",
    makerFundsFirst:
      "Maker zamkne svoje {sym} ako prvý — vy nikdy neposielate ako prví. Stále môžete zrušiť, kým nepodfinancujete svoju stranu, a engine automaticky vráti prostriedky po bezpečnostnom timelocku, ak sa swap zasekne.",
  },
  header: {
    activeMerchant: "Aktívny obchodník — kliknutím prepnete alebo spravujete",
    manageMerchants: "Spravovať obchodníkov…",
    noMerchant: "žiadny obchodník",
    openMenu: "Otvoriť menu",
    collapseMenu: "zbaliť menu",
    settings: "Nastavenia",
    language: "Jazyk",
    pactConnected: "Engine pripojený",
    pactUnreachable: "Engine nedostupný",
    liveSwapsOne: "1 prebiehajúci swap — kliknutím zobrazíte",
    liveSwapsMany: "{count} prebiehajúcich swapov — kliknutím zobrazíte",
    liveSwapsNone: "Žiadne prebiehajúce swapy",
    coinOk: "{name} — pripojené · vrchol {tip}",
    coinUnconfigured: "{name} — nenastavené",
    coinError: "{name} — {status}",
    relaysOk: "Nostr relays — {up}/{total} pripojených",
    relaysDown: "Nostr relays — žiadny z {total} nepripojený",
  },
  network: {
    mainnet: "MainNet",
    testnet: "TestNet",
    regtest: "RegTest",
    signet: "Signet",
    notRealFunds: "Nie sú to skutočné prostriedky — toto je sieť {network}",
  },
  // Watch-only mode: a viewer session with no coins. Browse the board and
  // withdraw your own offers, but no posting / taking / funding.
  watchOnly: {
    badge: "Iba sledovanie",
    badgeTip:
      "Režim iba na sledovanie — prehliadajte nástenku a stiahnite vlastné ponuky, ale nemôžete zverejňovať, prijímať ani podfinancovať. Nastavte mince v Nastaveniach, aby ste mohli obchodovať.",
    coinWizardButton: "Prehliadať v režime iba na sledovanie",
    coinWizardHint:
      "Preskočte nastavenie mincí a len prehliadajte nástenku (iba na čítanie). Stále môžete stiahnuť vlastné ponuky — užitočné na stiahnutie ponúk ponechaných inou reláciou. Kedykoľvek to vypnite v Nastaveniach.",
    postBlockedTitle: "Režim iba na sledovanie",
    postBlockedBody:
      "Toto je relácia iba na sledovanie, takže nemôže zverejňovať ponuky. Nastavte aspoň dve mince v Nastavenia → Mince, aby ste mohli obchodovať.",
    takeBlockedBody: "Režim iba na sledovanie — túto ponuku si môžete prezrieť, ale na jej prijatie je potrebné nastaviť mince.",
    takeBlockedTip: "Režim iba na sledovanie — nastavte mince v Nastaveniach, aby ste mohli prijímať ponuky.",
  },
  merchants: {
    title: "Vaši obchodníci",
    intro:
      "Obchodník je jedna obchodná identita — má vlastný seed a históriu swapov. Obchodovanie pod iným obchodníkom udržiava kontexty neprepojiteľné (jednorazová identita). Vaše hlavné mince žijú vo vašej vlastnej peňaženke, nie tu.",
    // First-run welcome (empty merchant manager).
    welcomeTitle: "Vitajte v Satchel",
    welcomeIntro:
      "Satchel obchoduje pod „obchodníkom“ — jednou obchodnou identitou s vlastným seedom. Zatiaľ žiadneho nemáte: vytvorte si nového alebo importujte existujúcu obnovovaciu frázu a začnite.",
    importMerchant: "Importovať obchodníka",
    none: "Zatiaľ žiadni obchodníci.",
    active: "aktívny",
    switch: "prepnúť",
    newMerchant: "Nový obchodník",
    thisMerchant: "tento obchodník",
    nameLabel: "Meno obchodníka",
    namePlaceholder: "napr. Hlavný",
    introFirst:
      "Nastavte si svoju prvú obchodnú identitu („obchodníka“). Drží iba horúce tranzitné kľúče pre prebiehajúce swapy — vaše hlavné mince zostávajú vo vašej vlastnej peňaženke.",
    introNew: "Nový obchodník je čerstvá, samostatná identita s vlastným seedom a históriou swapov.",
    // UI-5 merchant selector (phoenix wallet-selector parity).
    createNew: "Vytvoriť nového",
    import: "Importovať",
    load: "Načítať obchodníka",
    loaded: "načítaný",
    locked: "uzamknutý",
    lockedTip: "Šifrovaný seed — odomknite ho prístupovou frázou pri jeho načítaní.",
    close: "Zavrieť",
    idLabel: "priečinok",
    switching: "Prepínanie obchodníka…",
    switchingBody: "Opätovné spustenie enginu voči tomuto priečinku.",
  },
  // Seed create/import (SeedForm) + counterparty + status.
  seed: {
    intro: "Vytvorte úplne nový seed alebo importujte ten, ktorý už máte.",
    createNew: "Vytvoriť nový",
    createDesc: "Vygenerujte čerstvý seed. Obnovovaciu frázu si zálohujete sami.",
    import: "Importovať",
    importDesc: "Obnovte z existujúcej 12/24-slovnej frázy.",
    recoveryLabel: "Obnovovacia fráza",
    importPlaceholder: "slovo1 slovo2 slovo3 …",
    encrypt: "Šifrovať",
    encryptDesc:
      "Prístupová fráza chráni seed v pokoji. Zadávate ju raz za reláciu — Satchel ju nikdy neukladá. Poznámka: bezobslužné automatické vrátenie sa po reštarte pozastaví, kým ju znova nezadáte.",
    noPassphrase: "Bez prístupovej frázy (odporúčané)",
    noPassphraseDesc:
      "Automatické vrátenie funguje aj cez reštarty bez čohokoľvek na zadávanie — toto je len horúci tranzitný seed. Cena: prístup k súboru/hostiteľovi odhalí tranzitné kľúče a identitu tohto obchodníka.",
    passphraseLabel: "Prístupová fráza",
    passphrasePlaceholder: "zvoľte prístupovú frázu",
    createTitle: "Vytvoriť seed",
    importTitle: "Importovať seed",
    secureTitle: "Zabezpečiť {label}",
    revealTitle: "Zapíšte si svoju obnovovaciu frázu",
    revealBody:
      "Ktokoľvek s týmito slovami ovláda horúce kľúče tohto obchodníka. Satchel si neuchováva žiadnu kópiu — uložte ju offline. Ďalej potvrdíte niekoľko slov.",
    ackLabel: "Zapísal som si svoju obnovovaciu frázu.",
    // Multi-step onboarding (create/import -> secret -> confirm -> passphrase).
    chooseTitle: "Nastaviť {label}",
    enterTitle: "Importujte svoju obnovovaciu frázu",
    enterBody:
      "Zadajte každé slovo — počas písania sa automaticky dopĺňajú — alebo vložte celú frázu. Pred pokračovaním ju overíme.",
    wordCount: "{n} slov",
    wordAria: "Slovo {n}",
    checkIncomplete: "Zadajte všetkých {n} slov.",
    checkUnknown: "Niektoré slová nie sú v zozname slov BIP39 — skontrolujte zvýraznené.",
    checkBadChecksum: "Kontrolný súčet sa nezhoduje — znova skontrolujte slová a ich poradie.",
    checkOk: "Obnovovacia fráza vyzerá platne.",
    verifyTitle: "Potvrďte svoju zálohu",
    verifyBody: "Zadajte slová na týchto pozíciách, aby ste potvrdili, že ste si frázu zapísali.",
    verifyWord: "Slovo #{n}",
    verifyMismatch: "Tieto sa nezhodujú s vašou frázou — skontrolujte svoju zálohu.",
    passphraseTitle: "Chráňte seed",
    passphraseBody:
      "Voliteľne zašifrujte uložený seed prístupovou frázou. Toto môžete preskočiť — pozrite kompromis nižšie.",
  },
  counterparty: {
    you: "Toto ste vy",
    youShort: "vy",
    unknown: "neznáma identita",
    // Short fingerprint fallback (identity.ts shortId) when no pubkey is known.
    unknownShort: "neznáme",
  },
  status: {
    notConnectedTitle: "Nepripojené k enginu",
    disconnectedBody:
      "Satchel nedokáže dosiahnuť engine. Možno sa stále spúšťa, alebo sú pripojenia uzlov aktívneho obchodníka mimo prevádzky. Skúste znova alebo prepnite obchodníka cez výberník hore.",
    openInSatchel: "Otvoriť toto v Satchel",
    noTauriBody:
      "Toto je používateľské rozhranie Satchel — na dosiahnutie enginu potrebuje most Tauri. Spustite desktopovú aplikáciu (cargo tauri dev) namiesto prehliadača.",
  },
  settings: {
    title: "Nastavenia",
    subtitle: "Preferencie platné pre celú aplikáciu na tejto inštalácii.",
    // UI-3 Settings tabs.
    tabGeneral: "Všeobecné",
    tabCoins: "Mince",
    tabNetwork: "Sieť",
    tabAbout: "O aplikácii",
    appearance: "Vzhľad",
    theme: "Téma",
    themeDark: "Tmavá",
    themeLight: "Svetlá",
    themeSystem: "Systémová",
    themeHint: "Vyberte, ako Satchel vyzerá. Systémová sa riadi nastavením vášho OS.",
    language: "Jazyk",
    languageHint: "Ďalšie jazyky pribudnú, keď budú prispené preklady.",
    mode: "Režim",
    watchOnly: "Režim iba na sledovanie",
    watchOnlyHint:
      "Prehliadajte nástenku bez nastavenia mincí. Stále môžete stiahnuť vlastné ponuky, ale nemôžete zverejňovať, prijímať ani podfinancovať. Vypnite, aby ste mohli obchodovať (budete potrebovať aspoň dve pripojené mince).",
    network: "Sieť",
    boards: "Corkboards",
    boardsDesc:
      "Voliteľné samostatne hostované HTTP nástenky. Pridajte ktorékoľvek, ktorým dôverujete; nechajte prázdne, aby ste sa spoliehali na Nostr.",
    boardsNone: "Žiadne nakonfigurované",
    nostrRelays: "Nostr relays",
    nostrRelaysDesc:
      "Relays prenášajú nástenku cez decentralizovanú sieť — žiadny prevádzkovateľ nedokáže čítať ani párovať vaše ponuky. Predpripravené so štandardnou sadou; upravujte voľne.",
    nostrRelaysOff: "Vypnuté — transport Nostr zakázaný",
    addUrl: "Pridať",
    removeUrl: "Odstrániť",
    relayInvalid: "Zadajte URL relay typu ws:// alebo wss://",
    boardInvalid: "Zadajte URL nástenky typu http:// alebo https://",
    netSave: "Uložiť a znova pripojiť",
    netSaving: "Ukladanie a opätovné pripájanie…",
    netSaved: "Uložené",
    // Fees tab — fee-bump policy (per active merchant).
    tabFees: "Poplatky",
    fees: "Navyšovanie poplatkov",
    feesScope: "Tieto nastavenia sa vzťahujú na aktívneho obchodníka.",
    feesIntro:
      "Kompromisy bezpečnosť/náklady pre navýšenia poplatkov, nie je to nutné nastavenie. Nové hodnoty sa použijú na budúce navýšenia; už podfinancované swapy si ponechajú politiku, za ktorej boli podfinancované.",
    feeMax: "Max. sadzba poplatku (sat/vB)",
    feeMaxHint:
      "Strop pre každé navýšenie poplatku. Predvolene 500, zároveň aj tvrdé systémové maximum. Znížte, aby ste obmedzili náklady.",
    feeReservation: "Rezervácia na navýšenie podfinancovania (×)",
    feeReservationHint:
      "Zostatok, ktorý si kontrola prostriedkov odloží ako rezervu na navýšenie. Vyššia zachráni väčšie skoky poplatkov, ale viaže viac zostatku a odmietne viac swapov. Predvolene 3.",
    feeCommitted: "Predzásobenie redeemu (×)",
    feeCommittedHint:
      "O koľko sa vopred zaplatí navyše poplatok za v2 redeem, aby sa potvrdil, aj keď je Satchel zatvorený. Platí len pre nové swapy. Predvolene 2.",
    feeStep: "Krok eskalácie RBF (%)",
    feeStepHint: "Ako agresívne stúpa poplatok zaseknutej platby pri každom prechode plánovača. Predvolene 50.",
    feeSave: "Uložiť",
    feeSaving: "Ukladanie…",
    feeSaved: "Uložené",
    feeReset: "Obnoviť na predvolené",
    coins: "Mince a uzly",
    coinsHint: "Pripojte každú mincu k svojmu vlastnému uzlu. Genesis sa skontroluje skôr, než sa čokoľvek uloží.",
    about: "O aplikácii",
    version: "Verzia {version}",
    updateUpToDate: "Aktuálne",
    updateCheckPlaceholder: "Kontrola aktualizácií príde v neskoršom vydaní.",
    trustModel: "Kde žijú vaše kľúče",
    trustModelBody:
      "Tajomstvá žijú v enginu, nikdy v Satchel. Seed obchodníka sedí v dátovom priečinku enginu (šifrovaný alebo v čistom texte — podľa vašej voľby); Satchel neukladá žiadny seed ani prístupovú frázu. Seed je horúci zámerne (iba tranzitné kľúče) — väčší výnos preposielajte do vlastnej studenej peňaženky.",
  },
  coins: {
    intro:
      "Pripojte každú mincu k svojmu vlastnému uzlu. Prvá URL je vlastná peňaženka vášho uzla — financuje vaše swapové strany a prijíma výnos. Skôr než sa čokoľvek uloží, Satchel skontroluje genesis blok uzla, aby sa prostriedky nikdy nemohli poslať do nesprávneho reťazca. Pripojenia sa zdieľajú medzi všetkými vašimi obchodníkmi.",
    networkBadge: "Konfigurácia pre sieť {network}",
    needMerchant:
      "Najprv pripojte obchodníka — nastavenie mincí vyžaduje bežiaci engine. Použite výberník obchodníka vpravo hore.",
    pairsTitle: "Obchodné páry",
    pairsHint:
      "Páry sú odvodené z toho, čo každá minca dokáže — neexistuje pevný zoznam. Pár sa otvorí, keď sú pripojené obe jeho mince.",
    noPairs: "Žiadne dostupné páry.",
    notSetUp: "Nenastavené",
    connectedTip: "Pripojené · vrchol {tip}",
    connError: "Chyba pripojenia",
    setUp: "Nastaviť",
    editConnection: "Upraviť pripojenie",
    remove: "odstrániť",
    disconnectTip: "Odpojiť túto mincu",
    disconnectTitle: "Odpojiť {coin}?",
    disconnectBody: "Swapy, ktoré ju potrebujú, nebudú dostupné, kým sa znova nepripojíte.",
    ready: "Pripravené na obchodovanie",
    connectMissing: "Pripojiť {coins}",
    notBuildable: "Zatiaľ sa nedá zostaviť",
    // Swap-protocol chips on a pair (pact-htlc-v1 HTLC vs pact-htlc-v2 adaptor).
    protoPrivate: "Súkromné (Taproot)",
    protoPrivateTip: "Súkromný swap (Taproot/MuSig2 adaptor) — on-chain vyzerá ako bežná platba",
    protoHtlcTip: "Klasický HTLC swap",
    // Coin-setup backend choices (CoinSetup).
    backendCoreTitle: "RPC peňaženka jadra",
    backendCoreDesc: "Vlastná peňaženka vášho uzla financuje swap a prijíma výnos.",
    backendHardwareTitle: "Hardvér",
    backendHardwareDesc: "Podpisovanie Ledger / PSBT pre financovaciu stranu.",
    backendLater: "neskôr",
    // CoinSetup dialog.
    setupTitle: "Pripojiť {coin}",
    setupIntro:
      "Nasmerujte Satchel na svoj vlastný {sym} uzol. Nič sa neuloží, kým uzol neprejde kontrolou genesis bloku — vaše prostriedky sa kedykoľvek dotknú len skutočného reťazca {sym}.",
    backendUrlLabel: "URL backendu uzla",
    backendUrlHint:
      "Prvá URL = vlastná peňaženka vášho uzla (financuje swapy, prijíma výnos). Pridajte servery Electrum (tcp://host:port) za čiarkami pre ďalšie, nezávislé pohľady na reťazec.",
    fundingWallet: "Financovacia peňaženka",
    confirmationsLabel: "Potvrdení pred finalizáciou",
    confirmationsHint:
      "Ako hlboko musí byť financovanie alebo redeem na tomto reťazci, než na ňom swap zareaguje — bezpečnostná rezerva proti reorgu. Vyššia je bezpečnejšia, ale pomalšia; nechajte prázdne pre odporúčanú predvolenú hodnotu ({default}).",
    validateNode: "Overiť uzol",
    checking: "Kontrola uzla…",
    genesisOk: "Genesis sa zhoduje — toto je správny reťazec",
    genesisDetail: "výška vrcholu {tip} · genesis {hash}…",
    genesisBad: "Odmietnuté — neukladá sa",
    errorShort: "chyba",
    // Structured connection form (CoinSetup v2).
    rpcHostLabel: "RPC hostiteľ",
    rpcPortLabel: "RPC port",
    authMethodLabel: "Autentifikácia",
    authCookie: "Cookie súbor",
    authCookieDesc: "Automaticky načítať .cookie uzla z jeho dátového adresára (predvolené, žiadne heslo sa neukladá).",
    authUserPass: "Používateľ / heslo",
    authUserPassDesc: "rpcuser / rpcpassword z konfigurácie vášho uzla — potrebné pre vzdialený uzol.",
    rpcUserLabel: "RPC používateľské meno",
    rpcPasswordLabel: "RPC heslo",
    datadirLabel: "Dátový adresár uzla",
    cookiePathNote: "Cookie sa načíta z {path} pod týmto adresárom.",
    walletLabel: "Názov peňaženky (voliteľné)",
    walletPlaceholder: "peňaženka vášho uzla",
    needPort: "Najprv zadajte RPC port.",
    validateFirst: "Pred uložením overte uzol.",
    savingReconnecting: "Ukladanie a opätovné pripájanie…",
    connected: "{coin} pripojené",
    // Template picker (a coins.toml coin the engine version doesn't support).
    unsupportedByEngine: "Nepodporované",
    unsupportedByEngineTip:
      "Táto minca je definovaná v coins.toml, ale nie je zabudovaná do tejto verzie enginu, takže sa s ňou nedá obchodovať.",
  },
  coinWizard: {
    title: "Pripojte svoje mince",
    intro:
      "Vyberte aspoň dve mince a každú nasmerujte na svoj vlastný uzol. Swap potrebuje dva reťazce, takže obchodovanie sa odomkne, keď sú pripojené a aktívne dva uzly. Mince môžete pridať alebo zmeniť neskôr v Nastaveniach.",
    progress: "Pripojených {count} z {min} mincí",
    continue: "Pokračovať",
    live: "Aktívne",
    nodeDown: "Uzol mimo prevádzky",
  },
  wallets: {
    intro:
      "Toto sú peňaženky vašich vlastných uzlov (tých, ktoré engine používa na financovanie swapov a prijímanie výnosu) — vaše kľúče, váš stroj. Satchel nikdy nedrží vaše mince.",
    hotSeedNudge:
      "Toto je míňacia peňaženka na horúcom seede, nie trezor — väčšie zostatky preposielajte do vlastnej studenej/jadrovej peňaženky.",
    notConnected: "Nepripojené",
    notConnectedBody: "Najprv pripojte obchodníka — zobrazenie peňaženky vyžaduje bežiaci engine.",
    noCoins: "Zatiaľ žiadne nastavené mince",
    noCoinsBody: "Pripojte mincu v Nastavenia → Mince a jej peňaženka sa zobrazí tu.",
    goToCoins: "Prejsť na Mince",
    watchOnlyTitle: "V režime iba na sledovanie nie sú žiadne peňaženky",
    watchOnlyBody:
      "Toto je relácia iba na sledovanie bez pripojených mincí, takže nie sú žiadne peňaženky na zobrazenie. Vypnite režim iba na sledovanie v Nastaveniach a pripojte mincu, aby ste mohli financovať swapy.",
    walletName: "peňaženka · {wallet}",
    walletScopedHint: "Každé RPC pre túto mincu je obmedzené na túto peňaženku uzla.",
    walletDefault: "predvolená peňaženka (neobmedzená)",
    walletDefaultHint:
      "Pre túto mincu nie je nastavená žiadna peňaženka, takže RPC používajú predvolenú peňaženku uzla. Nastavte jednu v Nastavenia → Mince, aby každé volanie smerovalo na konkrétnu peňaženku.",
    balanceLabel: "Zostatok {symbol}",
    receive: "Prijať",
    send: "Odoslať",
    sendTo: "Odoslať na adresu",
    amount: "Množstvo",
    sendTitle: "Odoslať {amount} {sym}?",
    sendConfirmBody: "Na {to}\n\nToto sa minie z peňaženky vášho vlastného uzla a nedá sa vrátiť späť.",
  },
  corkboard: {
    noBoardTitle: "Žiadny pripojený Corkboard",
    noBoardBody:
      "Corkboard je zdieľaná nástenka, na ktorú makeri pripínajú ponuky. Nikdy nepáruje obchody ani nedrží mince — nasmerujte Satchel na ten, ktorému dôverujete, aby ste prehliadali a zverejňovali.",
    noPairs: "Žiadne dostupné páry",
    board: "Corkboard",
    boardSettings: "Konfigurovať v Nastaveniach",
    filterAll: "Všetky",
    filterMine: "Moje",
    offered: "{symbol} ponúknuté",
    noOffers: "Žiadne ponuky, ktoré môžete práve teraz prijať",
    noOffersBody:
      "Ponuky sa tu objavia hneď, ako maker zverejní niektorú pre pár, ktorý ste nastavili. Môžete tiež zverejniť svoju vlastnú.",
    hiddenOffers:
      "{count} ďalších ponúk pre páry, ktoré ste nepripojili. Nastavte obe mince, aby ste s nimi obchodovali:",
    yourOffer: "vaša ponuka",
    offerStaged: "zverejňuje sa…",
    offerStagedTip:
      "Zverejnené z tohto zariadenia a čaká na potvrdenie späť z relay. Je inzerované; aktívnym sa stane, keď ho relay zopakuje.",
    take: "Prijať ponuku",
    legDown: "Uzol jednej mince z tohto páru je mimo prevádzky — spustite ho (alebo skontrolujte Nastavenia → Mince) pred prijatím.",
    withdraw: "Stiahnuť",
    withdrawTip: "Stiahnite okamžite — ponuka nikdy nezamyká prostriedky",
    safetyRefund: "bezpečnostné vrátenie",
    safetyRefundTip:
      "Ak sa swap zasekne, obe strany dostanú prostriedky automaticky späť — strana takera sa odomkne ako prvá, vaša o niečo neskôr. Nikto nezostane zaseknutý.",
    activeTitle: "Vaše aktívne swapy",
    states: {
      open: "otvorené",
      takenByUs: "prijaté vami",
      revoked: "stiahnuté",
      expired: "vypršané",
    },
    // Two-sided order-book view of the Corkboard. Bids = makers giving the
    // quote coin to get the base; asks = the reverse. The ladder is a way to
    // READ the board — it never matches or prioritises (load-bearing).
    book: {
      bids: "Dopyty",
      asks: "Ponuky",
      bidsHint: "chcú {base} · platia {quote}",
      asksHint: "predávajú {base} · za {quote}",
      price: "Cena",
      size: "Veľkosť",
      noBids: "Žiadne dopyty",
      noAsks: "Žiadne ponuky",
      spread: "Rozpätie {pct}",
      spreadOneSided: "Jednostranné",
      crossed: "prekrížené",
      crossedTip: "Najvyšší dopyt ≥ najnižšia ponuka. Nástenka nikdy automaticky nepáruje, takže tieto prekrývajúce sa ponuky len tak ležia — prijmite ktorúkoľvek stranu.",
      mid: "stred {price}",
      levelOffers: "{count} ponúk za túto cenu — vyberte jednu na prijatie",
      depthTip: "Celkovo {sym} v ponuke za túto cenu naprieč {count} oznámeniami.",
      takerNote: "Ak ju prijmete, dáte {give} a dostanete {get}.",
      selectLevel: "Vyberte cenovú úroveň vyššie, aby ste videli ponuky na nej.",
      paneHeader: "{size} {base} @ {price} {unit}",
      denomTip: "Zobrazovacia jednotka pre sumy {coin}",
      showMore: "Zobraziť ďalších {count}",
      showLess: "Zobraziť horných {count}",
    },
  },
  relays: {
    title: "Relays",
    subtitle: "Živá konektivita k vašim Nostr relays — sieť, cez ktorú cestujú vaše ponuky a prijatia. Pridajte alebo odstráňte relays v Nastavenia → Sieť.",
    connectedCount: "{up} / {total} pripojených",
    refresh: "Obnoviť",
    ms: "{ms} ms",
    up: "online",
    down: "offline",
    statsTip: "{success}/{attempts} úspešných pripojení · ↓{down} ↑{up}",
    none: "Žiadne nakonfigurované relays",
    noneBody: "Pridajte Nostr relay v Nastavenia → Sieť, aby ste mohli zverejňovať a prijímať ponuky cez sieť.",
    goToNetwork: "Prejsť na Nastavenia",
    notConnected: "Nepripojené",
    notConnectedBody: "Zobrazenie relay vyžaduje bežiaci engine — najprv pripojte obchodníka.",
  },
  swaps: {
    title: "Swapy",
    hint: "Vaša úplná účtovná kniha — prebiehajúce swapy hore, dokončené obchody dole. Na aktívne swapy môžete reagovať aj z Corkboard.",
    activeTitle: "Prebieha",
    historyTitle: "História",
    none: "Zatiaľ žiadne swapy — prijmite ponuku na Corkboard.",
    // Active-swaps dock action buttons + the refund-time label.
    cancel: "zrušiť",
    refund: "vrátiť",
    dump: "vypísať logy",
    dumpHint: "Skopírujte diagnostický balík bez tajomstiev (stav + riadky logu) pre tento swap, na vloženie vývojárom.",
    dumpCopied: "Diagnostika skopírovaná — vložte vývojárom.",
    dumpFailed: "Diagnostický balík sa nepodarilo skopírovať.",
    refundAt: "vrátenie {when}",
    // Confirm dialogs for acting on a live swap (ActiveSwaps).
    cancelTitle: "Zrušiť tento swap?",
    cancelConfirm: "Zrušiť swap",
    cancelKeep: "Ponechať ho",
    // Abort reason recorded on the swap when cancelled from Satchel.
    cancelReason: "zrušené v Satchel",
    cancelBody:
      "Toto opúšťa swap skôr, než ste podfinancovali. Nič z vášho nie je ešte zamknuté, takže nič nestratíte — ponuka sa len nedokončí.",
    refundTitle: "Stiahnuť svoje prostriedky späť?",
    refundConfirm: "Vrátiť",
    refundBody:
      "Bezpečnostný timelock uplynul, takže si môžete prevziať prostriedky, ktoré ste zamkli. Toto vysiela vaše vrátenie teraz; engine to po uplynutí termínu urobí aj automaticky.",
    col: {
      swap: "swap",
      role: "rola",
      state: "stav",
      amounts: "dáva → dostáva",
      when: "kedy",
      finalTx: "finálna tx",
    },
    // Expandable per-leg on-chain detail (the audit trail). We show both
    // funding txs + OUR settlement; never the counterparty's settlement or the
    // swap secret.
    audit: {
      toggle: "Zobraziť on-chain detail",
      title: "On-chain detail",
      youLocked: "vy ste zamkli",
      theyLocked: "oni zamkli",
      funding: "Financovanie",
      received: "Prijaté",
      refunded: "Vrátené",
      pending: "zatiaľ nie on-chain",
      copy: "Kopírovať id transakcie",
      copied: "Id transakcie skopírované",
    },
  },
  fees: {
    title: "Náhľad sieťových nákladov",
    estimated: "odhadované",
    provisionalNote: "Tento build pactd zatiaľ nesprístupňuje odhad poplatkov.",
    summary: "Swap sú 2 on-chain transakcie, za ktoré platíte: financovanie na dávajúcom reťazci, redeem na prijímajúcom reťazci.",
    fallbackTip: "Uzol bol nedostupný, takže sa použila konzervatívna predvolená sadzba poplatku — berte to ako odhad.",
    ifItStalls: "(ak sa zasekne)",
  },
  funds: {
    insufficient:
      "Nedostatok {sym} na financovanie tohto swapu — potrebných ~{need} {sym} (suma + financovací poplatok), peňaženka má {have} {sym}.",
  },
  wizard: {
    welcome: "Vitajte v Satchel",
    connectTitle: "Pripojiť engine Pact",
    connectIntro:
      "Satchel je tenký klient enginu Pact — jadra, ktoré drží vaše kľúče a spúšťa swapy. Vyberte, ako ho dosiahnuť.",
    managed: "Spustiť zabudovaný engine Pact",
    managedDesc: "Satchel spustí a dohliada na svoj vlastný engine Pact. Odporúčané.",
    external: "Pripojiť k externému enginu Pact",
    externalDesc: "Nasmerujte na engine Pact, ktorý už prevádzkujete (pred spustením nastavte SATCHEL_PACTD_URL + cookie).",
    externalNote:
      "Externý režim sa vyberá cez premenné prostredia pred spustením Satchel. Znova spustite s nastaveným SATCHEL_PACTD_URL, aby ste ho použili.",
    coinsTitle: "Pridajte svoje mince",
    coinsIntro:
      "Po vytvorení vášho obchodníka pripojte každú mincu k svojmu vlastnému uzlu v Nastavenia → Mince. Vyberte mincu a backend (verejný Electrum pre nulové nastavenie alebo váš vlastný uzol); genesis sa skontroluje voči tejto sieti, než sa čokoľvek uloží.",
    coinsTemplatesSoon: "Šablóny mincí na jedno kliknutie pribudnú v neskoršom vydaní.",
    back: "Späť",
    continue: "Pokračovať",
    finish: "Dokončiť nastavenie",
  },
  // UI-4 docked activity log.
  log: {
    title: "Aktivita",
    empty: "— log aktivity —",
    count: "{count} riadkov",
    collapse: "Zbaliť log",
    expand: "Rozbaliť log",
    // Activity-log lines emitted by the frontend. `{err}` carries a raw engine
    // message (itself not translated — it comes from pactd over the wire).
    noTauri: "nebeží vnútri Satchel — toto rozhranie potrebuje most Tauri",
    startupError: "spustenie: {err}",
    notConnected: "nepripojené: {err}",
    connected: "pripojené k pactd {version} ({protocol})",
    listcoinsError: "listcoins: {err}",
    watchOnlyError: "iba sledovanie: {err}",
    switchedMerchant: "prepnuté na obchodníka {id}",
    switchMerchantError: "prepnúť obchodníka: {err}",
    loadMerchantError: "načítať obchodníka: {err}",
    merchantCreated: "obchodník {id} vytvorený",
    merchantReady: "obchodník pripravený",
    actionOk: "{action} {id}: ok",
    actionError: "{action} {id}: {err}",
    diagCopied: "diagnostika pre {id} skopírovaná ({count} riadkov logu) — vložte vývojárom",
    dumpError: "výpis {id}: {err}",
    coinDisconnected: "{coin} odpojené",
    removeCoinError: "odstrániť mincu: {err}",
    tookOffer: "ponuka {id} prijatá — teraz sa zobrazuje vo vašich aktívnych swapoch nižšie",
    takeError: "prijatie: {err}",
    offerWithdrawn: "ponuka {id} stiahnutá",
    withdrawError: "stiahnutie: {err}",
    postedOffer: "ponuka {id} zverejnená — kedykoľvek stiahnite; nič nie je zamknuté",
    createdSlip: "vytvorený súkromný ponukový lístok — pošlite ho svojmu priateľovi",
    tookPrivateOffer: "súkromná ponuka {id} prijatá — teraz sa zobrazuje vo vašich aktívnych swapoch",
    cancelledPrivateOffer: "súkromná ponuka {id} zrušená",
    cancelError: "zrušenie: {err}",
    noticeboardUpdated: "nástenka aktualizovaná",
    feePolicyUpdated: "politika poplatkov aktualizovaná",
  },
  // Relative-time + freshness prose from format.ts (rendered via the tr() mirror,
  // since those are pure non-component helpers). Unit letters stay inside the
  // template so a translation owns the whole phrase.
  format: {
    ageUnknown: "vek neznámy",
    justNow: "práve teraz",
    minutesAgo: "pred {n} min",
    hoursAgo: "pred {n} h",
    daysAgo: "pred {n} d",
    expiryNow: "teraz",
    expirySoon: "čoskoro",
    inMinutes: "o ~{n} min",
    inHours: "o ~{n} h",
    inDays: "o ~{n} d",
    posted: "zverejnené {age}",
    expires: "vyprší {time}",
  },
  // Plain-language swap story per (role, state) — the honest "who is exposed
  // when" framing shown on every active swap. {a}/{b} are coin tickers; {t1}/{t2}
  // are local refund times. Rendered via tr() (narrate() is a pure helper).
  narrate: {
    initiating:
      "Prijatie odoslané — čaká sa, kým maker začne swap. Nič nie je ešte zamknuté; samo sa zruší, ak neodpovie.",
    created: "Ponuka odoslaná — čaká sa, kým druhá strana súhlasí. Nič nie je viazané.",
    acceptedMaker: "Podmienky dohodnuté. Ďalej: zamknite svoje {a}. Kým nepodfinancujete, môžete stále voľne zrušiť.",
    acceptedTaker: "Podmienky dohodnuté. Druhá strana zamkne svoje {a} ako prvá — vy nikdy neposielate ako prví.",
    noncesExchanged:
      "Nastavovanie súkromného swapu — výmena podpisového materiálu. Nič nie je ešte zamknuté.",
    signedMaker:
      "Obe strany podpísali. Váš démon zamkne {a}, potom automaticky nárokuje {b}. Ak sa čokoľvek zasekne, vaše {a} sa vráti o {t1}.",
    signedTaker:
      "Obe strany podpísali. Váš démon zamkne {b} a nárokuje {a} v okamihu, keď sa druhá strana pohne. Záchranná sieť: vrátenie o {t2}.",
    fundedAMaker:
      "Vaše {a} je zamknuté. Čaká sa, kým druhá strana zamkne svoje {b}. Ak to nikdy neurobí, vaše {a} sa vráti automaticky o {t1}.",
    fundedATaker:
      "Ich {a} je zamknuté a overené. Ďalej: zamknite svoje {b}. Záchranná sieť: automatické vrátenie o {t2}, ak sa čokoľvek zasekne.",
    fundedBMaker: "Oboje zamknuté. Váš démon nárokuje {b}, akonáhle je bezpečne potvrdené.",
    fundedBTaker: "Oboje zamknuté. Váš démon nárokuje {a} v okamihu, keď druhá strana prevezme svoje {b}.",
    redeemedB:
      "Nárokovali ste {b} — čaká sa na potvrdenie. Vaše zamknuté {a} zostáva chránené, kým toto nie je finálne.",
    completed: "Swap dokončený — {coin} je vo vašej peňaženke.",
    refunded: "Swap sa nedokončil, takže vaše {coin} sa vrátilo automaticky. Nestratilo sa nič okrem poplatkov.",
    aborted: "Zrušené skôr, než sa pohli akékoľvek peniaze.",
  },
  progress: {
    settlement: "Potvrdzovanie nároku",
    theirFunding: "Čakanie na ich uzamknutie",
    oursFunding: "Uzamykanie vašich prostriedkov",
    feeBumped: "Poplatok zvýšený",
    reorg: "Zistená reorganizácia — opätovná kontrola",
  },
  exit: {
    // Exit-gate dialog (fund safety, C6). The engine manages alone, so "keep
    // running" detaches it (it keeps watching timelocks + servicing offers).
    liveTitle: "Prebieha swap",
    liveBodyOne:
      "1 swap prebieha. Riadi sa on-chain timelockmi — engine musí bežať ďalej, aby pred termínom vykonal redeem alebo vrátenie.",
    liveBodyMany:
      "{count} swapov prebieha. Riadia sa on-chain timelockmi — engine musí bežať ďalej, aby pred termínom vykonal redeem alebo vrátenie.",
    keepRunningExplain:
      "Zatvorenie okna ponechá engine bežať na pozadí, takže swap dokončí bez rozhrania. Satchel môžete kedykoľvek znova otvoriť a skontrolovať ho.",
    forceQuitWarn: "Vynútené ukončenie teraz zastaví engine a môže spôsobiť stratu prostriedkov.",
    // {word} is the confirm word below; a translation may localize both together.
    typeToConfirm: "Ak chcete napriek tomu vynútene ukončiť, napíšte nižšie {word}.",
    confirmWord: "QUIT",
    keepRunning: "Nechať bežať, zatvoriť okno",
    keepWithdraw: "Nechať bežať + stiahnuť ponuky",
    keepLeaveOffers: "Nechať bežať, ponechať ponuky",
    forceQuit: "Vynútene ukončiť",
    offersTitle: "Máte zverejnené ponuky",
    offersBodyOne:
      "1 vaša ponuka je stále na Corkboard. Ponuky nič nezamykajú, ale jej ponechaním môžu protistrany stále prijať, kým je Satchel zatvorený — engine prijatie obslúži.",
    offersBodyMany:
      "{count} vašich ponúk je stále na Corkboard. Ponuky nič nezamykajú, ale ich ponechaním môžu protistrany stále prijať, kým je Satchel zatvorený — engine prijatia obslúži.",
    withdrawExit: "Stiahnuť všetky a ukončiť",
  },
  unlock: {
    title: "Odomknúť obchodníka",
    body:
      "Seed tohto obchodníka je šifrovaný. Zadajte jeho prístupovú frázu na odomknutie pre túto reláciu — Satchel ju drží iba v pamäti a pri ukončení ju zabudne.",
    switchMerchant: "Prepnúť obchodníka",
    unlock: "Odomknúť",
  },
  common: {
    cancel: "Zrušiť",
    confirm: "Potvrdiť",
    save: "Uložiť",
    done: "Hotovo",
    later: "Neskôr",
    retry: "Skúsiť pripojenie znova",
  },
};
