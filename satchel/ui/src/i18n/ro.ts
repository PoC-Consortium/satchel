// The Romanian (Română) string bundle. Mirrors en.ts key-for-key; only values are translated.
import type { Bundle } from "./en";

export const ro: Bundle = {
  app: {
    name: "Satchel",
  },
  // In-app update notifications (sidebar version badge + dialog).
  update: {
    title: "Actualizare disponibilă",
    upToDate: "Ești la zi",
    current: "Instalată",
    latest: "Cea mai recentă",
    notesTitle: "Note de lansare",
    get: "Obține actualizarea",
    dismiss: "Respinge",
    close: "Închide",
    badgeTooltip: "Actualizare disponibilă — apasă pentru detalii",
    versionTooltip: "Apasă pentru a verifica actualizările",
  },
  // Risk disclaimer (first-run welcome + Settings → About).
  disclaimer: {
    title: "Auto-custodie — cheile tale, responsabilitatea ta",
    body: "Satchel efectuează atomic swap-uri non-custodiale: doar tu îți deții cheile, iar seed-ul unui merchant păstrează cheile fierbinți de tranzit cât timp un swap este în desfășurare. Protocoalele de swap (v1 HTLC și v2 Taproot/MuSig2) sunt revizuite și active pe mainnet. Licențiate MIT și oferite ca atare, fără nicio garanție — fă o copie de rezervă a frazei de recuperare și folosește pe propriul risc.",
  },
  nav: {
    public: "Public",
    corkboard: "Corkboard",
    postOffer: "Postează o ofertă",
    private: "Privat",
    privateCreate: "Creează bilet",
    privateReceive: "Acceptă un bilet",
    privateSlips: "Biletele mele",
    swaps: "Swap-uri",
    relays: "Relay-uri",
    wallets: "Portofele",
    settings: "Setări",
    coins: "Monede",
  },
  makeOffer: {
    title: "Postează o ofertă",
    intro:
      "Postează o ofertă semnată pe Corkboard. Nimic nu este blocat — este doar un anunț; retrage-o oricând, iar un swap pornește doar când cineva o acceptă și ambele părți finanțează.",
    give: "Tu dai",
    want: "Tu primești",
    // Canonical pair + direction: pick the pair, choose Sell/Buy the base, enter
    // the base amount and a quote-per-base price (invariant to direction).
    pair: "Pereche",
    noPairs: "Nicio pereche tranzacționabilă — conectează cel puțin două monede în Setări → Monede.",
    sell: "Vinde {sym}",
    buy: "Cumpără {sym}",
    amount: "Sumă",
    youGive: "Tu dai",
    youGet: "Tu primești",
    price: "Preț",
    priceUnit: "{unit} per {base}",
    pricePlaceholder: "preț unitar",
    balance: "Sold: {amt} {sym}",
    balanceLoading: "Sold: …",
    noCoins: "Nicio monedă configurată",
    legDown: "Nodul uneia dintre aceste monede este oprit — pornește-l (sau verifică Setări → Monede) înainte de a posta.",
    // Swap-protocol pin (only offered when a pair+network supports more than
    // one). v2 label reuses coins.protoPrivate ("Private (Taproot)").
    protocol: "Tip de swap",
    protoStandard: "Standard (HTLC)",
    // Titles for the review/confirm dialog shown before posting (see OfferForm).
    reviewOfferTitle: "Verifică oferta ta",
    reviewSlipTitle: "Verifică biletul tău",
    term: "Timelock de siguranță",
    termShort: "Scurt",
    termMedium: "Mediu",
    termLong: "Lung",
    termHint: {
      short: "Scurt — fondurile se rambursează automat cel mai rapid dacă tranzacția se blochează (~12h / 6h), cu cea mai mică marjă de siguranță.",
      medium: "Mediu — fereastră de rambursare echilibrată (~24h / 12h).",
      long: "Lung (cel mai sigur) — cea mai largă marjă de siguranță; rambursare automată după ~36h / 18h dacă tranzacția se blochează.",
    },
    validFor: "Valabilă pentru (minute)",
    validForMins: "{mins} min",
    validForHint:
      "Cât timp rămâne listată oferta. Cât timp ești online, este menținută activă automat; după aceea expiră. Închiderea aplicației o retrage.",
    note: "Ofertă cu dimensiune fixă — nimic nu este blocat până când cineva nu o acceptă. Sumele sunt on-chain; plătești comisioanele de rețea în plus, iar Corkboard nu percepe nimic. Timelock-ul este fereastra de rambursare automată dacă un swap se blochează.",
    post: "Postează oferta",
    makeSlip: "Creează bilet",
    slipTitle: "Biletul tău de ofertă privată",
    slipExplainer:
      "Trimite asta prietenului tău. El o lipește în Satchel pentru a o accepta. Nimic nu este blocat; expiră în {ttl}.",
    copy: "Copiază",
    copied: "Copiat",
    makeAnother: "Creează altul",
    myPrivateTitle: "Ofertele mele private",
    myPrivateEmpty: "Nicio ofertă privată în desfășurare.",
    privateExpires: "expiră {when}",
    privateExpired: "expirat",
    cancel: "Anulează",
    cancelTip: "Oprește onorarea acestui bilet — un prieten care încă îl deține nu îl mai poate accepta.",
  },
  takeSlip: {
    intro:
      "Un prieten ți-a trimis un bilet de ofertă privată (începe cu pactoffer1:). Lipește-l aici pentru a-l verifica și accepta — exact ca o ofertă de pe board.",
    placeholder: "pactoffer1:…",
    take: "Verifică și acceptă",
    invalid: "Asta nu pare a fi un bilet — ar trebui să înceapă cu pactoffer1:.",
    previewLabel: "Acest bilet oferă",
  },
  // PRIVATE nav group (off-market slips) — screen titles + intros. The form,
  // slip output, my-slips list and take flow reuse makeOffer.*/takeSlip.* copy.
  private: {
    createTitle: "Creează o ofertă privată",
    createIntro:
      "Construiește o ofertă semnată și dă-o unui prieten ca bilet prin propriul tău chat. Nimic nu este listat nicăieri — și nimic nu este blocat până când amândoi nu finanțați.",
    slipsIntro:
      "Biletele pe care le-ai creat. Oricine deține un bilet îl poate accepta până când expiră; anulează unul pentru a opri onorarea lui înainte de atunci.",
    slipsEmptyBody: "Creează o ofertă privată pentru a obține un bilet pe care îl poți trimite unui prieten.",
    receiveTitle: "Acceptă o ofertă privată",
    received: "Acceptat — urmărește-o în Swap-uri.",
  },
  // Shared take-confirmation dialog (board take + slip take).
  takeConfirm: {
    title: "Accepți această ofertă?",
    confirm: "Acceptă oferta",
    counterparty: "Contrapartidă",
    youGive: "Tu dai",
    youReceive: "Tu primești",
    safetyRefund: "Rambursare de siguranță",
    offerAge: "Vechimea ofertei",
    makerFundsFirst:
      "Maker-ul își blochează {sym} primul — tu nu trimiți niciodată primul. Poți totuși anula înainte de a-ți finanța partea, iar engine-ul rambursează automat după timelock-ul de siguranță dacă swap-ul se blochează.",
  },
  header: {
    activeMerchant: "Merchant activ — apasă pentru a schimba sau gestiona",
    manageMerchants: "Gestionează merchant-urile…",
    noMerchant: "niciun merchant",
    openMenu: "Deschide meniul",
    collapseMenu: "restrânge meniul",
    settings: "Setări",
    language: "Limbă",
    pactConnected: "Engine conectat",
    pactUnreachable: "Engine inaccesibil",
    liveSwapsOne: "1 swap în desfășurare — apasă pentru a vedea",
    liveSwapsMany: "{count} swap-uri în desfășurare — apasă pentru a vedea",
    liveSwapsNone: "Niciun swap în desfășurare",
    coinOk: "{name} — conectat · tip {tip}",
    coinUnconfigured: "{name} — neconfigurat",
    coinError: "{name} — {status}",
    relaysOk: "Relay-uri Nostr — {up}/{total} conectate",
    relaysDown: "Relay-uri Nostr — niciunul din {total} conectat",
  },
  network: {
    mainnet: "MainNet",
    testnet: "TestNet",
    regtest: "RegTest",
    signet: "Signet",
    notRealFunds: "Nu sunt fonduri reale — aceasta este rețeaua {network}",
  },
  // Watch-only mode: a viewer session with no coins. Browse the board and
  // withdraw your own offers, but no posting / taking / funding.
  watchOnly: {
    badge: "Doar vizualizare",
    badgeTip:
      "Mod doar-vizualizare — răsfoiește board-ul și retrage-ți propriile oferte, dar nu poți posta, accepta sau finanța. Configurează monede în Setări pentru a tranzacționa.",
    coinWizardButton: "Răsfoiește în mod doar-vizualizare",
    coinWizardHint:
      "Sari peste configurarea monedelor și doar răsfoiește board-ul (doar citire). Poți totuși să-ți retragi propriile oferte — util pentru a retrage oferte lăsate active de o altă sesiune. Dezactivează oricând în Setări.",
    postBlockedTitle: "Mod doar-vizualizare",
    postBlockedBody:
      "Aceasta este o sesiune doar-vizualizare, deci nu poate posta oferte. Configurează cel puțin două monede în Setări → Monede pentru a tranzacționa.",
    takeBlockedBody: "Mod doar-vizualizare — poți verifica această ofertă, dar acceptarea ei necesită monede configurate.",
    takeBlockedTip: "Mod doar-vizualizare — configurează monede în Setări pentru a accepta oferte.",
  },
  merchants: {
    title: "Merchant-urile tale",
    intro:
      "Un merchant este o singură identitate de tranzacționare — cu propriul seed și istoric de swap-uri. Tranzacționarea sub un merchant diferit menține contextele neasociabile (o identitate de unică folosință). Monedele tale principale stau în propriul portofel, nu aici.",
    // First-run welcome (empty merchant manager).
    welcomeTitle: "Bun venit la Satchel",
    welcomeIntro:
      "Satchel tranzacționează sub un „merchant” — o identitate de tranzacționare cu propriul seed. Nu ai încă niciunul: creează unul nou sau importă o frază de recuperare existentă pentru a începe.",
    importMerchant: "Importă un merchant",
    none: "Niciun merchant încă.",
    switch: "schimbă",
    newMerchant: "Merchant nou",
    thisMerchant: "acest merchant",
    nameLabel: "Numele merchant-ului",
    namePlaceholder: "de ex. Principal",
    introFirst:
      "Configurează prima ta identitate de tranzacționare (un „merchant”). Aceasta deține doar chei fierbinți de tranzit pentru swap-urile în desfășurare — monedele tale principale rămân în propriul portofel.",
    introNew: "Un merchant nou este o identitate proaspătă, separată, cu propriul seed și istoric de swap-uri.",
    // UI-5 merchant selector (phoenix wallet-selector parity).
    createNew: "Creează nou",
    import: "Importă",
    load: "Încarcă merchant",
    loaded: "încărcat",
    locked: "blocat",
    lockedTip: "Seed criptat — deblochează-l cu parola ta când îl încarci.",
    close: "Închide",
    idLabel: "folder",
    switching: "Se schimbă merchant-ul…",
    switchingBody: "Se relansează engine-ul pe acel folder.",
  },
  // Seed create/import (SeedForm) + counterparty + status.
  seed: {
    intro: "Creează un seed nou-nouț sau importă unul pe care îl ai deja.",
    createNew: "Creează nou",
    createDesc: "Generează un seed proaspăt. Tu faci copia de rezervă a frazei de recuperare.",
    import: "Importă",
    importDesc: "Restaurează dintr-o frază existentă de 12/24 de cuvinte.",
    recoveryLabel: "Frază de recuperare",
    encrypt: "Criptează",
    encryptDesc:
      "O parolă protejează seed-ul în repaus. O introduci o dată pe sesiune — Satchel nu o stochează niciodată. Notă: rambursarea automată nesupravegheată se întrerupe după o repornire până când o reintroduci.",
    noPassphrase: "Fără parolă (recomandat)",
    noPassphraseDesc:
      "Rambursarea automată continuă să funcționeze peste reporniri fără nimic de introdus — acesta este doar un seed fierbinte de tranzit. Cost: accesul la fișier/gazdă expune cheile de tranzit + identitatea acestui merchant.",
    passphraseLabel: "Parolă",
    passphrasePlaceholder: "alege o parolă",
    revealTitle: "Notează-ți fraza de recuperare",
    revealBody:
      "Oricine deține aceste cuvinte controlează cheile fierbinți ale acestui merchant. Satchel nu păstrează nicio copie — stochează-o offline. În continuare vei confirma câteva cuvinte.",
    ackLabel: "Mi-am notat fraza de recuperare.",
    // Multi-step onboarding (create/import -> secret -> confirm -> passphrase).
    chooseTitle: "Configurează {label}",
    enterTitle: "Importă-ți fraza de recuperare",
    enterBody:
      "Scrie fiecare cuvânt — se completează automat pe măsură ce avansezi — sau lipește întreaga frază. O verificăm înainte să continui.",
    wordCount: "{n} cuvinte",
    wordAria: "Cuvântul {n}",
    checkIncomplete: "Introdu toate cele {n} cuvinte.",
    checkUnknown: "Unele cuvinte nu sunt în lista BIP39 — verifică-le pe cele evidențiate.",
    checkBadChecksum: "Suma de control nu se potrivește — reverifică-ți cuvintele și ordinea lor.",
    checkOk: "Fraza de recuperare pare validă.",
    verifyTitle: "Confirmă copia de rezervă",
    verifyBody: "Scrie cuvintele de la aceste poziții pentru a confirma că ai notat fraza.",
    verifyWord: "Cuvântul #{n}",
    verifyMismatch: "Acelea nu se potrivesc cu fraza ta — verifică-ți copia de rezervă.",
    passphraseTitle: "Protejează seed-ul",
    passphraseBody:
      "Opțional, criptează seed-ul stocat cu o parolă. Poți sări peste asta — vezi compromisul de mai jos.",
  },
  counterparty: {
    you: "Acesta ești tu",
    youShort: "tu",
    unknown: "identitate necunoscută",
    // Short fingerprint fallback (identity.ts shortId) when no pubkey is known.
    unknownShort: "necunoscut",
  },
  status: {
    notConnectedTitle: "Neconectat la engine",
    disconnectedBody:
      "Satchel nu poate ajunge la engine. Poate că încă pornește sau conexiunile la noduri ale merchant-ului activ sunt oprite. Reîncearcă sau schimbă merchant-ul din selectorul de sus.",
    openInSatchel: "Deschide asta în Satchel",
    noTauriBody:
      "Aceasta este interfața Satchel — are nevoie de puntea Tauri pentru a ajunge la engine. Lansează aplicația desktop (cargo tauri dev) în loc de un browser.",
  },
  settings: {
    title: "Setări",
    subtitle: "Preferințe pentru întreaga aplicație, pentru această instalare.",
    // UI-3 Settings tabs.
    tabGeneral: "General",
    tabCoins: "Monede",
    tabNetwork: "Rețea",
    tabAbout: "Despre",
    appearance: "Aspect",
    theme: "Temă",
    themeDark: "Întunecat",
    themeLight: "Luminos",
    themeSystem: "Sistem",
    themeHint: "Alege cum arată Satchel. Sistem urmează setarea sistemului tău de operare.",
    language: "Limbă",
    languageHint: "Mai multe limbi apar pe măsură ce sunt contribuite traduceri.",
    mode: "Mod",
    watchOnly: "Mod doar-vizualizare",
    watchOnlyHint:
      "Răsfoiește board-ul fără a configura monede. Poți totuși să-ți retragi propriile oferte, dar nu poți posta, accepta sau finanța. Dezactivează pentru a tranzacționa (vei avea nevoie de cel puțin două monede conectate).",
    network: "Rețea",
    boards: "Corkboard-uri",
    boardsDesc:
      "Board-uri HTTP opționale găzduite personal. Adaugă oricare în care ai încredere; lasă gol pentru a te baza pe Nostr.",
    boardsNone: "Niciunul configurat",
    nostrRelays: "Relay-uri Nostr",
    nostrRelaysDesc:
      "Relay-urile transportă noticeboard-ul printr-o rețea descentralizată — niciun operator nu poate citi sau potrivi ofertele tale. Preconfigurate cu un set implicit; editează liber.",
    nostrRelaysOff: "Oprit — transportul Nostr este dezactivat",
    addUrl: "Adaugă",
    removeUrl: "Elimină",
    relayInvalid: "Introdu un URL de relay ws:// sau wss://",
    boardInvalid: "Introdu un URL de board http:// sau https://",
    netSave: "Salvează și reconectează",
    netSaving: "Se salvează și se reconectează…",
    netSaved: "Salvat",
    // Fees tab — fee-bump policy (per active merchant).
    tabFees: "Comisioane",
    fees: "Creșterea comisioanelor",
    feesScope: "Aceste setări se aplică merchant-ului activ.",
    feesIntro:
      "Compromisuri siguranță/cost pentru creșterea comisioanelor, nu o configurare obligatorie. Valorile noi se aplică creșterilor viitoare; swap-urile deja finanțate păstrează politica sub care au fost finanțate.",
    feeMax: "Feerate maxim (sat/vB)",
    feeMaxHint:
      "Plafon pentru fiecare creștere de comision. Implicit 500, totodată maximul absolut al sistemului. Coboară-l pentru a limita costurile.",
    feeReservation: "Rezervă pentru creșterea finanțării (×)",
    feeReservationHint:
      "Soldul pe care verificarea fondurilor îl pune deoparte ca marjă pentru creștere. Mai mare salvează spike-uri de comision mai mari, dar blochează mai mult sold și respinge mai multe swap-uri. Implicit 3.",
    feeCommitted: "Supra-provizionare la redeem (×)",
    feeCommittedHint:
      "Cu cât în plus este pre-plătit comisionul de redeem v2 astfel încât să se confirme chiar și când Satchel este închis. Se aplică doar swap-urilor noi. Implicit 2.",
    feeSave: "Salvează",
    feeSaving: "Se salvează…",
    feeSaved: "Salvat",
    feeReset: "Resetează la valorile implicite",
    coins: "Monede și noduri",
    coinsHint: "Conectează fiecare monedă la propriul tău nod. Genesis-ul este verificat înainte de a salva orice.",
    about: "Despre",
    version: "Versiunea {version}",
    updateUpToDate: "La zi",
    updateCheckPlaceholder: "Verificarea actualizărilor sosește într-o lansare ulterioară.",
    trustModel: "Unde stau cheile tale",
    trustModelBody:
      "Secretele stau în engine, niciodată în Satchel. Seed-ul merchant-ului stă în folderul de date al engine-ului (criptat sau în text simplu — alegerea ta); Satchel nu stochează niciun seed sau parolă. Seed-ul este fierbinte prin design (doar chei de tranzit) — mută câștigurile considerabile în propriul tău portofel rece.",
  },
  coins: {
    intro:
      "Conectează fiecare monedă la propriul tău nod. Primul URL este chiar portofelul nodului tău — finanțează picioarele swap-ului și primește câștigurile. Înainte de a salva orice, Satchel verifică blocul genesis al nodului astfel încât fondurile să nu poată fi trimise niciodată pe lanțul greșit. Conexiunile sunt partajate între toate merchant-urile tale.",
    networkBadge: "Se configurează pentru rețeaua {network}",
    needMerchant:
      "Conectează mai întâi un merchant — configurarea monedelor necesită engine-ul în funcțiune. Folosește selectorul de merchant din dreapta sus.",
    pairsTitle: "Perechi de tranzacționare",
    pairsHint:
      "Perechile sunt derivate din ce poate face fiecare monedă — nu există o listă fixă. O pereche se deschide odată ce ambele sale monede sunt conectate.",
    noPairs: "Nicio pereche disponibilă.",
    notSetUp: "Neconfigurat",
    connectedTip: "Conectat · tip {tip}",
    connError: "Eroare de conexiune",
    setUp: "Configurează",
    editConnection: "Editează conexiunea",
    remove: "elimină",
    disconnectTip: "Deconectează această monedă",
    disconnectTitle: "Deconectezi {coin}?",
    disconnectBody: "Swap-urile care au nevoie de ea nu vor fi disponibile până când nu reconectezi.",
    ready: "Gata de tranzacționare",
    connectMissing: "Conectează {coins}",
    notBuildable: "Încă nu se poate construi",
    // Swap-protocol chips on a pair (pact-htlc-v1 HTLC vs pact-htlc-v2 adaptor).
    protoPrivate: "Privat (Taproot)",
    protoPrivateTip: "Swap privat (adaptor Taproot/MuSig2) — arată ca o plată obișnuită on-chain",
    protoHtlcTip: "Swap HTLC clasic",
    // Coin-setup backend choices (CoinSetup).
    // CoinSetup dialog.
    setupTitle: "Conectează {coin}",
    setupIntro:
      "Îndreaptă Satchel către propriul tău nod {sym}. Nimic nu este salvat până când nodul nu trece o verificare a blocului genesis — fondurile tale ating doar lanțul {sym} real.",
    confirmationsLabel: "Confirmări înainte de finalizare",
    confirmationsHint:
      "Cât de adânc trebuie să fie o finanțare sau un redeem pe acest lanț înainte ca un swap să acționeze pe el — marja de siguranță împotriva reorg-urilor. Mai mare este mai sigur, dar mai lent; lasă gol pentru valoarea implicită recomandată ({default}).",
    validateNode: "Validează nodul",
    checking: "Se verifică nodul…",
    genesisOk: "Genesis se potrivește — acesta este lanțul corect",
    genesisDetail: "înălțime tip {tip} · genesis {hash}…",
    genesisBad: "Respins — nu se salvează",
    errorShort: "eroare",
    // Structured connection form (CoinSetup v2).
    rpcHostLabel: "Host RPC",
    rpcPortLabel: "Port RPC",
    authMethodLabel: "Autentificare",
    authCookie: "Fișier cookie",
    authCookieDesc: "Citește automat .cookie al nodului din directorul său de date (implicit, fără parolă stocată).",
    authUserPass: "Utilizator / parolă",
    authUserPassDesc: "rpcuser / rpcpassword din configurarea nodului tău — necesar pentru un nod la distanță.",
    rpcUserLabel: "Nume utilizator RPC",
    rpcPasswordLabel: "Parolă RPC",
    datadirLabel: "Director de date al nodului",
    cookiePathNote: "Cookie-ul este citit din {path} sub acest director.",
    walletLabel: "Nume portofel (opțional)",
    walletPlaceholder: "portofelul nodului tău",
    needPort: "Introdu mai întâi portul RPC.",
    validateFirst: "Validează nodul înainte de a salva.",
    savingReconnecting: "Se salvează și se reconectează…",
    connected: "{coin} conectat",
    // Template picker (a coins.toml coin the engine version doesn't support).
    unsupportedByEngine: "Nesuportat",
    unsupportedByEngineTip:
      "Această monedă este definită în coins.toml dar nu este integrată în această versiune a engine-ului, deci nu poate fi tranzacționată.",
  },
  coinWizard: {
    title: "Conectează-ți monedele",
    intro:
      "Alege cel puțin două monede și îndreaptă fiecare către propriul tău nod. Un swap are nevoie de două lanțuri, așa că tranzacționarea se deblochează odată ce două noduri sunt conectate și active. Poți adăuga sau schimba monede mai târziu în Setări.",
    progress: "{count} din {min} monede conectate",
    continue: "Continuă",
    live: "Activ",
    nodeDown: "Nod oprit",
  },
  wallets: {
    intro:
      "Acestea sunt portofelele propriilor tale noduri (cele pe care engine-ul le folosește pentru a finanța swap-uri și a primi câștiguri) — cheile tale, mașina ta. Satchel nu îți deține niciodată monedele.",
    hotSeedNudge:
      "Acesta este un portofel de cheltuieli pe un seed fierbinte, nu un seif — mută soldurile considerabile în propriul tău portofel rece/core.",
    notConnected: "Neconectat",
    notConnectedBody: "Conectează mai întâi un merchant — vederea portofelului necesită engine-ul în funcțiune.",
    noCoins: "Nicio monedă configurată încă",
    noCoinsBody: "Conectează o monedă în Setări → Monede și portofelul ei apare aici.",
    goToCoins: "Mergi la Monede",
    watchOnlyTitle: "Niciun portofel în modul doar-vizualizare",
    watchOnlyBody:
      "Aceasta este o sesiune doar-vizualizare fără monede conectate, deci nu există portofele de afișat. Dezactivează doar-vizualizare în Setări și conectează o monedă pentru a finanța swap-uri.",
    walletName: "portofel · {wallet}",
    walletScopedHint: "Fiecare RPC pentru această monedă este limitat la acest portofel al nodului.",
    walletDefault: "portofel implicit (nelimitat)",
    walletDefaultHint:
      "Niciun portofel setat pentru această monedă, deci RPC-urile folosesc portofelul implicit al nodului. Setează unul în Setări → Monede pentru a limita fiecare apel la un portofel specific.",
    balanceLabel: "Sold {symbol}",
  },
  corkboard: {
    noBoardTitle: "Niciun Corkboard conectat",
    noBoardBody:
      "Un Corkboard este un avizier partajat unde maker-ii fixează oferte. Nu potrivește niciodată tranzacții și nu deține monede — îndreaptă Satchel către unul în care ai încredere pentru a răsfoi și posta.",
    noPairs: "Nicio pereche disponibilă",
    board: "Corkboard",
    boardSettings: "Configurează în Setări",
    filterAll: "Toate",
    filterMine: "Ale mele",
    noOffers: "Nicio ofertă pe care o poți accepta acum",
    noOffersBody:
      "Ofertele apar aici imediat ce un maker postează una pentru o pereche pe care ai configurat-o. Poți de asemenea să postezi propria ofertă.",
    hiddenOffers:
      "{count} ofertă(e) în plus pentru perechi pe care nu le-ai conectat. Configurează ambele monede pentru a le tranzacționa:",
    yourOffer: "oferta ta",
    offerStaged: "se postează…",
    offerStagedTip:
      "Postată de pe acest dispozitiv și în așteptarea confirmării înapoi de la un relay. Se anunță; devine activă odată ce un relay o reflectă.",
    take: "Acceptă oferta",
    legDown: "Nodul uneia dintre monedele acestei perechi este oprit — pornește-l (sau verifică Setări → Monede) înainte de a accepta.",
    withdraw: "Retrage",
    withdrawTip: "Retrage instant — o ofertă nu blochează niciodată fonduri",
    safetyRefund: "rambursare de siguranță",
    safetyRefundTip:
      "Dacă swap-ul se blochează, ambele părți se rambursează automat — piciorul taker-ului se deblochează primul, al tău puțin mai târziu. Nimeni nu rămâne blocat.",
    activeTitle: "Swap-urile tale active",
    states: {
      takenByUs: "acceptat de tine",
      revoked: "retras",
      expired: "expirat",
    },
    // Two-sided order-book view of the Corkboard. Bids = makers giving the
    // quote coin to get the base; asks = the reverse. The ladder is a way to
    // READ the board — it never matches or prioritises (load-bearing).
    book: {
      bids: "Bids",
      asks: "Asks",
      bidsHint: "vor {base} · plătind {quote}",
      asksHint: "vând {base} · pentru {quote}",
      price: "Preț",
      size: "Mărime",
      noBids: "Niciun bid",
      noAsks: "Niciun ask",
      spread: "Spread {pct}",
      spreadOneSided: "Pe o singură parte",
      crossed: "încrucișat",
      crossedTip: "Cel mai bun bid ≥ cel mai bun ask. Board-ul nu potrivește niciodată automat, deci aceste oferte suprapuse pur și simplu stau acolo — acceptă oricare parte.",
      mid: "mediu {price}",
      levelOffers: "{count} ofertă(e) la acest preț — alege una de acceptat",
      depthTip: "Total {sym} oferit la acest preț pe {count} anunț(uri).",
      selectLevel: "Alege un nivel de preț de mai sus pentru a vedea ofertele de acolo.",
      paneHeader: "{size} {base} @ {price} {unit}",
      denomTip: "Unitate de afișare pentru sumele în {coin}",
      showMore: "Arată încă {count}",
      showLess: "Arată primele {count}",
    },
  },
  relays: {
    title: "Relay-uri",
    subtitle: "Conectivitate live la relay-urile tale Nostr — rețeaua prin care călătoresc ofertele și acceptările tale. Adaugă sau elimină relay-uri în Setări → Rețea.",
    connectedCount: "{up} / {total} conectate",
    refresh: "Reîmprospătează",
    ms: "{ms} ms",
    up: "activ",
    down: "oprit",
    statsTip: "{success}/{attempts} conectări reușite · ↓{down} ↑{up}",
    none: "Niciun relay configurat",
    noneBody: "Adaugă un relay Nostr în Setări → Rețea pentru a publica și primi oferte prin rețea.",
    goToNetwork: "Mergi la Setări",
    notConnected: "Neconectat",
    notConnectedBody: "Vederea relay-urilor necesită engine-ul în funcțiune — conectează mai întâi un merchant.",
  },
  swaps: {
    title: "Swap-uri",
    hint: "Registrul tău complet — swap-urile în desfășurare sus, tranzacțiile finalizate jos. Poți de asemenea acționa asupra swap-urilor active din Corkboard.",
    activeTitle: "În desfășurare",
    historyTitle: "Istoric",
    none: "Niciun swap încă — acceptă o ofertă pe Corkboard.",
    // Active-swaps dock action buttons + the refund-time label.
    cancel: "anulează",
    refund: "rambursează",
    dump: "extrage jurnale",
    dumpHint: "Copiază un pachet de diagnosticare fără secrete (stare + linii de jurnal) pentru acest swap, pentru a-l lipi dezvoltatorilor.",
    dumpCopied: "Diagnosticare copiată — lipește-o dezvoltatorilor.",
    dumpFailed: "Nu s-a putut copia pachetul de diagnosticare.",
    refundAt: "rambursare {when}",
    // Confirm dialogs for acting on a live swap (ActiveSwaps).
    cancelTitle: "Anulezi acest swap?",
    cancelConfirm: "Anulează swap-ul",
    cancelKeep: "Păstrează-l",
    // Abort reason recorded on the swap when cancelled from Satchel.
    cancelReason: "anulat în Satchel",
    cancelBody:
      "Aceasta abandonează swap-ul înainte să fi finanțat. Nimic de-al tău nu este blocat încă, deci nu pierzi nimic — doar că oferta nu se va finaliza.",
    refundTitle: "Îți retragi fondurile înapoi?",
    refundConfirm: "Rambursează",
    refundBody:
      "Timelock-ul de siguranță a trecut, deci poți recupera fondurile pe care le-ai blocat. Aceasta difuzează rambursarea ta acum; engine-ul o face de asemenea automat după termenul-limită.",
    col: {
      swap: "swap",
      role: "rol",
      state: "stare",
      amounts: "dă → primește",
      when: "când",
      finalTx: "tx final",
    },
    // Expandable per-leg on-chain detail (the audit trail). We show both
    // funding txs + OUR settlement; never the counterparty's settlement or the
    // swap secret.
    audit: {
      toggle: "Arată detaliile on-chain",
      title: "Detalii on-chain",
      youLocked: "tu ai blocat",
      theyLocked: "ei au blocat",
      funding: "Finanțare",
      received: "Primit",
      refunded: "Rambursat",
      pending: "încă nu este on-chain",
      copy: "Copiază id-ul tranzacției",
      copied: "Id-ul tranzacției copiat",
    },
  },
  fees: {
    title: "Previzualizare cost de rețea",
    estimated: "estimat",
    provisionalNote: "Acest build pactd nu expune încă estimarea comisioanelor.",
    summary: "Un swap înseamnă 2 tranzacții on-chain pe care le plătești: finanțarea pe lanțul-dat, redeem-ul pe lanțul-primit.",
    fallbackTip: "Un nod a fost inaccesibil, deci s-a folosit un feerate implicit conservator — tratează-le ca pe o estimare.",
    ifItStalls: "(dacă se blochează)",
  },
  funds: {
    insufficient:
      "Insuficient {sym} pentru a finanța acest swap — ai nevoie de ~{need} {sym} (sumă + comision de finanțare), portofelul are {have} {sym}.",
  },
  wizard: {
    back: "Înapoi",
    continue: "Continuă",
  },
  // UI-4 docked activity log.
  log: {
    title: "Activitate",
    empty: "— jurnal de activitate —",
    count: "{count} linii",
    collapse: "Restrânge jurnalul",
    expand: "Extinde jurnalul",
    // Activity-log lines emitted by the frontend. `{err}` carries a raw engine
    // message (itself not translated — it comes from pactd over the wire).
    noTauri: "nu rulează în interiorul Satchel — această interfață are nevoie de puntea Tauri",
    startupError: "pornire: {err}",
    notConnected: "neconectat: {err}",
    connected: "conectat la pactd {version} ({protocol})",
    listcoinsError: "listcoins: {err}",
    watchOnlyError: "doar-vizualizare: {err}",
    switchedMerchant: "s-a schimbat la merchant-ul {id}",
    switchMerchantError: "schimbare merchant: {err}",
    loadMerchantError: "încărcare merchant: {err}",
    merchantCreated: "merchant-ul {id} creat",
    merchantReady: "merchant pregătit",
    actionOk: "{action} {id}: ok",
    actionError: "{action} {id}: {err}",
    diagCopied: "diagnosticare pentru {id} copiată ({count} linii de jurnal) — lipește-o dezvoltatorilor",
    dumpError: "extragere {id}: {err}",
    coinDisconnected: "{coin} deconectat",
    removeCoinError: "eliminare monedă: {err}",
    tookOffer: "ofertă acceptată {id} — apare acum în swap-urile tale active de mai jos",
    takeError: "acceptare: {err}",
    offerWithdrawn: "oferta {id} retrasă",
    withdrawError: "retragere: {err}",
    postedOffer: "ofertă postată {id} — retrage oricând; nimic nu este blocat",
    createdSlip: "s-a creat un bilet de ofertă privată — trimite-l prietenului tău",
    tookPrivateOffer: "ofertă privată acceptată {id} — apare acum în swap-urile tale active",
    cancelledPrivateOffer: "ofertă privată anulată {id}",
    cancelError: "anulare: {err}",
    noticeboardUpdated: "noticeboard actualizat",
    feePolicyUpdated: "politica de comisioane actualizată",
  },
  // Relative-time + freshness prose from format.ts (rendered via the tr() mirror,
  // since those are pure non-component helpers). Unit letters stay inside the
  // template so a translation owns the whole phrase.
  format: {
    ageUnknown: "vechime necunoscută",
    justNow: "chiar acum",
    minutesAgo: "acum {n}m",
    hoursAgo: "acum {n}h",
    daysAgo: "acum {n}z",
    expiryNow: "acum",
    expirySoon: "curând",
    inMinutes: "în ~{n}m",
    inHours: "în ~{n}h",
    inDays: "în ~{n}z",
    posted: "postat {age}",
    expires: "expiră {time}",
  },
  // Plain-language swap story per (role, state) — the honest "who is exposed
  // when" framing shown on every active swap. {a}/{b} are coin tickers; {t1}/{t2}
  // are local refund times. Rendered via tr() (narrate() is a pure helper).
  narrate: {
    finalizing: "Ți-ai revendicat {got} — confirmări finale. Ține aplicația deschisă până se îngroapă; {gave} tăi rămân protejați până atunci.",
    initiating:
      "Acceptare trimisă — se așteaptă ca maker-ul să pornească swap-ul. Nimic nu este blocat încă; se anulează singur dacă nu răspund.",
    created: "Ofertă trimisă — se așteaptă ca cealaltă parte să fie de acord. Nimic nu este angajat.",
    acceptedMaker: "Termeni conveniți. Următorul pas: blochează-ți {a}. Până când finanțezi, poți încă anula liber.",
    acceptedTaker: "Termeni conveniți. Cealaltă parte își blochează {a} primul — tu nu trimiți niciodată primul.",
    noncesExchanged:
      "Se pregătește swap-ul privat — se schimbă materialul de semnare. Nimic nu este blocat încă.",
    signedMaker:
      "Ambele părți au semnat. Daemon-ul tău blochează {a}, apoi revendică {b} automat. Dacă ceva se blochează, {a} se întoarce la {t1}.",
    signedTaker:
      "Ambele părți au semnat. Daemon-ul tău blochează {b} și revendică {a} în momentul în care cealaltă parte se mișcă. Plasă de siguranță: rambursare la {t2}.",
    fundedAMaker:
      "{a} este blocat. Se așteaptă ca cealaltă parte să-și blocheze {b}. Dacă nu o fac niciodată, {a} se întoarce automat la {t1}.",
    fundedATaker:
      "{a} lor este blocat și verificat. Următorul pas: blochează-ți {b}. Plasă de siguranță: rambursare automată la {t2} dacă ceva se blochează.",
    fundedBMaker: "Ambele blocate. Daemon-ul tău revendică {b} imediat ce este confirmat în siguranță.",
    fundedBTaker: "Ambele blocate. Daemon-ul tău va revendica {a} în momentul în care cealaltă parte își ia {b}.",
    completed: "Swap finalizat — {coin} este în portofelul tău.",
    refunded: "Swap-ul nu s-a finalizat, deci {coin} tău s-a întors automat. Nimic pierdut în afară de comisioane.",
    aborted: "Anulat înainte ca vreun ban să se miște.",
  },
  progress: {
    awaitingLock: "Se așteaptă blocarea lor",
    awaitingClaim: "Se așteaptă revendicarea lor",
    theirLock: "Se confirmă blocarea lor",
    securing: "Se securizează {coin} tăi",
    blocks: "+{n} blocuri",
    feeBumped: "Comision majorat",
    reorg: "Reorganizare detectată — se reverifică",
  },
  exit: {
    // Exit-gate dialog (fund safety, C6). The engine manages alone, so "keep
    // running" detaches it (it keeps watching timelocks + servicing offers).
    liveTitle: "Un swap este în desfășurare",
    liveBodyOne:
      "1 swap este în plină desfășurare. Este guvernat de timelock-uri on-chain — engine-ul trebuie să continue să ruleze pentru a face redeem sau rambursare înainte de termenul-limită.",
    liveBodyMany:
      "{count} swap-uri sunt în plină desfășurare. Sunt guvernate de timelock-uri on-chain — engine-ul trebuie să continue să ruleze pentru a face redeem sau rambursare înainte de termenul-limită.",
    keepRunningExplain:
      "Închiderea ferestrei menține engine-ul în funcțiune în fundal, astfel încât finalizează swap-ul fără interfață. Poți redeschide Satchel oricând pentru a-l verifica.",
    forceQuitWarn: "Forțarea închiderii acum oprește engine-ul și poate duce la pierderea fondurilor.",
    // {word} is the confirm word below; a translation may localize both together.
    typeToConfirm: "Pentru a forța totuși închiderea, scrie {word} mai jos.",
    confirmWord: "QUIT",
    keepRunning: "Menține în funcțiune, închide fereastra",
    keepWithdraw: "Menține în funcțiune + retrage ofertele",
    keepLeaveOffers: "Menține în funcțiune, lasă ofertele active",
    forceQuit: "Forțează închiderea",
    offersTitle: "Ai oferte postate",
    offersBodyOne:
      "1 ofertă de-a ta este încă pe Corkboard. Ofertele nu blochează nimic, dar lăsând-o activă înseamnă că contrapartidele o pot încă accepta cât timp Satchel este închis — engine-ul va deservi acceptarea.",
    offersBodyMany:
      "{count} oferte de-ale tale sunt încă pe Corkboard. Ofertele nu blochează nimic, dar lăsându-le active înseamnă că contrapartidele le pot încă accepta cât timp Satchel este închis — engine-ul va deservi acceptările.",
    withdrawExit: "Retrage tot și ieși",
  },
  unlock: {
    title: "Deblochează merchant-ul",
    body:
      "Seed-ul acestui merchant este criptat. Introdu parola lui pentru a-l debloca pentru această sesiune — Satchel îl ține doar în memorie și îl uită la ieșire.",
    switchMerchant: "Schimbă merchant-ul",
    unlock: "Deblochează",
  },
  common: {
    cancel: "Anulează",
    confirm: "Confirmă",
    save: "Salvează",
    done: "Gata",
    later: "Mai târziu",
    retry: "Reîncearcă conexiunea",
  },
};
