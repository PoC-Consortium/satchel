// The Serbian (Српски) string bundle. Mirrors en.ts key-for-key; only values are translated.
import type { Bundle } from "./en";

export const sr: Bundle = {
  app: {
    name: "Satchel",
    tagline: "свопови без поверења",
  },
  // In-app update notifications (sidebar version badge + dialog).
  update: {
    title: "Доступно ажурирање",
    upToDate: "Имате најновију верзију",
    current: "Инсталирано",
    latest: "Најновије",
    notesTitle: "Белешке о издању",
    get: "Преузми ажурирање",
    dismiss: "Одбаци",
    close: "Затвори",
    badgeTooltip: "Доступно ажурирање — кликните за детаље",
    versionTooltip: "Кликните да проверите ажурирања",
  },
  // Risk disclaimer (first-run welcome + Settings → About).
  disclaimer: {
    title: "Самостално чување — ваши кључеви, ваша одговорност",
    body: "Satchel изводи некастодијалне атомске свопове: само ви држите своје кључеве, а семе трговца држи вруће транзитне кључеве док је своп у току. Своп протоколи (v1 HTLC и v2 Taproot/MuSig2) су прегледани и активни на mainnet-у. Под MIT лиценцом и испоручен у затеченом стању, без икакве гаранције — направите резервну копију своје фразе за опоравак и користите на сопствени ризик.",
  },
  nav: {
    public: "Јавно",
    corkboard: "Corkboard",
    postOffer: "Објави понуду",
    private: "Приватно",
    privateCreate: "Направи листић",
    privateReceive: "Прихвати листић",
    privateSlips: "Моји листићи",
    swaps: "Свопови",
    relays: "Релеји",
    wallets: "Новчаници",
    settings: "Подешавања",
    coins: "Новчићи",
  },
  makeOffer: {
    title: "Објави понуду",
    intro:
      "Објавите потписану понуду на Corkboard. Ништа се не закључава — то је само оглас; повуците је у било ком тренутку, а своп почиње тек када је неко прихвати и обе стране изврше уплату.",
    give: "Ви дајете",
    want: "Ви примате",
    // Canonical pair + direction: pick the pair, choose Sell/Buy the base, enter
    // the base amount and a quote-per-base price (invariant to direction).
    pair: "Пар",
    noPairs: "Нема пари за трговање — повежите бар два новчића у Подешавања → Новчићи.",
    sell: "Продај {sym}",
    buy: "Купи {sym}",
    amount: "Износ",
    youGive: "Ви дајете",
    youGet: "Ви добијате",
    price: "Цена",
    priceUnit: "{unit} по {base}",
    pricePlaceholder: "јединична цена",
    balance: "Стање: {amt} {sym}",
    balanceLoading: "Стање: …",
    noCoins: "Нема подешених новчића",
    sameCoin: "Новчићи које дајете и примате морају бити различити.",
    legDown: "Чвор једног од ових новчића не ради — покрените га (или проверите Подешавања → Новчићи) пре објаве.",
    // Swap-protocol pin (only offered when a pair+network supports more than
    // one). v2 label reuses coins.protoPrivate ("Private (Taproot)").
    protocol: "Тип свопа",
    protoStandard: "Стандардни (HTLC)",
    // Titles for the review/confirm dialog shown before posting (see OfferForm).
    reviewOfferTitle: "Прегледајте своју понуду",
    reviewSlipTitle: "Прегледајте свој листић",
    term: "Безбедносни временски закључ",
    termShort: "Кратак",
    termMedium: "Средњи",
    termLong: "Дуг",
    termHint: {
      short: "Кратак — средства се аутоматски враћају најбрже ако трговина застане (~12ч / 6ч), уз најмању безбедносну маргину.",
      medium: "Средњи — уравнотежен прозор за повраћај (~24ч / 12ч).",
      long: "Дуг (најбезбеднији) — највећа безбедносна маргина; аутоматски повраћај након ~36ч / 18ч ако трговина застане.",
    },
    validFor: "Важи (минута)",
    validForMins: "{mins} мин",
    validForHint:
      "Колико дуго понуда остаје приказана. Док сте онлајн, аутоматски се освежава; након тога истиче. Затварање апликације је повлачи.",
    note: "Понуда фиксне величине — ништа се не закључава док је неко не прихвати. Износи су on-chain; додатно плаћате мрежне накнаде, а Corkboard не наплаћује ништа. Временски закључ је прозор за аутоматски повраћај ако своп застане.",
    post: "Објави понуду",
    makeSlip: "Направи листић",
    slipTitle: "Ваш приватни листић са понудом",
    slipExplainer:
      "Пошаљите ово свом пријатељу. Они га налепе у Satchel да би га прихватили. Ништа се не закључава; истиче за {ttl}.",
    copy: "Копирај",
    copied: "Копирано",
    makeAnother: "Направи још један",
    myPrivateTitle: "Моје приватне понуде",
    myPrivateEmpty: "Нема активних приватних понуда.",
    privateExpires: "истиче {when}",
    privateExpired: "истекло",
    cancel: "Откажи",
    cancelTip: "Прекини поштовање овог листића — пријатељ који га још држи више не може да га прихвати.",
  },
  takeSlip: {
    open: "Налепи листић",
    title: "Прихвати приватну понуду",
    intro:
      "Пријатељ вам је послао приватни листић са понудом (почиње са pactoffer1:). Налепите га овде да бисте га прегледали и прихватили — баш као понуду са табле.",
    placeholder: "pactoffer1:…",
    take: "Прегледај и прихвати",
    invalid: "Ово не личи на листић — требало би да почиње са pactoffer1:.",
    previewLabel: "Овај листић нуди",
  },
  // PRIVATE nav group (off-market slips) — screen titles + intros. The form,
  // slip output, my-slips list and take flow reuse makeOffer.*/takeSlip.* copy.
  private: {
    createTitle: "Направи приватну понуду",
    createIntro:
      "Направите потписану понуду и предајте је пријатељу као листић преко сопственог чета. Ништа се нигде не приказује — и ништа се не закључава док обоје не извршите уплату.",
    slipsIntro:
      "Листићи које сте направили. Свако ко држи листић може да га прихвати док не истекне; откажите неки да бисте престали да га поштујете пре тога.",
    slipsEmptyBody: "Направите приватну понуду да добијете листић који можете послати пријатељу.",
    receiveTitle: "Прихвати приватну понуду",
    received: "Прихваћено — пратите у Своповима.",
  },
  // Shared take-confirmation dialog (board take + slip take).
  takeConfirm: {
    title: "Прихватити ову понуду?",
    confirm: "Прихвати понуду",
    counterparty: "Друга страна",
    youGive: "Ви дајете",
    youReceive: "Ви примате",
    safetyRefund: "Безбедносни повраћај",
    offerAge: "Старост понуде",
    makerFundsFirst:
      "Maker прво закључава свој {sym} — ви никада не шаљете први. И даље можете да откажете пре него што уплатите своју страну, а engine аутоматски враћа средства након безбедносног временског закључа ако своп застане.",
  },
  header: {
    activeMerchant: "Активни трговац — кликните да промените или управљате",
    manageMerchants: "Управљај трговцима…",
    noMerchant: "нема трговца",
    openMenu: "Отвори мени",
    collapseMenu: "скупи мени",
    settings: "Подешавања",
    language: "Језик",
    pactConnected: "Engine повезан",
    pactUnreachable: "Engine недоступан",
    liveSwapsOne: "1 своп у току — кликните за приказ",
    liveSwapsMany: "{count} свопова у току — кликните за приказ",
    liveSwapsNone: "Нема свопова у току",
    coinOk: "{name} — повезан · врх {tip}",
    coinUnconfigured: "{name} — није подешен",
    coinError: "{name} — {status}",
    relaysOk: "Nostr релеји — {up}/{total} повезано",
    relaysDown: "Nostr релеји — ниједан од {total} није повезан",
  },
  network: {
    mainnet: "MainNet",
    testnet: "TestNet",
    regtest: "RegTest",
    signet: "Signet",
    notRealFunds: "Нису права средства — ово је {network} мрежа",
  },
  // Watch-only mode: a viewer session with no coins. Browse the board and
  // withdraw your own offers, but no posting / taking / funding.
  watchOnly: {
    badge: "Само за преглед",
    badgeTip:
      "Режим само за преглед — прегледајте таблу и повлачите сопствене понуде, али не можете да објављујете, прихватате нити уплаћујете. Подесите новчиће у Подешавањима да бисте трговали.",
    coinWizardButton: "Прегледај у режиму само за преглед",
    coinWizardHint:
      "Прескочите подешавање новчића и само прегледајте таблу (само за читање). И даље можете повући сопствене понуде — згодно за уклањање понуда које је оставила нека друга сесија. Искључите у било ком тренутку у Подешавањима.",
    postBlockedTitle: "Режим само за преглед",
    postBlockedBody:
      "Ово је сесија само за преглед, па не може да објављује понуде. Подесите бар два новчића у Подешавања → Новчићи да бисте трговали.",
    takeBlockedBody: "Режим само за преглед — можете прегледати ову понуду, али за прихватање је потребно подесити новчиће.",
    takeBlockedTip: "Режим само за преглед — подесите новчиће у Подешавањима да бисте прихватали понуде.",
  },
  merchants: {
    title: "Ваши трговци",
    intro:
      "Трговац је један трговачки идентитет — са сопственим семеном и историјом свопова. Трговање под другим трговцем чини контексте неповезивим (привремени идентитет). Ваши главни новчићи живе у вашем сопственом новчанику, не овде.",
    // First-run welcome (empty merchant manager).
    welcomeTitle: "Добро дошли у Satchel",
    welcomeIntro:
      "Satchel тргује под „трговцем” — једним трговачким идентитетом са сопственим семеном. Још увек немате ниједан: направите нов, или увезите постојећу фразу за опоравак да бисте почели.",
    importMerchant: "Увези трговца",
    none: "Још увек нема трговаца.",
    active: "активан",
    switch: "промени",
    newMerchant: "Нови трговац",
    thisMerchant: "овај трговац",
    nameLabel: "Име трговца",
    namePlaceholder: "нпр. Главни",
    introFirst:
      "Подесите свој први трговачки идентитет („трговца”). Држи само вруће транзитне кључеве за свопове у току — ваши главни новчићи остају у вашем сопственом новчанику.",
    introNew: "Нови трговац је свеж, одвојен идентитет са сопственим семеном и историјом свопова.",
    // UI-5 merchant selector (phoenix wallet-selector parity).
    createNew: "Направи нов",
    import: "Увези",
    load: "Учитај трговца",
    loaded: "учитан",
    locked: "закључан",
    lockedTip: "Шифровано семе — откључајте лозинком када га учитате.",
    close: "Затвори",
    idLabel: "фасцикла",
    switching: "Мењање трговца…",
    switchingBody: "Поновно покретање engine-а за ту фасциклу.",
  },
  // Seed create/import (SeedForm) + counterparty + status.
  seed: {
    intro: "Направите потпуно ново семе, или увезите оно које већ имате.",
    createNew: "Направи нов",
    createDesc: "Генеришите ново семе. Ви правите резервну копију фразе за опоравак.",
    import: "Увези",
    importDesc: "Обновите из постојеће фразе од 12/24 речи.",
    recoveryLabel: "Фраза за опоравак",
    importPlaceholder: "реч1 реч2 реч3 …",
    encrypt: "Шифруј",
    encryptDesc:
      "Лозинка штити семе у мировању. Уносите је једном по сесији — Satchel је никада не чува. Напомена: ненадзирани аутоматски повраћај се паузира након поновног покретања док је поново не унесете.",
    noPassphrase: "Без лозинке (препоручено)",
    noPassphraseDesc:
      "Аутоматски повраћај наставља да ради кроз поновна покретања без ичега за уношење — ово је само вруће транзитно семе. Цена: приступ датотеци/хосту открива транзитне кључеве и идентитет овог трговца.",
    passphraseLabel: "Лозинка",
    passphrasePlaceholder: "изаберите лозинку",
    createTitle: "Направи семе",
    importTitle: "Увези семе",
    secureTitle: "Обезбеди {label}",
    revealTitle: "Запишите своју фразу за опоравак",
    revealBody:
      "Свако ко има ове речи контролише вруће кључеве овог трговца. Satchel не задржава копију — чувајте офлајн. Затим ћете потврдити неколико речи.",
    ackLabel: "Записао сам своју фразу за опоравак.",
    // Multi-step onboarding (create/import -> secret -> confirm -> passphrase).
    chooseTitle: "Подеси {label}",
    enterTitle: "Увезите своју фразу за опоравак",
    enterBody:
      "Унесите сваку реч — допуњавају се аутоматски како куцате — или налепите целу фразу. Проверавамо је пре него што наставите.",
    wordCount: "{n} речи",
    wordAria: "Реч {n}",
    checkIncomplete: "Унесите свих {n} речи.",
    checkUnknown: "Неке речи нису на BIP39 листи речи — проверите означене.",
    checkBadChecksum: "Контролни збир се не поклапа — поново проверите речи и њихов редослед.",
    checkOk: "Фраза за опоравак изгледа исправно.",
    verifyTitle: "Потврдите своју резервну копију",
    verifyBody: "Унесите речи на овим позицијама да бисте потврдили да сте записали фразу.",
    verifyWord: "Реч #{n}",
    verifyMismatch: "Те се не поклапају са вашом фразом — проверите своју резервну копију.",
    passphraseTitle: "Заштитите семе",
    passphraseBody:
      "Опционо шифрујте сачувано семе лозинком. Ово можете прескочити — погледајте компромис испод.",
  },
  counterparty: {
    you: "Ово сте ви",
    youShort: "ви",
    unknown: "непознат идентитет",
    // Short fingerprint fallback (identity.ts shortId) when no pubkey is known.
    unknownShort: "непознато",
  },
  status: {
    notConnectedTitle: "Није повезано са engine-ом",
    disconnectedBody:
      "Satchel не може да дође до engine-а. Можда се још покреће, или су везе чвора активног трговца недоступне. Покушајте поново, или промените трговца из селектора на врху.",
    openInSatchel: "Отвори ово у Satchel-у",
    noTauriBody:
      "Ово је Satchel-ов UI — потребан му је Tauri мост да дође до engine-а. Покрените десктоп апликацију (cargo tauri dev) уместо прегледача.",
  },
  settings: {
    title: "Подешавања",
    subtitle: "Поставке за целу апликацију за ову инсталацију.",
    // UI-3 Settings tabs.
    tabGeneral: "Опште",
    tabCoins: "Новчићи",
    tabNetwork: "Мрежа",
    tabAbout: "О апликацији",
    appearance: "Изглед",
    theme: "Тема",
    themeDark: "Тамна",
    themeLight: "Светла",
    themeSystem: "Системска",
    themeHint: "Изаберите како Satchel изгледа. Системска прати подешавање вашег ОС-а.",
    language: "Језик",
    languageHint: "Још језика стиже како се преводи доприносе.",
    mode: "Режим",
    watchOnly: "Режим само за преглед",
    watchOnlyHint:
      "Прегледајте таблу без подешавања новчића. И даље можете повући сопствене понуде, али не можете да објављујете, прихватате нити уплаћујете. Искључите да бисте трговали (биће вам потребна бар два повезана новчића).",
    network: "Мрежа",
    boards: "Corkboard-ови",
    boardsDesc:
      "Опционе самостално хостоване HTTP табле. Додајте било коју којој верујете; оставите празно да се ослоните на Nostr.",
    boardsNone: "Ниједна није подешена",
    nostrRelays: "Nostr релеји",
    nostrRelaysDesc:
      "Релеји преносе огласну таблу преко децентрализоване мреже — ниједан оператер не може да чита или упарује ваше понуде. Унапред подешени са подразумеваним скупом; уређујте слободно.",
    nostrRelaysOff: "Искључено — Nostr транспорт онемогућен",
    addUrl: "Додај",
    removeUrl: "Уклони",
    relayInvalid: "Унесите ws:// или wss:// URL релеја",
    boardInvalid: "Унесите http:// или https:// URL табле",
    netSave: "Сачувај и поново повежи",
    netSaving: "Чување и поновно повезивање…",
    netSaved: "Сачувано",
    // Fees tab — fee-bump policy (per active merchant).
    tabFees: "Накнаде",
    fees: "Подизање накнаде",
    feesScope: "Ова подешавања важе за активног трговца.",
    feesIntro:
      "Компромиси безбедност/трошак за подизање накнаде, није обавезно подешавање. Нове вредности важе за будућа подизања; свопови који су већ уплаћени задржавају политику под којом су уплаћени.",
    feeMax: "Макс. стопа накнаде (sat/vB)",
    feeMaxHint:
      "Горња граница за свако подизање накнаде. Подразумевано 500, такође и тврди системски максимум. Смањите да ограничите трошкове.",
    feeReservation: "Резервација за подизање при уплати (×)",
    feeReservationHint:
      "Колико провера средстава одваја као резерву за подизање накнаде. Веће спасава веће скокове накнаде, али веже више стања и одбија више свопова. Подразумевано 3.",
    feeCommitted: "Преобезбеђење за откуп (×)",
    feeCommittedHint:
      "Колико се додатно унапред плати v2 накнада за откуп тако да се потврди чак и када је Satchel затворен. Важи само за нове свопове. Подразумевано 2.",
    feeSave: "Сачувај",
    feeSaving: "Чување…",
    feeSaved: "Сачувано",
    feeReset: "Врати на подразумевано",
    coins: "Новчићи и чворови",
    coinsHint: "Повежите сваки новчић на сопствени чвор. Genesis се проверава пре него што се било шта сачува.",
    about: "О апликацији",
    version: "Верзија {version}",
    updateUpToDate: "Ажурно",
    updateCheckPlaceholder: "Провера ажурирања стиже у каснијем издању.",
    trustModel: "Где живе ваши кључеви",
    trustModelBody:
      "Тајне живе у engine-у, никада у Satchel-у. Семе трговца се налази у фасцикли података engine-а (шифровано или у обичном тексту — ваш избор); Satchel не чува ниједно семе нити лозинку. Семе је вруће по дизајну (само транзитни кључеви) — пребаците значајне приходе у сопствени хладни новчаник.",
  },
  coins: {
    intro:
      "Повежите сваки новчић на сопствени чвор. Први URL је сопствени новчаник вашег чвора — он уплаћује ваше делове свопа и прима приходе. Пре него што се било шта сачува, Satchel проверава genesis блок чвора тако да се средства никада не могу послати на погрешан ланац. Везе се деле између свих ваших трговаца.",
    networkBadge: "Подешавање за {network} мрежу",
    needMerchant:
      "Прво повежите трговца — подешавање новчића захтева да engine ради. Користите селектор трговца горе десно.",
    pairsTitle: "Парови за трговање",
    pairsHint:
      "Парови се изводе из онога што сваки новчић може да ради — не постоји фиксна листа. Пар се отвара када су оба његова новчића повезана.",
    noPairs: "Нема доступних парова.",
    notSetUp: "Није подешено",
    connectedTip: "Повезано · врх {tip}",
    connError: "Грешка везе",
    setUp: "Подеси",
    editConnection: "Уреди везу",
    remove: "уклони",
    disconnectTip: "Прекини везу овог новчића",
    disconnectTitle: "Прекинути везу {coin}?",
    disconnectBody: "Свопови којима је потребан неће бити доступни док поново не повежете.",
    ready: "Спреман за трговање",
    connectMissing: "Повежи {coins}",
    notBuildable: "Још се не може изградити",
    // Swap-protocol chips on a pair (pact-htlc-v1 HTLC vs pact-htlc-v2 adaptor).
    protoPrivate: "Приватни (Taproot)",
    protoPrivateTip: "Приватни своп (Taproot/MuSig2 адаптор) — изгледа као обична уплата on-chain",
    protoHtlcTip: "Класични HTLC своп",
    // Coin-setup backend choices (CoinSetup).
    backendCoreTitle: "Core RPC новчаник",
    backendCoreDesc: "Сопствени новчаник вашег чвора уплаћује своп и прима приходе.",
    backendHardwareTitle: "Хардвер",
    backendHardwareDesc: "Ledger / PSBT потписивање за део уплате.",
    backendLater: "касније",
    // CoinSetup dialog.
    setupTitle: "Повежи {coin}",
    setupIntro:
      "Усмерите Satchel ка сопственом {sym} чвору. Ништа се не чува док чвор не прође проверу genesis блока — ваша средства икада додирују само прави {sym} ланац.",
    backendUrlLabel: "URL(ови) позадинског дела чвора",
    backendUrlHint:
      "Први URL = сопствени новчаник вашег чвора (уплаћује свопове, прима приходе). Додајте Electrum сервере (tcp://host:port) после зареза за додатне, независне приказе ланца.",
    fundingWallet: "Новчаник за уплату",
    confirmationsLabel: "Потврде пре коначног",
    confirmationsHint:
      "Колико дубока уплата или откуп на овом ланцу мора бити пре него што своп реагује — маргина безбедности од реорга. Веће је безбедније али спорије; оставите празно за препоручену подразумевану вредност ({default}).",
    validateNode: "Провери чвор",
    checking: "Провера чвора…",
    genesisOk: "Genesis се поклапа — ово је прави ланац",
    genesisDetail: "висина врха {tip} · genesis {hash}…",
    genesisBad: "Одбијено — не чува се",
    errorShort: "грешка",
    // Structured connection form (CoinSetup v2).
    rpcHostLabel: "RPC хост",
    rpcPortLabel: "RPC порт",
    authMethodLabel: "Аутентификација",
    authCookie: "Cookie датотека",
    authCookieDesc: "Аутоматски читај .cookie чвора из његове фасцикле података (подразумевано, без чувања лозинке).",
    authUserPass: "Корисник / лозинка",
    authUserPassDesc: "rpcuser / rpcpassword из конфигурације вашег чвора — потребно за удаљени чвор.",
    rpcUserLabel: "RPC корисничко име",
    rpcPasswordLabel: "RPC лозинка",
    datadirLabel: "Фасцикла података чвора",
    cookiePathNote: "Cookie се чита из {path} у овој фасцикли.",
    walletLabel: "Име новчаника (опционо)",
    walletPlaceholder: "новчаник вашег чвора",
    needPort: "Прво унесите RPC порт.",
    validateFirst: "Проверите чвор пре чувања.",
    savingReconnecting: "Чување и поновно повезивање…",
    connected: "{coin} повезан",
    // Template picker (a coins.toml coin the engine version doesn't support).
    unsupportedByEngine: "Неподржан",
    unsupportedByEngineTip:
      "Овај новчић је дефинисан у coins.toml али није уграђен у ову верзију engine-а, па се њиме не може трговати.",
  },
  coinWizard: {
    title: "Повежите своје новчиће",
    intro:
      "Изаберите бар два новчића и усмерите сваки ка сопственом чвору. Свопу су потребна два ланца, па се трговање откључава када су два чвора повезана и активна. Новчиће можете додати или променити касније у Подешавањима.",
    progress: "{count} од {min} новчића повезано",
    continue: "Настави",
    live: "Активан",
    nodeDown: "Чвор не ради",
  },
  wallets: {
    intro:
      "Ово су новчаници ваших сопствених чворова (они које engine користи за уплату свопова и пријем прихода) — ваши кључеви, ваша машина. Satchel никада не држи ваше новчиће.",
    hotSeedNudge:
      "Ово је новчаник за трошење на врућем семену, не трезор — пребаците значајна стања у сопствени хладни/core новчаник.",
    notConnected: "Није повезано",
    notConnectedBody: "Прво повежите трговца — приказ новчаника захтева да engine ради.",
    noCoins: "Још нема подешених новчића",
    noCoinsBody: "Повежите новчић у Подешавања → Новчићи и његов новчаник се појављује овде.",
    goToCoins: "Иди на Новчиће",
    watchOnlyTitle: "Нема новчаника у режиму само за преглед",
    watchOnlyBody:
      "Ово је сесија само за преглед без повезаних новчића, па нема новчаника за приказ. Искључите режим само за преглед у Подешавањима и повежите новчић да бисте уплаћивали свопове.",
    walletName: "новчаник · {wallet}",
    walletScopedHint: "Сваки RPC за овај новчић је ограничен на овај новчаник чвора.",
    walletDefault: "подразумевани новчаник (без опсега)",
    walletDefaultHint:
      "За овај новчић није постављен новчаник, па RPC-ови користе подразумевани новчаник чвора. Поставите један у Подешавања → Новчићи да бисте сваки позив ограничили на одређени новчаник.",
    balanceLabel: "{symbol} стање",
    receive: "Прими",
    send: "Пошаљи",
    sendTo: "Пошаљи на адресу",
    amount: "Износ",
    sendTitle: "Послати {amount} {sym}?",
    sendConfirmBody: "На {to}\n\nОво троши из сопственог новчаника вашег чвора и не може се опозвати.",
  },
  corkboard: {
    noBoardTitle: "Нема повезаног Corkboard-а",
    noBoardBody:
      "Corkboard је заједничка огласна табла на коју makeri каче понуде. Никада не упарује трговине нити држи новчиће — усмерите Satchel ка некој којој верујете да бисте прегледали и објављивали.",
    noPairs: "Нема доступних парова",
    board: "Corkboard",
    boardSettings: "Подеси у Подешавањима",
    filterAll: "Све",
    filterMine: "Моје",
    offered: "{symbol} понуђено",
    noOffers: "Тренутно нема понуда које можете прихватити",
    noOffersBody:
      "Понуде се појављују овде чим maker објави неку за пар који сте подесили. Можете и сами објавити своју.",
    hiddenOffers:
      "{count} додатних понуда за парове које нисте повезали. Подесите оба новчића да бисте њима трговали:",
    yourOffer: "ваша понуда",
    offerStaged: "објављивање…",
    offerStagedTip:
      "Објављено са овог уређаја и чека потврду назад са релеја. Оглашава се; постаје активно када га релеј одјекне.",
    take: "Прихвати понуду",
    legDown: "Чвор једног новчића из овог пара не ради — покрените га (или проверите Подешавања → Новчићи) пре прихватања.",
    withdraw: "Повуци",
    withdrawTip: "Повуците одмах — понуда никада не закључава средства",
    safetyRefund: "безбедносни повраћај",
    safetyRefundTip:
      "Ако своп застане, обе стране добијају аутоматски повраћај — део takera се прво откључава, ваш мало касније. Нико не остаје заглављен.",
    activeTitle: "Ваши активни свопови",
    states: {
      open: "отворено",
      takenByUs: "прихватили сте",
      revoked: "повучено",
      expired: "истекло",
    },
    // Two-sided order-book view of the Corkboard. Bids = makers giving the
    // quote coin to get the base; asks = the reverse. The ladder is a way to
    // READ the board — it never matches or prioritises (load-bearing).
    book: {
      bids: "Понуде за куповину",
      asks: "Понуде за продају",
      bidsHint: "желе {base} · плаћају {quote}",
      asksHint: "продају {base} · за {quote}",
      price: "Цена",
      size: "Величина",
      noBids: "Нема понуда за куповину",
      noAsks: "Нема понуда за продају",
      spread: "Распон {pct}",
      spreadOneSided: "Једнострано",
      crossed: "укрштено",
      crossedTip: "Највиша куповна ≥ најнижа продајна. Табла никада не упарује аутоматски, па ове преклапајуће понуде само стоје — прихватите било коју страну.",
      mid: "средина {price}",
      levelOffers: "{count} понуда по овој цени — изаберите једну за прихватање",
      depthTip: "Укупно {sym} у понуди по овој цени кроз {count} огласа.",
      takerNote: "Прихватањем дајете {give} и примате {get}.",
      selectLevel: "Изаберите ниво цене изнад да видите понуде на њему.",
      paneHeader: "{size} {base} @ {price} {unit}",
      denomTip: "Јединица приказа за {coin} износе",
      showMore: "Прикажи још {count}",
      showLess: "Прикажи првих {count}",
    },
  },
  relays: {
    title: "Релеји",
    subtitle: "Активна повезаност са вашим Nostr релејима — мрежа преко које путују ваше понуде и прихватања. Додајте или уклоните релеје у Подешавања → Мрежа.",
    connectedCount: "{up} / {total} повезано",
    refresh: "Освежи",
    ms: "{ms} ms",
    up: "горе",
    down: "доле",
    statsTip: "{success}/{attempts} успешних повезивања · ↓{down} ↑{up}",
    none: "Нема подешених релеја",
    noneBody: "Додајте Nostr релеј у Подешавања → Мрежа да бисте објављивали и примали понуде преко мреже.",
    goToNetwork: "Иди на Подешавања",
    notConnected: "Није повезано",
    notConnectedBody: "Приказ релеја захтева да engine ради — прво повежите трговца.",
  },
  swaps: {
    title: "Свопови",
    hint: "Ваш потпуни регистар — свопови у току на врху, завршене трговине испод. Можете деловати на активне свопове и са Corkboard-а.",
    activeTitle: "У току",
    historyTitle: "Историја",
    none: "Још нема свопова — прихватите понуду на Corkboard-у.",
    // Active-swaps dock action buttons + the refund-time label.
    cancel: "откажи",
    refund: "повраћај",
    dump: "избаци логове",
    dumpHint: "Копирајте дијагностички пакет без тајни (стање + редови лога) за овај своп, да налепите програмерима.",
    dumpCopied: "Дијагностика копирана — налепите програмерима.",
    dumpFailed: "Није могуће копирати дијагностички пакет.",
    refundAt: "повраћај {when}",
    // Confirm dialogs for acting on a live swap (ActiveSwaps).
    cancelTitle: "Отказати овај своп?",
    cancelConfirm: "Откажи своп",
    cancelKeep: "Задржи",
    // Abort reason recorded on the swap when cancelled from Satchel.
    cancelReason: "отказано у Satchel-у",
    cancelBody:
      "Ово напушта своп пре него што сте уплатили. Ништа ваше још није закључано, па не губите ништа — понуда се само неће завршити.",
    refundTitle: "Повући своја средства назад?",
    refundConfirm: "Повраћај",
    refundBody:
      "Безбедносни временски закључ је прошао, па можете повратити средства која сте закључали. Ово сада емитује ваш повраћај; engine то такође ради аутоматски након рока.",
    col: {
      swap: "своп",
      role: "улога",
      state: "стање",
      amounts: "даје → прима",
      when: "када",
      finalTx: "коначна тx",
    },
    // Expandable per-leg on-chain detail (the audit trail). We show both
    // funding txs + OUR settlement; never the counterparty's settlement or the
    // swap secret.
    audit: {
      toggle: "Прикажи on-chain детаље",
      title: "On-chain детаљи",
      youLocked: "ви сте закључали",
      theyLocked: "они су закључали",
      funding: "Уплата",
      received: "Примљено",
      refunded: "Враћено",
      pending: "још није on-chain",
      copy: "Копирај ид трансакције",
      copied: "Ид трансакције копиран",
    },
  },
  fees: {
    title: "Преглед мрежног трошка",
    estimated: "процењено",
    provisionalNote: "Ова pactd верзија још не излаже процену накнаде.",
    summary: "Своп су 2 on-chain трансакције које плаћате: уплата на ланцу који дајете, откуп на ланцу који примате.",
    fallbackTip: "Чвор је био недоступан, па је коришћена конзервативна подразумевана стопа накнаде — третирајте ово као процену.",
    ifItStalls: "(ако застане)",
  },
  funds: {
    insufficient:
      "Нема довољно {sym} за уплату овог свопа — потребно ~{need} {sym} (износ + накнада за уплату), новчаник има {have} {sym}.",
  },
  wizard: {
    welcome: "Добро дошли у Satchel",
    connectTitle: "Повежи Pact engine",
    connectIntro:
      "Satchel је танак клијент Pact engine-а — језгра које држи ваше кључеве и изводи свопове. Изаберите како да дођете до њега.",
    managed: "Покрени уграђени Pact engine",
    managedDesc: "Satchel покреће и надзире сопствени Pact engine. Препоручено.",
    external: "Повежи се на спољни Pact engine",
    externalDesc: "Усмерите ка Pact engine-у који већ покрећете (поставите SATCHEL_PACTD_URL + cookie пре покретања).",
    externalNote:
      "Спољни режим се бира преко променљивих окружења пре покретања Satchel-а. Поново покрените са постављеним SATCHEL_PACTD_URL да бисте га користили.",
    coinsTitle: "Додај своје новчиће",
    coinsIntro:
      "Након што се ваш трговац направи, повежите сваки новчић на сопствени чвор у Подешавања → Новчићи. Изаберите новчић и позадински део (јавни Electrum за нула подешавања, или сопствени чвор); genesis се проверава са овом мрежом пре него што се било шта сачува.",
    coinsTemplatesSoon: "Шаблони новчића на један клик стижу овде у каснијем издању.",
    back: "Назад",
    continue: "Настави",
    finish: "Заврши подешавање",
  },
  // UI-4 docked activity log.
  log: {
    title: "Активност",
    empty: "— дневник активности —",
    count: "{count} редова",
    collapse: "Скупи дневник",
    expand: "Прошири дневник",
    // Activity-log lines emitted by the frontend. `{err}` carries a raw engine
    // message (itself not translated — it comes from pactd over the wire).
    noTauri: "не ради унутар Satchel-а — овом UI-ју је потребан Tauri мост",
    startupError: "покретање: {err}",
    notConnected: "није повезано: {err}",
    connected: "повезано са pactd {version} ({protocol})",
    listcoinsError: "listcoins: {err}",
    watchOnlyError: "само за преглед: {err}",
    switchedMerchant: "пребачено на трговца {id}",
    switchMerchantError: "промена трговца: {err}",
    loadMerchantError: "учитавање трговца: {err}",
    merchantCreated: "трговац {id} направљен",
    merchantReady: "трговац спреман",
    actionOk: "{action} {id}: ок",
    actionError: "{action} {id}: {err}",
    diagCopied: "дијагностика за {id} копирана ({count} редова лога) — налепите програмерима",
    dumpError: "избацивање {id}: {err}",
    coinDisconnected: "{coin} прекинута веза",
    removeCoinError: "уклањање новчића: {err}",
    tookOffer: "прихваћена понуда {id} — сада се појављује у вашим активним своповима испод",
    takeError: "прихватање: {err}",
    offerWithdrawn: "понуда {id} повучена",
    withdrawError: "повлачење: {err}",
    postedOffer: "објављена понуда {id} — повуците у било ком тренутку; ништа није закључано",
    createdSlip: "направљен приватни листић са понудом — пошаљите га свом пријатељу",
    tookPrivateOffer: "прихваћена приватна понуда {id} — сада се појављује у вашим активним своповима",
    cancelledPrivateOffer: "отказана приватна понуда {id}",
    cancelError: "отказивање: {err}",
    noticeboardUpdated: "огласна табла ажурирана",
    feePolicyUpdated: "политика накнаде ажурирана",
  },
  // Relative-time + freshness prose from format.ts (rendered via the tr() mirror,
  // since those are pure non-component helpers). Unit letters stay inside the
  // template so a translation owns the whole phrase.
  format: {
    ageUnknown: "старост непозната",
    justNow: "управо сада",
    minutesAgo: "пре {n}м",
    hoursAgo: "пре {n}ч",
    daysAgo: "пре {n}д",
    expiryNow: "сада",
    expirySoon: "ускоро",
    inMinutes: "за ~{n}м",
    inHours: "за ~{n}ч",
    inDays: "за ~{n}д",
    posted: "објављено {age}",
    expires: "истиче {time}",
  },
  // Plain-language swap story per (role, state) — the honest "who is exposed
  // when" framing shown on every active swap. {a}/{b} are coin tickers; {t1}/{t2}
  // are local refund times. Rendered via tr() (narrate() is a pure helper).
  narrate: {
    initiating:
      "Прихватање послато — чека се да maker покрене своп. Ништа још није закључано; отказује се само од себе ако не одговоре.",
    created: "Понуда послата — чека се да се друга страна сложи. Ништа није обавезано.",
    acceptedMaker: "Услови договорени. Следеће: закључајте свој {a}. Док не уплатите, и даље можете слободно да откажете.",
    acceptedTaker: "Услови договорени. Друга страна прво закључава свој {a} — ви никада не шаљете први.",
    noncesExchanged:
      "Подешавање приватног свопа — размена материјала за потписивање. Ништа још није закључано.",
    signedMaker:
      "Обе стране су потписале. Ваш daemon закључава {a}, затим аутоматски преузима {b}. Ако нешто застане, ваш {a} се враћа у {t1}.",
    signedTaker:
      "Обе стране су потписале. Ваш daemon закључава {b} и преузима {a} оног тренутка када се друга страна помери. Сигурносна мрежа: повраћај у {t2}.",
    fundedAMaker:
      "Ваш {a} је закључан. Чека се да друга страна закључа свој {b}. Ако то никада не учине, ваш {a} се аутоматски враћа у {t1}.",
    fundedATaker:
      "Њихов {a} је закључан и проверен. Следеће: закључајте свој {b}. Сигурносна мрежа: аутоматски повраћај у {t2} ако нешто застане.",
    fundedBMaker: "Обоје закључано. Ваш daemon преузима {b} чим се безбедно потврди.",
    fundedBTaker: "Обоје закључано. Ваш daemon ће преузети {a} оног тренутка када друга страна узме свој {b}.",
    redeemedB:
      "Преузели сте {b} — чека се да се потврди. Ваш закључани {a} остаје заштићен док ово не буде коначно.",
    completed: "Своп завршен — {coin} је у вашем новчанику.",
    refunded: "Своп се није завршио, па се ваш {coin} аутоматски вратио. Ништа изгубљено осим накнада.",
    aborted: "Отказано пре него што се иједан новац померио.",
  },
  exit: {
    // Exit-gate dialog (fund safety, C6). The engine manages alone, so "keep
    // running" detaches it (it keeps watching timelocks + servicing offers).
    liveTitle: "Своп је у току",
    liveBodyOne:
      "1 своп је у току. Управљан је on-chain временским закључима — engine мора да настави да ради да би извршио откуп или повраћај пре рока.",
    liveBodyMany:
      "{count} свопова је у току. Управљани су on-chain временским закључима — engine мора да настави да ради да би извршио откуп или повраћај пре рока.",
    keepRunningExplain:
      "Затварање прозора оставља engine да ради у позадини, тако да завршава своп без интерфејса. Можете поново отворити Satchel у било ком тренутку да проверите.",
    forceQuitWarn: "Принудно затварање сада зауставља engine и може изгубити средства.",
    // {word} is the confirm word below; a translation may localize both together.
    typeToConfirm: "Да бисте ипак принудно затворили, унесите {word} испод.",
    confirmWord: "QUIT",
    keepRunning: "Настави да ради, затвори прозор",
    keepWithdraw: "Настави да ради + повуци понуде",
    keepLeaveOffers: "Настави да ради, остави понуде",
    forceQuit: "Принудно затвори",
    offersTitle: "Имате објављене понуде",
    offersBodyOne:
      "1 ваша понуда је још на Corkboard-у. Понуде ништа не закључавају, али остављање значи да друге стране могу да је прихвате док је Satchel затворен — engine ће опслужити прихватање.",
    offersBodyMany:
      "{count} ваших понуда је још на Corkboard-у. Понуде ништа не закључавају, али остављање значи да друге стране могу да их прихвате док је Satchel затворен — engine ће опслужити прихватања.",
    withdrawExit: "Повуци све и изађи",
  },
  unlock: {
    title: "Откључај трговца",
    body:
      "Семе овог трговца је шифровано. Унесите његову лозинку да га откључате за ову сесију — Satchel га држи само у меморији и заборавља при изласку.",
    switchMerchant: "Промени трговца",
    unlock: "Откључај",
  },
  common: {
    cancel: "Откажи",
    confirm: "Потврди",
    save: "Сачувај",
    done: "Готово",
    later: "Касније",
    retry: "Покушај поново везу",
  },
};
