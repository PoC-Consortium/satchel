// The Swahili (Kiswahili) string bundle. Mirrors en.ts key-for-key; only values are translated.
import type { Bundle } from "./en";

export const sw: Bundle = {
  app: {
    name: "Satchel",
    tagline: "swap zisizo na uaminifu wa tatu",
  },
  // In-app update notifications (sidebar version badge + dialog).
  update: {
    title: "Sasisho linapatikana",
    upToDate: "Uko na toleo jipya zaidi",
    current: "Lililosakinishwa",
    latest: "Jipya zaidi",
    notesTitle: "Maelezo ya toleo",
    get: "Pata sasisho",
    dismiss: "Ondoa",
    close: "Funga",
    badgeTooltip: "Sasisho linapatikana — bofya kwa maelezo",
    versionTooltip: "Bofya kuangalia masasisho",
  },
  // Risk disclaimer (first-run welcome + Settings → About).
  disclaimer: {
    title: "Kujihifadhi mwenyewe — funguo zako, jukumu lako",
    body: "Satchel hufanya swap za atomiki zisizo na ulinzi wa tatu: wewe pekee ndiye unayeshikilia funguo zako, na mbegu ya mfanyabiashara hushikilia funguo za usafiri za muda wakati swap inaendelea. Itifaki za swap (v1 HTLC na v2 Taproot/MuSig2) zimekaguliwa na zinafanya kazi kwenye MainNet. Zina leseni ya MIT na zinatolewa kama zilivyo, bila dhamana yoyote — hifadhi nakala ya kifungu chako cha kurejesha na zitumie kwa hatari yako mwenyewe.",
  },
  nav: {
    public: "Hadharani",
    corkboard: "Corkboard",
    postOffer: "Chapisha ofa",
    private: "Faragha",
    privateCreate: "Tengeneza karatasi",
    privateReceive: "Chukua karatasi",
    privateSlips: "Karatasi zangu",
    swaps: "Swap",
    relays: "Relays",
    wallets: "Pochi",
    settings: "Mipangilio",
    coins: "Sarafu",
  },
  makeOffer: {
    title: "Chapisha ofa",
    intro:
      "Chapisha ofa iliyotiwa saini kwenye Corkboard. Hakuna kinachofungwa — ni tangazo tu; ondoa wakati wowote, na swap huanza tu mtu anapoichukua na pande zote mbili zifadhili.",
    give: "Unatoa",
    want: "Unapokea",
    // Canonical pair + direction: pick the pair, choose Sell/Buy the base, enter
    // the base amount and a quote-per-base price (invariant to direction).
    pair: "Jozi",
    noPairs: "Hakuna jozi za kubadilishana — unganisha angalau sarafu mbili kwenye Mipangilio → Sarafu.",
    sell: "Uza {sym}",
    buy: "Nunua {sym}",
    amount: "Kiasi",
    youGive: "Unatoa",
    youGet: "Unapata",
    price: "Bei",
    priceUnit: "{unit} kwa {base}",
    pricePlaceholder: "bei ya kipimo",
    balance: "Salio: {amt} {sym}",
    balanceLoading: "Salio: …",
    noCoins: "Hakuna sarafu iliyosanidiwa",
    sameCoin: "Sarafu ya kutoa na ya kupokea lazima ziwe tofauti.",
    legDown: "Nodi ya mojawapo ya sarafu hizi imezimwa — iwashe (au angalia Mipangilio → Sarafu) kabla ya kuchapisha.",
    // Swap-protocol pin (only offered when a pair+network supports more than
    // one). v2 label reuses coins.protoPrivate ("Private (Taproot)").
    protocol: "Aina ya swap",
    protoStandard: "Kawaida (HTLC)",
    // Titles for the review/confirm dialog shown before posting (see OfferForm).
    reviewOfferTitle: "Kagua ofa yako",
    reviewSlipTitle: "Kagua karatasi yako",
    term: "Timelock ya usalama",
    termShort: "Fupi",
    termMedium: "Wastani",
    termLong: "Ndefu",
    termHint: {
      short: "Fupi — fedha hurejeshwa kiotomatiki haraka zaidi iwapo biashara itakwama (~saa 12 / saa 6), kwa kiwango kidogo zaidi cha usalama.",
      medium: "Wastani — dirisha lililo na uwiano la kurejesha (~saa 24 / saa 12).",
      long: "Ndefu (salama zaidi) — kiwango kikubwa zaidi cha usalama; rejesho la kiotomatiki baada ya ~saa 36 / saa 18 iwapo biashara itakwama.",
    },
    validFor: "Halali kwa (dakika)",
    validForMins: "dakika {mins}",
    validForHint:
      "Muda ofa itabaki imeorodheshwa. Ukiwa mtandaoni huhuishwa kiotomatiki; baada ya hapo huisha muda wake. Kufunga programu huiondoa.",
    note: "Ofa ya ukubwa usiobadilika — hakuna kinachofungwa hadi mtu aichukue. Viasi viko kwenye mnyororo; unalipa ada za mtandao juu yake na Corkboard haitozi chochote. Timelock ni dirisha la rejesho la kiotomatiki swap ikikwama.",
    post: "Chapisha ofa",
    makeSlip: "Tengeneza karatasi",
    slipTitle: "Karatasi yako ya ofa ya faragha",
    slipExplainer:
      "Mtumie rafiki yako. Anaibandika kwenye Satchel ili kuichukua. Hakuna kinachofungwa; huisha muda wake baada ya {ttl}.",
    copy: "Nakili",
    copied: "Imenakiliwa",
    makeAnother: "Tengeneza nyingine",
    myPrivateTitle: "Ofa zangu za faragha",
    myPrivateEmpty: "Hakuna ofa za faragha zilizosalia.",
    privateExpires: "huisha {when}",
    privateExpired: "imeisha muda",
    cancel: "Ghairi",
    cancelTip: "Acha kuheshimu karatasi hii — rafiki ambaye bado anaishikilia hawezi tena kuichukua.",
  },
  takeSlip: {
    open: "Bandika karatasi",
    title: "Chukua ofa ya faragha",
    intro:
      "Rafiki amekutumia karatasi ya ofa ya faragha (huanza na pactoffer1:). Ibandike hapa ili kuikagua na kuichukua — kama vile ofa kutoka kwenye ubao.",
    placeholder: "pactoffer1:…",
    take: "Kagua na chukua",
    invalid: "Hiyo haionekani kama karatasi — inapaswa kuanza na pactoffer1:.",
    previewLabel: "Karatasi hii inatoa",
  },
  // PRIVATE nav group (off-market slips) — screen titles + intros. The form,
  // slip output, my-slips list and take flow reuse makeOffer.*/takeSlip.* copy.
  private: {
    createTitle: "Tengeneza ofa ya faragha",
    createIntro:
      "Tengeneza ofa iliyotiwa saini na umpe rafiki kama karatasi kupitia mazungumzo yako mwenyewe. Hakuna kinachoorodheshwa popote — na hakuna kinachofungwa hadi nyote wawili mfadhili.",
    slipsIntro:
      "Karatasi ulizozitengeneza. Yeyote anayeshikilia karatasi anaweza kuichukua hadi muda wake uishe; ighairi ili uache kuiheshimu kabla ya hapo.",
    slipsEmptyBody: "Tengeneza ofa ya faragha ili upate karatasi unayoweza kumtumia rafiki.",
    receiveTitle: "Chukua ofa ya faragha",
    received: "Imechukuliwa — ifuatilie kwenye Swap.",
  },
  // Shared take-confirmation dialog (board take + slip take).
  takeConfirm: {
    title: "Uchukue ofa hii?",
    confirm: "Chukua ofa",
    counterparty: "Mshirika wa biashara",
    youGive: "Unatoa",
    youReceive: "Unapokea",
    safetyRefund: "Rejesho la usalama",
    offerAge: "Umri wa ofa",
    makerFundsFirst:
      "Mtoa ofa hufunga {sym} yake kwanza — wewe hutumi kwanza kamwe. Bado unaweza kughairi kabla ya kufadhili upande wako, na injini hurejesha kiotomatiki baada ya timelock ya usalama iwapo swap itakwama.",
  },
  header: {
    activeMerchant: "Mfanyabiashara anayetumika — bofya kubadili au kusimamia",
    manageMerchants: "Simamia Wafanyabiashara…",
    noMerchant: "hakuna mfanyabiashara",
    openMenu: "Fungua menyu",
    collapseMenu: "kunja menyu",
    settings: "Mipangilio",
    language: "Lugha",
    pactConnected: "Injini imeunganishwa",
    pactUnreachable: "Injini haifikiki",
    liveSwapsOne: "Swap 1 inaendelea — bofya kuona",
    liveSwapsMany: "Swap {count} zinaendelea — bofya kuona",
    liveSwapsNone: "Hakuna swap zinazoendelea",
    coinOk: "{name} — imeunganishwa · kilele {tip}",
    coinUnconfigured: "{name} — haijasanidiwa",
    coinError: "{name} — {status}",
    relaysOk: "Relays za Nostr — {up}/{total} zimeunganishwa",
    relaysDown: "Relays za Nostr — hakuna ya {total} iliyounganishwa",
  },
  network: {
    mainnet: "MainNet",
    testnet: "TestNet",
    regtest: "RegTest",
    signet: "Signet",
    notRealFunds: "Si fedha halisi — huu ni mtandao wa {network}",
  },
  // Watch-only mode: a viewer session with no coins. Browse the board and
  // withdraw your own offers, but no posting / taking / funding.
  watchOnly: {
    badge: "Kutazama tu",
    badgeTip:
      "Hali ya kutazama tu — vinjari ubao na uondoe ofa zako mwenyewe, lakini huwezi kuchapisha, kuchukua au kufadhili. Sanidi sarafu kwenye Mipangilio ili ufanye biashara.",
    coinWizardButton: "Vinjari katika hali ya kutazama tu",
    coinWizardHint:
      "Ruka usanidi wa sarafu na uvinjari ubao tu (kusoma pekee). Bado unaweza kuondoa ofa zako mwenyewe — ni rahisi kwa kuondoa ofa zilizoachwa na kipindi kingine. Izime wakati wowote kwenye Mipangilio.",
    postBlockedTitle: "Hali ya kutazama tu",
    postBlockedBody:
      "Hiki ni kipindi cha kutazama tu, kwa hivyo hakiwezi kuchapisha ofa. Sanidi angalau sarafu mbili kwenye Mipangilio → Sarafu ili ufanye biashara.",
    takeBlockedBody: "Hali ya kutazama tu — unaweza kukagua ofa hii, lakini kuichukua kunahitaji sarafu zisanidiwe.",
    takeBlockedTip: "Hali ya kutazama tu — sanidi sarafu kwenye Mipangilio ili kuchukua ofa.",
  },
  merchants: {
    title: "Wafanyabiashara wako",
    intro:
      "Mfanyabiashara ni utambulisho mmoja wa biashara — mbegu yake mwenyewe na historia ya swap. Kufanya biashara chini ya mfanyabiashara tofauti huweka muktadha usioweza kuunganishwa (utambulisho wa muda). Sarafu zako kuu zinakaa kwenye pochi yako mwenyewe, si hapa.",
    // First-run welcome (empty merchant manager).
    welcomeTitle: "Karibu Satchel",
    welcomeIntro:
      "Satchel hufanya biashara chini ya “mfanyabiashara” — utambulisho mmoja wa biashara wenye mbegu yake mwenyewe. Bado huna yeyote: tengeneza mpya, au ingiza kifungu cha kurejesha kilichopo ili kuanza.",
    importMerchant: "Ingiza mfanyabiashara",
    none: "Bado hakuna wafanyabiashara.",
    active: "anayetumika",
    switch: "badili",
    newMerchant: "Mfanyabiashara mpya",
    thisMerchant: "mfanyabiashara huyu",
    nameLabel: "Jina la mfanyabiashara",
    namePlaceholder: "k.m. Mkuu",
    introFirst:
      "Sanidi utambulisho wako wa kwanza wa biashara (“mfanyabiashara”). Hushikilia tu funguo za usafiri za muda za swap zinazoendelea — sarafu zako kuu zinabaki kwenye pochi yako mwenyewe.",
    introNew: "Mfanyabiashara mpya ni utambulisho mpya, tofauti wenye mbegu yake mwenyewe na historia ya swap.",
    // UI-5 merchant selector (phoenix wallet-selector parity).
    createNew: "Tengeneza mpya",
    import: "Ingiza",
    load: "Pakia Mfanyabiashara",
    loaded: "amepakiwa",
    locked: "amefungwa",
    lockedTip: "Mbegu iliyosimbwa — fungua kwa nenosiri lako unapoipakia.",
    close: "Funga",
    idLabel: "folda",
    switching: "Inabadili mfanyabiashara…",
    switchingBody: "Inazindua tena injini dhidi ya folda hiyo.",
  },
  // Seed create/import (SeedForm) + counterparty + status.
  seed: {
    intro: "Tengeneza mbegu mpya kabisa, au ingiza uliyo nayo tayari.",
    createNew: "Tengeneza mpya",
    createDesc: "Zalisha mbegu mpya. Wewe huhifadhi nakala ya kifungu cha kurejesha.",
    import: "Ingiza",
    importDesc: "Rejesha kutoka kifungu kilichopo cha maneno 12/24.",
    recoveryLabel: "Kifungu cha kurejesha",
    importPlaceholder: "neno1 neno2 neno3 …",
    encrypt: "Simba",
    encryptDesc:
      "Nenosiri hulinda mbegu ikiwa imehifadhiwa. Unaiingiza mara moja kwa kila kipindi — Satchel haihifadhi kamwe. Kumbuka: rejesho la kiotomatiki lisilo na usimamizi husitisha baada ya kuwasha upya hadi uiingize tena.",
    noPassphrase: "Bila nenosiri (kinapendekezwa)",
    noPassphraseDesc:
      "Rejesho la kiotomatiki huendelea kufanya kazi hata baada ya kuwasha upya bila kitu cha kuingiza — hii ni mbegu ya usafiri ya muda pekee. Gharama: ufikiaji wa faili/mwenyeji hufichua funguo za usafiri na utambulisho wa mfanyabiashara huyu.",
    passphraseLabel: "Nenosiri",
    passphrasePlaceholder: "chagua nenosiri",
    createTitle: "Tengeneza mbegu",
    importTitle: "Ingiza mbegu",
    secureTitle: "Linda {label}",
    revealTitle: "Andika kifungu chako cha kurejesha",
    revealBody:
      "Yeyote mwenye maneno haya hudhibiti funguo motomoto za mfanyabiashara huyu. Satchel haihifadhi nakala — ihifadhi nje ya mtandao. Utathibitisha maneno machache baadaye.",
    ackLabel: "Nimeandika kifungu changu cha kurejesha.",
    // Multi-step onboarding (create/import -> secret -> confirm -> passphrase).
    chooseTitle: "Sanidi {label}",
    enterTitle: "Ingiza kifungu chako cha kurejesha",
    enterBody:
      "Andika kila neno — hukamilika kiotomatiki unapoendelea — au bandika kifungu kizima. Tunakithibitisha kabla huja endelea.",
    wordCount: "maneno {n}",
    wordAria: "Neno {n}",
    checkIncomplete: "Ingiza maneno yote {n}.",
    checkUnknown: "Baadhi ya maneno hayamo katika orodha ya maneno ya BIP39 — angalia yaliyoangaziwa.",
    checkBadChecksum: "Checksum hailingani — angalia tena maneno yako na mpangilio wake.",
    checkOk: "Kifungu cha kurejesha kinaonekana halali.",
    verifyTitle: "Thibitisha nakala yako",
    verifyBody: "Andika maneno katika nafasi hizi ili kuthibitisha kwamba uliandika kifungu.",
    verifyWord: "Neno #{n}",
    verifyMismatch: "Hayo hayalingani na kifungu chako — angalia nakala yako.",
    passphraseTitle: "Linda mbegu",
    passphraseBody:
      "Kwa hiari simba mbegu iliyohifadhiwa kwa nenosiri. Unaweza kuruka hili — angalia faida na hasara hapa chini.",
  },
  counterparty: {
    you: "Huyu ni wewe",
    youShort: "wewe",
    unknown: "utambulisho usiojulikana",
    // Short fingerprint fallback (identity.ts shortId) when no pubkey is known.
    unknownShort: "haijulikani",
  },
  status: {
    notConnectedTitle: "Haujaunganishwa na injini",
    disconnectedBody:
      "Satchel haiwezi kufikia injini. Huenda bado inaanza, au miunganisho ya nodi ya mfanyabiashara anayetumika imezimwa. Jaribu tena, au badili mfanyabiashara kutoka kwa kiteuzi kilicho juu.",
    openInSatchel: "Fungua hii ndani ya Satchel",
    noTauriBody:
      "Hii ni UI ya Satchel — inahitaji daraja la Tauri kufikia injini. Zindua programu ya kompyuta (cargo tauri dev) badala ya kivinjari.",
  },
  settings: {
    title: "Mipangilio",
    subtitle: "Mapendeleo ya programu nzima kwa usakinishaji huu.",
    // UI-3 Settings tabs.
    tabGeneral: "Jumla",
    tabCoins: "Sarafu",
    tabNetwork: "Mtandao",
    tabAbout: "Kuhusu",
    appearance: "Muonekano",
    theme: "Mandhari",
    themeDark: "Giza",
    themeLight: "Mwanga",
    themeSystem: "Mfumo",
    themeHint: "Chagua jinsi Satchel inavyoonekana. Mfumo hufuata mpangilio wa OS yako.",
    language: "Lugha",
    languageHint: "Lugha zaidi huongezwa kadiri tafsiri zinavyochangwa.",
    mode: "Hali",
    watchOnly: "Hali ya kutazama tu",
    watchOnlyHint:
      "Vinjari ubao bila kusanidi sarafu. Bado unaweza kuondoa ofa zako mwenyewe, lakini huwezi kuchapisha, kuchukua au kufadhili. Zima ili ufanye biashara (utahitaji angalau sarafu mbili zimeunganishwa).",
    network: "Mtandao",
    boards: "Corkboards",
    boardsDesc:
      "Corkboards za HTTP za hiari unazoziendesha mwenyewe. Ongeza unazoziamini; acha tupu ili kutegemea Nostr.",
    boardsNone: "Hakuna iliyosanidiwa",
    nostrRelays: "Relays za Nostr",
    nostrRelaysDesc:
      "Relays husafirisha ubao wa matangazo kwenye mtandao uliogatuliwa — hakuna mwendeshaji anayeweza kusoma au kulinganisha ofa zako. Zimewekwa mapema na seti chaguomsingi; hariri kwa uhuru.",
    nostrRelaysOff: "Imezimwa — usafirishaji wa Nostr umezimwa",
    addUrl: "Ongeza",
    removeUrl: "Ondoa",
    relayInvalid: "Ingiza URL ya relay ya ws:// au wss://",
    boardInvalid: "Ingiza URL ya ubao ya http:// au https://",
    netSave: "Hifadhi na uunganishe tena",
    netSaving: "Inahifadhi na kuunganisha tena…",
    netSaved: "Imehifadhiwa",
    // Fees tab — fee-bump policy (per active merchant).
    tabFees: "Ada",
    fees: "Kuongeza ada",
    feesScope: "Mipangilio hii inatumika kwa mfanyabiashara anayetumika.",
    feesIntro:
      "Faida na hasara za usalama/gharama za kuongeza ada, si usanidi unaohitajika. Thamani mpya hutumika kwa nyongeza za baadaye; swap zilizofadhiliwa tayari hubaki na sera ziliyofadhiliwa chini yake.",
    feeMax: "Kiwango cha juu cha ada (sat/vB)",
    feeMaxHint:
      "Kikomo cha juu kwa kila nyongeza ya ada. Chaguomsingi 500, pia kiwango cha juu zaidi cha mfumo. Kipunguze ili kudhibiti gharama.",
    feeReservation: "Akiba ya nyongeza ya ufadhili (×)",
    feeReservationHint:
      "Salio ambalo ukaguzi wa fedha hutenga kama nafasi ya nyongeza. Cha juu zaidi huokoa upandaji mkubwa wa ada lakini hufunga salio zaidi na kukataa swap zaidi. Chaguomsingi 3.",
    feeCommitted: "Utoaji wa ziada wa kukomboa (×)",
    feeCommittedHint:
      "Kiasi gani cha ziada ada ya kukomboa ya v2 hulipwa mapema ili ithibitishwe hata wakati Satchel imefungwa. Inatumika kwa swap mpya tu. Chaguomsingi 2.",
    feeStep: "Hatua ya kupandisha ya RBF (%)",
    feeStepHint: "Jinsi ada ya matumizi yaliyokwama inavyopanda kwa nguvu kila mzunguko wa ratiba. Chaguomsingi 50.",
    feeSave: "Hifadhi",
    feeSaving: "Inahifadhi…",
    feeSaved: "Imehifadhiwa",
    feeReset: "Rejesha kwa chaguomsingi",
    coins: "Sarafu na nodi",
    coinsHint: "Unganisha kila sarafu kwenye nodi yako mwenyewe. Genesis hukaguliwa kabla ya kuhifadhi chochote.",
    about: "Kuhusu",
    version: "Toleo {version}",
    updateUpToDate: "Liko sawa",
    updateCheckPlaceholder: "Ukaguzi wa sasisho utafika katika toleo la baadaye.",
    trustModel: "Mahali funguo zako zinakaa",
    trustModelBody:
      "Siri zinakaa kwenye injini, kamwe si kwenye Satchel. Mbegu ya mfanyabiashara hukaa kwenye folda ya data ya injini (imesimbwa au maandishi wazi — chaguo lako); Satchel haihifadhi mbegu au nenosiri. Mbegu ni motomoto kwa muundo (funguo za usafiri tu) — fagia mapato makubwa hadi pochi yako mwenyewe ya baridi.",
  },
  coins: {
    intro:
      "Unganisha kila sarafu kwenye nodi yako mwenyewe. URL ya kwanza ni pochi ya nodi yako mwenyewe — hufadhili sehemu zako za swap na kupokea mapato. Kabla ya kuhifadhi chochote, Satchel hukagua kizuizi cha genesis cha nodi ili fedha zisiweze kamwe kutumwa kwenye mnyororo usio sahihi. Miunganisho hushirikiwa kati ya wafanyabiashara wako wote.",
    networkBadge: "Inasanidi kwa mtandao wa {network}",
    needMerchant:
      "Unganisha mfanyabiashara kwanza — usanidi wa sarafu unahitaji injini iendeshwe. Tumia kiteuzi cha mfanyabiashara kilicho juu kulia.",
    pairsTitle: "Jozi za biashara",
    pairsHint:
      "Jozi hutokana na kile kila sarafu inachoweza kufanya — hakuna orodha thabiti. Jozi hufunguka mara sarafu zake zote mbili zinapounganishwa.",
    noPairs: "Hakuna jozi zinazopatikana.",
    notSetUp: "Haijasanidiwa",
    connectedTip: "Imeunganishwa · kilele {tip}",
    connError: "Hitilafu ya muunganisho",
    setUp: "Sanidi",
    editConnection: "Hariri muunganisho",
    remove: "ondoa",
    disconnectTip: "Tenganisha sarafu hii",
    disconnectTitle: "Tenganisha {coin}?",
    disconnectBody: "Swap zinazoihitaji hazitapatikana hadi uunganishe tena.",
    ready: "Tayari kufanya biashara",
    connectMissing: "Unganisha {coins}",
    notBuildable: "Bado haiwezi kujengwa",
    // Swap-protocol chips on a pair (pact-htlc-v1 HTLC vs pact-htlc-v2 adaptor).
    protoPrivate: "Faragha (Taproot)",
    protoPrivateTip: "Swap ya faragha (kibadilishaji cha Taproot/MuSig2) — huonekana kama malipo ya kawaida kwenye mnyororo",
    protoHtlcTip: "Swap ya HTLC ya kawaida",
    // Coin-setup backend choices (CoinSetup).
    backendCoreTitle: "Pochi ya Core RPC",
    backendCoreDesc: "Pochi ya nodi yako mwenyewe hufadhili swap na kupokea mapato.",
    backendHardwareTitle: "Vifaa",
    backendHardwareDesc: "Utiaji saini wa Ledger / PSBT kwa sehemu ya ufadhili.",
    backendLater: "baadaye",
    // CoinSetup dialog.
    setupTitle: "Unganisha {coin}",
    setupIntro:
      "Elekeza Satchel kwenye nodi yako mwenyewe ya {sym}. Hakuna kinachohifadhiwa hadi nodi ipite ukaguzi wa kizuizi cha genesis — fedha zako huwa hugusa tu mnyororo halisi wa {sym}.",
    backendUrlLabel: "URL ya nodi ya nyuma",
    backendUrlHint:
      "URL ya kwanza = pochi ya nodi yako mwenyewe (hufadhili swap, hupokea mapato). Ongeza seva za Electrum (tcp://mwenyeji:bandari) baada ya koma kwa mionekano ya ziada, huru ya mnyororo.",
    fundingWallet: "Pochi ya ufadhili",
    confirmationsLabel: "Uthibitisho kabla ya mwisho",
    confirmationsHint:
      "Ufadhili au ukombozi kwenye mnyororo huu lazima uwe na kina kiasi gani kabla swap haijachukua hatua — kiwango cha usalama wa reorg. Cha juu zaidi ni salama lakini polepole; acha wazi kwa chaguomsingi kilichopendekezwa ({default}).",
    validateNode: "Thibitisha nodi",
    checking: "Inakagua nodi…",
    genesisOk: "Genesis imelingana — huu ndio mnyororo sahihi",
    genesisDetail: "urefu wa kilele {tip} · genesis {hash}…",
    genesisBad: "Imekataliwa — haihifadhi",
    errorShort: "hitilafu",
    // Structured connection form (CoinSetup v2).
    rpcHostLabel: "Mwenyeji wa RPC",
    rpcPortLabel: "Bandari ya RPC",
    authMethodLabel: "Uthibitishaji",
    authCookie: "Faili ya cookie",
    authCookieDesc: "Soma kiotomatiki .cookie ya nodi kutoka kwa saraka yake ya data (chaguomsingi, hakuna nenosiri linalohifadhiwa).",
    authUserPass: "Mtumiaji / nenosiri",
    authUserPassDesc: "rpcuser / rpcpassword kutoka kwa usanidi wa nodi yako — inahitajika kwa nodi ya mbali.",
    rpcUserLabel: "Jina la mtumiaji la RPC",
    rpcPasswordLabel: "Nenosiri la RPC",
    datadirLabel: "Saraka ya data ya nodi",
    cookiePathNote: "Cookie husomwa kutoka {path} chini ya saraka hii.",
    walletLabel: "Jina la pochi (hiari)",
    walletPlaceholder: "pochi ya nodi yako",
    needPort: "Ingiza bandari ya RPC kwanza.",
    validateFirst: "Thibitisha nodi kabla ya kuhifadhi.",
    savingReconnecting: "Inahifadhi na kuunganisha tena…",
    connected: "{coin} imeunganishwa",
    // Template picker (a coins.toml coin the engine version doesn't support).
    unsupportedByEngine: "Haisaidiwi",
    unsupportedByEngineTip:
      "Sarafu hii imefafanuliwa katika coins.toml lakini haijajengwa ndani ya toleo hili la injini, kwa hivyo haiwezi kufanyiwa biashara.",
  },
  coinWizard: {
    title: "Unganisha sarafu zako",
    intro:
      "Chagua angalau sarafu mbili na uelekeze kila moja kwenye nodi yako mwenyewe. Swap inahitaji minyororo miwili, kwa hivyo biashara hufunguka mara nodi mbili zinapounganishwa na kufanya kazi. Unaweza kuongeza au kubadilisha sarafu baadaye kwenye Mipangilio.",
    progress: "{count} kati ya {min} sarafu zimeunganishwa",
    continue: "Endelea",
    live: "Inafanya kazi",
    nodeDown: "Nodi imezimwa",
  },
  wallets: {
    intro:
      "Hizi ni pochi za nodi zako mwenyewe (zile injini inazotumia kufadhili swap na kupokea mapato) — funguo zako, mashine yako. Satchel haishikilii kamwe sarafu zako.",
    hotSeedNudge:
      "Hii ni pochi ya matumizi kwenye mbegu motomoto, si hazina — fagia salio kubwa hadi pochi yako mwenyewe ya baridi/core.",
    notConnected: "Haijaunganishwa",
    notConnectedBody: "Unganisha mfanyabiashara kwanza — mwonekano wa pochi unahitaji injini iendeshwe.",
    noCoins: "Bado hakuna sarafu iliyosanidiwa",
    noCoinsBody: "Unganisha sarafu kwenye Mipangilio → Sarafu na pochi yake huonekana hapa.",
    goToCoins: "Nenda Sarafu",
    watchOnlyTitle: "Hakuna pochi katika hali ya kutazama tu",
    watchOnlyBody:
      "Hiki ni kipindi cha kutazama tu bila sarafu zilizounganishwa, kwa hivyo hakuna pochi za kuonyesha. Zima kutazama tu kwenye Mipangilio na uunganishe sarafu ili kufadhili swap.",
    walletName: "pochi · {wallet}",
    walletScopedHint: "Kila RPC ya sarafu hii imefungwa kwa pochi hii ya nodi.",
    walletDefault: "pochi chaguomsingi (haijafungwa)",
    walletDefaultHint:
      "Hakuna pochi iliyowekwa kwa sarafu hii, kwa hivyo RPC hutumia pochi chaguomsingi ya nodi. Weka moja kwenye Mipangilio → Sarafu ili kufunga kila wito kwa pochi maalum.",
    balanceLabel: "salio la {symbol}",
    receive: "Pokea",
    send: "Tuma",
    sendTo: "Tuma kwa anwani",
    amount: "Kiasi",
    sendTitle: "Tuma {amount} {sym}?",
    sendConfirmBody: "Kwa {to}\n\nHii hutumia kutoka pochi ya nodi yako mwenyewe na haiwezi kutenduliwa.",
  },
  corkboard: {
    noBoardTitle: "Hakuna Corkboard iliyounganishwa",
    noBoardBody:
      "Corkboard ni ubao wa matangazo unaoshirikiwa ambapo watoa ofa hubandika ofa. Haulingani biashara kamwe wala kushikilia sarafu — elekeza Satchel kwenye mojawapo unaouamini ili kuvinjari na kuchapisha.",
    noPairs: "Hakuna jozi zinazopatikana",
    board: "Corkboard",
    boardSettings: "Sanidi kwenye Mipangilio",
    filterAll: "Zote",
    filterMine: "Zangu",
    offered: "{symbol} zinazotolewa",
    noOffers: "Hakuna ofa unayoweza kuchukua sasa hivi",
    noOffersBody:
      "Ofa huonekana hapa mara tu mtoa ofa anapochapisha moja kwa jozi uliyoisanidi. Unaweza pia kuchapisha yako mwenyewe.",
    hiddenOffers:
      "Ofa {count} zaidi kwa jozi ambazo hujaziunganisha. Sanidi sarafu zote mbili ili kuzifanyia biashara:",
    yourOffer: "ofa yako",
    offerStaged: "inachapisha…",
    offerStagedTip:
      "Imechapishwa kutoka kifaa hiki na inasubiri kuthibitishwa kutoka kwa relay. Inatangaza; inaanza kufanya kazi mara relay inapoiakisi.",
    take: "Chukua ofa",
    legDown: "Nodi ya mojawapo ya jozi hii imezimwa — iwashe (au angalia Mipangilio → Sarafu) kabla ya kuchukua.",
    withdraw: "Ondoa",
    withdrawTip: "Ondoa papo hapo — ofa haifungi kamwe fedha",
    safetyRefund: "rejesho la usalama",
    safetyRefundTip:
      "Iwapo swap itakwama, pande zote mbili hurejeshwa kiotomatiki — sehemu ya mchukuaji hufunguka kwanza, yako baadaye kidogo. Hakuna anayeishia kukwama.",
    activeTitle: "Swap zako zinazoendelea",
    states: {
      open: "wazi",
      takenByUs: "imechukuliwa na wewe",
      revoked: "imeondolewa",
      expired: "imeisha muda",
    },
    // Two-sided order-book view of the Corkboard. Bids = makers giving the
    // quote coin to get the base; asks = the reverse. The ladder is a way to
    // READ the board — it never matches or prioritises (load-bearing).
    book: {
      bids: "Zabuni",
      asks: "Mauzo",
      bidsHint: "wanataka {base} · wakilipa {quote}",
      asksHint: "wanauza {base} · kwa {quote}",
      price: "Bei",
      size: "Ukubwa",
      noBids: "Hakuna zabuni",
      noAsks: "Hakuna mauzo",
      spread: "Tofauti {pct}",
      spreadOneSided: "Upande mmoja",
      crossed: "imevuka",
      crossedTip: "Zabuni ya juu ≥ mauzo ya juu. Ubao haulingani kiotomatiki kamwe, kwa hivyo ofa hizi zinazoingiliana hukaa tu — chukua upande wowote.",
      mid: "kati {price}",
      levelOffers: "Ofa {count} kwa bei hii — chagua moja ya kuchukua",
      depthTip: "Jumla ya {sym} zinazotolewa kwa bei hii katika matangazo {count}.",
      takerNote: "Ukiichukua, unatoa {give} na unapokea {get}.",
      selectLevel: "Chagua kiwango cha bei hapo juu ili kuona ofa zilizopo.",
      paneHeader: "{size} {base} @ {price} {unit}",
      denomTip: "Kipimo cha kuonyesha kwa viasi vya {coin}",
      showMore: "Onyesha {count} zaidi",
      showLess: "Onyesha {count} za juu",
    },
  },
  relays: {
    title: "Relays",
    subtitle: "Muunganisho hai wa relays zako za Nostr — mtandao ofa na uchukuaji wako husafiri juu yake. Ongeza au ondoa relays kwenye Mipangilio → Mtandao.",
    connectedCount: "{up} / {total} zimeunganishwa",
    refresh: "Onyesha upya",
    ms: "{ms} ms",
    up: "hai",
    down: "zima",
    statsTip: "{success}/{attempts} miunganisho iliyofanikiwa · ↓{down} ↑{up}",
    none: "Hakuna relays zilizosanidiwa",
    noneBody: "Ongeza relay ya Nostr kwenye Mipangilio → Mtandao ili kuchapisha na kupokea ofa kwenye mtandao.",
    goToNetwork: "Nenda Mipangilio",
    notConnected: "Haijaunganishwa",
    notConnectedBody: "Mwonekano wa relay unahitaji injini iendeshwe — unganisha mfanyabiashara kwanza.",
  },
  swaps: {
    title: "Swap",
    hint: "Daftari lako kamili — swap zinazoendelea juu, biashara zilizokamilika chini. Unaweza pia kuchukua hatua kwenye swap hai kutoka Corkboard.",
    activeTitle: "Zinazoendelea",
    historyTitle: "Historia",
    none: "Bado hakuna swap — chukua ofa kwenye Corkboard.",
    // Active-swaps dock action buttons + the refund-time label.
    cancel: "ghairi",
    refund: "rejesha",
    dump: "toa kumbukumbu",
    dumpHint: "Nakili kifurushi cha uchunguzi kisicho na siri (hali + mistari ya kumbukumbu) kwa swap hii, ili kuwabandikia wasanidi programu.",
    dumpCopied: "Uchunguzi umenakiliwa — wabandikie wasanidi programu.",
    dumpFailed: "Haikuweza kunakili kifurushi cha uchunguzi.",
    refundAt: "rejesha {when}",
    // Confirm dialogs for acting on a live swap (ActiveSwaps).
    cancelTitle: "Ughairi swap hii?",
    cancelConfirm: "Ghairi swap",
    cancelKeep: "Iweke",
    // Abort reason recorded on the swap when cancelled from Satchel.
    cancelReason: "imeghairiwa katika Satchel",
    cancelBody:
      "Hii huachana na swap kabla hujafadhili. Hakuna chochote chako kilichofungwa bado, kwa hivyo hupotezi chochote — ofa haitakamilika tu.",
    refundTitle: "Urudishe fedha zako?",
    refundConfirm: "Rejesha",
    refundBody:
      "Timelock ya usalama imepita, kwa hivyo unaweza kudai tena fedha ulizofunga. Hii hutangaza rejesho lako sasa; injini pia hufanya hivyo kiotomatiki baada ya tarehe ya mwisho.",
    col: {
      swap: "swap",
      role: "jukumu",
      state: "hali",
      amounts: "anatoa → anapokea",
      when: "lini",
      finalTx: "muamala wa mwisho",
    },
    // Expandable per-leg on-chain detail (the audit trail). We show both
    // funding txs + OUR settlement; never the counterparty's settlement or the
    // swap secret.
    audit: {
      toggle: "Onyesha maelezo ya mnyororo",
      title: "Maelezo ya mnyororo",
      youLocked: "ulifunga",
      theyLocked: "walifunga",
      funding: "Ufadhili",
      received: "Imepokelewa",
      refunded: "Imerejeshwa",
      pending: "bado haijafika kwenye mnyororo",
      copy: "Nakili kitambulisho cha muamala",
      copied: "Kitambulisho cha muamala kimenakiliwa",
    },
  },
  fees: {
    title: "Hakikisho la gharama ya mtandao",
    estimated: "inakadiriwa",
    provisionalNote: "Toleo hili la pactd halifichui ukadiriaji wa ada bado.",
    summary: "Swap ni miamala 2 ya mnyororo unayolipia: ufadhili kwenye mnyororo wa kutoa, ukombozi kwenye mnyororo wa kupokea.",
    fallbackTip: "Nodi haikufikika, kwa hivyo kiwango cha ada cha chaguomsingi cha tahadhari kilitumika — vichukue kama kisio.",
    ifItStalls: "(ikikwama)",
  },
  funds: {
    insufficient:
      "Hakuna {sym} ya kutosha kufadhili swap hii — inahitaji ~{need} {sym} (kiasi + ada ya ufadhili), pochi ina {have} {sym}.",
  },
  wizard: {
    welcome: "Karibu Satchel",
    connectTitle: "Unganisha injini ya Pact",
    connectIntro:
      "Satchel ni mteja mwepesi wa injini ya Pact — kiini kinachoshikilia funguo zako na kuendesha swap. Chagua jinsi ya kuifikia.",
    managed: "Endesha injini ya Pact iliyojengwa ndani",
    managedDesc: "Satchel huzindua na kusimamia injini yake mwenyewe ya Pact. Inapendekezwa.",
    external: "Unganisha kwenye injini ya nje ya Pact",
    externalDesc: "Elekeza kwenye injini ya Pact unayoiendesha tayari (weka SATCHEL_PACTD_URL + cookie kabla ya kuzindua).",
    externalNote:
      "Hali ya nje huchaguliwa kupitia vigeu vya mazingira kabla ya kuzindua Satchel. Zindua tena na SATCHEL_PACTD_URL ikiwa imewekwa ili kuitumia.",
    coinsTitle: "Ongeza sarafu zako",
    coinsIntro:
      "Baada ya mfanyabiashara wako kutengenezwa, unganisha kila sarafu kwenye nodi yako mwenyewe kwenye Mipangilio → Sarafu. Chagua sarafu na nodi ya nyuma (Electrum ya umma kwa usanidi sufuri, au nodi yako mwenyewe); genesis hukaguliwa dhidi ya mtandao huu kabla ya kuhifadhi chochote.",
    coinsTemplatesSoon: "Violezo vya sarafu vya kubofya-moja vinafika hapa katika toleo la baadaye.",
    back: "Nyuma",
    continue: "Endelea",
    finish: "Maliza usanidi",
  },
  // UI-4 docked activity log.
  log: {
    title: "Shughuli",
    empty: "— kumbukumbu ya shughuli —",
    count: "mistari {count}",
    collapse: "Kunja kumbukumbu",
    expand: "Panua kumbukumbu",
    // Activity-log lines emitted by the frontend. `{err}` carries a raw engine
    // message (itself not translated — it comes from pactd over the wire).
    noTauri: "haiendeshwi ndani ya Satchel — UI hii inahitaji daraja la Tauri",
    startupError: "kuanza: {err}",
    notConnected: "haijaunganishwa: {err}",
    connected: "imeunganishwa na pactd {version} ({protocol})",
    listcoinsError: "listcoins: {err}",
    watchOnlyError: "kutazama-tu: {err}",
    switchedMerchant: "imebadilishwa kwa mfanyabiashara {id}",
    switchMerchantError: "badili mfanyabiashara: {err}",
    loadMerchantError: "pakia mfanyabiashara: {err}",
    merchantCreated: "mfanyabiashara {id} ametengenezwa",
    merchantReady: "mfanyabiashara tayari",
    actionOk: "{action} {id}: sawa",
    actionError: "{action} {id}: {err}",
    diagCopied: "uchunguzi wa {id} umenakiliwa (mistari {count} ya kumbukumbu) — wabandikie wasanidi",
    dumpError: "toa {id}: {err}",
    coinDisconnected: "{coin} imetenganishwa",
    removeCoinError: "ondoa sarafu: {err}",
    tookOffer: "imechukua ofa {id} — sasa inaonekana kwenye swap zako zinazoendelea hapa chini",
    takeError: "chukua: {err}",
    offerWithdrawn: "ofa {id} imeondolewa",
    withdrawError: "ondoa: {err}",
    postedOffer: "ofa {id} imechapishwa — ondoa wakati wowote; hakuna kinachofungwa",
    createdSlip: "karatasi ya ofa ya faragha imetengenezwa — mtumie rafiki yako",
    tookPrivateOffer: "imechukua ofa ya faragha {id} — sasa inaonekana kwenye swap zako zinazoendelea",
    cancelledPrivateOffer: "ofa ya faragha {id} imeghairiwa",
    cancelError: "ghairi: {err}",
    noticeboardUpdated: "ubao wa matangazo umesasishwa",
    feePolicyUpdated: "sera ya ada imesasishwa",
  },
  // Relative-time + freshness prose from format.ts (rendered via the tr() mirror,
  // since those are pure non-component helpers). Unit letters stay inside the
  // template so a translation owns the whole phrase.
  format: {
    ageUnknown: "umri haujulikani",
    justNow: "sasa hivi",
    minutesAgo: "dakika {n} zilizopita",
    hoursAgo: "saa {n} zilizopita",
    daysAgo: "siku {n} zilizopita",
    expiryNow: "sasa",
    expirySoon: "hivi karibuni",
    inMinutes: "baada ya ~dakika {n}",
    inHours: "baada ya ~saa {n}",
    inDays: "baada ya ~siku {n}",
    posted: "imechapishwa {age}",
    expires: "huisha {time}",
  },
  // Plain-language swap story per (role, state) — the honest "who is exposed
  // when" framing shown on every active swap. {a}/{b} are coin tickers; {t1}/{t2}
  // are local refund times. Rendered via tr() (narrate() is a pure helper).
  narrate: {
    initiating:
      "Uchukuaji umetumwa — inasubiri mtoa ofa aanze swap. Hakuna kinachofungwa bado; hujighairi yenyewe iwapo hawajibu.",
    created: "Ofa imetumwa — inasubiri upande mwingine ukubali. Hakuna kinachoahidiwa.",
    acceptedMaker: "Masharti yamekubaliwa. Ifuatayo: funga {a} yako. Hadi ufadhili, bado unaweza kughairi kwa uhuru.",
    acceptedTaker: "Masharti yamekubaliwa. Upande mwingine hufunga {a} yao kwanza — wewe hutumi kwanza kamwe.",
    noncesExchanged:
      "Inaanzisha swap ya faragha — inabadilishana nyenzo za kutia saini. Hakuna kinachofungwa bado.",
    signedMaker:
      "Pande zote mbili zimetia saini. Daemon yako hufunga {a}, kisha hudai {b} kiotomatiki. Iwapo chochote kitakwama, {a} yako hurudi saa {t1}.",
    signedTaker:
      "Pande zote mbili zimetia saini. Daemon yako hufunga {b} na hudai {a} mara upande mwingine unapotenda. Wavu wa usalama: rejesho saa {t2}.",
    fundedAMaker:
      "{a} yako imefungwa. Inasubiri upande mwingine ufunge {b} yao. Iwapo hawatafanya kamwe, {a} yako hurudi kiotomatiki saa {t1}.",
    fundedATaker:
      "{a} yao imefungwa na kuthibitishwa. Ifuatayo: funga {b} yako. Wavu wa usalama: rejesho la kiotomatiki saa {t2} iwapo chochote kitakwama.",
    fundedBMaker: "Zote zimefungwa. Daemon yako hudai {b} mara tu inapothibitishwa kwa usalama.",
    fundedBTaker: "Zote zimefungwa. Daemon yako itadai {a} mara upande mwingine unapochukua {b} yao.",
    redeemedB:
      "Umedai {b} — inasubiri ithibitishwe. {a} yako iliyofungwa hubaki imelindwa hadi hili liwe la mwisho.",
    completed: "Swap imekamilika — {coin} iko kwenye pochi yako.",
    refunded: "Swap haikukamilika, kwa hivyo {coin} yako ilirudi kiotomatiki. Hakuna kilichopotea ila ada.",
    aborted: "Imeghairiwa kabla pesa yoyote haijahamishwa.",
  },
  exit: {
    // Exit-gate dialog (fund safety, C6). The engine manages alone, so "keep
    // running" detaches it (it keeps watching timelocks + servicing offers).
    liveTitle: "Swap inaendelea",
    liveBodyOne:
      "Swap 1 inaendelea. Inadhibitiwa na timelocks za mnyororo — injini lazima iendelee kufanya kazi ili kukomboa au kurejesha kabla ya tarehe ya mwisho.",
    liveBodyMany:
      "Swap {count} zinaendelea. Zinadhibitiwa na timelocks za mnyororo — injini lazima iendelee kufanya kazi ili kukomboa au kurejesha kabla ya tarehe ya mwisho.",
    keepRunningExplain:
      "Kufunga dirisha huweka injini ikiendelea kufanya kazi nyuma, kwa hivyo humaliza swap bila kichwa. Unaweza kufungua Satchel tena wakati wowote ili kuiangalia.",
    forceQuitWarn: "Kulazimisha kufunga sasa husimamisha injini na kunaweza kupoteza fedha.",
    // {word} is the confirm word below; a translation may localize both together.
    typeToConfirm: "Ili kulazimisha kufunga hata hivyo, andika {word} hapa chini.",
    confirmWord: "QUIT",
    keepRunning: "Iendelee kufanya kazi, funga dirisha",
    keepWithdraw: "Iendelee kufanya kazi + ondoa ofa",
    keepLeaveOffers: "Iendelee kufanya kazi, acha ofa zibaki",
    forceQuit: "Lazimisha kufunga",
    offersTitle: "Una ofa zilizochapishwa",
    offersBodyOne:
      "Ofa 1 yako bado iko kwenye Corkboard. Ofa hazifungi chochote, lakini kuiacha ina maana washirika bado wanaweza kuichukua wakati Satchel imefungwa — injini itashughulikia uchukuaji.",
    offersBodyMany:
      "Ofa {count} zako bado ziko kwenye Corkboard. Ofa hazifungi chochote, lakini kuziacha ina maana washirika bado wanaweza kuzichukua wakati Satchel imefungwa — injini itashughulikia uchukuaji.",
    withdrawExit: "Ondoa zote na utoke",
  },
  unlock: {
    title: "Fungua mfanyabiashara",
    body:
      "Mbegu ya mfanyabiashara huyu imesimbwa. Ingiza nenosiri lake ili kuifungua kwa kipindi hiki — Satchel huishikilia kwenye kumbukumbu pekee na huisahau wakati wa kutoka.",
    switchMerchant: "Badili mfanyabiashara",
    unlock: "Fungua",
  },
  common: {
    cancel: "Ghairi",
    confirm: "Thibitisha",
    save: "Hifadhi",
    done: "Imekamilika",
    later: "Baadaye",
    retry: "Jaribu muunganisho tena",
  },
};
