// The Hindi (हिंदी) string bundle. Mirrors en.ts key-for-key; only values are translated.
import type { Bundle } from "./en";

export const hi: Bundle = {
  app: {
    name: "Satchel",
    tagline: "भरोसा-रहित (trustless) स्वैप",
  },
  // In-app update notifications (sidebar version badge + dialog).
  update: {
    title: "अपडेट उपलब्ध है",
    upToDate: "आप अप-टू-डेट हैं",
    current: "इंस्टॉल किया हुआ",
    latest: "नवीनतम",
    notesTitle: "रिलीज़ नोट्स",
    get: "अपडेट प्राप्त करें",
    dismiss: "खारिज करें",
    close: "बंद करें",
    badgeTooltip: "अपडेट उपलब्ध है — विवरण के लिए क्लिक करें",
    versionTooltip: "अपडेट जाँचने के लिए क्लिक करें",
  },
  // Risk disclaimer (first-run welcome + Settings → About).
  disclaimer: {
    title: "स्व-अभिरक्षा (self-custody) — आपकी keys, आपकी ज़िम्मेदारी",
    body: "Satchel नॉन-कस्टोडियल atomic swap करता है: keys केवल आपके पास रहती हैं, और स्वैप के दौरान एक merchant का seed hot ट्रांज़िट keys रखता है। स्वैप protocol (v1 HTLC और v2 Taproot/MuSig2) की समीक्षा हो चुकी है और ये mainnet पर लाइव हैं। MIT-लाइसेंस के तहत, बिना किसी वारंटी के, जैसा है वैसा ही प्रदान किया गया — अपना recovery phrase बैक अप करें और अपने जोखिम पर उपयोग करें।",
  },
  nav: {
    public: "सार्वजनिक",
    corkboard: "Corkboard",
    postOffer: "एक offer पोस्ट करें",
    private: "निजी",
    privateCreate: "slip बनाएँ",
    privateReceive: "एक slip लें",
    privateSlips: "मेरी slips",
    swaps: "स्वैप",
    relays: "Relays",
    wallets: "Wallets",
    settings: "सेटिंग्स",
    coins: "Coins",
  },
  makeOffer: {
    title: "एक offer पोस्ट करें",
    intro:
      "Corkboard पर एक हस्ताक्षरित offer पोस्ट करें। कुछ भी लॉक नहीं होता — यह केवल एक विज्ञापन है; कभी भी वापस लें, और स्वैप तभी शुरू होता है जब कोई इसे लेता है और दोनों पक्ष फंडिंग करते हैं।",
    give: "आप देते हैं",
    want: "आप प्राप्त करते हैं",
    // Canonical pair + direction: pick the pair, choose Sell/Buy the base, enter
    // the base amount and a quote-per-base price (invariant to direction).
    pair: "Pair",
    noPairs: "कोई व्यापार-योग्य pair नहीं — Settings → Coins में कम से कम दो coins जोड़ें।",
    sell: "{sym} बेचें",
    buy: "{sym} खरीदें",
    amount: "राशि",
    youGive: "आप देते हैं",
    youGet: "आप पाते हैं",
    price: "कीमत",
    priceUnit: "{unit} प्रति {base}",
    pricePlaceholder: "इकाई कीमत",
    balance: "बैलेंस: {amt} {sym}",
    balanceLoading: "बैलेंस: …",
    noCoins: "कोई coins कॉन्फ़िगर नहीं हैं",
    sameCoin: "देने और पाने वाले coins अलग होने चाहिए।",
    legDown: "इन coins में से एक का node बंद है — पोस्ट करने से पहले उसे शुरू करें (या Settings → Coins जाँचें)।",
    // Swap-protocol pin (only offered when a pair+network supports more than
    // one). v2 label reuses coins.protoPrivate ("Private (Taproot)").
    protocol: "स्वैप का प्रकार",
    protoStandard: "मानक (HTLC)",
    // Titles for the review/confirm dialog shown before posting (see OfferForm).
    reviewOfferTitle: "अपने offer की समीक्षा करें",
    reviewSlipTitle: "अपनी slip की समीक्षा करें",
    term: "सुरक्षा timelock",
    termShort: "छोटा",
    termMedium: "मध्यम",
    termLong: "लंबा",
    termHint: {
      short: "छोटा — यदि व्यापार अटक जाए तो funds सबसे तेज़ी से auto-refund हो जाते हैं (~12 घं. / 6 घं.), सबसे छोटे सुरक्षा मार्जिन के साथ।",
      medium: "मध्यम — संतुलित refund विंडो (~24 घं. / 12 घं.)।",
      long: "लंबा (सबसे सुरक्षित) — सबसे चौड़ा सुरक्षा मार्जिन; व्यापार अटकने पर ~36 घं. / 18 घं. बाद auto-refund।",
    },
    validFor: "मान्यता अवधि (मिनट)",
    validForMins: "{mins} मिनट",
    validForHint:
      "offer कितने समय तक सूचीबद्ध रहता है। जब तक आप ऑनलाइन हैं यह अपने आप ताज़ा रखा जाता है; इसके बाद यह समाप्त हो जाता है। ऐप बंद करने पर यह वापस ले लिया जाता है।",
    note: "निश्चित-आकार का offer — कोई इसे लेने तक कुछ भी लॉक नहीं होता। राशियाँ on-chain हैं; आप ऊपर से network fees चुकाते हैं और Corkboard कुछ भी शुल्क नहीं लेता। timelock स्वैप अटकने पर auto-refund विंडो है।",
    post: "offer पोस्ट करें",
    makeSlip: "slip बनाएँ",
    slipTitle: "आपकी निजी offer slip",
    slipExplainer:
      "इसे अपने मित्र को भेजें। वे इसे लेने के लिए Satchel में पेस्ट करते हैं। कुछ भी लॉक नहीं होता; यह {ttl} में समाप्त हो जाती है।",
    copy: "कॉपी करें",
    copied: "कॉपी हो गया",
    makeAnother: "एक और बनाएँ",
    myPrivateTitle: "मेरे निजी offers",
    myPrivateEmpty: "कोई बकाया निजी offers नहीं।",
    privateExpires: "{when} समाप्त होती है",
    privateExpired: "समाप्त हो गई",
    cancel: "रद्द करें",
    cancelTip: "इस slip को मानना बंद करें — जिस मित्र के पास यह अभी भी है, वह इसे नहीं ले पाएगा।",
  },
  takeSlip: {
    open: "एक slip पेस्ट करें",
    title: "एक निजी offer लें",
    intro:
      "एक मित्र ने आपको एक निजी offer slip भेजी है (यह pactoffer1: से शुरू होती है)। समीक्षा करने और लेने के लिए इसे यहाँ पेस्ट करें — बिल्कुल बोर्ड के किसी offer की तरह।",
    placeholder: "pactoffer1:…",
    take: "समीक्षा करें और लें",
    invalid: "यह slip जैसी नहीं दिखती — इसे pactoffer1: से शुरू होना चाहिए।",
    previewLabel: "यह slip प्रदान करती है",
  },
  // PRIVATE nav group (off-market slips) — screen titles + intros. The form,
  // slip output, my-slips list and take flow reuse makeOffer.*/takeSlip.* copy.
  private: {
    createTitle: "एक निजी offer बनाएँ",
    createIntro:
      "एक हस्ताक्षरित offer बनाएँ और इसे अपनी निजी चैट पर एक slip के रूप में किसी मित्र को सौंप दें। कहीं भी कुछ सूचीबद्ध नहीं होता — और जब तक आप दोनों फंडिंग न करें, कुछ भी लॉक नहीं होता।",
    slipsIntro:
      "आपके द्वारा बनाई गई slips। slip रखने वाला कोई भी व्यक्ति इसके समाप्त होने तक इसे ले सकता है; उससे पहले मानना बंद करने के लिए किसी को रद्द करें।",
    slipsEmptyBody: "एक slip पाने के लिए एक निजी offer बनाएँ जिसे आप किसी मित्र को भेज सकें।",
    receiveTitle: "एक निजी offer लें",
    received: "ले लिया गया — इसे Swaps में फ़ॉलो करें।",
  },
  // Shared take-confirmation dialog (board take + slip take).
  takeConfirm: {
    title: "यह offer लें?",
    confirm: "offer लें",
    counterparty: "प्रतिपक्ष (Counterparty)",
    youGive: "आप देते हैं",
    youReceive: "आप प्राप्त करते हैं",
    safetyRefund: "सुरक्षा refund",
    offerAge: "offer की आयु",
    makerFundsFirst:
      "maker पहले अपने {sym} लॉक करता है — आप कभी पहले नहीं भेजते। अपनी ओर से फंडिंग करने से पहले आप अब भी रद्द कर सकते हैं, और स्वैप अटकने पर engine सुरक्षा timelock के बाद auto-refund कर देता है।",
  },
  header: {
    activeMerchant: "सक्रिय merchant — स्विच या प्रबंधित करने के लिए क्लिक करें",
    manageMerchants: "Merchants प्रबंधित करें…",
    noMerchant: "कोई merchant नहीं",
    openMenu: "मेन्यू खोलें",
    collapseMenu: "मेन्यू संक्षिप्त करें",
    settings: "सेटिंग्स",
    language: "भाषा",
    pactConnected: "engine कनेक्टेड",
    pactUnreachable: "engine तक नहीं पहुँचा जा सका",
    liveSwapsOne: "1 स्वैप चालू — देखने के लिए क्लिक करें",
    liveSwapsMany: "{count} स्वैप चालू — देखने के लिए क्लिक करें",
    liveSwapsNone: "कोई स्वैप चालू नहीं",
    coinOk: "{name} — कनेक्टेड · tip {tip}",
    coinUnconfigured: "{name} — सेट अप नहीं किया गया",
    coinError: "{name} — {status}",
    relaysOk: "Nostr relays — {up}/{total} कनेक्टेड",
    relaysDown: "Nostr relays — {total} में से कोई कनेक्टेड नहीं",
  },
  network: {
    mainnet: "MainNet",
    testnet: "TestNet",
    regtest: "RegTest",
    signet: "Signet",
    notRealFunds: "असली funds नहीं — यह {network} network है",
  },
  // Watch-only mode: a viewer session with no coins. Browse the board and
  // withdraw your own offers, but no posting / taking / funding.
  watchOnly: {
    badge: "केवल देखें",
    badgeTip:
      "केवल-देखें मोड — बोर्ड ब्राउज़ करें और अपने offers वापस लें, लेकिन आप पोस्ट, ले या फंड नहीं कर सकते। व्यापार के लिए Settings में coins सेट अप करें।",
    coinWizardButton: "केवल-देखें मोड में ब्राउज़ करें",
    coinWizardHint:
      "coin सेटअप छोड़ें और बस बोर्ड ब्राउज़ करें (read-only)। आप अब भी अपने offers वापस ले सकते हैं — किसी अन्य सत्र द्वारा छोड़े गए offers हटाने के लिए सुविधाजनक। इसे Settings में कभी भी बंद करें।",
    postBlockedTitle: "केवल-देखें मोड",
    postBlockedBody:
      "यह एक केवल-देखें सत्र है, इसलिए यह offers पोस्ट नहीं कर सकता। व्यापार के लिए Settings → Coins में कम से कम दो coins सेट अप करें।",
    takeBlockedBody: "केवल-देखें मोड — आप इस offer की समीक्षा कर सकते हैं, लेकिन इसे लेने के लिए coins सेट अप होने चाहिए।",
    takeBlockedTip: "केवल-देखें मोड — offers लेने के लिए Settings में coins सेट अप करें।",
  },
  merchants: {
    title: "आपके merchants",
    intro:
      "एक merchant एक व्यापारिक पहचान है — इसका अपना seed और स्वैप इतिहास होता है। किसी अलग merchant के तहत व्यापार करने से संदर्भ अलग-अलग बने रहते हैं (एक बर्नर पहचान)। आपके मुख्य coins आपके अपने wallet में रहते हैं, यहाँ नहीं।",
    // First-run welcome (empty merchant manager).
    welcomeTitle: "Satchel में आपका स्वागत है",
    welcomeIntro:
      "Satchel एक “merchant” के तहत व्यापार करता है — एक व्यापारिक पहचान जिसका अपना seed होता है। आपके पास अभी कोई नहीं है: शुरू करने के लिए एक नया बनाएँ, या एक मौजूदा recovery phrase आयात करें।",
    importMerchant: "एक merchant आयात करें",
    none: "अभी कोई merchants नहीं।",
    active: "सक्रिय",
    switch: "स्विच करें",
    newMerchant: "नया merchant",
    thisMerchant: "यह merchant",
    nameLabel: "Merchant नाम",
    namePlaceholder: "जैसे Main",
    introFirst:
      "अपनी पहली व्यापारिक पहचान (एक “merchant”) सेट अप करें। यह केवल चालू स्वैप के लिए hot ट्रांज़िट keys रखता है — आपके मुख्य coins आपके अपने wallet में रहते हैं।",
    introNew: "एक नया merchant एक नई, अलग पहचान है जिसका अपना seed और स्वैप इतिहास होता है।",
    // UI-5 merchant selector (phoenix wallet-selector parity).
    createNew: "नया बनाएँ",
    import: "आयात करें",
    load: "Merchant लोड करें",
    loaded: "लोड किया गया",
    locked: "लॉक्ड",
    lockedTip: "एन्क्रिप्टेड seed — लोड करते समय अपने passphrase से अनलॉक करें।",
    close: "बंद करें",
    idLabel: "फ़ोल्डर",
    switching: "Merchant स्विच किया जा रहा है…",
    switchingBody: "उस फ़ोल्डर के विरुद्ध engine पुनः लॉन्च किया जा रहा है।",
  },
  // Seed create/import (SeedForm) + counterparty + status.
  seed: {
    intro: "एक बिल्कुल नया seed बनाएँ, या जो आपके पास पहले से है उसे आयात करें।",
    createNew: "नया बनाएँ",
    createDesc: "एक नया seed जनरेट करें। आप recovery phrase का बैकअप लेते हैं।",
    import: "आयात करें",
    importDesc: "एक मौजूदा 12/24-शब्द वाले phrase से पुनर्स्थापित करें।",
    recoveryLabel: "Recovery phrase",
    importPlaceholder: "word1 word2 word3 …",
    encrypt: "एन्क्रिप्ट करें",
    encryptDesc:
      "एक passphrase seed को निष्क्रिय अवस्था में सुरक्षित रखता है। आप इसे प्रति सत्र एक बार दर्ज करते हैं — Satchel इसे कभी संग्रहीत नहीं करता। ध्यान दें: पुनरारंभ के बाद आपके इसे फिर से दर्ज करने तक अनअटेंडेड auto-refund रुक जाता है।",
    noPassphrase: "कोई passphrase नहीं (अनुशंसित)",
    noPassphraseDesc:
      "रिबूट के दौरान auto-refund बिना कुछ दर्ज किए काम करता रहता है — यह केवल एक hot ट्रांज़िट seed है। लागत: फ़ाइल/होस्ट तक पहुँच इस merchant की ट्रांज़िट keys + पहचान को उजागर करती है।",
    passphraseLabel: "Passphrase",
    passphrasePlaceholder: "एक passphrase चुनें",
    createTitle: "seed बनाएँ",
    importTitle: "seed आयात करें",
    secureTitle: "{label} सुरक्षित करें",
    revealTitle: "अपना recovery phrase लिख लें",
    revealBody:
      "इन शब्दों वाला कोई भी व्यक्ति इस merchant की hot keys को नियंत्रित करता है। Satchel कोई प्रति नहीं रखता — इसे ऑफ़लाइन संग्रहीत करें। आगे आप कुछ शब्दों की पुष्टि करेंगे।",
    ackLabel: "मैंने अपना recovery phrase लिख लिया है।",
    // Multi-step onboarding (create/import -> secret -> confirm -> passphrase).
    chooseTitle: "{label} सेट अप करें",
    enterTitle: "अपना recovery phrase आयात करें",
    enterBody:
      "हर शब्द टाइप करें — जैसे-जैसे आप बढ़ते हैं वे अपने आप पूरे होते हैं — या पूरा phrase पेस्ट करें। आगे बढ़ने से पहले हम इसे जाँचते हैं।",
    wordCount: "{n} शब्द",
    wordAria: "शब्द {n}",
    checkIncomplete: "सभी {n} शब्द दर्ज करें।",
    checkUnknown: "कुछ शब्द BIP39 wordlist में नहीं हैं — हाइलाइट किए गए शब्दों की जाँच करें।",
    checkBadChecksum: "Checksum मेल नहीं खाता — अपने शब्दों और उनके क्रम की फिर से जाँच करें।",
    checkOk: "Recovery phrase मान्य लगता है।",
    verifyTitle: "अपने बैकअप की पुष्टि करें",
    verifyBody: "यह पुष्टि करने के लिए कि आपने phrase लिख लिया है, इन स्थानों पर शब्द टाइप करें।",
    verifyWord: "शब्द #{n}",
    verifyMismatch: "ये आपके phrase से मेल नहीं खाते — अपना बैकअप जाँचें।",
    passphraseTitle: "seed को सुरक्षित करें",
    passphraseBody:
      "वैकल्पिक रूप से संग्रहीत seed को एक passphrase से एन्क्रिप्ट करें। आप इसे छोड़ सकते हैं — नीचे ट्रेड-ऑफ़ देखें।",
  },
  counterparty: {
    you: "यह आप हैं",
    youShort: "आप",
    unknown: "अज्ञात पहचान",
    // Short fingerprint fallback (identity.ts shortId) when no pubkey is known.
    unknownShort: "अज्ञात",
  },
  status: {
    notConnectedTitle: "engine से कनेक्ट नहीं है",
    disconnectedBody:
      "Satchel engine तक नहीं पहुँच पा रहा। यह अब भी शुरू हो रहा हो सकता है, या सक्रिय merchant के node कनेक्शन बंद हो सकते हैं। पुनः प्रयास करें, या ऊपर सेलेक्टर से merchant स्विच करें।",
    openInSatchel: "इसे Satchel में खोलें",
    noTauriBody:
      "यह Satchel का UI है — engine तक पहुँचने के लिए इसे Tauri ब्रिज की आवश्यकता है। ब्राउज़र के बजाय डेस्कटॉप ऐप (cargo tauri dev) लॉन्च करें।",
  },
  settings: {
    title: "सेटिंग्स",
    subtitle: "इस इंस्टॉल के लिए ऐप-व्यापी प्राथमिकताएँ।",
    // UI-3 Settings tabs.
    tabGeneral: "सामान्य",
    tabCoins: "Coins",
    tabNetwork: "Network",
    tabAbout: "बारे में",
    appearance: "दिखावट",
    theme: "थीम",
    themeDark: "डार्क",
    themeLight: "लाइट",
    themeSystem: "सिस्टम",
    themeHint: "चुनें कि Satchel कैसा दिखता है। सिस्टम आपकी OS सेटिंग का अनुसरण करता है।",
    language: "भाषा",
    languageHint: "जैसे-जैसे अनुवाद योगदान किए जाते हैं और भाषाएँ जुड़ती जाती हैं।",
    mode: "मोड",
    watchOnly: "केवल-देखें मोड",
    watchOnlyHint:
      "coins सेट अप किए बिना बोर्ड ब्राउज़ करें। आप अब भी अपने offers वापस ले सकते हैं, लेकिन पोस्ट, ले या फंड नहीं कर सकते। व्यापार के लिए बंद करें (आपको कम से कम दो coins कनेक्ट करने होंगे)।",
    network: "Network",
    boards: "Corkboards",
    boardsDesc:
      "वैकल्पिक स्व-होस्टेड HTTP बोर्ड। जिन पर आप भरोसा करते हैं उन्हें जोड़ें; Nostr पर निर्भर रहने के लिए खाली छोड़ दें।",
    boardsNone: "कोई कॉन्फ़िगर नहीं",
    nostrRelays: "Nostr relays",
    nostrRelaysDesc:
      "Relays एक विकेंद्रीकृत network पर noticeboard को ले जाते हैं — कोई ऑपरेटर आपके offers को पढ़ या मिलान नहीं कर सकता। एक डिफ़ॉल्ट सेट के साथ पहले से जुड़ा हुआ; स्वतंत्र रूप से संपादित करें।",
    nostrRelaysOff: "बंद — Nostr ट्रांसपोर्ट अक्षम",
    addUrl: "जोड़ें",
    removeUrl: "हटाएँ",
    relayInvalid: "एक ws:// या wss:// relay URL दर्ज करें",
    boardInvalid: "एक http:// या https:// बोर्ड URL दर्ज करें",
    netSave: "सहेजें और पुनः कनेक्ट करें",
    netSaving: "सहेजा जा रहा है और पुनः कनेक्ट हो रहा है…",
    netSaved: "सहेजा गया",
    // Fees tab — fee-bump policy (per active merchant).
    tabFees: "Fees",
    fees: "Fee बढ़ाना (bumping)",
    feesScope: "ये सेटिंग्स सक्रिय merchant पर लागू होती हैं।",
    feesIntro:
      "fee bumps के लिए सुरक्षा/लागत ट्रेड-ऑफ़, आवश्यक सेटअप नहीं। नए मान भविष्य के bumps पर लागू होते हैं; पहले से फंडेड स्वैप उसी policy को बनाए रखते हैं जिसके तहत वे फंड हुए थे।",
    feeMax: "अधिकतम feerate (sat/vB)",
    feeMaxHint:
      "हर fee bump के लिए सीमा। डिफ़ॉल्ट 500, जो सिस्टम की कठोर अधिकतम सीमा भी है। लागत सीमित करने के लिए इसे कम करें।",
    feeReservation: "Funding bump आरक्षण (×)",
    feeReservationHint:
      "funds चेक bump हेडरूम के रूप में जितना बैलेंस अलग रखता है। अधिक बड़े fee स्पाइक से बचाता है पर अधिक बैलेंस बाँधता है और अधिक स्वैप अस्वीकार करता है। डिफ़ॉल्ट 3।",
    feeCommitted: "Redeem ओवर-प्रोविज़न (×)",
    feeCommittedHint:
      "v2 redeem fee कितनी अतिरिक्त पहले से चुकाई जाती है ताकि Satchel बंद होने पर भी यह कन्फ़र्म हो। केवल नए स्वैप पर लागू। डिफ़ॉल्ट 2।",
    feeSave: "सहेजें",
    feeSaving: "सहेजा जा रहा है…",
    feeSaved: "सहेजा गया",
    feeReset: "डिफ़ॉल्ट पर रीसेट करें",
    coins: "Coins और nodes",
    coinsHint: "हर coin को अपने node से कनेक्ट करें। कुछ भी सहेजने से पहले Genesis जाँचा जाता है।",
    about: "बारे में",
    version: "संस्करण {version}",
    updateUpToDate: "अप-टू-डेट",
    updateCheckPlaceholder: "अपडेट जाँच बाद की रिलीज़ में आएगी।",
    trustModel: "आपकी keys कहाँ रहती हैं",
    trustModelBody:
      "Secrets engine में रहते हैं, कभी Satchel में नहीं। merchant seed engine के डेटा फ़ोल्डर में रहता है (एन्क्रिप्टेड या प्लेनटेक्स्ट — आपकी पसंद); Satchel कोई seed या passphrase संग्रहीत नहीं करता। seed डिज़ाइन के अनुसार hot है (केवल ट्रांज़िट keys) — बड़ी आय को अपने cold wallet में स्वीप करें।",
  },
  coins: {
    intro:
      "हर coin को अपने node से कनेक्ट करें। पहला URL आपके node का अपना wallet है — यह आपके स्वैप legs को फंड करता है और आय प्राप्त करता है। कुछ भी सहेजने से पहले, Satchel node के genesis block की जाँच करता है ताकि funds कभी गलत chain पर न भेजे जाएँ। कनेक्शन आपके सभी merchants में साझा होते हैं।",
    networkBadge: "{network} network के लिए कॉन्फ़िगर किया जा रहा है",
    needMerchant:
      "पहले एक merchant कनेक्ट करें — coin सेटअप के लिए engine का चलना आवश्यक है। ऊपर दाईं ओर merchant सेलेक्टर का उपयोग करें।",
    pairsTitle: "व्यापार जोड़े (Pairs)",
    pairsHint:
      "Pairs इस आधार पर निकाले जाते हैं कि हर coin क्या कर सकता है — कोई निश्चित सूची नहीं है। एक pair तभी खुलती है जब इसके दोनों coins कनेक्ट हो जाते हैं।",
    noPairs: "कोई pairs उपलब्ध नहीं।",
    notSetUp: "सेट अप नहीं किया गया",
    connectedTip: "कनेक्टेड · tip {tip}",
    connError: "कनेक्शन त्रुटि",
    setUp: "सेट अप करें",
    editConnection: "कनेक्शन संपादित करें",
    remove: "हटाएँ",
    disconnectTip: "इस coin को डिस्कनेक्ट करें",
    disconnectTitle: "{coin} डिस्कनेक्ट करें?",
    disconnectBody: "जब तक आप पुनः कनेक्ट नहीं करते, इसकी ज़रूरत वाले स्वैप उपलब्ध नहीं होंगे।",
    ready: "व्यापार के लिए तैयार",
    connectMissing: "{coins} कनेक्ट करें",
    notBuildable: "अभी निर्माण-योग्य नहीं",
    // Swap-protocol chips on a pair (pact-htlc-v1 HTLC vs pact-htlc-v2 adaptor).
    protoPrivate: "Private (Taproot)",
    protoPrivateTip: "निजी स्वैप (Taproot/MuSig2 adaptor) — on-chain एक सामान्य भुगतान जैसा दिखता है",
    protoHtlcTip: "क्लासिक HTLC स्वैप",
    // Coin-setup backend choices (CoinSetup).
    backendCoreTitle: "Core RPC wallet",
    backendCoreDesc: "आपके node का अपना wallet स्वैप को फंड करता है और आय प्राप्त करता है।",
    backendHardwareTitle: "Hardware",
    backendHardwareDesc: "funding leg के लिए Ledger / PSBT साइनिंग।",
    backendLater: "बाद में",
    // CoinSetup dialog.
    setupTitle: "{coin} कनेक्ट करें",
    setupIntro:
      "Satchel को अपने स्वयं के {sym} node पर इंगित करें। node के genesis-block चेक पास करने तक कुछ भी सहेजा नहीं जाता — आपके funds केवल असली {sym} chain को ही छूते हैं।",
    backendUrlLabel: "Node backend URL(s)",
    backendUrlHint:
      "पहला URL = आपके node का अपना wallet (स्वैप फंड करता है, आय प्राप्त करता है)। अतिरिक्त, स्वतंत्र chain व्यू के लिए कॉमा के बाद Electrum सर्वर (tcp://host:port) जोड़ें।",
    fundingWallet: "Funding wallet",
    confirmationsLabel: "अंतिम होने से पहले confirmations",
    confirmationsHint:
      "इस chain पर एक स्वैप के कार्य करने से पहले funding या redeem कितनी गहरी होनी चाहिए — reorg-सुरक्षा मार्जिन। अधिक सुरक्षित पर धीमा; अनुशंसित डिफ़ॉल्ट ({default}) के लिए खाली छोड़ें।",
    validateNode: "Node सत्यापित करें",
    checking: "node जाँचा जा रहा है…",
    genesisOk: "Genesis मेल खाया — यह सही chain है",
    genesisDetail: "tip ऊँचाई {tip} · genesis {hash}…",
    genesisBad: "अस्वीकृत — सहेजा नहीं जा रहा",
    errorShort: "त्रुटि",
    // Structured connection form (CoinSetup v2).
    rpcHostLabel: "RPC host",
    rpcPortLabel: "RPC port",
    authMethodLabel: "प्रमाणीकरण",
    authCookie: "Cookie फ़ाइल",
    authCookieDesc: "node की .cookie उसकी डेटा डायरेक्टरी से अपने आप पढ़ें (डिफ़ॉल्ट, कोई पासवर्ड संग्रहीत नहीं)।",
    authUserPass: "उपयोगकर्ता / पासवर्ड",
    authUserPassDesc: "आपके node के कॉन्फ़िग से rpcuser / rpcpassword — एक रिमोट node के लिए आवश्यक।",
    rpcUserLabel: "RPC उपयोगकर्ता नाम",
    rpcPasswordLabel: "RPC पासवर्ड",
    datadirLabel: "Node डेटा डायरेक्टरी",
    cookiePathNote: "cookie इस डायरेक्टरी के अंतर्गत {path} से पढ़ी जाती है।",
    walletLabel: "Wallet नाम (वैकल्पिक)",
    walletPlaceholder: "आपके node का wallet",
    needPort: "पहले RPC port दर्ज करें।",
    validateFirst: "सहेजने से पहले node सत्यापित करें।",
    savingReconnecting: "सहेजा जा रहा है और पुनः कनेक्ट हो रहा है…",
    connected: "{coin} कनेक्टेड",
    // Template picker (a coins.toml coin the engine version doesn't support).
    unsupportedByEngine: "असमर्थित",
    unsupportedByEngineTip:
      "यह coin coins.toml में परिभाषित है पर engine के इस संस्करण में निर्मित नहीं है, इसलिए इसका व्यापार नहीं किया जा सकता।",
  },
  coinWizard: {
    title: "अपने coins कनेक्ट करें",
    intro:
      "कम से कम दो coins चुनें और हर एक को अपने node पर इंगित करें। एक स्वैप के लिए दो chains चाहिए, इसलिए दो nodes कनेक्ट और लाइव होने पर व्यापार खुल जाता है। आप बाद में Settings में coins जोड़ या बदल सकते हैं।",
    progress: "{min} में से {count} coins कनेक्टेड",
    continue: "जारी रखें",
    live: "लाइव",
    nodeDown: "Node बंद",
  },
  wallets: {
    intro:
      "ये आपके अपने nodes के wallets हैं (वे जिन्हें engine स्वैप फंड करने और आय प्राप्त करने के लिए उपयोग करता है) — आपकी keys, आपकी मशीन। Satchel कभी आपके coins नहीं रखता।",
    hotSeedNudge:
      "यह एक hot seed पर एक खर्च-योग्य wallet है, vault नहीं — बड़ी राशियों को अपने cold/core wallet में स्वीप करें।",
    notConnected: "कनेक्ट नहीं है",
    notConnectedBody: "पहले एक merchant कनेक्ट करें — wallet व्यू के लिए engine का चलना आवश्यक है।",
    noCoins: "अभी कोई coins सेट अप नहीं",
    noCoinsBody: "Settings → Coins में एक coin कनेक्ट करें और इसका wallet यहाँ दिखाई देगा।",
    goToCoins: "Coins पर जाएँ",
    watchOnlyTitle: "केवल-देखें मोड में कोई wallets नहीं",
    watchOnlyBody:
      "यह बिना किसी coins के एक केवल-देखें सत्र है, इसलिए दिखाने के लिए कोई wallets नहीं हैं। Settings में केवल-देखें बंद करें और स्वैप फंड करने के लिए एक coin कनेक्ट करें।",
    walletName: "wallet · {wallet}",
    walletScopedHint: "इस coin के लिए हर RPC इस node wallet तक सीमित है।",
    walletDefault: "डिफ़ॉल्ट wallet (सीमित नहीं)",
    walletDefaultHint:
      "इस coin के लिए कोई wallet सेट नहीं है, इसलिए RPCs node के डिफ़ॉल्ट wallet का उपयोग करते हैं। हर कॉल को एक विशिष्ट wallet तक सीमित करने के लिए Settings → Coins में एक सेट करें।",
    balanceLabel: "{symbol} बैलेंस",
    receive: "प्राप्त करें",
    send: "भेजें",
    sendTo: "पते पर भेजें",
    amount: "राशि",
    sendTitle: "{amount} {sym} भेजें?",
    sendConfirmBody: "{to} को\n\nयह आपके अपने node के wallet से खर्च होता है और इसे पूर्ववत नहीं किया जा सकता।",
  },
  corkboard: {
    noBoardTitle: "कोई Corkboard कनेक्ट नहीं",
    noBoardBody:
      "Corkboard एक साझा बुलेटिन बोर्ड है जहाँ makers offers पिन करते हैं। यह कभी व्यापारों का मिलान नहीं करता या coins नहीं रखता — ब्राउज़ और पोस्ट करने के लिए Satchel को एक भरोसेमंद बोर्ड पर इंगित करें।",
    noPairs: "कोई pairs उपलब्ध नहीं",
    board: "Corkboard",
    boardSettings: "Settings में कॉन्फ़िगर करें",
    filterAll: "सभी",
    filterMine: "मेरे",
    offered: "{symbol} प्रस्तावित",
    noOffers: "अभी आप कोई offer नहीं ले सकते",
    noOffersBody:
      "जैसे ही कोई maker आपके द्वारा सेट अप किए गए किसी pair के लिए offer पोस्ट करता है, offers यहाँ दिखाई देते हैं। आप अपना खुद का भी पोस्ट कर सकते हैं।",
    hiddenOffers:
      "उन pairs के लिए {count} और offer(s) जिन्हें आपने कनेक्ट नहीं किया। उनका व्यापार करने के लिए दोनों coins सेट अप करें:",
    yourOffer: "आपका offer",
    offerStaged: "पोस्ट हो रहा है…",
    offerStagedTip:
      "इस डिवाइस से पोस्ट किया गया और एक relay से वापस पुष्टि की प्रतीक्षा में। यह विज्ञापित हो रहा है; relay द्वारा इसे प्रतिध्वनित करते ही यह लाइव हो जाता है।",
    take: "offer लें",
    legDown: "इस pair के nodes में से एक बंद है — लेने से पहले उसे शुरू करें (या Settings → Coins जाँचें)।",
    withdraw: "वापस लें",
    withdrawTip: "तुरंत वापस लें — एक offer कभी funds लॉक नहीं करता",
    safetyRefund: "सुरक्षा refund",
    safetyRefundTip:
      "यदि स्वैप अटक जाता है, तो दोनों पक्ष auto-refund होते हैं — taker का leg पहले अनलॉक होता है, आपका थोड़ा बाद में। कोई फँसा नहीं रहता।",
    activeTitle: "आपके सक्रिय स्वैप",
    states: {
      open: "खुला",
      takenByUs: "आपके द्वारा लिया गया",
      revoked: "वापस लिया गया",
      expired: "समाप्त",
    },
    // Two-sided order-book view of the Corkboard. Bids = makers giving the
    // quote coin to get the base; asks = the reverse. The ladder is a way to
    // READ the board — it never matches or prioritises (load-bearing).
    book: {
      bids: "Bids",
      asks: "Asks",
      bidsHint: "{base} चाहिए · {quote} दे रहे",
      asksHint: "{base} बेच रहे · {quote} के लिए",
      price: "कीमत",
      size: "आकार",
      noBids: "कोई bids नहीं",
      noAsks: "कोई asks नहीं",
      spread: "Spread {pct}",
      spreadOneSided: "एकतरफ़ा",
      crossed: "क्रॉस्ड",
      crossedTip: "शीर्ष bid ≥ शीर्ष ask। बोर्ड कभी auto-match नहीं करता, इसलिए ये अतिव्यापी offers बस पड़े रहते हैं — कोई भी पक्ष लें।",
      mid: "mid {price}",
      levelOffers: "इस कीमत पर {count} offer(s) — लेने के लिए एक चुनें",
      depthTip: "{count} नोटिस में इस कीमत पर प्रस्तावित कुल {sym}।",
      takerNote: "इसे लेने पर, आप {give} देते हैं और {get} प्राप्त करते हैं।",
      selectLevel: "वहाँ के offers देखने के लिए ऊपर एक कीमत स्तर चुनें।",
      paneHeader: "{size} {base} @ {price} {unit}",
      denomTip: "{coin} राशियों के लिए प्रदर्शन इकाई",
      showMore: "{count} और दिखाएँ",
      showLess: "शीर्ष {count} दिखाएँ",
    },
  },
  relays: {
    title: "Relays",
    subtitle: "आपके Nostr relays से लाइव कनेक्टिविटी — वह network जिस पर आपके offers और takes यात्रा करते हैं। Settings → Network में relays जोड़ें या हटाएँ।",
    connectedCount: "{up} / {total} कनेक्टेड",
    refresh: "रिफ़्रेश करें",
    ms: "{ms} ms",
    up: "अप",
    down: "डाउन",
    statsTip: "{success}/{attempts} सफल कनेक्ट · ↓{down} ↑{up}",
    none: "कोई relays कॉन्फ़िगर नहीं",
    noneBody: "network पर offers प्रकाशित और प्राप्त करने के लिए Settings → Network में एक Nostr relay जोड़ें।",
    goToNetwork: "Settings पर जाएँ",
    notConnected: "कनेक्ट नहीं है",
    notConnectedBody: "relay व्यू के लिए engine का चलना आवश्यक है — पहले एक merchant कनेक्ट करें।",
  },
  swaps: {
    title: "स्वैप",
    hint: "आपका पूरा ledger — चालू स्वैप ऊपर, समाप्त व्यापार नीचे। आप Corkboard से भी लाइव स्वैप पर कार्य कर सकते हैं।",
    activeTitle: "चालू",
    historyTitle: "इतिहास",
    none: "अभी कोई स्वैप नहीं — Corkboard पर एक offer लें।",
    // Active-swaps dock action buttons + the refund-time label.
    cancel: "रद्द करें",
    refund: "refund",
    dump: "लॉग डंप करें",
    dumpHint: "इस स्वैप के लिए एक secret-मुक्त डायग्नोस्टिक्स बंडल (state + लॉग लाइनें) कॉपी करें, डेवलपर्स को पेस्ट करने के लिए।",
    dumpCopied: "डायग्नोस्टिक्स कॉपी हो गए — डेवलपर्स को पेस्ट करें।",
    dumpFailed: "डायग्नोस्टिक्स बंडल कॉपी नहीं हो सका।",
    refundAt: "refund {when}",
    // Confirm dialogs for acting on a live swap (ActiveSwaps).
    cancelTitle: "इस स्वैप को रद्द करें?",
    cancelConfirm: "स्वैप रद्द करें",
    cancelKeep: "इसे रखें",
    // Abort reason recorded on the swap when cancelled from Satchel.
    cancelReason: "Satchel में रद्द किया गया",
    cancelBody:
      "यह फंडिंग से पहले स्वैप को छोड़ देता है। अभी आपका कुछ भी लॉक नहीं है, इसलिए आप कुछ नहीं खोते — offer बस पूरा नहीं होगा।",
    refundTitle: "अपने funds वापस खींचें?",
    refundConfirm: "Refund",
    refundBody:
      "सुरक्षा timelock बीत चुका है, इसलिए आप अपने लॉक किए funds वापस ले सकते हैं। यह अभी आपका refund प्रसारित करता है; engine डेडलाइन के बाद इसे अपने आप भी करता है।",
    col: {
      swap: "स्वैप",
      role: "भूमिका",
      state: "स्थिति",
      amounts: "देता है → प्राप्त करता है",
      when: "कब",
      finalTx: "अंतिम tx",
    },
    // Expandable per-leg on-chain detail (the audit trail). We show both
    // funding txs + OUR settlement; never the counterparty's settlement or the
    // swap secret.
    audit: {
      toggle: "On-chain विवरण दिखाएँ",
      title: "On-chain विवरण",
      youLocked: "आपने लॉक किया",
      theyLocked: "उन्होंने लॉक किया",
      funding: "Funding",
      received: "प्राप्त",
      refunded: "Refund किया गया",
      pending: "अभी on-chain नहीं",
      copy: "transaction id कॉपी करें",
      copied: "Transaction id कॉपी हो गया",
    },
  },
  fees: {
    title: "Network लागत पूर्वावलोकन",
    estimated: "अनुमानित",
    provisionalNote: "यह pactd बिल्ड अभी fee अनुमान उजागर नहीं करता।",
    summary: "एक स्वैप 2 on-chain transactions हैं जिनका आप भुगतान करते हैं: give-chain पर funding, receive-chain पर redeem।",
    fallbackTip: "एक node तक नहीं पहुँचा जा सका, इसलिए एक रूढ़िवादी डिफ़ॉल्ट fee दर का उपयोग किया गया — इन्हें एक अनुमान मानें।",
    ifItStalls: "(यदि यह अटक जाए)",
  },
  funds: {
    insufficient:
      "इस स्वैप को फंड करने के लिए पर्याप्त {sym} नहीं — ~{need} {sym} चाहिए (राशि + funding fee), wallet में {have} {sym} है।",
  },
  wizard: {
    welcome: "Satchel में आपका स्वागत है",
    connectTitle: "Pact engine कनेक्ट करें",
    connectIntro:
      "Satchel Pact engine का एक पतला क्लाइंट है — वह कोर जो आपकी keys रखता है और स्वैप चलाता है। चुनें कि इस तक कैसे पहुँचना है।",
    managed: "बिल्ट-इन Pact engine चलाएँ",
    managedDesc: "Satchel अपना खुद का Pact engine लॉन्च और निगरानी करता है। अनुशंसित।",
    external: "एक बाहरी Pact engine से कनेक्ट करें",
    externalDesc: "जो Pact engine आप पहले से चलाते हैं उस पर इंगित करें (लॉन्च से पहले SATCHEL_PACTD_URL + cookie सेट करें)।",
    externalNote:
      "बाहरी मोड Satchel लॉन्च करने से पहले environment variables के ज़रिए चुना जाता है। इसका उपयोग करने के लिए SATCHEL_PACTD_URL सेट करके पुनः लॉन्च करें।",
    coinsTitle: "अपने coins जोड़ें",
    coinsIntro:
      "आपका merchant बनने के बाद, Settings → Coins में हर coin को अपने node से कनेक्ट करें। एक coin और एक backend चुनें (शून्य-सेटअप के लिए सार्वजनिक Electrum, या आपका अपना node); कुछ भी सहेजने से पहले genesis को इस network के विरुद्ध जाँचा जाता है।",
    coinsTemplatesSoon: "वन-क्लिक coin टेम्पलेट बाद की रिलीज़ में यहाँ आएँगे।",
    back: "वापस",
    continue: "जारी रखें",
    finish: "सेटअप पूरा करें",
  },
  // UI-4 docked activity log.
  log: {
    title: "गतिविधि",
    empty: "— गतिविधि लॉग —",
    count: "{count} लाइनें",
    collapse: "लॉग संक्षिप्त करें",
    expand: "लॉग विस्तृत करें",
    // Activity-log lines emitted by the frontend. `{err}` carries a raw engine
    // message (itself not translated — it comes from pactd over the wire).
    noTauri: "Satchel के अंदर नहीं चल रहा — इस UI को Tauri ब्रिज की आवश्यकता है",
    startupError: "startup: {err}",
    notConnected: "कनेक्ट नहीं: {err}",
    connected: "pactd {version} से कनेक्टेड ({protocol})",
    listcoinsError: "listcoins: {err}",
    watchOnlyError: "केवल-देखें: {err}",
    switchedMerchant: "merchant {id} पर स्विच किया गया",
    switchMerchantError: "merchant स्विच करें: {err}",
    loadMerchantError: "merchant लोड करें: {err}",
    merchantCreated: "merchant {id} बनाया गया",
    merchantReady: "merchant तैयार",
    actionOk: "{action} {id}: ok",
    actionError: "{action} {id}: {err}",
    diagCopied: "{id} के लिए डायग्नोस्टिक्स कॉपी हो गए ({count} लॉग लाइनें) — devs को पेस्ट करें",
    dumpError: "dump {id}: {err}",
    coinDisconnected: "{coin} डिस्कनेक्टेड",
    removeCoinError: "coin हटाएँ: {err}",
    tookOffer: "offer {id} लिया गया — यह अब नीचे आपके सक्रिय स्वैप में दिखाई देता है",
    takeError: "take: {err}",
    offerWithdrawn: "offer {id} वापस लिया गया",
    withdrawError: "withdraw: {err}",
    postedOffer: "offer {id} पोस्ट किया गया — कभी भी वापस लें; कुछ भी लॉक नहीं है",
    createdSlip: "एक निजी offer slip बनाई गई — इसे अपने मित्र को भेजें",
    tookPrivateOffer: "निजी offer {id} लिया गया — यह अब आपके सक्रिय स्वैप में दिखाई देता है",
    cancelledPrivateOffer: "निजी offer {id} रद्द किया गया",
    cancelError: "cancel: {err}",
    noticeboardUpdated: "noticeboard अपडेट किया गया",
    feePolicyUpdated: "fee policy अपडेट की गई",
  },
  // Relative-time + freshness prose from format.ts (rendered via the tr() mirror,
  // since those are pure non-component helpers). Unit letters stay inside the
  // template so a translation owns the whole phrase.
  format: {
    ageUnknown: "आयु अज्ञात",
    justNow: "अभी-अभी",
    minutesAgo: "{n} मि. पहले",
    hoursAgo: "{n} घं. पहले",
    daysAgo: "{n} दि. पहले",
    expiryNow: "अभी",
    expirySoon: "जल्द ही",
    inMinutes: "~{n} मि. में",
    inHours: "~{n} घं. में",
    inDays: "~{n} दि. में",
    posted: "{age} पोस्ट किया गया",
    expires: "{time} समाप्त होता है",
  },
  // Plain-language swap story per (role, state) — the honest "who is exposed
  // when" framing shown on every active swap. {a}/{b} are coin tickers; {t1}/{t2}
  // are local refund times. Rendered via tr() (narrate() is a pure helper).
  narrate: {
    initiating:
      "Take भेजा गया — maker द्वारा स्वैप शुरू करने की प्रतीक्षा में। अभी कुछ भी लॉक नहीं है; यदि वे जवाब नहीं देते तो यह अपने आप रद्द हो जाता है।",
    created: "Offer भेजा गया — दूसरे पक्ष की सहमति की प्रतीक्षा में। कुछ भी प्रतिबद्ध नहीं है।",
    acceptedMaker: "शर्तें तय हुईं। आगे: अपना {a} लॉक करें। फंडिंग करने तक आप अब भी स्वतंत्र रूप से रद्द कर सकते हैं।",
    acceptedTaker: "शर्तें तय हुईं। दूसरा पक्ष पहले अपना {a} लॉक करता है — आप कभी पहले नहीं भेजते।",
    noncesExchanged:
      "निजी स्वैप सेट किया जा रहा है — साइनिंग सामग्री का आदान-प्रदान। अभी कुछ भी लॉक नहीं है।",
    signedMaker:
      "दोनों पक्षों ने हस्ताक्षर किए। आपका daemon {a} लॉक करता है, फिर {b} अपने आप क्लेम करता है। यदि कुछ अटकता है, तो आपका {a} {t1} पर वापस आ जाता है।",
    signedTaker:
      "दोनों पक्षों ने हस्ताक्षर किए। आपका daemon {b} लॉक करता है और दूसरे पक्ष के आगे बढ़ते ही {a} क्लेम करता है। सुरक्षा जाल: {t2} पर refund।",
    fundedAMaker:
      "आपका {a} लॉक है। दूसरे पक्ष द्वारा अपना {b} लॉक करने की प्रतीक्षा में। यदि वे कभी नहीं करते, तो आपका {a} {t1} पर अपने आप वापस आ जाता है।",
    fundedATaker:
      "उनका {a} लॉक और सत्यापित है। आगे: अपना {b} लॉक करें। सुरक्षा जाल: कुछ अटकने पर {t2} पर स्वचालित refund।",
    fundedBMaker: "दोनों लॉक हुए। आपका daemon {b} को सुरक्षित रूप से कन्फ़र्म होते ही क्लेम कर लेता है।",
    fundedBTaker: "दोनों लॉक हुए। दूसरे पक्ष द्वारा अपना {b} लेते ही आपका daemon {a} क्लेम करेगा।",
    redeemedB:
      "आपने {b} क्लेम किया — इसके कन्फ़र्म होने की प्रतीक्षा में। यह अंतिम होने तक आपका लॉक किया {a} सुरक्षित रहता है।",
    completed: "स्वैप पूरा — {coin} आपके wallet में है।",
    refunded: "स्वैप पूरा नहीं हुआ, इसलिए आपका {coin} अपने आप वापस आ गया। fees के अलावा कुछ नहीं खोया।",
    aborted: "किसी पैसे के हिलने से पहले रद्द किया गया।",
  },
  progress: {
    awaitingLock: "उनके लॉक की प्रतीक्षा",
    awaitingClaim: "उनके दावे की प्रतीक्षा",
    theirLock: "उनका लॉक पुष्टि हो रहा",
    securing: "आपके {coin} सुरक्षित कर रहे",
    blocks: "+{n} ब्लॉक",
    feeBumped: "शुल्क बढ़ाया गया",
    reorg: "रीऑर्ग पाया गया — फिर से जाँच",
  },
  exit: {
    // Exit-gate dialog (fund safety, C6). The engine manages alone, so "keep
    // running" detaches it (it keeps watching timelocks + servicing offers).
    liveTitle: "एक स्वैप चालू है",
    liveBodyOne:
      "1 स्वैप मध्य-प्रवाह में है। यह on-chain timelocks द्वारा नियंत्रित है — डेडलाइन से पहले redeem या refund करने के लिए engine का चलते रहना ज़रूरी है।",
    liveBodyMany:
      "{count} स्वैप मध्य-प्रवाह में हैं। ये on-chain timelocks द्वारा नियंत्रित हैं — डेडलाइन से पहले redeem या refund करने के लिए engine का चलते रहना ज़रूरी है।",
    keepRunningExplain:
      "विंडो बंद करने पर engine पृष्ठभूमि में चलता रहता है, इसलिए यह स्वैप को बिना UI के पूरा करता है। आप इसकी जाँच के लिए Satchel कभी भी फिर से खोल सकते हैं।",
    forceQuitWarn: "अभी ज़बरदस्ती बंद करने से engine रुक जाता है और funds खो सकते हैं।",
    // {word} is the confirm word below; a translation may localize both together.
    typeToConfirm: "फिर भी ज़बरदस्ती बंद करने के लिए, नीचे {word} टाइप करें।",
    confirmWord: "QUIT",
    keepRunning: "चलते रहने दें, विंडो बंद करें",
    keepWithdraw: "चलते रहने दें + offers वापस लें",
    keepLeaveOffers: "चलते रहने दें, offers बने रहने दें",
    forceQuit: "ज़बरदस्ती बंद करें",
    offersTitle: "आपके offers पोस्ट किए गए हैं",
    offersBodyOne:
      "आपका 1 offer अभी भी Corkboard पर है। Offers कुछ भी लॉक नहीं करते, पर इसे बने रहने देने का मतलब है कि Satchel बंद रहते हुए भी प्रतिपक्ष इसे ले सकते हैं — engine उस take की सेवा करेगा।",
    offersBodyMany:
      "आपके {count} offers अभी भी Corkboard पर हैं। Offers कुछ भी लॉक नहीं करते, पर इन्हें बने रहने देने का मतलब है कि Satchel बंद रहते हुए भी प्रतिपक्ष इन्हें ले सकते हैं — engine उन takes की सेवा करेगा।",
    withdrawExit: "सभी वापस लें और बाहर निकलें",
  },
  unlock: {
    title: "Merchant अनलॉक करें",
    body:
      "इस merchant का seed एन्क्रिप्टेड है। इस सत्र के लिए इसे अनलॉक करने हेतु इसका passphrase दर्ज करें — Satchel इसे केवल मेमोरी में रखता है और बाहर निकलने पर भूल जाता है।",
    switchMerchant: "Merchant स्विच करें",
    unlock: "अनलॉक करें",
  },
  common: {
    cancel: "रद्द करें",
    confirm: "पुष्टि करें",
    save: "सहेजें",
    done: "पूर्ण",
    later: "बाद में",
    retry: "कनेक्शन पुनः प्रयास करें",
  },
};
