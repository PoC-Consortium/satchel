// The Finnish (Suomi) string bundle. Mirrors en.ts key-for-key; only values are translated.
import type { Bundle } from "./en";

export const fi: Bundle = {
  app: {
    name: "Satchel",
  },
  // In-app update notifications (sidebar version badge + dialog).
  update: {
    title: "Päivitys saatavilla",
    upToDate: "Olet ajan tasalla",
    current: "Asennettu",
    latest: "Uusin",
    notesTitle: "Julkaisutiedot",
    get: "Hae päivitys",
    dismiss: "Hylkää",
    close: "Sulje",
    badgeTooltip: "Päivitys saatavilla — napsauta nähdäksesi lisätiedot",
    versionTooltip: "Napsauta tarkistaaksesi päivitykset",
  },
  // Risk disclaimer (first-run welcome + Settings → About).
  disclaimer: {
    title: "Itsehallinta — sinun avaimesi, sinun vastuusi",
    body: "Satchel suorittaa ei-säilyttäviä atomic swapeja: vain sinä hallitset avaimiasi, ja kauppiaan seed pitää hallussaan kuumia siirtoavaimia swapin ollessa käynnissä. Swap-protokollat (v1 HTLC ja v2 Taproot/MuSig2) on katselmoitu ja ovat käytössä mainnetissä. MIT-lisensoitu ja tarjottu sellaisenaan ilman takuuta — varmuuskopioi palautuslauseesi ja käytä omalla vastuullasi.",
  },
  nav: {
    public: "Julkinen",
    corkboard: "Corkboard",
    postOffer: "Julkaise tarjous",
    private: "Yksityinen",
    privateCreate: "Luo lipuke",
    privateReceive: "Ota lipuke vastaan",
    privateSlips: "Omat lipukkeet",
    swaps: "Swapit",
    relays: "Releet",
    wallets: "Lompakot",
    contacts: "Contacts",
    settings: "Asetukset",
    coins: "Kolikot",
  },
  makeOffer: {
    title: "Julkaise tarjous",
    intro:
      "Julkaise allekirjoitettu tarjous Corkboardille. Mitään ei lukita — se on vain ilmoitus; vedä pois milloin tahansa, ja swap alkaa vasta kun joku ottaa sen ja molemmat osapuolet rahoittavat sen.",
    give: "Annat",
    want: "Saat",
    // Canonical pair + direction: pick the pair, choose Sell/Buy the base, enter
    // the base amount and a quote-per-base price (invariant to direction).
    pair: "Pari",
    noPairs: "Ei kaupattavia pareja — yhdistä vähintään kaksi kolikkoa kohdassa Asetukset → Kolikot.",
    sell: "Myy {sym}",
    buy: "Osta {sym}",
    amount: "Määrä",
    youGive: "Annat",
    youGet: "Saat",
    price: "Hinta",
    priceUnit: "{unit} per {base}",
    pricePlaceholder: "yksikköhinta",
    balance: "Saldo: {amt} {sym}",
    balanceLoading: "Saldo: …",
    noCoins: "Ei määritettyjä kolikoita",
    legDown: "Yksi näiden kolikoiden solmuista on alhaalla — käynnistä se (tai tarkista Asetukset → Kolikot) ennen julkaisua.",
    // Swap-protocol pin (only offered when a pair+network supports more than
    // one). v2 label reuses coins.protoPrivate ("Private (Taproot)").
    protocol: "Swap-tyyppi",
    protoStandard: "Vakio (HTLC)",
    // Titles for the review/confirm dialog shown before posting (see OfferForm).
    reviewOfferTitle: "Tarkista tarjouksesi",
    reviewSlipTitle: "Tarkista lipukkeesi",
    term: "Turvallisuus-timelock",
    termShort: "Lyhyt",
    termMedium: "Keskipitkä",
    termLong: "Pitkä",
    termHint: {
      short: "Lyhyt — varat palautetaan automaattisesti nopeimmin, jos kauppa jumiutuu (~12 h / 6 h), pienimmällä turvamarginaalilla.",
      medium: "Keskipitkä — tasapainoinen palautusikkuna (~24 h / 12 h).",
      long: "Pitkä (turvallisin) — laajin turvamarginaali; automaattinen palautus ~36 h / 18 h kuluttua, jos kauppa jumiutuu.",
    },
    validFor: "Voimassa (minuuttia)",
    validForMins: "{mins} min",
    validForHint:
      "Kuinka kauan tarjous pysyy listattuna. Kun olet linjoilla, se pidetään automaattisesti tuoreena; tämän jälkeen se vanhenee. Sovelluksen sulkeminen vetää sen pois.",
    note: "Kiinteäkokoinen tarjous — mitään ei lukita ennen kuin joku ottaa sen. Määrät ovat ketjussa; maksat verkkomaksut päälle ja Corkboard ei veloita mitään. Timelock on automaattinen palautusikkuna, jos swap jumiutuu.",
    post: "Julkaise tarjous",
    makeSlip: "Luo lipuke",
    slipTitle: "Yksityinen tarjouslipukkeesi",
    slipExplainer:
      "Lähetä tämä ystävällesi. He liittävät sen Satcheliin ottaakseen sen. Mitään ei lukita; se vanhenee {ttl} kuluttua.",
    copy: "Kopioi",
    copied: "Kopioitu",
    makeAnother: "Tee toinen",
    myPrivateTitle: "Omat yksityiset tarjoukset",
    myPrivateEmpty: "Ei avoimia yksityisiä tarjouksia.",
    privateExpires: "vanhenee {when}",
    privateExpired: "vanhentunut",
    cancel: "Peruuta",
    cancelTip: "Lopeta tämän lipukkeen kunnioittaminen — ystävä, jolla se yhä on, ei voi enää ottaa sitä.",
  },
  takeSlip: {
    intro:
      "Ystävä lähetti sinulle yksityisen tarjouslipukkeen (se alkaa pactoffer1:). Liitä se tähän tarkistaaksesi ja ottaaksesi sen — aivan kuten tarjouksen taululta.",
    placeholder: "pactoffer1:…",
    take: "Tarkista ja ota",
    invalid: "Tämä ei näytä lipukkeelta — sen pitäisi alkaa pactoffer1:.",
    previewLabel: "Tämä lipuke tarjoaa",
  },
  // PRIVATE nav group (off-market slips) — screen titles + intros. The form,
  // slip output, my-slips list and take flow reuse makeOffer.*/takeSlip.* copy.
  private: {
    createTitle: "Luo yksityinen tarjous",
    createIntro:
      "Rakenna allekirjoitettu tarjous ja anna se ystävälle lipukkeena omassa chatissasi. Mitään ei listata mihinkään — eikä mitään lukita ennen kuin molemmat rahoitatte.",
    slipsIntro:
      "Luomasi lipukkeet. Kuka tahansa, jolla on lipuke, voi ottaa sen ennen kuin se vanhenee; peruuta lipuke lopettaaksesi sen kunnioittamisen ennen sitä.",
    slipsEmptyBody: "Luo yksityinen tarjous saadaksesi lipukkeen, jonka voit lähettää ystävälle.",
    receiveTitle: "Ota yksityinen tarjous",
    received: "Otettu — seuraa sitä kohdassa Swapit.",
  },
  // Shared take-confirmation dialog (board take + slip take).
  takeConfirm: {
    title: "Otetaanko tämä tarjous?",
    confirm: "Ota tarjous",
    counterparty: "Vastapuoli",
    youGive: "Annat",
    youReceive: "Saat",
    safetyRefund: "Turvapalautus",
    offerAge: "Tarjouksen ikä",
    makerFundsFirst:
      "Tekijä lukitsee {sym} ensin — sinä et koskaan lähetä ensimmäisenä. Voit silti peruuttaa ennen kuin rahoitat oman osasi, ja moottori palauttaa varat automaattisesti turvallisuus-timelockin jälkeen, jos swap jumiutuu.",
  },
  header: {
    activeMerchant: "Aktiivinen kauppias — napsauta vaihtaaksesi tai hallitaksesi",
    manageMerchants: "Hallitse kauppiaita…",
    noMerchant: "ei kauppiasta",
    openMenu: "Avaa valikko",
    collapseMenu: "tiivistä valikko",
    settings: "Asetukset",
    language: "Kieli",
    pactConnected: "Moottori yhdistetty",
    pactUnreachable: "Moottoriin ei saada yhteyttä",
    liveSwapsOne: "1 swap käynnissä — napsauta nähdäksesi",
    liveSwapsMany: "{count} swapia käynnissä — napsauta nähdäksesi",
    liveSwapsNone: "Ei käynnissä olevia swapeja",
    coinOk: "{name} — yhdistetty · kärki {tip}",
    coinUnconfigured: "{name} — ei määritetty",
    coinError: "{name} — {status}",
    relaysOk: "Nostr-releet — {up}/{total} yhdistetty",
    relaysDown: "Nostr-releet — yhtään {total}:sta ei yhdistetty",
  },
  network: {
    mainnet: "MainNet",
    testnet: "TestNet",
    regtest: "RegTest",
    signet: "Signet",
    notRealFunds: "Ei oikeita varoja — tämä on {network}-verkko",
  },
  // Watch-only mode: a viewer session with no coins. Browse the board and
  // withdraw your own offers, but no posting / taking / funding.
  watchOnly: {
    badge: "Vain katselu",
    badgeTip:
      "Vain katselu -tila — selaa taulua ja vedä pois omat tarjouksesi, mutta et voi julkaista, ottaa tai rahoittaa. Määritä kolikot Asetuksissa kaupankäyntiä varten.",
    coinWizardButton: "Selaa vain katselu -tilassa",
    coinWizardHint:
      "Ohita kolikoiden määritys ja selaa vain taulua (vain luku). Voit silti vetää pois omat tarjouksesi — kätevää toisen istunnon jättämien tarjousten poistamiseen. Kytke se pois milloin tahansa Asetuksissa.",
    postBlockedTitle: "Vain katselu -tila",
    postBlockedBody:
      "Tämä on vain katselu -istunto, joten se ei voi julkaista tarjouksia. Määritä vähintään kaksi kolikkoa kohdassa Asetukset → Kolikot kaupankäyntiä varten.",
    takeBlockedBody: "Vain katselu -tila — voit tarkistaa tämän tarjouksen, mutta sen ottaminen vaatii määritetyt kolikot.",
    takeBlockedTip: "Vain katselu -tila — määritä kolikot Asetuksissa ottaaksesi tarjouksia.",
  },
  merchants: {
    title: "Kauppiaasi",
    intro:
      "Kauppias on yksi kaupankäynti-identiteetti — sen oma seed ja swap-historia. Kaupankäynti eri kauppiaalla pitää kontekstit linkittämättöminä (kertakäyttöinen identiteetti). Pääkolikkosi ovat omassa lompakossasi, eivät täällä.",
    // First-run welcome (empty merchant manager).
    welcomeTitle: "Tervetuloa Satcheliin",
    welcomeIntro:
      "Satchel käy kauppaa ”kauppiaan” kautta — yksi kaupankäynti-identiteetti omalla seedillään. Sinulla ei ole vielä yhtään: luo uusi tai tuo olemassa oleva palautuslause aloittaaksesi.",
    importMerchant: "Tuo kauppias",
    none: "Ei vielä kauppiaita.",
    switch: "vaihda",
    newMerchant: "Uusi kauppias",
    thisMerchant: "tämä kauppias",
    nameLabel: "Kauppiaan nimi",
    namePlaceholder: "esim. Pää",
    rename: "Nimeä uudelleen",
    introFirst:
      "Määritä ensimmäinen kaupankäynti-identiteettisi (”kauppias”). Se pitää hallussaan vain kuumia siirtoavaimia käynnissä oleviin swapeihin — pääkolikkosi pysyvät omassa lompakossasi.",
    introNew: "Uusi kauppias on tuore, erillinen identiteetti omalla seedillään ja swap-historiallaan.",
    // UI-5 merchant selector (phoenix wallet-selector parity).
    createNew: "Luo uusi",
    import: "Tuo",
    load: "Lataa kauppias",
    loaded: "ladattu",
    locked: "lukittu",
    lockedTip: "Salattu seed — avaa salasanallasi, kun lataat sen.",
    close: "Sulje",
    idLabel: "kansio",
    switching: "Vaihdetaan kauppiasta…",
    switchingBody: "Käynnistetään moottori uudelleen kyseistä kansiota vastaan.",
  },
  // Seed create/import (SeedForm) + counterparty + status.
  seed: {
    intro: "Luo aivan uusi seed tai tuo sellainen, joka sinulla jo on.",
    createNew: "Luo uusi",
    createDesc: "Generoi tuore seed. Sinä varmuuskopioit palautuslauseen.",
    import: "Tuo",
    importDesc: "Palauta olemassa olevasta 12/24-sanaisesta lauseesta.",
    recoveryLabel: "Palautuslause",
    encrypt: "Salaa",
    encryptDesc:
      "Salasana suojaa seedin levossa. Syötät sen kerran istuntoa kohden — Satchel ei koskaan tallenna sitä. Huom: valvomaton automaattinen palautus pysähtyy uudelleenkäynnistyksen jälkeen, kunnes syötät sen uudelleen.",
    noPassphrase: "Ei salasanaa (suositeltava)",
    noPassphraseDesc:
      "Automaattinen palautus toimii uudelleenkäynnistysten läpi ilman, että mitään tarvitsee syöttää — tämä on vain kuuma siirto-seed. Hinta: tiedosto-/isäntäkoneen käyttö paljastaa tämän kauppiaan siirtoavaimet + identiteetin.",
    passphraseLabel: "Salasana",
    passphrasePlaceholder: "valitse salasana",
    revealTitle: "Kirjoita palautuslauseesi muistiin",
    revealBody:
      "Kuka tahansa, jolla on nämä sanat, hallitsee tämän kauppiaan kuumia avaimia. Satchel ei säilytä kopiota — tallenna se offline-tilassa. Vahvistat seuraavaksi muutaman sanan.",
    ackLabel: "Olen kirjoittanut palautuslauseeni muistiin.",
    // Multi-step onboarding (create/import -> secret -> confirm -> passphrase).
    chooseTitle: "Määritä {label}",
    enterTitle: "Tuo palautuslauseesi",
    enterBody:
      "Kirjoita jokainen sana — ne täydentyvät automaattisesti — tai liitä koko lause. Tarkistamme sen ennen kuin jatkat.",
    wordCount: "{n} sanaa",
    wordAria: "Sana {n}",
    checkIncomplete: "Syötä kaikki {n} sanaa.",
    checkUnknown: "Jotkin sanat eivät ole BIP39-sanalistalla — tarkista korostetut.",
    checkBadChecksum: "Tarkistussumma ei täsmää — tarkista sanasi ja niiden järjestys.",
    checkOk: "Palautuslause näyttää kelvolliselta.",
    verifyTitle: "Vahvista varmuuskopiosi",
    verifyBody: "Kirjoita sanat näissä kohdissa vahvistaaksesi, että kirjoitit lauseen muistiin.",
    verifyWord: "Sana #{n}",
    verifyMismatch: "Nämä eivät täsmää lauseesi kanssa — tarkista varmuuskopiosi.",
    passphraseTitle: "Suojaa seed",
    passphraseBody:
      "Salaa valinnaisesti tallennettu seed salasanalla. Voit ohittaa tämän — katso kompromissi alta.",
  },
  counterparty: {
    you: "Tämä olet sinä",
    youShort: "sinä",
    unknown: "tuntematon identiteetti",
    // Short fingerprint fallback (identity.ts shortId) when no pubkey is known.
    unknownShort: "tuntematon",
  },
  contacts: {
    // TODO(i18n): translate — English fallback for now.
    title: "Contacts",
    subtitle: "Your private nicknames for the people you trade with.",
    privacyNote:
      "Contacts are stored only on this device and are never shared, published, or sent to a relay. A nickname is your label — the identicon and fingerprint remain the real identity.",
    searchPlaceholder: "Search nick, note, or key",
    empty: "No contacts yet. Click a counterparty's identicon anywhere to add one.",
    emptyFiltered: "No contacts match this filter.",
    count: "{n} contacts",
    colWho: "Identity",
    colNick: "Nickname",
    colNote: "Notes",
    colStatus: "Standing",
    colAdded: "Added",
    colActions: "",
    filterAll: "All",
    filterTrusted: "Trusted",
    filterBlocked: "Blocked",
    // Corkboard toggle: drop blocked makers' offers from the ladder.
    hideBlocked: "Hide blocked offers",
    statusTrusted: "Trusted",
    statusNeutral: "Neutral",
    statusBlocked: "Blocked",
    menuAdd: "Add to contacts…",
    menuEdit: "Edit contact…",
    menuMarkTrusted: "Mark as trusted",
    menuMarkNeutral: "Mark as neutral",
    menuMarkBlocked: "Block",
    menuCopyKey: "Copy public key",
    menuOpen: "Open in Contacts",
    keyCopied: "Public key copied",
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
    blockedWarning: "You blocked this counterparty",
    blockedWarningBody:
      "You marked this person as blocked. Blocking is only a personal reminder — it does not stop the trade. Continue only if you mean to.",
  },
  status: {
    notConnectedTitle: "Ei yhteyttä moottoriin",
    disconnectedBody:
      "Satchel ei tavoita moottoria. Se voi olla vielä käynnistymässä, tai aktiivisen kauppiaan solmuyhteydet voivat olla alhaalla. Yritä uudelleen tai vaihda kauppiasta yläreunan valitsimesta.",
    openInSatchel: "Avaa tämä Satchelissa",
    noTauriBody:
      "Tämä on Satchelin käyttöliittymä — se tarvitsee Tauri-sillan tavoittaakseen moottorin. Käynnistä työpöytäsovellus (cargo tauri dev) selaimen sijaan.",
  },
  settings: {
    title: "Asetukset",
    subtitle: "Sovelluksen laajuiset asetukset tälle asennukselle.",
    // UI-3 Settings tabs.
    tabGeneral: "Yleiset",
    tabCoins: "Kolikot",
    tabNetwork: "Verkko",
    tabAbout: "Tietoja",
    appearance: "Ulkoasu",
    theme: "Teema",
    themeDark: "Tumma",
    themeLight: "Vaalea",
    themeSystem: "Järjestelmä",
    themeHint: "Valitse, miltä Satchel näyttää. Järjestelmä seuraa käyttöjärjestelmäsi asetusta.",
    language: "Kieli",
    languageHint: "Lisää kieliä saapuu, kun käännöksiä lahjoitetaan.",
    mode: "Tila",
    watchOnly: "Vain katselu -tila",
    watchOnlyHint:
      "Selaa taulua määrittämättä kolikoita. Voit silti vetää pois omat tarjouksesi, mutta et voi julkaista, ottaa tai rahoittaa. Kytke pois käydäksesi kauppaa (tarvitset vähintään kaksi yhdistettyä kolikkoa).",
    network: "Verkko",
    boards: "Corkboardit",
    boardsDesc:
      "Valinnaiset itse isännöidyt HTTP-taulut. Lisää mitä tahansa, joihin luotat; jätä tyhjäksi luottaaksesi Nostriin.",
    boardsNone: "Ei määritettyjä",
    nostrRelays: "Nostr-releet",
    nostrRelaysDesc:
      "Releet kuljettavat ilmoitustaulua hajautetun verkon yli — kukaan operaattori ei voi lukea tai sovittaa tarjouksiasi. Esiasennettu oletusjoukolla; muokkaa vapaasti.",
    nostrRelaysOff: "Pois — Nostr-kuljetus poistettu käytöstä",
    addUrl: "Lisää",
    removeUrl: "Poista",
    relayInvalid: "Syötä ws:// tai wss:// relee-URL",
    boardInvalid: "Syötä http:// tai https:// taulu-URL",
    netSave: "Tallenna ja yhdistä uudelleen",
    netSaving: "Tallennetaan ja yhdistetään uudelleen…",
    netSaved: "Tallennettu",
    // Fees tab — fee-bump policy (per active merchant).
    tabFees: "Maksut",
    fees: "Maksun korotus",
    feesScope: "Nämä asetukset koskevat aktiivista kauppiasta.",
    feesIntro:
      "Turvallisuus-/kustannuskompromissit maksun korotuksille, ei pakollinen määritys. Uudet arvot koskevat tulevia korotuksia; jo rahoitetut swapit säilyttävät käytännön, jolla ne rahoitettiin.",
    feeMax: "Maksimimaksu (sat/vB)",
    feeMaxHint:
      "Katto jokaiselle maksun korotukselle. Oletus 500, myös järjestelmän kova maksimi. Laske sitä rajataksesi kustannuksia.",
    feeReservation: "Rahoituskorotuksen varaus (×)",
    feeReservationHint:
      "Saldo, jonka varojen tarkistus varaa korotuspuskuriksi. Korkeampi pelastaa suuremmista maksupiikeistä, mutta sitoo enemmän saldoa ja hylkää enemmän swapeja. Oletus 3.",
    feeCommitted: "Lunastuksen ylimitoitus (×)",
    feeCommittedHint:
      "Kuinka paljon ylimääräistä v2-lunastusmaksu maksetaan etukäteen, jotta se vahvistuu silloinkin kun Satchel on suljettu. Koskee vain uusia swapeja. Oletus 2.",
    feeSave: "Tallenna",
    feeSaving: "Tallennetaan…",
    feeSaved: "Tallennettu",
    feeReset: "Palauta oletukset",
    coins: "Kolikot ja solmut",
    coinsHint: "Yhdistä jokainen kolikko omaan solmuusi. Genesis tarkistetaan ennen kuin mitään tallennetaan.",
    about: "Tietoja",
    version: "Versio {version}",
    updateUpToDate: "Ajan tasalla",
    updateCheckPlaceholder: "Päivitystarkistus saapuu myöhemmässä julkaisussa.",
    trustModel: "Missä avaimesi sijaitsevat",
    trustModelBody:
      "Salaisuudet sijaitsevat moottorissa, ei koskaan Satchelissa. Kauppiaan seed sijaitsee moottorin datakansiossa (salattuna tai selkokielisenä — sinun valintasi); Satchel ei tallenna seediä tai salasanaa. Seed on suunnittelultaan kuuma (vain siirtoavaimet) — pyyhkäise huomattavat tuotot omaan kylmälompakkoosi.",
  },
  coins: {
    intro:
      "Yhdistä jokainen kolikko omaan solmuusi. Ensimmäinen URL on solmusi oma lompakko — se rahoittaa swap-osasi ja vastaanottaa tuotot. Ennen kuin mitään tallennetaan, Satchel tarkistaa solmun genesis-lohkon, jotta varoja ei voida koskaan lähettää väärään ketjuun. Yhteydet jaetaan kaikkien kauppiaidesi kesken.",
    networkBadge: "Määritetään {network}-verkolle",
    needMerchant:
      "Yhdistä ensin kauppias — kolikoiden määritys vaatii moottorin käynnissä. Käytä kauppiasvalitsinta oikeassa yläkulmassa.",
    pairsTitle: "Kaupankäyntiparit",
    pairsHint:
      "Parit johdetaan siitä, mitä kukin kolikko osaa — ei ole kiinteää listaa. Pari avautuu, kun molemmat sen kolikot on yhdistetty.",
    noPairs: "Ei pareja saatavilla.",
    notSetUp: "Ei määritetty",
    connectedTip: "Yhdistetty · kärki {tip}",
    connError: "Yhteysvirhe",
    setUp: "Määritä",
    editConnection: "Muokkaa yhteyttä",
    remove: "poista",
    disconnectTip: "Katkaise tämän kolikon yhteys",
    disconnectTitle: "Katkaistaanko {coin}:n yhteys?",
    disconnectBody: "Sitä tarvitsevat swapit eivät ole käytettävissä, ennen kuin yhdistät uudelleen.",
    ready: "Valmis kaupankäyntiin",
    connectMissing: "Yhdistä {coins}",
    notBuildable: "Ei vielä rakennettavissa",
    // Swap-protocol chips on a pair (pact-htlc-v1 HTLC vs pact-htlc-v2 adaptor).
    protoPrivate: "Yksityinen (Taproot)",
    protoPrivateTip: "Yksityinen swap (Taproot/MuSig2-adapteri) — näyttää ketjussa tavalliselta maksulta",
    protoHtlcTip: "Klassinen HTLC-swap",
    // Coin-setup backend choices (CoinSetup).
    // CoinSetup dialog.
    setupTitle: "Yhdistä {coin}",
    setupIntro:
      "Osoita Satchel omaan {sym}-solmuusi. Mitään ei tallenneta ennen kuin solmu läpäisee genesis-lohkon tarkistuksen — varasi koskettavat vain oikeaa {sym}-ketjua.",
    confirmationsLabel: "Vahvistuksia ennen lopullisuutta",
    confirmationsHint:
      "Kuinka syvällä rahoituksen tai lunastuksen tässä ketjussa on oltava ennen kuin swap toimii sen perusteella — reorg-turvamarginaali. Korkeampi on turvallisempi mutta hitaampi; jätä tyhjäksi suositellulle oletukselle ({default}).",
    validateNode: "Vahvista solmu",
    checking: "Tarkistetaan solmua…",
    genesisOk: "Genesis täsmäsi — tämä on oikea ketju",
    genesisDetail: "kärjen korkeus {tip} · genesis {hash}…",
    genesisBad: "Hylätty — ei tallenneta",
    errorShort: "virhe",
    // Structured connection form (CoinSetup v2).
    rpcHostLabel: "RPC-isäntä",
    rpcPortLabel: "RPC-portti",
    authMethodLabel: "Todennus",
    authCookie: "Cookie-tiedosto",
    authCookieDesc: "Lue solmun .cookie automaattisesti sen datakansiosta (oletus, salasanaa ei tallenneta).",
    authUserPass: "Käyttäjä / salasana",
    authUserPassDesc: "rpcuser / rpcpassword solmusi konfiguraatiosta — tarvitaan etäsolmulle.",
    rpcUserLabel: "RPC-käyttäjätunnus",
    rpcPasswordLabel: "RPC-salasana",
    datadirLabel: "Solmun datakansio",
    cookiePathNote: "Cookie luetaan polusta {path} tämän kansion alta.",
    walletLabel: "Lompakon nimi (valinnainen)",
    walletPlaceholder: "solmusi lompakko",
    needPort: "Syötä ensin RPC-portti.",
    validateFirst: "Vahvista solmu ennen tallennusta.",
    savingReconnecting: "Tallennetaan ja yhdistetään uudelleen…",
    connected: "{coin} yhdistetty",
    // Template picker (a coins.toml coin the engine version doesn't support).
    unsupportedByEngine: "Ei tuettu",
    unsupportedByEngineTip:
      "Tämä kolikko on määritelty coins.toml-tiedostossa, mutta sitä ei ole sisäänrakennettu tähän moottorin versioon, joten sillä ei voi käydä kauppaa.",
  },
  coinWizard: {
    title: "Yhdistä kolikkosi",
    intro:
      "Valitse vähintään kaksi kolikkoa ja osoita kukin omaan solmuusi. Swap tarvitsee kaksi ketjua, joten kaupankäynti avautuu, kun kaksi solmua on yhdistetty ja toiminnassa. Voit lisätä tai muuttaa kolikoita myöhemmin Asetuksissa.",
    progress: "{count} / {min} kolikkoa yhdistetty",
    continue: "Jatka",
    live: "Toiminnassa",
    nodeDown: "Solmu alhaalla",
  },
  wallets: {
    intro:
      "Nämä ovat omien solmujesi lompakoita (ne joita moottori käyttää swapien rahoittamiseen ja tuottojen vastaanottamiseen) — sinun avaimesi, sinun koneesi. Satchel ei koskaan pidä hallussaan kolikoitasi.",
    hotSeedNudge:
      "Tämä on käyttölompakko kuumalla seedillä, ei holvi — pyyhkäise huomattavat saldot omaan kylmä-/core-lompakkoosi.",
    notConnected: "Ei yhdistetty",
    notConnectedBody: "Yhdistä ensin kauppias — lompakkonäkymä vaatii moottorin käynnissä.",
    noCoins: "Ei vielä määritettyjä kolikoita",
    noCoinsBody: "Yhdistä kolikko kohdassa Asetukset → Kolikot ja sen lompakko ilmestyy tähän.",
    goToCoins: "Siirry Kolikoihin",
    watchOnlyTitle: "Ei lompakoita vain katselu -tilassa",
    watchOnlyBody:
      "Tämä on vain katselu -istunto ilman yhdistettyjä kolikoita, joten näytettäviä lompakoita ei ole. Kytke vain katselu pois Asetuksissa ja yhdistä kolikko rahoittaaksesi swapeja.",
    walletName: "lompakko · {wallet}",
    walletScopedHint: "Jokainen tämän kolikon RPC on rajattu tähän solmun lompakkoon.",
    walletDefault: "oletuslompakko (ei rajattu)",
    walletDefaultHint:
      "Tälle kolikolle ei ole asetettu lompakkoa, joten RPC:t käyttävät solmun oletuslompakkoa. Aseta sellainen kohdassa Asetukset → Kolikot rajataksesi jokaisen kutsun tiettyyn lompakkoon.",
    balanceLabel: "{symbol}-saldo",
  },
  corkboard: {
    noBoardTitle: "Ei yhdistettyä Corkboardia",
    noBoardBody:
      "Corkboard on jaettu ilmoitustaulu, jolle tekijät kiinnittävät tarjouksia. Se ei koskaan sovita kauppoja tai pidä hallussaan kolikoita — osoita Satchel sellaiseen, johon luotat, selataksesi ja julkaistaksesi.",
    noPairs: "Ei pareja saatavilla",
    board: "Corkboard",
    boardSettings: "Määritä Asetuksissa",
    filterAll: "Kaikki",
    filterMine: "Omat",
    noOffers: "Ei tarjouksia, jotka voisit ottaa juuri nyt",
    noOffersBody:
      "Tarjoukset ilmestyvät tähän heti, kun tekijä julkaisee sellaisen parille, jonka olet määrittänyt. Voit myös julkaista omasi.",
    yourOffer: "tarjouksesi",
    offerStaged: "julkaistaan…",
    offerStagedTip:
      "Julkaistu tältä laitteelta ja odottaa vahvistusta releeltä. Se mainostaa; siitä tulee aktiivinen, kun relee toistaa sen.",
    take: "Ota tarjous",
    legDown: "Yksi tämän parin solmuista on alhaalla — käynnistä se (tai tarkista Asetukset → Kolikot) ennen ottamista.",
    withdraw: "Vedä pois",
    withdrawTip: "Vedä pois välittömästi — tarjous ei koskaan lukitse varoja",
    safetyRefund: "turvapalautus",
    safetyRefundTip:
      "Jos swap jumiutuu, molemmat osapuolet palauttavat automaattisesti — ottajan osa avautuu ensin, sinun hieman myöhemmin. Kukaan ei jää jumiin.",
    activeTitle: "Aktiiviset swapisi",
    states: {
      takenByUs: "ottamasi",
      revoked: "vedetty pois",
      expired: "vanhentunut",
    },
    // Two-sided order-book view of the Corkboard. Bids = makers giving the
    // quote coin to get the base; asks = the reverse. The ladder is a way to
    // READ the board — it never matches or prioritises (load-bearing).
    book: {
      bids: "Ostotarjoukset",
      asks: "Myyntitarjoukset",
      bidsHint: "halutaan {base} · maksetaan {quote}",
      asksHint: "myydään {base} · hintaan {quote}",
      price: "Hinta",
      size: "Koko",
      noBids: "Ei ostotarjouksia",
      noAsks: "Ei myyntitarjouksia",
      spread: "Spreadi {pct}",
      spreadOneSided: "Yksipuolinen",
      crossed: "risteävä",
      crossedTip: "Korkein osto ≥ matalin myynti. Taulu ei koskaan sovita automaattisesti, joten nämä päällekkäiset tarjoukset vain odottavat — ota kumpi puoli tahansa.",
      mid: "keskihinta {price}",
      levelOffers: "{count} tarjousta tällä hinnalla — valitse yksi otettavaksi",
      depthTip: "Yhteensä {sym} tarjolla tällä hinnalla {count} ilmoituksen yli.",
      selectLevel: "Valitse hintataso yllä nähdäksesi siellä olevat tarjoukset.",
      paneHeader: "{size} {base} @ {price} {unit}",
      denomTip: "{coin}-määrien näyttöyksikkö",
      showMore: "Näytä {count} lisää",
      showLess: "Näytä ylin {count}",
    },
  },
  relays: {
    title: "Releet",
    subtitle: "Reaaliaikainen yhteys Nostr-releisiisi — verkko, jonka yli tarjouksesi ja ottosi kulkevat. Lisää tai poista releitä kohdassa Asetukset → Verkko.",
    connectedCount: "{up} / {total} yhdistetty",
    refresh: "Päivitä",
    ms: "{ms} ms",
    up: "ylhäällä",
    down: "alhaalla",
    statsTip: "{success}/{attempts} onnistunutta yhteyttä · ↓{down} ↑{up}",
    none: "Ei määritettyjä releitä",
    noneBody: "Lisää Nostr-relee kohdassa Asetukset → Verkko julkaistaksesi ja vastaanottaaksesi tarjouksia verkon yli.",
    goToNetwork: "Siirry Asetuksiin",
    notConnected: "Ei yhdistetty",
    notConnectedBody: "Releenäkymä vaatii moottorin käynnissä — yhdistä ensin kauppias.",
  },
  swaps: {
    maker: "Maker",
    taker: "Taker",
    title: "Swapit",
    hint: "Koko kirjanpitosi — käynnissä olevat swapit ylhäällä, valmiit kaupat alla. Voit myös toimia aktiivisten swapien suhteen Corkboardilta.",
    activeTitle: "Käynnissä",
    historyTitle: "Historia",
    none: "Ei vielä swapeja — ota tarjous Corkboardilta.",
    // Active-swaps dock action buttons + the refund-time label.
    cancel: "peruuta",
    refund: "palauta",
    dump: "vedosta lokit",
    dumpHint: "Kopioi salaisuudeton diagnostiikkapaketti (tila + lokirivit) tälle swapille kehittäjille liitettäväksi.",
    dumpCopied: "Diagnostiikka kopioitu — liitä kehittäjille.",
    dumpFailed: "Diagnostiikkapakettia ei voitu kopioida.",
    refundAt: "palautus {when}",
    // Confirm dialogs for acting on a live swap (ActiveSwaps).
    cancelTitle: "Peruutetaanko tämä swap?",
    cancelConfirm: "Peruuta swap",
    cancelKeep: "Pidä se",
    // Abort reason recorded on the swap when cancelled from Satchel.
    cancelReason: "peruutettu Satchelissa",
    cancelBody:
      "Tämä hylkää swapin ennen kuin olet rahoittanut. Mitään sinun ei ole vielä lukittu, joten et menetä mitään — tarjous ei vain valmistu.",
    refundTitle: "Vedetäänkö varasi takaisin?",
    refundConfirm: "Palauta",
    refundBody:
      "Turvallisuus-timelock on kulunut, joten voit lunastaa lukitsemasi varat takaisin. Tämä lähettää palautuksesi nyt; moottori tekee sen myös automaattisesti määräajan jälkeen.",
    col: {
      swap: "swap",
      role: "rooli",
      state: "tila",
      amounts: "antaa → saa",
      when: "milloin",
      finalTx: "lopullinen tx",
    },
    // Expandable per-leg on-chain detail (the audit trail). We show both
    // funding txs + OUR settlement; never the counterparty's settlement or the
    // swap secret.
    audit: {
      toggle: "Näytä ketjun tiedot",
      title: "Ketjun tiedot",
      youLocked: "lukitsit",
      theyLocked: "he lukitsivat",
      funding: "Rahoitus",
      received: "Vastaanotettu",
      refunded: "Palautettu",
      pending: "ei vielä ketjussa",
      copy: "Kopioi transaktiotunnus",
      copied: "Transaktiotunnus kopioitu",
    },
  },
  fees: {
    title: "Verkkokustannuksen esikatselu",
    estimated: "arvioitu",
    provisionalNote: "Tämä pactd-koontiversio ei vielä paljasta maksuarviointia.",
    summary: "Swap on 2 ketjutransaktiota, jotka maksat: rahoitus anto-ketjussa, lunastus saanti-ketjussa.",
    fallbackTip: "Solmuun ei saatu yhteyttä, joten käytettiin varovaista oletusmaksutasoa — kohtele näitä arvauksena.",
    ifItStalls: "(jos se jumiutuu)",
  },
  funds: {
    insufficient:
      "Ei tarpeeksi {sym} tämän swapin rahoittamiseen — tarvitaan ~{need} {sym} (määrä + rahoitusmaksu), lompakossa on {have} {sym}.",
  },
  wizard: {
    back: "Takaisin",
    continue: "Jatka",
  },
  // UI-4 docked activity log.
  log: {
    title: "Toiminta",
    empty: "— toimintaloki —",
    count: "{count} riviä",
    collapse: "Tiivistä loki",
    expand: "Laajenna loki",
    // Activity-log lines emitted by the frontend. `{err}` carries a raw engine
    // message (itself not translated — it comes from pactd over the wire).
    noTauri: "ei käynnissä Satchelin sisällä — tämä käyttöliittymä tarvitsee Tauri-sillan",
    startupError: "käynnistys: {err}",
    notConnected: "ei yhdistetty: {err}",
    connected: "yhdistetty pactd {version} ({protocol})",
    listcoinsError: "listcoins: {err}",
    watchOnlyError: "vain katselu: {err}",
    switchedMerchant: "vaihdettu kauppiaaseen {id}",
    renamedMerchant: "kauppias nimetty uudelleen: {name}",
    renameMerchantError: "nimeä kauppias uudelleen: {err}",
    switchMerchantError: "vaihda kauppias: {err}",
    loadMerchantError: "lataa kauppias: {err}",
    merchantCreated: "kauppias {id} luotu",
    merchantReady: "kauppias valmis",
    actionOk: "{action} {id}: ok",
    actionError: "{action} {id}: {err}",
    diagCopied: "diagnostiikka {id}:lle kopioitu ({count} lokiriviä) — liitä kehittäjille",
    dumpError: "vedos {id}: {err}",
    coinDisconnected: "{coin} yhteys katkaistu",
    removeCoinError: "poista kolikko: {err}",
    tookOffer: "otettu tarjous {id} — se näkyy nyt aktiivisissa swapeissasi alla",
    takeError: "ota: {err}",
    offerWithdrawn: "tarjous {id} vedetty pois",
    withdrawError: "vedä pois: {err}",
    postedOffer: "julkaistu tarjous {id} — vedä pois milloin tahansa; mitään ei lukita",
    createdSlip: "luotu yksityinen tarjouslipuke — lähetä se ystävällesi",
    tookPrivateOffer: "otettu yksityinen tarjous {id} — se näkyy nyt aktiivisissa swapeissasi",
    cancelledPrivateOffer: "peruutettu yksityinen tarjous {id}",
    cancelError: "peruuta: {err}",
    noticeboardUpdated: "ilmoitustaulu päivitetty",
    feePolicyUpdated: "maksukäytäntö päivitetty",
  },
  // Relative-time + freshness prose from format.ts (rendered via the tr() mirror,
  // since those are pure non-component helpers). Unit letters stay inside the
  // template so a translation owns the whole phrase.
  format: {
    ageUnknown: "ikä tuntematon",
    justNow: "juuri nyt",
    minutesAgo: "{n} min sitten",
    hoursAgo: "{n} h sitten",
    daysAgo: "{n} pv sitten",
    expiryNow: "nyt",
    expirySoon: "pian",
    inMinutes: "~{n} min kuluttua",
    inHours: "~{n} h kuluttua",
    inDays: "~{n} pv kuluttua",
    posted: "julkaistu {age}",
    expires: "vanhenee {time}",
  },
  // Plain-language swap story per (role, state) — the honest "who is exposed
  // when" framing shown on every active swap. {a}/{b} are coin tickers; {t1}/{t2}
  // are local refund times. Rendered via tr() (narrate() is a pure helper).
  narrate: {
    finalizing: "Lunastit {got} — viimeiset vahvistukset. Pidä sovellus auki, kunnes se hautautuu; {gave} pysyvät suojattuina siihen asti.",
    initiating:
      "Otto lähetetty — odotetaan tekijää aloittamaan swapin. Mitään ei vielä lukita; se peruuntuu itsestään, jos he eivät vastaa.",
    created: "Tarjous lähetetty — odotetaan toista osapuolta hyväksymään. Mitään ei ole sidottu.",
    acceptedMaker: "Ehdot sovittu. Seuraavaksi: lukitse {a}. Kunnes rahoitat, voit silti peruuttaa vapaasti.",
    acceptedTaker: "Ehdot sovittu. Toinen osapuoli lukitsee {a} ensin — sinä et koskaan lähetä ensimmäisenä.",
    noncesExchanged:
      "Määritetään yksityistä swapia — vaihdetaan allekirjoitusmateriaalia. Mitään ei vielä lukita.",
    signedMaker:
      "Molemmat osapuolet allekirjoittivat. Daemonisi lukitsee {a}, sitten lunastaa {b} automaattisesti. Jos jokin jumiutuu, {a} palautuu ajankohtana {t1}.",
    signedTaker:
      "Molemmat osapuolet allekirjoittivat. Daemonisi lukitsee {b} ja lunastaa {a} heti kun toinen osapuoli toimii. Turvaverkko: palautus ajankohtana {t2}.",
    fundedAMaker:
      "{a} on lukittu. Odotetaan toista osapuolta lukitsemaan {b}. Jos he eivät koskaan tee niin, {a} palautuu automaattisesti ajankohtana {t1}.",
    fundedATaker:
      "Heidän {a} on lukittu ja vahvistettu. Seuraavaksi: lukitse {b}. Turvaverkko: automaattinen palautus ajankohtana {t2}, jos jokin jumiutuu.",
    fundedBMaker: "Molemmat lukittu. Daemonisi lunastaa {b} heti kun se on turvallisesti vahvistettu.",
    fundedBTaker: "Molemmat lukittu. Daemonisi lunastaa {a} sillä hetkellä kun toinen osapuoli ottaa {b}.",
    completed: "Swap valmis — {coin} on lompakossasi.",
    refunded: "Swap ei valmistunut, joten {coin} palautui automaattisesti. Mitään ei menetetty paitsi maksut.",
    aborted: "Peruutettu ennen kuin mitään rahaa liikkui.",
  },
  progress: {
    awaitingLock: "Odotetaan heidän lukitustaan",
    awaitingClaim: "Odotetaan heidän lunastustaan",
    theirLock: "Vahvistetaan heidän lukitustaan",
    ourLock: "Vahvistetaan sinun lukitustasi",
    securing: "Turvataan {coin}",
    blocks: "+{n} lohkoa",
    feeBumped: "Maksua korotettu",
    reorg: "Reorg havaittu — tarkistetaan uudelleen",
  },
  exit: {
    // Exit-gate dialog (fund safety, C6). The engine manages alone, so "keep
    // running" detaches it (it keeps watching timelocks + servicing offers).
    liveTitle: "Swap on käynnissä",
    liveBodyOne:
      "1 swap on kesken. Sitä hallitsevat ketjun timelockit — moottorin on pysyttävä käynnissä lunastaakseen tai palauttaakseen ennen määräaikaa.",
    liveBodyMany:
      "{count} swapia on kesken. Niitä hallitsevat ketjun timelockit — moottorin on pysyttävä käynnissä lunastaakseen tai palauttaakseen ennen määräaikaa.",
    keepRunningExplain:
      "Ikkunan sulkeminen pitää moottorin käynnissä taustalla, joten se viimeistelee swapin ilman käyttöliittymää. Voit avata Satchelin uudelleen milloin tahansa tarkistaaksesi sen.",
    forceQuitWarn: "Pakkolopetus nyt pysäyttää moottorin ja voi menettää varoja.",
    // {word} is the confirm word below; a translation may localize both together.
    typeToConfirm: "Pakkolopettaaksesi silti, kirjoita {word} alle.",
    confirmWord: "QUIT",
    keepRunning: "Pidä käynnissä, sulje ikkuna",
    keepWithdraw: "Pidä käynnissä + vedä tarjoukset pois",
    keepLeaveOffers: "Pidä käynnissä, jätä tarjoukset esille",
    forceQuit: "Pakkolopeta",
    offersTitle: "Sinulla on julkaistuja tarjouksia",
    offersBodyOne:
      "1 tarjouksesi on yhä Corkboardilla. Tarjoukset eivät lukitse mitään, mutta sen esillä jättäminen tarkoittaa, että vastapuolet voivat silti ottaa sen, kun Satchel on suljettu — moottori palvelee oton.",
    offersBodyMany:
      "{count} tarjoustasi on yhä Corkboardilla. Tarjoukset eivät lukitse mitään, mutta niiden esillä jättäminen tarkoittaa, että vastapuolet voivat silti ottaa ne, kun Satchel on suljettu — moottori palvelee otot.",
    withdrawExit: "Vedä kaikki pois ja poistu",
  },
  unlock: {
    title: "Avaa kauppias",
    body:
      "Tämän kauppiaan seed on salattu. Syötä sen salasana avataksesi sen tälle istunnolle — Satchel pitää sen vain muistissa ja unohtaa sen poistuttaessa.",
    switchMerchant: "Vaihda kauppias",
    unlock: "Avaa",
  },
  common: {
    cancel: "Peruuta",
    confirm: "Vahvista",
    save: "Tallenna",
    done: "Valmis",
    later: "Myöhemmin",
    retry: "Yritä yhteyttä uudelleen",
  },
};
