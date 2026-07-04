// The Croatian (Hrvatski) string bundle. Mirrors en.ts key-for-key; only values are translated.
import type { Bundle } from "./en";

export const hr: Bundle = {
  app: {
    name: "Satchel",
  },
  // In-app update notifications (sidebar version badge + dialog).
  update: {
    title: "Dostupna nadogradnja",
    upToDate: "Imate najnoviju verziju",
    current: "Instalirano",
    latest: "Najnovije",
    notesTitle: "Bilješke o izdanju",
    get: "Preuzmi nadogradnju",
    dismiss: "Odbaci",
    close: "Zatvori",
    badgeTooltip: "Dostupna nadogradnja — kliknite za detalje",
    versionTooltip: "Kliknite za provjeru nadogradnji",
  },
  // Risk disclaimer (first-run welcome + Settings → About).
  disclaimer: {
    title: "Samostalno skrbništvo — vaši ključevi, vaša odgovornost",
    body: "Satchel izvodi atomske swapove bez skrbništva: jedino vi držite svoje ključeve, a seed trgovca drži vruće tranzitne ključeve dok je swap u tijeku. Swap protokoli (v1 HTLC i v2 Taproot/MuSig2) pregledani su i u upotrebi na mainnetu. Pod MIT licencom i pružen takav kakav jest, bez ikakvog jamstva — napravite sigurnosnu kopiju recovery fraze i koristite na vlastitu odgovornost.",
  },
  nav: {
    public: "Javno",
    corkboard: "Corkboard",
    postOffer: "Objavi ponudu",
    private: "Privatno",
    privateCreate: "Stvori slip",
    privateReceive: "Preuzmi slip",
    privateSlips: "Moji slipovi",
    swaps: "Swapovi",
    relays: "Relayi",
    wallets: "Novčanici",
    contacts: "Contacts",
    settings: "Postavke",
    coins: "Kovanice",
  },
  makeOffer: {
    title: "Objavi ponudu",
    intro:
      "Objavite potpisanu ponudu na Corkboard. Ništa nije zaključano — to je samo oglas; povucite ga bilo kada, a swap počinje tek kad ga netko preuzme i obje strane financiraju.",
    give: "Dajete",
    want: "Primate",
    // Canonical pair + direction: pick the pair, choose Sell/Buy the base, enter
    // the base amount and a quote-per-base price (invariant to direction).
    pair: "Par",
    noPairs: "Nema parova za trgovanje — povežite barem dvije kovanice u Postavke → Kovanice.",
    sell: "Prodaj {sym}",
    buy: "Kupi {sym}",
    amount: "Iznos",
    youGive: "Dajete",
    youGet: "Dobivate",
    price: "Cijena",
    priceUnit: "{unit} po {base}",
    pricePlaceholder: "jedinična cijena",
    balance: "Stanje: {amt} {sym}",
    balanceLoading: "Stanje: …",
    noCoins: "Nema konfiguriranih kovanica",
    legDown: "Čvor jedne od ovih kovanica je nedostupan — pokrenite ga (ili provjerite Postavke → Kovanice) prije objave.",
    // Swap-protocol pin (only offered when a pair+network supports more than
    // one). v2 label reuses coins.protoPrivate ("Private (Taproot)").
    protocol: "Vrsta swapa",
    protoStandard: "Standardni (HTLC)",
    // Titles for the review/confirm dialog shown before posting (see OfferForm).
    reviewOfferTitle: "Pregledajte svoju ponudu",
    reviewSlipTitle: "Pregledajte svoj slip",
    term: "Sigurnosni timelock",
    termShort: "Kratak",
    termMedium: "Srednji",
    termLong: "Dug",
    termHint: {
      short: "Kratak — sredstva se najbrže automatski vraćaju ako trgovina zapne (~12 h / 6 h), uz najmanju sigurnosnu marginu.",
      medium: "Srednji — uravnotežen prozor za povrat (~24 h / 12 h).",
      long: "Dug (najsigurniji) — najšira sigurnosna margina; automatski povrat nakon ~36 h / 18 h ako trgovina zapne.",
    },
    validFor: "Vrijedi (minuta)",
    validForMins: "{mins} min",
    validForHint:
      "Koliko dugo ponuda ostaje na popisu. Dok ste online, automatski se osvježava; nakon toga istječe. Zatvaranjem aplikacije povlači se.",
    note: "Ponuda fiksne veličine — ništa nije zaključano dok je netko ne preuzme. Iznosi su on-chain; mrežne naknade plaćate dodatno, a Corkboard ne naplaćuje ništa. Timelock je prozor za automatski povrat ako swap zapne.",
    post: "Objavi ponudu",
    makeSlip: "Stvori slip",
    slipTitle: "Vaš privatni slip ponude",
    slipExplainer:
      "Pošaljite ovo svom prijatelju. On ga zalijepi u Satchel kako bi ga preuzeo. Ništa nije zaključano; istječe za {ttl}.",
    copy: "Kopiraj",
    copied: "Kopirano",
    makeAnother: "Napravi drugi",
    myPrivateTitle: "Moje privatne ponude",
    myPrivateEmpty: "Nema aktivnih privatnih ponuda.",
    privateExpires: "istječe {when}",
    privateExpired: "isteklo",
    cancel: "Odustani",
    cancelTip: "Prestani priznavati ovaj slip — prijatelj koji ga još drži više ga ne može preuzeti.",
  },
  takeSlip: {
    intro:
      "Prijatelj vam je poslao privatni slip ponude (počinje s pactoffer1:). Zalijepite ga ovdje za pregled i preuzimanje — baš kao ponudu s ploče.",
    placeholder: "pactoffer1:…",
    take: "Pregledaj i preuzmi",
    invalid: "Ovo ne izgleda kao slip — trebao bi početi s pactoffer1:.",
    previewLabel: "Ovaj slip nudi",
  },
  // PRIVATE nav group (off-market slips) — screen titles + intros. The form,
  // slip output, my-slips list and take flow reuse makeOffer.*/takeSlip.* copy.
  private: {
    createTitle: "Stvori privatnu ponudu",
    createIntro:
      "Sastavite potpisanu ponudu i predajte je prijatelju kao slip putem vlastitog chata. Nigdje nije izlistano — i ništa nije zaključano dok oboje ne financirate.",
    slipsIntro:
      "Slipovi koje ste stvorili. Tko god drži slip može ga preuzeti dok ne istekne; otkažite ga da prestanete priznavati prije toga.",
    slipsEmptyBody: "Stvorite privatnu ponudu da dobijete slip koji možete poslati prijatelju.",
    receiveTitle: "Preuzmi privatnu ponudu",
    received: "Preuzeto — pratite u Swapovima.",
  },
  // Shared take-confirmation dialog (board take + slip take).
  takeConfirm: {
    title: "Preuzeti ovu ponudu?",
    confirm: "Preuzmi ponudu",
    counterparty: "Druga strana",
    youGive: "Dajete",
    youReceive: "Primate",
    safetyRefund: "Sigurnosni povrat",
    offerAge: "Starost ponude",
    makerFundsFirst:
      "Maker prvi zaključava svoj {sym} — vi nikad ne šaljete prvi. I dalje možete otkazati prije nego što financirate svoju stranu, a engine automatski vraća sredstva nakon sigurnosnog timelocka ako swap zapne.",
  },
  header: {
    activeMerchant: "Aktivni trgovac — kliknite za zamjenu ili upravljanje",
    manageMerchants: "Upravljaj trgovcima…",
    noMerchant: "nema trgovca",
    openMenu: "Otvori izbornik",
    collapseMenu: "sažmi izbornik",
    settings: "Postavke",
    language: "Jezik",
    pactConnected: "Engine povezan",
    pactUnreachable: "Engine nedostupan",
    liveSwapsOne: "1 swap u tijeku — kliknite za prikaz",
    liveSwapsMany: "{count} swapova u tijeku — kliknite za prikaz",
    liveSwapsNone: "Nema swapova u tijeku",
    coinOk: "{name} — povezano · vrh {tip}",
    coinUnconfigured: "{name} — nije postavljeno",
    coinError: "{name} — {status}",
    relaysOk: "Nostr relayi — {up}/{total} povezano",
    relaysDown: "Nostr relayi — nijedan od {total} nije povezan",
  },
  network: {
    mainnet: "MainNet",
    testnet: "TestNet",
    regtest: "RegTest",
    signet: "Signet",
    notRealFunds: "Nisu stvarna sredstva — ovo je {network} mreža",
  },
  // Watch-only mode: a viewer session with no coins. Browse the board and
  // withdraw your own offers, but no posting / taking / funding.
  watchOnly: {
    badge: "Samo za pregled",
    badgeTip:
      "Način samo za pregled — pregledavajte ploču i povlačite vlastite ponude, ali ne možete objavljivati, preuzimati ni financirati. Postavite kovanice u Postavkama za trgovanje.",
    coinWizardButton: "Pregledavaj u načinu samo za pregled",
    coinWizardHint:
      "Preskočite postavljanje kovanica i samo pregledavajte ploču (samo za čitanje). I dalje možete povući vlastite ponude — zgodno za uklanjanje ponuda koje je ostavila druga sesija. Isključite ga bilo kada u Postavkama.",
    postBlockedTitle: "Način samo za pregled",
    postBlockedBody:
      "Ovo je sesija samo za pregled, pa ne može objavljivati ponude. Postavite barem dvije kovanice u Postavke → Kovanice za trgovanje.",
    takeBlockedBody: "Način samo za pregled — možete pregledati ovu ponudu, ali za preuzimanje su potrebne postavljene kovanice.",
    takeBlockedTip: "Način samo za pregled — postavite kovanice u Postavkama za preuzimanje ponuda.",
  },
  merchants: {
    title: "Vaši trgovci",
    intro:
      "Trgovac je jedan trgovački identitet — vlastiti seed i povijest swapova. Trgovanje pod drugim trgovcem čini kontekste nepovezivima (burner identitet). Vaše glavne kovanice žive u vašem vlastitom novčaniku, ne ovdje.",
    // First-run welcome (empty merchant manager).
    welcomeTitle: "Dobro došli u Satchel",
    welcomeIntro:
      "Satchel trguje pod „trgovcem” — jednim trgovačkim identitetom s vlastitim seedom. Još nemate nijedan: stvorite novi ili uvezite postojeću recovery frazu za početak.",
    importMerchant: "Uvezi trgovca",
    none: "Još nema trgovaca.",
    switch: "zamijeni",
    newMerchant: "Novi trgovac",
    thisMerchant: "ovaj trgovac",
    nameLabel: "Ime trgovca",
    namePlaceholder: "npr. Glavni",
    rename: "Preimenuj",
    introFirst:
      "Postavite svoj prvi trgovački identitet („trgovca”). On drži samo vruće tranzitne ključeve za swapove u tijeku — vaše glavne kovanice ostaju u vašem vlastitom novčaniku.",
    introNew: "Novi trgovac je svjež, zaseban identitet s vlastitim seedom i poviješću swapova.",
    // UI-5 merchant selector (phoenix wallet-selector parity).
    createNew: "Stvori novi",
    import: "Uvezi",
    load: "Učitaj trgovca",
    loaded: "učitan",
    locked: "zaključan",
    lockedTip: "Šifrirani seed — otključajte ga zaporkom kad ga učitavate.",
    close: "Zatvori",
    idLabel: "mapa",
    switching: "Mijenjanje trgovca…",
    switchingBody: "Ponovno pokretanje enginea nad tom mapom.",
  },
  // Seed create/import (SeedForm) + counterparty + status.
  seed: {
    intro: "Stvorite potpuno novi seed ili uvezite onaj koji već imate.",
    createNew: "Stvori novi",
    createDesc: "Generirajte svjež seed. Vi radite sigurnosnu kopiju recovery fraze.",
    import: "Uvezi",
    importDesc: "Obnovite iz postojeće fraze od 12/24 riječi.",
    recoveryLabel: "Recovery fraza",
    encrypt: "Šifriraj",
    encryptDesc:
      "Zaporka štiti seed u mirovanju. Unosite je jednom po sesiji — Satchel je nikad ne pohranjuje. Napomena: automatski povrat bez nadzora pauzira se nakon ponovnog pokretanja dok je ponovno ne unesete.",
    noPassphrase: "Bez zaporke (preporučeno)",
    noPassphraseDesc:
      "Automatski povrat radi i kroz ponovna pokretanja bez ičega za unos — ovo je samo vrući tranzitni seed. Cijena: pristup datoteci/hostu izlaže tranzitne ključeve i identitet ovog trgovca.",
    passphraseLabel: "Zaporka",
    passphrasePlaceholder: "odaberite zaporku",
    revealTitle: "Zapišite svoju recovery frazu",
    revealBody:
      "Tko god ima ove riječi kontrolira vruće ključeve ovog trgovca. Satchel ne čuva nikakvu kopiju — pohranite je offline. Sljedeće ćete potvrditi nekoliko riječi.",
    ackLabel: "Zapisao/la sam svoju recovery frazu.",
    // Multi-step onboarding (create/import -> secret -> confirm -> passphrase).
    chooseTitle: "Postavi {label}",
    enterTitle: "Uvezite svoju recovery frazu",
    enterBody:
      "Upišite svaku riječ — automatski se dovršavaju dok pišete — ili zalijepite cijelu frazu. Provjeravamo je prije nego što nastavite.",
    wordCount: "{n} riječi",
    wordCountHint:
      "12 riječi sasvim je dovoljno — ovo je vrući tranzitni novčanik, a ne hladna pohrana. Odaberite 24 ako vam je draža dulja fraza.",
    wordAria: "Riječ {n}",
    checkIncomplete: "Unesite svih {n} riječi.",
    checkUnknown: "Neke riječi nisu u BIP39 popisu riječi — provjerite istaknute.",
    checkBadChecksum: "Kontrolni zbroj se ne podudara — ponovno provjerite riječi i njihov redoslijed.",
    checkOk: "Recovery fraza izgleda valjano.",
    verifyTitle: "Potvrdite svoju sigurnosnu kopiju",
    verifyBody: "Upišite riječi na ovim pozicijama kako biste potvrdili da ste zapisali frazu.",
    verifyWord: "Riječ #{n}",
    verifyMismatch: "Te se ne podudaraju s vašom frazom — provjerite svoju sigurnosnu kopiju.",
    passphraseTitle: "Zaštitite seed",
    passphraseBody:
      "Po želji šifrirajte pohranjeni seed zaporkom. Ovo možete preskočiti — pogledajte kompromis ispod.",
  },
  counterparty: {
    you: "Ovo ste vi",
    youShort: "vi",
    unknown: "nepoznat identitet",
    // Short fingerprint fallback (identity.ts shortId) when no pubkey is known.
    unknownShort: "nepoznato",
  },
  contacts: {
    title: "Kontakti",
    subtitle: "Vaši privatni nadimci za ljude s kojima trgujete.",
    privacyNote:
      "Kontakti se pohranjuju samo na ovom uređaju i nikada se ne dijele, objavljuju niti šalju na relay. Nadimak je vaša oznaka — identikon i otisak ostaju stvarni identitet.",
    searchPlaceholder: "Pretraži nadimak, bilješku ili ključ",
    empty: "Još nema kontakata. Kliknite identikon druge strane bilo gdje da biste ga dodali.",
    emptyFiltered: "Nema kontakata koji odgovaraju ovom filtru.",
    count: "{n} kontakata",
    colWho: "Identitet",
    colNick: "Nadimak",
    colNote: "Bilješke",
    colStatus: "Status",
    colAdded: "Dodano",
    colActions: "",
    filterAll: "Sve",
    filterTrusted: "Pouzdani",
    filterBlocked: "Blokirani",
    // Corkboard toggle: drop blocked makers' offers from the ladder.
    hideBlocked: "Sakrij blokirane ponude",
    statusTrusted: "Pouzdan",
    statusNeutral: "Neutralan",
    statusBlocked: "Blokiran",
    menuAdd: "Dodaj u kontakte…",
    menuEdit: "Uredi kontakt…",
    menuMarkTrusted: "Označi kao pouzdanog",
    menuMarkNeutral: "Označi kao neutralnog",
    menuMarkBlocked: "Blokiraj",
    menuCopyKey: "Kopiraj javni ključ",
    menuOpen: "Otvori u Kontaktima",
    keyCopied: "Javni ključ kopiran",
    editTitle: "Uredi kontakt",
    addTitle: "Dodaj kontakt",
    nickLabel: "Nadimak",
    nickPlaceholder: "npr. Alice sa sastanka",
    noteLabel: "Bilješke",
    notePlaceholder: "Bilo što što želite zapamtiti — kako ih kontaktirati, prošle trgovine…",
    save: "Spremi",
    cancel: "Odustani",
    remove: "Ukloni kontakt",
    removeConfirmTitle: "Ukloniti kontakt?",
    removeConfirmBody: "Ovo briše vaš lokalni nadimak i bilješke za {who}. Ne može se poništiti.",
    blockedWarning: "Blokirali ste ovu drugu stranu",
    blockedWarningBody:
      "Označili ste ovu osobu kao blokiranu. Blokiranje je samo osobni podsjetnik — ne zaustavlja trgovinu. Nastavite samo ako to stvarno želite.",
  },
  status: {
    notConnectedTitle: "Nije povezano s engineom",
    disconnectedBody:
      "Satchel ne može doseći engine. Možda se još pokreće ili su veze čvorova aktivnog trgovca nedostupne. Pokušajte ponovno ili zamijenite trgovca iz birača na vrhu.",
    openInSatchel: "Otvori ovo u Satchelu",
    noTauriBody:
      "Ovo je Satchelovo korisničko sučelje — treba mu Tauri most za dosezanje enginea. Pokrenite desktop aplikaciju (cargo tauri dev) umjesto preglednika.",
  },
  settings: {
    title: "Postavke",
    subtitle: "Postavke za cijelu aplikaciju za ovu instalaciju.",
    // UI-3 Settings tabs.
    tabGeneral: "Općenito",
    tabCoins: "Kovanice",
    tabNetwork: "Mreža",
    tabAbout: "O aplikaciji",
    appearance: "Izgled",
    theme: "Tema",
    themeDark: "Tamna",
    themeLight: "Svijetla",
    themeSystem: "Sustav",
    themeHint: "Odaberite kako Satchel izgleda. Sustav prati postavku vašeg OS-a.",
    language: "Jezik",
    languageHint: "Više jezika stiže kako se doprinose prijevodi.",
    mode: "Način",
    watchOnly: "Način samo za pregled",
    watchOnlyHint:
      "Pregledavajte ploču bez postavljanja kovanica. I dalje možete povući vlastite ponude, ali ne možete objavljivati, preuzimati ni financirati. Isključite za trgovanje (trebat će vam barem dvije povezane kovanice).",
    network: "Mreža",
    boards: "Corkboardovi",
    boardsDesc:
      "Neobavezne samostalno hostane HTTP ploče. Dodajte one kojima vjerujete; ostavite prazno da se oslonite na Nostr.",
    boardsNone: "Nijedan nije konfiguriran",
    nostrRelays: "Nostr relayi",
    nostrRelaysDesc:
      "Relayi prenose oglasnu ploču preko decentralizirane mreže — nijedan operater ne može čitati ni uparivati vaše ponude. Unaprijed postavljeni sa zadanim setom; uređujte slobodno.",
    nostrRelaysOff: "Isključeno — Nostr transport onemogućen",
    addUrl: "Dodaj",
    removeUrl: "Ukloni",
    relayInvalid: "Unesite ws:// ili wss:// URL relaya",
    boardInvalid: "Unesite http:// ili https:// URL ploče",
    netSave: "Spremi i ponovno poveži",
    netSaving: "Spremanje i ponovno povezivanje…",
    netSaved: "Spremljeno",
    // Fees tab — fee-bump policy (per active merchant).
    tabFees: "Naknade",
    fees: "Povećanje naknade",
    feesScope: "Ove se postavke odnose na aktivnog trgovca.",
    feesIntro:
      "Kompromisi sigurnosti/troška za povećanje naknade, nisu obavezno postavljanje. Nove vrijednosti vrijede za buduća povećanja; već financirani swapovi zadržavaju politiku pod kojom su financirani.",
    feeMax: "Maks. feerate (sat/vB)",
    feeMaxHint:
      "Gornja granica za svako povećanje naknade. Zadano 500, ujedno i tvrdi sistemski maksimum. Smanjite je da ograničite troškove.",
    feeReservation: "Rezervacija za povećanje financiranja (×)",
    feeReservationHint:
      "Stanje koje provjera sredstava izdvaja kao rezervu za povećanje. Više spašava veće skokove naknade, ali veže više stanja i odbija više swapova. Zadano 3.",
    feeCommitted: "Predoplata naknade za otkup (×)",
    feeCommittedHint:
      "Koliko se dodatno unaprijed plaća v2 naknada za otkup kako bi se potvrdila čak i kad je Satchel zatvoren. Vrijedi samo za nove swapove. Zadano 2.",
    feeSave: "Spremi",
    feeSaving: "Spremanje…",
    feeSaved: "Spremljeno",
    feeReset: "Vrati na zadano",
    coins: "Kovanice i čvorovi",
    coinsHint: "Povežite svaku kovanicu s vlastitim čvorom. Genesis se provjerava prije nego što se išta spremi.",
    about: "O aplikaciji",
    version: "Verzija {version}",
    updateUpToDate: "Ažurirano",
    updateCheckPlaceholder: "Provjera nadogradnji stiže u kasnijem izdanju.",
    trustModel: "Gdje žive vaši ključevi",
    trustModelBody:
      "Tajne žive u engineu, nikad u Satchelu. Seed trgovca nalazi se u podatkovnoj mapi enginea (šifriran ili u čistom tekstu — vaš izbor); Satchel ne pohranjuje nikakav seed ni zaporku. Seed je vruć po dizajnu (samo tranzitni ključevi) — prebacite veću dobit u vlastiti hladni novčanik.",
  },
  coins: {
    intro:
      "Povežite svaku kovanicu s vlastitim čvorom. Prvi URL je vlastiti novčanik vašeg čvora — on financira vaše swap dijelove i prima dobit. Prije nego što se išta spremi, Satchel provjerava genesis blok čvora kako sredstva nikad ne bi mogla biti poslana na pogrešan lanac. Veze se dijele među svim vašim trgovcima.",
    networkBadge: "Konfiguriranje za {network} mrežu",
    needMerchant:
      "Prvo povežite trgovca — postavljanje kovanica treba pokrenuti engine. Koristite birač trgovca u gornjem desnom kutu.",
    pairsTitle: "Trgovački parovi",
    pairsHint:
      "Parovi proizlaze iz onoga što svaka kovanica može — nema fiksnog popisa. Par se otvara čim su obje njegove kovanice povezane.",
    noPairs: "Nema dostupnih parova.",
    notSetUp: "Nije postavljeno",
    connectedTip: "Povezano · vrh {tip}",
    connError: "Greška veze",
    setUp: "Postavi",
    editConnection: "Uredi vezu",
    remove: "ukloni",
    disconnectTip: "Odspoji ovu kovanicu",
    disconnectTitle: "Odspojiti {coin}?",
    disconnectBody: "Swapovi koji je trebaju neće biti dostupni dok se ponovno ne povežete.",
    ready: "Spremno za trgovanje",
    connectMissing: "Povežite {coins}",
    notBuildable: "Još nije izgradivo",
    // Swap-protocol chips on a pair (pact-htlc-v1 HTLC vs pact-htlc-v2 adaptor).
    protoPrivate: "Privatno (Taproot)",
    protoPrivateTip: "Privatni swap (Taproot/MuSig2 adaptor) — izgleda kao obična uplata on-chain",
    protoHtlcTip: "Klasičan HTLC swap",
    // Coin-setup backend choices (CoinSetup).
    // CoinSetup dialog.
    setupTitle: "Poveži {coin}",
    setupIntro:
      "Usmjerite Satchel na vlastiti {sym} čvor. Ništa se ne sprema dok čvor ne prođe provjeru genesis bloka — vaša sredstva ikad dodiruju samo stvarni {sym} lanac.",
    confirmationsLabel: "Potvrde prije konačnog",
    confirmationsHint:
      "Koliko duboko financiranje ili otkup na ovom lancu mora biti prije nego što swap reagira — margina sigurnosti od reorga. Više je sigurnije, ali sporije; ostavite prazno za preporučenu zadanu vrijednost ({default}).",
    validateNode: "Provjeri čvor",
    checking: "Provjera čvora…",
    genesisOk: "Genesis se podudara — ovo je pravi lanac",
    genesisDetail: "visina vrha {tip} · genesis {hash}…",
    genesisBad: "Odbijeno — ne spremam",
    errorShort: "greška",
    // Structured connection form (CoinSetup v2).
    rpcHostLabel: "RPC host",
    rpcPortLabel: "RPC port",
    authMethodLabel: "Autentikacija",
    authCookie: "Cookie datoteka",
    authCookieDesc: "Automatski čitaj .cookie čvora iz njegove podatkovne mape (zadano, bez pohranjene lozinke).",
    authUserPass: "Korisnik / lozinka",
    authUserPassDesc: "rpcuser / rpcpassword iz konfiguracije vašeg čvora — potrebno za udaljeni čvor.",
    rpcUserLabel: "RPC korisničko ime",
    rpcPasswordLabel: "RPC lozinka",
    datadirLabel: "Podatkovna mapa čvora",
    cookiePathNote: "Cookie se čita iz {path} unutar ove mape.",
    walletLabel: "Ime novčanika (neobavezno)",
    walletPlaceholder: "novčanik vašeg čvora",
    needPort: "Prvo unesite RPC port.",
    validateFirst: "Provjerite čvor prije spremanja.",
    savingReconnecting: "Spremanje i ponovno povezivanje…",
    connected: "{coin} povezan",
    // Nodeless (Electrum) connection mode (epic #58).
    modeLabel: "Vrsta veze",
    modeNode: "Vlastiti čvor",
    modeNodeDesc: "Core RPC — novčanik čvora financira swapove. Maksimalna suverenost.",
    modeNodeless: "Electrum",
    modeNodelessDesc:
      "Čvor nije potreban: podaci o lancu dolaze s Electrum poslužitelja, a novčanik živi na vašem Pact seedu.",
    electrumUrlsLabel: "Electrum poslužitelji",
    electrumUrlsHelp:
      "Jedan po retku: tcp://host:port ili ssl://host:port. Mainnet zahtijeva barem dva neovisna poslužitelja koji unakrsno provjeravaju stanje lanca.",
    electrumNeedUrl: "Unesite barem jedan URL Electrum poslužitelja (tcp:// ili ssl://).",
    electrumBadUrl: "Electrum URL-ovi moraju počinjati s tcp:// ili ssl:// — dobiveno: {url}",
    validateServers: "Provjeri poslužitelje",
    connRpcLocal: "RPC (lokalni)",
    connRpcRemote: "RPC (udaljeni)",
    connElectrumLocal: "Electrum (lokalni)",
    connElectrumRemote: "Electrum (udaljeni)",
    connRpcTip:
      "Ova kovanica komunicira s čvorom tipa Bitcoin Core putem RPC-a; swapove financira novčanik čvora.",
    connElectrumTip:
      "Ova se kovanica povezuje s Electrum poslužiteljima — bez čvora. Novčanik živi na vašem Pact seedu.",
    switchHidesTitle: "Ovo skriva vaš novčanik na Pact seedu",
    switchHidesBody:
      "Vaš novčanik na Pact seedu za ovu kovanicu još uvijek drži {balance} {sym}. Prebacivanje na vezu s čvorom ga skriva — kovanice ostaju sigurne na vašem seedu i ponovno se pojavljuju čim se vratite na Electrum, no do tada se neće prikazivati niti financirati swapove. Razmislite da ih prvo nekamo pošaljete.",
    switchHidesConfirm: "Svejedno prebaci",
    // Template picker (a coins.toml coin the engine version doesn't support).
    unsupportedByEngine: "Nepodržano",
    unsupportedByEngineTip:
      "Ova kovanica je definirana u coins.toml, ali nije ugrađena u ovu verziju enginea, pa se njome ne može trgovati.",
  },
  coinWizard: {
    title: "Povežite svoje kovanice",
    intro:
      "Odaberite barem dvije kovanice i usmjerite svaku na vlastiti čvor. Swap treba dva lanca, pa se trgovanje otključava kad su dva čvora povezana i aktivna. Kovanice možete dodati ili promijeniti kasnije u Postavkama.",
    progress: "{count} od {min} kovanica povezano",
    continue: "Nastavi",
    live: "Aktivno",
    nodeDown: "Čvor nedostupan",
  },
  wallets: {
    intro:
      "Ovo su novčanici vaših vlastitih čvorova (oni koje engine koristi za financiranje swapova i primanje dobiti) — vaši ključevi, vaš stroj. Satchel nikad ne drži vaše kovanice.",
    hotSeedNudge:
      "Ovo je potrošni novčanik na vrućem seedu, ne trezor — prebacite veća stanja u vlastiti hladni/core novčanik.",
    notConnected: "Nije povezano",
    notConnectedBody: "Prvo povežite trgovca — prikaz novčanika treba pokrenuti engine.",
    noCoins: "Još nema postavljenih kovanica",
    noCoinsBody: "Povežite kovanicu u Postavke → Kovanice i njezin se novčanik pojavi ovdje.",
    goToCoins: "Idi na Kovanice",
    watchOnlyTitle: "Nema novčanika u načinu samo za pregled",
    watchOnlyBody:
      "Ovo je sesija samo za pregled bez povezanih kovanica, pa nema novčanika za prikaz. Isključite način samo za pregled u Postavkama i povežite kovanicu za financiranje swapova.",
    walletName: "novčanik · {wallet}",
    walletScopedHint: "Svaki RPC za ovu kovanicu ograničen je na ovaj novčanik čvora.",
    walletDefault: "zadani novčanik (bez ograničenja)",
    walletDefaultHint:
      "Za ovu kovanicu nije postavljen novčanik, pa RPC-ovi koriste zadani novčanik čvora. Postavite jedan u Postavke → Kovanice da ograničite svaki poziv na određeni novčanik.",
    balanceLabel: "Stanje {symbol}",
    // ---- nodeless (pact-seed bdk) wallet: send / receive / activity --------
    pactSeed: "novčanik na Pact seedu",
    pactSeedHint:
      "Ova kovanica radi bez čvora: njezin novčanik živi na vašem Pact seedu, sinkroniziran s Electrum poslužitelja — čvor nije potreban. Slanje, primanje i povijest su upravo ovdje.",
    receive: "Primi",
    send: "Pošalji",
    activity: "Aktivnost",
    copy: "Kopiraj",
    copied: "Kopirano",
    close: "Zatvori",
    refresh: "Osvježi",
    receiveTitle: "Primi {sym}",
    receiveIntro:
      "Svježa adresa iz vašeg novčanika na Pact seedu. Kovanice poslane ovamo pojavljuju se u stanju nakon potvrde.",
    receiveIntroRpc:
      "Svježa adresa iz novčanika vašeg čvora. Kovanice poslane ovamo pojavljuju se u stanju nakon potvrde.",
    receiveFreshNote:
      "Svaki put kad otvorite ovaj dijalog dobivate svježu adresu. Stare adrese i dalje rade — svježe su jednostavno bolje za privatnost.",
    sendTitle: "Pošalji {sym}",
    sendIntro: "Raspoloživo: {balance} {sym}.",
    sendAddressLabel: "{sym} adresa primatelja",
    sendAmountLabel: "Iznos",
    sendNeedAddress: "Unesite adresu primatelja.",
    sendNeedAmount: "Unesite iznos.",
    sendOverBalance: "Više od raspoloživog stanja.",
    sendFeeNote: "Mrežna naknada dodaje se povrh iznosa i bira se automatski prema trenutnom tržištu naknada.",
    sendBroadcast: "Poslano — {txid}… je na putu ({sym}).",
    sendConfirm: "Pošalji",
    activityTitle: "Aktivnost {sym}",
    activityEmpty: "Još ništa — primite kovanice ili dovršite swap i pojavit će se ovdje.",
    activityWhen: "Kada",
    activityDirection: "Smjer",
    activityAmount: "Iznos ({sym})",
    activityFee: "Naknada",
    activityConfs: "Potv.",
    activityTxid: "Transakcija",
    activityPending: "na čekanju",
    activitySent: "Poslano",
    activityReceived: "Primljeno",
  },
  corkboard: {
    noBoardTitle: "Nijedan Corkboard nije povezan",
    noBoardBody:
      "Corkboard je dijeljena oglasna ploča na koju makeri zakvače ponude. Nikad ne uparuje trgovine ni ne drži kovanice — usmjerite Satchel na onu kojoj vjerujete za pregled i objavu.",
    noPairs: "Nema dostupnih parova",
    board: "Corkboard",
    boardSettings: "Konfiguriraj u Postavkama",
    filterAll: "Sve",
    filterMine: "Moje",
    noOffers: "Trenutno nema ponuda koje možete preuzeti",
    noOffersBody:
      "Ponude se pojavljuju ovdje čim maker objavi neku za par koji ste postavili. Možete objaviti i vlastitu.",
    yourOffer: "vaša ponuda",
    offerStaged: "objavljujem…",
    offerStagedTip:
      "Objavljeno s ovog uređaja i čeka potvrdu natrag s relaya. Oglašava se; postaje aktivno kad ga relay odjekne natrag.",
    take: "Preuzmi ponudu",
    legDown: "Čvor jedne kovanice ovog para je nedostupan — pokrenite ga (ili provjerite Postavke → Kovanice) prije preuzimanja.",
    withdraw: "Povuci",
    withdrawTip: "Povucite trenutačno — ponuda nikad ne zaključava sredstva",
    safetyRefund: "sigurnosni povrat",
    safetyRefundTip:
      "Ako swap zapne, obje strane se automatski vraćaju — takerov dio se prvi otključava, vaš malo kasnije. Nitko ne ostane zaglavljen.",
    activeTitle: "Vaši aktivni swapovi",
    states: {
      takenByUs: "preuzeli ste vi",
      revoked: "povučeno",
      expired: "isteklo",
    },
    // Two-sided order-book view of the Corkboard. Bids = makers giving the
    // quote coin to get the base; asks = the reverse. The ladder is a way to
    // READ the board — it never matches or prioritises (load-bearing).
    book: {
      bids: "Ponude (bids)",
      asks: "Tražnje (asks)",
      bidsHint: "želi {base} · plaća {quote}",
      asksHint: "prodaje {base} · za {quote}",
      price: "Cijena",
      size: "Veličina",
      noBids: "Nema bidova",
      noAsks: "Nema askova",
      spread: "Raspon {pct}",
      spreadOneSided: "Jednostrano",
      crossed: "ukršteno",
      crossedTip: "Najviši bid ≥ najniži ask. Ploča nikad ne uparuje automatski, pa ove preklapajuće ponude jednostavno stoje — preuzmite bilo koju stranu.",
      mid: "sredina {price}",
      levelOffers: "{count} ponuda po ovoj cijeni — odaberite jednu za preuzimanje",
      depthTip: "Ukupno {sym} u ponudi po ovoj cijeni kroz {count} oglas(a).",
      selectLevel: "Odaberite razinu cijene iznad da vidite ponude tamo.",
      paneHeader: "{size} {base} @ {price} {unit}",
      denomTip: "Prikazna jedinica za iznose {coin}",
      showMore: "Prikaži još {count}",
      showLess: "Prikaži najboljih {count}",
    },
  },
  relays: {
    title: "Relayi",
    subtitle: "Aktivna povezanost s vašim Nostr relayima — mrežom kojom putuju vaše ponude i preuzimanja. Dodajte ili uklonite relaye u Postavke → Mreža.",
    connectedCount: "{up} / {total} povezano",
    refresh: "Osvježi",
    ms: "{ms} ms",
    up: "aktivan",
    down: "nedostupan",
    statsTip: "{success}/{attempts} uspješnih povezivanja · ↓{down} ↑{up}",
    none: "Nema konfiguriranih relaya",
    noneBody: "Dodajte Nostr relay u Postavke → Mreža za objavu i primanje ponuda preko mreže.",
    goToNetwork: "Idi na Postavke",
    notConnected: "Nije povezano",
    notConnectedBody: "Prikaz relaya treba pokrenuti engine — prvo povežite trgovca.",
  },
  swaps: {
    maker: "Maker",
    taker: "Taker",
    title: "Swapovi",
    hint: "Vaša cjelovita knjiga — swapovi u tijeku na vrhu, dovršene trgovine ispod. Na aktivne swapove možete djelovati i s Corkboarda.",
    activeTitle: "U tijeku",
    historyTitle: "Povijest",
    none: "Još nema swapova — preuzmite ponudu na Corkboardu.",
    // Active-swaps dock action buttons + the refund-time label.
    cancel: "otkaži",
    dump: "ispiši logove",
    dumpHint: "Kopirajte dijagnostički paket bez tajni (stanje + retke loga) za ovaj swap, za prosljeđivanje developerima.",
    dumpCopied: "Dijagnostika kopirana — proslijedite developerima.",
    dumpFailed: "Nije moguće kopirati dijagnostički paket.",
    refundAt: "povrat {when}",
    // Confirm dialogs for acting on a live swap (ActiveSwaps).
    cancelTitle: "Otkazati ovaj swap?",
    cancelConfirm: "Otkaži swap",
    cancelKeep: "Zadrži ga",
    // Abort reason recorded on the swap when cancelled from Satchel.
    cancelReason: "otkazano u Satchelu",
    cancelBody:
      "Ovo napušta swap prije nego što ste financirali. Ništa vaše još nije zaključano, pa ne gubite ništa — ponuda se jednostavno neće dovršiti.",
    col: {
      swap: "swap",
      role: "uloga",
      state: "stanje",
      amounts: "daje → prima",
      when: "kada",
      finalTx: "konačna tx",
    },
    // Expandable per-leg on-chain detail (the audit trail). We show both
    // funding txs + OUR settlement; never the counterparty's settlement or the
    // swap secret.
    audit: {
      toggle: "Prikaži on-chain detalje",
      title: "On-chain detalji",
      youLocked: "vi ste zaključali",
      theyLocked: "oni su zaključali",
      funding: "Financiranje",
      received: "Primljeno",
      refunded: "Vraćeno",
      pending: "još nije on-chain",
      copy: "Kopiraj id transakcije",
      copied: "Id transakcije kopiran",
    },
  },
  fees: {
    title: "Pregled mrežnog troška",
    estimated: "procijenjeno",
    provisionalNote: "Ovaj pactd build još ne izlaže procjenu naknade.",
    summary: "Swap su 2 on-chain transakcije koje plaćate: financiranje na lancu koji dajete, otkup na lancu koji primate.",
    fallbackTip: "Čvor je bio nedostupan, pa je korišten konzervativan zadani feerate — shvatite ovo kao procjenu.",
    ifItStalls: "(ako zapne)",
  },
  funds: {
    insufficient:
      "Nedovoljno {sym} za financiranje ovog swapa — treba ~{need} {sym} (iznos + naknada za financiranje), novčanik ima {have} {sym}.",
  },
  wizard: {
    back: "Natrag",
    continue: "Nastavi",
  },
  // UI-4 docked activity log.
  log: {
    title: "Aktivnost",
    empty: "— zapisnik aktivnosti —",
    count: "{count} redaka",
    collapse: "Sažmi zapisnik",
    expand: "Proširi zapisnik",
    // Activity-log lines emitted by the frontend. `{err}` carries a raw engine
    // message (itself not translated — it comes from pactd over the wire).
    noTauri: "ne radi unutar Satchela — ovom sučelju treba Tauri most",
    startupError: "pokretanje: {err}",
    notConnected: "nije povezano: {err}",
    connected: "povezano s pactd {version} ({protocol})",
    listcoinsError: "listcoins: {err}",
    watchOnlyError: "samo za pregled: {err}",
    switchedMerchant: "prebačeno na trgovca {id}",
    renamedMerchant: "trgovac preimenovan u {name}",
    renameMerchantError: "preimenovanje trgovca: {err}",
    switchMerchantError: "zamjena trgovca: {err}",
    loadMerchantError: "učitavanje trgovca: {err}",
    merchantCreated: "trgovac {id} stvoren",
    merchantReady: "trgovac spreman",
    actionOk: "{action} {id}: ok",
    actionError: "{action} {id}: {err}",
    diagCopied: "dijagnostika za {id} kopirana ({count} redaka loga) — proslijedite developerima",
    dumpError: "ispis {id}: {err}",
    coinDisconnected: "{coin} odspojen",
    removeCoinError: "uklanjanje kovanice: {err}",
    tookOffer: "preuzeta ponuda {id} — sada se pojavljuje u vašim aktivnim swapovima ispod",
    takeError: "preuzimanje: {err}",
    offerWithdrawn: "ponuda {id} povučena",
    withdrawError: "povlačenje: {err}",
    postedOffer: "objavljena ponuda {id} — povucite bilo kada; ništa nije zaključano",
    createdSlip: "stvoren privatni slip ponude — pošaljite ga svom prijatelju",
    tookPrivateOffer: "preuzeta privatna ponuda {id} — sada se pojavljuje u vašim aktivnim swapovima",
    cancelledPrivateOffer: "otkazana privatna ponuda {id}",
    cancelError: "otkazivanje: {err}",
    noticeboardUpdated: "oglasna ploča ažurirana",
    feePolicyUpdated: "politika naknada ažurirana",
  },
  // Relative-time + freshness prose from format.ts (rendered via the tr() mirror,
  // since those are pure non-component helpers). Unit letters stay inside the
  // template so a translation owns the whole phrase.
  format: {
    ageUnknown: "starost nepoznata",
    justNow: "upravo sad",
    minutesAgo: "prije {n} min",
    hoursAgo: "prije {n} h",
    daysAgo: "prije {n} d",
    expiryNow: "sad",
    expirySoon: "uskoro",
    inMinutes: "za ~{n} min",
    inHours: "za ~{n} h",
    inDays: "za ~{n} d",
    posted: "objavljeno {age}",
    expires: "istječe {time}",
  },
  // Plain-language swap story per (role, state) — the honest "who is exposed
  // when" framing shown on every active swap. {a}/{b} are coin tickers; {t1}/{t2}
  // are local refund times. Rendered via tr() (narrate() is a pure helper).
  narrate: {
    finalizing: "Zatražili ste svoje {got} — završne potvrde. Držite aplikaciju otvorenom dok se ne zakopa; vaši {gave} ostaju zaštićeni do tada.",
    initiating:
      "Preuzimanje poslano — čeka se da maker pokrene swap. Ništa još nije zaključano; otkazuje se samo od sebe ako ne odgovori.",
    created: "Ponuda poslana — čeka se da druga strana pristane. Ništa nije obvezano.",
    acceptedMaker: "Uvjeti dogovoreni. Sljedeće: zaključajte svoj {a}. Dok ne financirate, još uvijek možete slobodno otkazati.",
    acceptedTaker: "Uvjeti dogovoreni. Druga strana prva zaključava svoj {a} — vi nikad ne šaljete prvi.",
    noncesExchanged:
      "Postavljanje privatnog swapa — razmjena materijala za potpisivanje. Ništa još nije zaključano.",
    signedMaker:
      "Obje strane potpisale i vaš {a} je zaključan. Vaš daemon automatski preuzima {b} čim druga strana zaključa i potvrdi svoju stranu. Ako išta zapne, vaš {a} se vraća u {t1}.",
    signedTaker:
      "Obje strane potpisale. Čim se njihov {a} potvrdi, vaš daemon zaključava vaš {b}, a zatim automatski preuzima {a}. Čim je vaš {b} zaključan, vraća se u {t2} ako išta zapne.",
    fundedAMaker:
      "Vaš {a} je zaključan. Čeka se da druga strana zaključa svoj {b}. Ako nikad ne zaključa, vaš {a} se automatski vraća u {t1}.",
    fundedATaker:
      "Njihov {a} je zaključan i potvrđen. Sljedeće: zaključajte svoj {b}. Sigurnosna mreža: automatski povrat u {t2} ako išta zapne.",
    fundedBMaker: "Oboje zaključano. Vaš daemon preuzima {b} čim bude sigurno potvrđen.",
    fundedBTaker: "Oboje zaključano. Vaš daemon će preuzeti {a} u trenutku kad druga strana preuzme svoj {b}.",
    completed: "Swap dovršen — {coin} je u vašem novčaniku.",
    refunded: "Swap nije dovršen, pa vam se {coin} automatski vratio. Ništa izgubljeno osim naknada.",
    aborted: "Otkazano prije nego što je išta novca pokrenuto.",
  },
  progress: {
    awaitingLock: "Čekanje na njihovo zaključavanje",
    awaitingClaim: "Čekanje na njihovo preuzimanje",
    theirLock: "Potvrđivanje njihovog zaključavanja",
    ourLock: "Potvrđivanje vašeg zaključavanja",
    securing: "Osiguravanje vaših {coin}",
    funding: "Zaključavanje vaših {coin} — otključajte novčanik ako zapne",
    blocks: "+{n} blokova",
    feeBumped: "Naknada povećana",
    reorg: "Otkrivena reorganizacija — ponovna provjera",
  },
  exit: {
    // Exit-gate dialog (fund safety, C6). The engine manages alone, so "keep
    // running" detaches it (it keeps watching timelocks + servicing offers).
    liveTitle: "Swap je u tijeku",
    liveBodyOne:
      "1 swap je usred tijeka. Njime upravljaju on-chain timelockovi — engine mora nastaviti raditi kako bi otkupio ili vratio prije roka.",
    liveBodyMany:
      "{count} swapova je usred tijeka. Njima upravljaju on-chain timelockovi — engine mora nastaviti raditi kako bi otkupio ili vratio prije roka.",
    keepRunningExplain:
      "Zatvaranjem prozora engine nastavlja raditi u pozadini, pa swap dovršava bez sučelja. Satchel možete ponovno otvoriti bilo kada da ga provjerite.",
    forceQuitWarn: "Prisilni izlaz sada zaustavlja engine i može izgubiti sredstva.",
    // {word} is the confirm word below; a translation may localize both together.
    typeToConfirm: "Za prisilni izlaz unatoč tome, upišite {word} ispod.",
    confirmWord: "QUIT",
    keepRunning: "Nastavi raditi, zatvori prozor",
    keepWithdraw: "Nastavi raditi + povuci ponude",
    keepLeaveOffers: "Nastavi raditi, ostavi ponude",
    forceQuit: "Prisilni izlaz",
    offersTitle: "Imate objavljene ponude",
    offersBodyOne:
      "1 vaša ponuda je još na Corkboardu. Ponude ništa ne zaključavaju, ali ako je ostavite, druge strane je i dalje mogu preuzeti dok je Satchel zatvoren — engine će obraditi preuzimanje.",
    offersBodyMany:
      "{count} vaših ponuda je još na Corkboardu. Ponude ništa ne zaključavaju, ali ako ih ostavite, druge strane ih i dalje mogu preuzeti dok je Satchel zatvoren — engine će obraditi preuzimanja.",
    withdrawExit: "Povuci sve i izađi",
  },
  unlock: {
    title: "Otključaj trgovca",
    body:
      "Seed ovog trgovca je šifriran. Unesite njegovu zaporku da ga otključate za ovu sesiju — Satchel ga drži samo u memoriji i zaboravlja pri izlasku.",
    switchMerchant: "Zamijeni trgovca",
    unlock: "Otključaj",
  },
  common: {
    cancel: "Odustani",
    confirm: "Potvrdi",
    save: "Spremi",
    done: "Gotovo",
    later: "Kasnije",
    retry: "Pokušaj ponovno povezivanje",
  },
};
