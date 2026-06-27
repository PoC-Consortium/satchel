// The Italian (Italiano) string bundle. Mirrors en.ts key-for-key; only values are translated.
import type { Bundle } from "./en";

export const it: Bundle = {
  app: {
    name: "Satchel",
  },
  // In-app update notifications (sidebar version badge + dialog).
  update: {
    title: "Aggiornamento disponibile",
    upToDate: "Sei aggiornato",
    current: "Installato",
    latest: "Più recente",
    notesTitle: "Note di rilascio",
    get: "Ottieni l'aggiornamento",
    dismiss: "Ignora",
    close: "Chiudi",
    badgeTooltip: "Aggiornamento disponibile — clicca per i dettagli",
    versionTooltip: "Clicca per cercare aggiornamenti",
  },
  // Risk disclaimer (first-run welcome + Settings → About).
  disclaimer: {
    title: "Auto-custodia — le tue chiavi, la tua responsabilità",
    body: "Satchel esegue atomic swap non custodiali: tu solo detieni le tue chiavi, e il seed di un merchant detiene chiavi di transito calde mentre uno swap è in corso. I protocolli di swap (v1 HTLC e v2 Taproot/MuSig2) sono revisionati e attivi su mainnet. Concesso con licenza MIT e fornito così com'è, senza alcuna garanzia — esegui il backup della tua frase di recupero e usalo a tuo rischio.",
  },
  nav: {
    public: "Pubblico",
    corkboard: "Corkboard",
    postOffer: "Pubblica un'offerta",
    private: "Privato",
    privateCreate: "Crea slip",
    privateReceive: "Accetta una slip",
    privateSlips: "Le mie slip",
    swaps: "Swap",
    relays: "Relay",
    wallets: "Wallet",
    settings: "Impostazioni",
    coins: "Coin",
  },
  makeOffer: {
    title: "Pubblica un'offerta",
    intro:
      "Pubblica un'offerta firmata sul Corkboard. Nulla viene bloccato — è solo un annuncio; ritirala in qualsiasi momento, e uno swap parte solo quando qualcuno la accetta e entrambe le parti effettuano il funding.",
    give: "Tu dai",
    want: "Tu ricevi",
    // Canonical pair + direction: pick the pair, choose Sell/Buy the base, enter
    // the base amount and a quote-per-base price (invariant to direction).
    pair: "Coppia",
    noPairs: "Nessuna coppia scambiabile — collega almeno due coin in Impostazioni → Coin.",
    sell: "Vendi {sym}",
    buy: "Compra {sym}",
    amount: "Importo",
    youGive: "Tu dai",
    youGet: "Tu ottieni",
    price: "Prezzo",
    priceUnit: "{unit} per {base}",
    pricePlaceholder: "prezzo unitario",
    balance: "Saldo: {amt} {sym}",
    balanceLoading: "Saldo: …",
    noCoins: "Nessuna coin configurata",
    legDown: "Il nodo di una di queste coin è offline — avvialo (o controlla Impostazioni → Coin) prima di pubblicare.",
    // Swap-protocol pin (only offered when a pair+network supports more than
    // one). v2 label reuses coins.protoPrivate ("Private (Taproot)").
    protocol: "Tipo di swap",
    protoStandard: "Standard (HTLC)",
    // Titles for the review/confirm dialog shown before posting (see OfferForm).
    reviewOfferTitle: "Rivedi la tua offerta",
    reviewSlipTitle: "Rivedi la tua slip",
    term: "Timelock di sicurezza",
    termShort: "Breve",
    termMedium: "Medio",
    termLong: "Lungo",
    termHint: {
      short: "Breve — i fondi vengono auto-rimborsati più rapidamente se lo scambio si blocca (~12h / 6h), con il margine di sicurezza più piccolo.",
      medium: "Medio — finestra di rimborso bilanciata (~24h / 12h).",
      long: "Lungo (più sicuro) — margine di sicurezza più ampio; auto-rimborso dopo ~36h / 18h se lo scambio si blocca.",
    },
    validFor: "Valida per (minuti)",
    validForMins: "{mins} min",
    validForHint:
      "Per quanto tempo l'offerta resta in elenco. Mentre sei online viene mantenuta aggiornata automaticamente; dopo questo periodo scade. Chiudere l'app la ritira.",
    note: "Offerta a dimensione fissa — nulla viene bloccato finché qualcuno non la accetta. Gli importi sono on-chain; paghi le fee di rete in aggiunta e il Corkboard non addebita nulla. Il timelock è la finestra di auto-rimborso se uno swap si blocca.",
    post: "Pubblica offerta",
    makeSlip: "Crea slip",
    slipTitle: "La tua slip di offerta privata",
    slipExplainer:
      "Invia questa al tuo amico. La incolla in Satchel per accettarla. Nulla viene bloccato; scade tra {ttl}.",
    copy: "Copia",
    copied: "Copiato",
    makeAnother: "Creane un'altra",
    myPrivateTitle: "Le mie offerte private",
    myPrivateEmpty: "Nessuna offerta privata in sospeso.",
    privateExpires: "scade {when}",
    privateExpired: "scaduta",
    cancel: "Annulla",
    cancelTip: "Smetti di onorare questa slip — un amico che la possiede ancora non potrà più accettarla.",
  },
  takeSlip: {
    intro:
      "Un amico ti ha inviato una slip di offerta privata (inizia con pactoffer1:). Incollala qui per rivederla e accettarla — esattamente come un'offerta dalla bacheca.",
    placeholder: "pactoffer1:…",
    take: "Rivedi e accetta",
    invalid: "Questa non sembra una slip — dovrebbe iniziare con pactoffer1:.",
    previewLabel: "Questa slip offre",
  },
  // PRIVATE nav group (off-market slips) — screen titles + intros. The form,
  // slip output, my-slips list and take flow reuse makeOffer.*/takeSlip.* copy.
  private: {
    createTitle: "Crea un'offerta privata",
    createIntro:
      "Costruisci un'offerta firmata e consegnala a un amico come slip tramite la tua chat. Nulla viene messo in elenco da nessuna parte — e nulla viene bloccato finché entrambi non effettuate il funding.",
    slipsIntro:
      "Le slip che hai creato. Chiunque possieda una slip può accettarla finché non scade; annullane una per smettere di onorarla prima di allora.",
    slipsEmptyBody: "Crea un'offerta privata per ottenere una slip da inviare a un amico.",
    receiveTitle: "Accetta un'offerta privata",
    received: "Accettata — seguila in Swap.",
  },
  // Shared take-confirmation dialog (board take + slip take).
  takeConfirm: {
    title: "Accettare questa offerta?",
    confirm: "Accetta offerta",
    counterparty: "Controparte",
    youGive: "Tu dai",
    youReceive: "Tu ricevi",
    safetyRefund: "Rimborso di sicurezza",
    offerAge: "Età dell'offerta",
    makerFundsFirst:
      "Il maker blocca i suoi {sym} per primo — tu non invii mai per primo. Puoi comunque annullare prima di effettuare il funding della tua parte, e l'engine auto-rimborsa dopo il timelock di sicurezza se lo swap si blocca.",
  },
  header: {
    activeMerchant: "Merchant attivo — clicca per cambiare o gestire",
    manageMerchants: "Gestisci merchant…",
    noMerchant: "nessun merchant",
    openMenu: "Apri menu",
    collapseMenu: "comprimi menu",
    settings: "Impostazioni",
    language: "Lingua",
    pactConnected: "Engine connesso",
    pactUnreachable: "Engine irraggiungibile",
    liveSwapsOne: "1 swap in corso — clicca per visualizzare",
    liveSwapsMany: "{count} swap in corso — clicca per visualizzare",
    liveSwapsNone: "Nessuno swap in corso",
    coinOk: "{name} — connesso · tip {tip}",
    coinUnconfigured: "{name} — non configurata",
    coinError: "{name} — {status}",
    relaysOk: "Relay Nostr — {up}/{total} connessi",
    relaysDown: "Relay Nostr — nessuno di {total} connesso",
  },
  network: {
    mainnet: "MainNet",
    testnet: "TestNet",
    regtest: "RegTest",
    signet: "Signet",
    notRealFunds: "Non sono fondi reali — questa è la rete {network}",
  },
  // Watch-only mode: a viewer session with no coins. Browse the board and
  // withdraw your own offers, but no posting / taking / funding.
  watchOnly: {
    badge: "Solo visualizzazione",
    badgeTip:
      "Modalità solo visualizzazione — sfoglia la bacheca e ritira le tue offerte, ma non puoi pubblicare, accettare o effettuare il funding. Configura le coin in Impostazioni per fare trading.",
    coinWizardButton: "Sfoglia in modalità solo visualizzazione",
    coinWizardHint:
      "Salta la configurazione delle coin e sfoglia semplicemente la bacheca (sola lettura). Puoi comunque ritirare le tue offerte — utile per rimuovere offerte lasciate da un'altra sessione. Disattivala in qualsiasi momento nelle Impostazioni.",
    postBlockedTitle: "Modalità solo visualizzazione",
    postBlockedBody:
      "Questa è una sessione di sola visualizzazione, quindi non può pubblicare offerte. Configura almeno due coin in Impostazioni → Coin per fare trading.",
    takeBlockedBody: "Modalità solo visualizzazione — puoi rivedere questa offerta, ma accettarla richiede coin configurate.",
    takeBlockedTip: "Modalità solo visualizzazione — configura le coin nelle Impostazioni per accettare offerte.",
  },
  merchants: {
    title: "I tuoi merchant",
    intro:
      "Un merchant è un'identità di trading — con un proprio seed e una propria cronologia di swap. Fare trading sotto un merchant diverso mantiene i contesti non collegabili (un'identità usa e getta). Le tue coin principali risiedono nel tuo wallet, non qui.",
    // First-run welcome (empty merchant manager).
    welcomeTitle: "Benvenuto in Satchel",
    welcomeIntro:
      "Satchel fa trading sotto un “merchant” — un'identità di trading con un proprio seed. Non ne hai ancora nessuna: creane una nuova, o importa una frase di recupero esistente per iniziare.",
    importMerchant: "Importa un merchant",
    none: "Ancora nessun merchant.",
    switch: "cambia",
    newMerchant: "Nuovo merchant",
    thisMerchant: "questo merchant",
    nameLabel: "Nome del merchant",
    namePlaceholder: "es. Principale",
    introFirst:
      "Configura la tua prima identità di trading (un “merchant”). Detiene solo chiavi di transito calde per gli swap in corso — le tue coin principali restano nel tuo wallet.",
    introNew: "Un nuovo merchant è un'identità nuova e separata, con un proprio seed e una propria cronologia di swap.",
    // UI-5 merchant selector (phoenix wallet-selector parity).
    createNew: "Crea nuovo",
    import: "Importa",
    load: "Carica merchant",
    loaded: "caricato",
    locked: "bloccato",
    lockedTip: "Seed cifrato — sbloccalo con la tua passphrase quando lo carichi.",
    close: "Chiudi",
    idLabel: "cartella",
    switching: "Cambio merchant…",
    switchingBody: "Riavvio dell'engine sulla nuova cartella.",
  },
  // Seed create/import (SeedForm) + counterparty + status.
  seed: {
    intro: "Crea un seed nuovo di zecca, o importane uno che già possiedi.",
    createNew: "Crea nuovo",
    createDesc: "Genera un seed nuovo. Sei tu a fare il backup della frase di recupero.",
    import: "Importa",
    importDesc: "Ripristina da una frase esistente di 12/24 parole.",
    recoveryLabel: "Frase di recupero",
    encrypt: "Cifra",
    encryptDesc:
      "Una passphrase protegge il seed a riposo. La inserisci una volta per sessione — Satchel non la memorizza mai. Nota: l'auto-rimborso non presidiato si sospende dopo un riavvio finché non la reinserisci.",
    noPassphrase: "Nessuna passphrase (consigliato)",
    noPassphraseDesc:
      "L'auto-rimborso continua a funzionare attraverso i riavvii senza nulla da inserire — questo è solo un seed di transito caldo. Costo: l'accesso al file/host espone le chiavi di transito e l'identità di questo merchant.",
    passphraseLabel: "Passphrase",
    passphrasePlaceholder: "scegli una passphrase",
    revealTitle: "Annota la tua frase di recupero",
    revealBody:
      "Chiunque possieda queste parole controlla le chiavi calde di questo merchant. Satchel non ne conserva alcuna copia — archiviala offline. Successivamente confermerai alcune parole.",
    ackLabel: "Ho annotato la mia frase di recupero.",
    // Multi-step onboarding (create/import -> secret -> confirm -> passphrase).
    chooseTitle: "Configura {label}",
    enterTitle: "Importa la tua frase di recupero",
    enterBody:
      "Digita ogni parola — si auto-completano man mano — oppure incolla l'intera frase. La verifichiamo prima di continuare.",
    wordCount: "{n} parole",
    wordAria: "Parola {n}",
    checkIncomplete: "Inserisci tutte le {n} parole.",
    checkUnknown: "Alcune parole non sono nella wordlist BIP39 — controlla quelle evidenziate.",
    checkBadChecksum: "Il checksum non corrisponde — ricontrolla le parole e il loro ordine.",
    checkOk: "La frase di recupero sembra valida.",
    verifyTitle: "Conferma il tuo backup",
    verifyBody: "Digita le parole in queste posizioni per confermare di aver annotato la frase.",
    verifyWord: "Parola #{n}",
    verifyMismatch: "Queste non corrispondono alla tua frase — controlla il tuo backup.",
    passphraseTitle: "Proteggi il seed",
    passphraseBody:
      "Facoltativamente cifra il seed memorizzato con una passphrase. Puoi saltare questo passaggio — vedi il compromesso qui sotto.",
  },
  counterparty: {
    you: "Questo sei tu",
    youShort: "tu",
    unknown: "identità sconosciuta",
    // Short fingerprint fallback (identity.ts shortId) when no pubkey is known.
    unknownShort: "sconosciuta",
  },
  status: {
    notConnectedTitle: "Non connesso all'engine",
    disconnectedBody:
      "Satchel non riesce a raggiungere l'engine. Potrebbe essere ancora in avvio, oppure le connessioni ai nodi del merchant attivo potrebbero essere offline. Riprova, o cambia merchant dal selettore in alto.",
    openInSatchel: "Apri questo in Satchel",
    noTauriBody:
      "Questa è l'interfaccia di Satchel — ha bisogno del bridge Tauri per raggiungere l'engine. Avvia l'app desktop (cargo tauri dev) invece di un browser.",
  },
  settings: {
    title: "Impostazioni",
    subtitle: "Preferenze a livello di app per questa installazione.",
    // UI-3 Settings tabs.
    tabGeneral: "Generali",
    tabCoins: "Coin",
    tabNetwork: "Rete",
    tabAbout: "Informazioni",
    appearance: "Aspetto",
    theme: "Tema",
    themeDark: "Scuro",
    themeLight: "Chiaro",
    themeSystem: "Sistema",
    themeHint: "Scegli l'aspetto di Satchel. Sistema segue l'impostazione del tuo OS.",
    language: "Lingua",
    languageHint: "Altre lingue arriveranno man mano che vengono contribuite le traduzioni.",
    mode: "Modalità",
    watchOnly: "Modalità solo visualizzazione",
    watchOnlyHint:
      "Sfoglia la bacheca senza configurare coin. Puoi comunque ritirare le tue offerte, ma non puoi pubblicare, accettare o effettuare il funding. Disattivala per fare trading (ti serviranno almeno due coin connesse).",
    network: "Rete",
    boards: "Corkboard",
    boardsDesc:
      "Bacheche HTTP self-hosted facoltative. Aggiungine quante ne ritieni affidabili; lascia vuoto per affidarti a Nostr.",
    boardsNone: "Nessuna configurata",
    nostrRelays: "Relay Nostr",
    nostrRelaysDesc:
      "I relay trasportano la bacheca su una rete decentralizzata — nessun operatore può leggere o abbinare le tue offerte. Precablati con un set predefinito; modificali liberamente.",
    nostrRelaysOff: "Disattivato — trasporto Nostr disabilitato",
    addUrl: "Aggiungi",
    removeUrl: "Rimuovi",
    relayInvalid: "Inserisci un URL di relay ws:// o wss://",
    boardInvalid: "Inserisci un URL di bacheca http:// o https://",
    netSave: "Salva e riconnetti",
    netSaving: "Salvataggio e riconnessione…",
    netSaved: "Salvato",
    // Fees tab — fee-bump policy (per active merchant).
    tabFees: "Fee",
    fees: "Fee bumping",
    feesScope: "Queste impostazioni si applicano al merchant attivo.",
    feesIntro:
      "Compromessi sicurezza/costo per i fee bump, non una configurazione obbligatoria. I nuovi valori si applicano ai bump futuri; gli swap già fundati mantengono la policy con cui sono stati fundati.",
    feeMax: "Feerate massimo (sat/vB)",
    feeMaxHint:
      "Tetto per ogni fee bump. Predefinito 500, è anche il massimo rigido di sistema. Abbassalo per limitare i costi.",
    feeReservation: "Riserva per bump di funding (×)",
    feeReservationHint:
      "Saldo che il controllo dei fondi accantona come margine per i bump. Un valore più alto salva i picchi di fee più grandi ma immobilizza più saldo e rifiuta più swap. Predefinito 3.",
    feeCommitted: "Sovra-provisioning del redeem (×)",
    feeCommittedHint:
      "Quanto extra viene pre-pagato per la fee del redeem v2 affinché venga confermato anche quando Satchel è chiuso. Si applica solo ai nuovi swap. Predefinito 2.",
    feeSave: "Salva",
    feeSaving: "Salvataggio…",
    feeSaved: "Salvato",
    feeReset: "Ripristina i valori predefiniti",
    coins: "Coin e nodi",
    coinsHint: "Collega ogni coin al tuo nodo. Il genesis viene verificato prima che qualcosa venga salvato.",
    about: "Informazioni",
    version: "Versione {version}",
    updateUpToDate: "Aggiornato",
    updateCheckPlaceholder: "Il controllo degli aggiornamenti arriverà in un rilascio successivo.",
    trustModel: "Dove risiedono le tue chiavi",
    trustModelBody:
      "I segreti risiedono nell'engine, mai in Satchel. Il seed del merchant si trova nella cartella dati dell'engine (cifrato o in chiaro — a tua scelta); Satchel non memorizza alcun seed o passphrase. Il seed è caldo per design (solo chiavi di transito) — trasferisci proventi consistenti al tuo cold wallet.",
  },
  coins: {
    intro:
      "Collega ogni coin al tuo nodo. Il primo URL è il wallet del tuo nodo — finanzia le tue leg di swap e riceve i proventi. Prima che qualcosa venga salvato, Satchel verifica il blocco genesis del nodo affinché i fondi non possano mai essere inviati alla chain sbagliata. Le connessioni sono condivise tra tutti i tuoi merchant.",
    networkBadge: "Configurazione per la rete {network}",
    needMerchant:
      "Collega prima un merchant — la configurazione delle coin richiede l'engine in esecuzione. Usa il selettore di merchant in alto a destra.",
    pairsTitle: "Coppie di trading",
    pairsHint:
      "Le coppie derivano da ciò che ogni coin può fare — non esiste un elenco fisso. Una coppia si apre quando entrambe le sue coin sono connesse.",
    noPairs: "Nessuna coppia disponibile.",
    notSetUp: "Non configurata",
    connectedTip: "Connesso · tip {tip}",
    connError: "Errore di connessione",
    setUp: "Configura",
    editConnection: "Modifica connessione",
    remove: "rimuovi",
    disconnectTip: "Disconnetti questa coin",
    disconnectTitle: "Disconnettere {coin}?",
    disconnectBody: "Gli swap che la richiedono non saranno disponibili finché non la riconnetti.",
    ready: "Pronta per il trading",
    connectMissing: "Collega {coins}",
    notBuildable: "Non ancora costruibile",
    // Swap-protocol chips on a pair (pact-htlc-v1 HTLC vs pact-htlc-v2 adaptor).
    protoPrivate: "Privato (Taproot)",
    protoPrivateTip: "Swap privato (adaptor Taproot/MuSig2) — appare come un normale pagamento on-chain",
    protoHtlcTip: "Swap HTLC classico",
    // Coin-setup backend choices (CoinSetup).
    // CoinSetup dialog.
    setupTitle: "Collega {coin}",
    setupIntro:
      "Punta Satchel al tuo nodo {sym}. Nulla viene salvato finché il nodo non supera una verifica del blocco genesis — i tuoi fondi toccano solo la vera chain {sym}.",
    confirmationsLabel: "Conferme prima del definitivo",
    confirmationsHint:
      "Quanto in profondità un funding o un redeem su questa chain deve essere prima che uno swap agisca su di esso — il margine di sicurezza contro i reorg. Più alto è più sicuro ma più lento; lascia vuoto per il predefinito consigliato ({default}).",
    validateNode: "Valida nodo",
    checking: "Verifica del nodo…",
    genesisOk: "Genesis corrispondente — questa è la chain corretta",
    genesisDetail: "altezza tip {tip} · genesis {hash}…",
    genesisBad: "Rifiutato — non salvo",
    errorShort: "errore",
    // Structured connection form (CoinSetup v2).
    rpcHostLabel: "Host RPC",
    rpcPortLabel: "Porta RPC",
    authMethodLabel: "Autenticazione",
    authCookie: "File cookie",
    authCookieDesc: "Legge automaticamente il .cookie del nodo dalla sua directory dati (predefinito, nessuna password memorizzata).",
    authUserPass: "Utente / password",
    authUserPassDesc: "Il rpcuser / rpcpassword dalla configurazione del tuo nodo — necessario per un nodo remoto.",
    rpcUserLabel: "Username RPC",
    rpcPasswordLabel: "Password RPC",
    datadirLabel: "Directory dati del nodo",
    cookiePathNote: "Il cookie viene letto da {path} sotto questa directory.",
    walletLabel: "Nome del wallet (facoltativo)",
    walletPlaceholder: "il wallet del tuo nodo",
    needPort: "Inserisci prima la porta RPC.",
    validateFirst: "Valida il nodo prima di salvare.",
    savingReconnecting: "Salvataggio e riconnessione…",
    connected: "{coin} connessa",
    // Template picker (a coins.toml coin the engine version doesn't support).
    unsupportedByEngine: "Non supportata",
    unsupportedByEngineTip:
      "Questa coin è definita in coins.toml ma non è integrata in questa versione dell'engine, quindi non può essere scambiata.",
  },
  coinWizard: {
    title: "Collega le tue coin",
    intro:
      "Scegli almeno due coin e punta ognuna al tuo nodo. Uno swap richiede due chain, quindi il trading si sblocca quando due nodi sono connessi e attivi. Puoi aggiungere o cambiare coin in seguito nelle Impostazioni.",
    progress: "{count} di {min} coin connesse",
    continue: "Continua",
    live: "Attivo",
    nodeDown: "Nodo offline",
  },
  wallets: {
    intro:
      "Questi sono i wallet dei tuoi nodi (quelli che l'engine usa per finanziare gli swap e ricevere i proventi) — le tue chiavi, la tua macchina. Satchel non detiene mai le tue coin.",
    hotSeedNudge:
      "Questo è un wallet di spesa su un seed caldo, non un caveau — trasferisci saldi consistenti al tuo cold/core wallet.",
    notConnected: "Non connesso",
    notConnectedBody: "Collega prima un merchant — la vista wallet richiede l'engine in esecuzione.",
    noCoins: "Ancora nessuna coin configurata",
    noCoinsBody: "Collega una coin in Impostazioni → Coin e il suo wallet apparirà qui.",
    goToCoins: "Vai a Coin",
    watchOnlyTitle: "Nessun wallet in modalità solo visualizzazione",
    watchOnlyBody:
      "Questa è una sessione di sola visualizzazione senza coin connesse, quindi non ci sono wallet da mostrare. Disattiva la sola visualizzazione nelle Impostazioni e collega una coin per finanziare gli swap.",
    walletName: "wallet · {wallet}",
    walletScopedHint: "Ogni RPC per questa coin è limitato a questo wallet del nodo.",
    walletDefault: "wallet predefinito (non limitato)",
    walletDefaultHint:
      "Nessun wallet impostato per questa coin, quindi le RPC usano il wallet predefinito del nodo. Impostane uno in Impostazioni → Coin per limitare ogni chiamata a un wallet specifico.",
    balanceLabel: "Saldo {symbol}",
  },
  corkboard: {
    noBoardTitle: "Nessun Corkboard connesso",
    noBoardBody:
      "Un Corkboard è una bacheca condivisa dove i maker affiggono le offerte. Non abbina mai gli scambi né detiene coin — punta Satchel a uno di cui ti fidi per sfogliare e pubblicare.",
    noPairs: "Nessuna coppia disponibile",
    board: "Corkboard",
    boardSettings: "Configura nelle Impostazioni",
    filterAll: "Tutte",
    filterMine: "Mie",
    noOffers: "Nessuna offerta che puoi accettare in questo momento",
    noOffersBody:
      "Le offerte compaiono qui non appena un maker ne pubblica una per una coppia che hai configurato. Puoi anche pubblicare le tue.",
    hiddenOffers:
      "{count} altra/e offerta/e per coppie che non hai collegato. Configura entrambe le coin per scambiarle:",
    yourOffer: "la tua offerta",
    offerStaged: "pubblicazione…",
    offerStagedTip:
      "Pubblicata da questo dispositivo e in attesa di conferma di ritorno da un relay. È in fase di annuncio; diventa attiva non appena un relay la riecheggia.",
    take: "Accetta offerta",
    legDown: "Il nodo di una delle coin di questa coppia è offline — avvialo (o controlla Impostazioni → Coin) prima di accettare.",
    withdraw: "Ritira",
    withdrawTip: "Ritira immediatamente — un'offerta non blocca mai i fondi",
    safetyRefund: "rimborso di sicurezza",
    safetyRefundTip:
      "Se lo swap si blocca, entrambe le parti si auto-rimborsano — la leg del taker si sblocca per prima, la tua poco dopo. Nessuno resta bloccato.",
    activeTitle: "I tuoi swap attivi",
    states: {
      takenByUs: "accettata da te",
      revoked: "ritirata",
      expired: "scaduta",
    },
    // Two-sided order-book view of the Corkboard. Bids = makers giving the
    // quote coin to get the base; asks = the reverse. The ladder is a way to
    // READ the board — it never matches or prioritises (load-bearing).
    book: {
      bids: "Bid",
      asks: "Ask",
      bidsHint: "vogliono {base} · pagando {quote}",
      asksHint: "vendono {base} · per {quote}",
      price: "Prezzo",
      size: "Quantità",
      noBids: "Nessuna bid",
      noAsks: "Nessuna ask",
      spread: "Spread {pct}",
      spreadOneSided: "Unilaterale",
      crossed: "incrociato",
      crossedTip: "Bid superiore ≥ ask superiore. La bacheca non abbina mai automaticamente, quindi queste offerte sovrapposte restano lì — accetta uno dei due lati.",
      mid: "medio {price}",
      levelOffers: "{count} offerta/e a questo prezzo — scegline una da accettare",
      depthTip: "Totale {sym} in offerta a questo prezzo su {count} annuncio/i.",
      selectLevel: "Scegli un livello di prezzo sopra per vedere le offerte lì presenti.",
      paneHeader: "{size} {base} @ {price} {unit}",
      denomTip: "Unità di visualizzazione per gli importi in {coin}",
      showMore: "Mostra altri {count}",
      showLess: "Mostra i primi {count}",
    },
  },
  relays: {
    title: "Relay",
    subtitle: "Connettività in tempo reale ai tuoi relay Nostr — la rete su cui viaggiano le tue offerte e accettazioni. Aggiungi o rimuovi relay in Impostazioni → Rete.",
    connectedCount: "{up} / {total} connessi",
    refresh: "Aggiorna",
    ms: "{ms} ms",
    up: "attivo",
    down: "offline",
    statsTip: "{success}/{attempts} connessioni riuscite · ↓{down} ↑{up}",
    none: "Nessun relay configurato",
    noneBody: "Aggiungi un relay Nostr in Impostazioni → Rete per pubblicare e ricevere offerte sulla rete.",
    goToNetwork: "Vai alle Impostazioni",
    notConnected: "Non connesso",
    notConnectedBody: "La vista dei relay richiede l'engine in esecuzione — collega prima un merchant.",
  },
  swaps: {
    maker: "Maker",
    taker: "Taker",
    title: "Swap",
    hint: "Il tuo registro completo — gli swap in corso in cima, gli scambi conclusi sotto. Puoi anche agire sugli swap attivi dal Corkboard.",
    activeTitle: "In corso",
    historyTitle: "Cronologia",
    none: "Ancora nessuno swap — accetta un'offerta sul Corkboard.",
    // Active-swaps dock action buttons + the refund-time label.
    cancel: "annulla",
    refund: "rimborsa",
    dump: "esporta log",
    dumpHint: "Copia un pacchetto diagnostico privo di segreti (stato + righe di log) per questo swap, da incollare agli sviluppatori.",
    dumpCopied: "Diagnostica copiata — incollala agli sviluppatori.",
    dumpFailed: "Impossibile copiare il pacchetto diagnostico.",
    refundAt: "rimborso {when}",
    // Confirm dialogs for acting on a live swap (ActiveSwaps).
    cancelTitle: "Annullare questo swap?",
    cancelConfirm: "Annulla swap",
    cancelKeep: "Mantienilo",
    // Abort reason recorded on the swap when cancelled from Satchel.
    cancelReason: "annullato in Satchel",
    cancelBody:
      "Questo abbandona lo swap prima che tu abbia effettuato il funding. Nulla di tuo è ancora bloccato, quindi non perdi nulla — l'offerta semplicemente non si completerà.",
    refundTitle: "Recuperare i tuoi fondi?",
    refundConfirm: "Rimborsa",
    refundBody:
      "Il timelock di sicurezza è scaduto, quindi puoi reclamare i fondi che hai bloccato. Questo trasmette ora il tuo rimborso; l'engine lo fa anche automaticamente dopo la scadenza.",
    col: {
      swap: "swap",
      role: "ruolo",
      state: "stato",
      amounts: "dà → riceve",
      when: "quando",
      finalTx: "tx finale",
    },
    // Expandable per-leg on-chain detail (the audit trail). We show both
    // funding txs + OUR settlement; never the counterparty's settlement or the
    // swap secret.
    audit: {
      toggle: "Mostra dettaglio on-chain",
      title: "Dettaglio on-chain",
      youLocked: "hai bloccato",
      theyLocked: "hanno bloccato",
      funding: "Funding",
      received: "Ricevuto",
      refunded: "Rimborsato",
      pending: "non ancora on-chain",
      copy: "Copia id transazione",
      copied: "Id transazione copiato",
    },
  },
  fees: {
    title: "Anteprima dei costi di rete",
    estimated: "stimato",
    provisionalNote: "Questa build di pactd non espone ancora la stima delle fee.",
    summary: "Uno swap è composto da 2 transazioni on-chain che paghi: funding sulla chain che dai, redeem sulla chain che ricevi.",
    fallbackTip: "Un nodo era irraggiungibile, quindi è stato usato un feerate predefinito conservativo — consideralo una stima.",
    ifItStalls: "(se si blocca)",
  },
  funds: {
    insufficient:
      "{sym} insufficienti per finanziare questo swap — servono ~{need} {sym} (importo + fee di funding), il wallet ne ha {have} {sym}.",
  },
  wizard: {
    back: "Indietro",
    continue: "Continua",
  },
  // UI-4 docked activity log.
  log: {
    title: "Attività",
    empty: "— registro attività —",
    count: "{count} righe",
    collapse: "Comprimi registro",
    expand: "Espandi registro",
    // Activity-log lines emitted by the frontend. `{err}` carries a raw engine
    // message (itself not translated — it comes from pactd over the wire).
    noTauri: "non in esecuzione dentro Satchel — questa UI richiede il bridge Tauri",
    startupError: "avvio: {err}",
    notConnected: "non connesso: {err}",
    connected: "connesso a pactd {version} ({protocol})",
    listcoinsError: "listcoins: {err}",
    watchOnlyError: "solo visualizzazione: {err}",
    switchedMerchant: "passato al merchant {id}",
    switchMerchantError: "cambio merchant: {err}",
    loadMerchantError: "caricamento merchant: {err}",
    merchantCreated: "merchant {id} creato",
    merchantReady: "merchant pronto",
    actionOk: "{action} {id}: ok",
    actionError: "{action} {id}: {err}",
    diagCopied: "diagnostica per {id} copiata ({count} righe di log) — incollala agli sviluppatori",
    dumpError: "dump {id}: {err}",
    coinDisconnected: "{coin} disconnessa",
    removeCoinError: "rimozione coin: {err}",
    tookOffer: "accettata l'offerta {id} — ora appare nei tuoi swap attivi qui sotto",
    takeError: "accettazione: {err}",
    offerWithdrawn: "offerta {id} ritirata",
    withdrawError: "ritiro: {err}",
    postedOffer: "offerta {id} pubblicata — ritirala in qualsiasi momento; nulla è bloccato",
    createdSlip: "creata una slip di offerta privata — inviala al tuo amico",
    tookPrivateOffer: "accettata l'offerta privata {id} — ora appare nei tuoi swap attivi",
    cancelledPrivateOffer: "offerta privata {id} annullata",
    cancelError: "annullamento: {err}",
    noticeboardUpdated: "bacheca aggiornata",
    feePolicyUpdated: "policy delle fee aggiornata",
  },
  // Relative-time + freshness prose from format.ts (rendered via the tr() mirror,
  // since those are pure non-component helpers). Unit letters stay inside the
  // template so a translation owns the whole phrase.
  format: {
    ageUnknown: "età sconosciuta",
    justNow: "proprio ora",
    minutesAgo: "{n}m fa",
    hoursAgo: "{n}h fa",
    daysAgo: "{n}g fa",
    expiryNow: "ora",
    expirySoon: "presto",
    inMinutes: "tra ~{n}m",
    inHours: "tra ~{n}h",
    inDays: "tra ~{n}g",
    posted: "pubblicata {age}",
    expires: "scade {time}",
  },
  // Plain-language swap story per (role, state) — the honest "who is exposed
  // when" framing shown on every active swap. {a}/{b} are coin tickers; {t1}/{t2}
  // are local refund times. Rendered via tr() (narrate() is a pure helper).
  narrate: {
    finalizing: "Hai richiesto i tuoi {got} — conferme finali. Tieni l'app aperta finché non è sepolto; i tuoi {gave} restano protetti fino ad allora.",
    initiating:
      "Accettazione inviata — in attesa che il maker avvii lo swap. Nulla è ancora bloccato; si annulla da solo se non rispondono.",
    created: "Offerta inviata — in attesa che l'altra parte accetti. Nulla è impegnato.",
    acceptedMaker: "Termini concordati. Prossimo passo: blocca i tuoi {a}. Finché non effettui il funding, puoi ancora annullare liberamente.",
    acceptedTaker: "Termini concordati. L'altra parte blocca i suoi {a} per prima — tu non invii mai per primo.",
    noncesExchanged:
      "Configurazione dello swap privato — scambio del materiale di firma. Nulla è ancora bloccato.",
    signedMaker:
      "Entrambe le parti hanno firmato. Il tuo daemon blocca i {a}, poi reclama i {b} automaticamente. Se qualcosa si blocca, i tuoi {a} tornano alle {t1}.",
    signedTaker:
      "Entrambe le parti hanno firmato. Il tuo daemon blocca i {b} e reclama i {a} nel momento in cui l'altra parte si muove. Rete di sicurezza: rimborso alle {t2}.",
    fundedAMaker:
      "I tuoi {a} sono bloccati. In attesa che l'altra parte blocchi i suoi {b}. Se non lo fanno mai, i tuoi {a} tornano automaticamente alle {t1}.",
    fundedATaker:
      "I loro {a} sono bloccati e verificati. Prossimo passo: blocca i tuoi {b}. Rete di sicurezza: rimborso automatico alle {t2} se qualcosa si blocca.",
    fundedBMaker: "Entrambi bloccati. Il tuo daemon reclama i {b} non appena sono confermati in sicurezza.",
    fundedBTaker: "Entrambi bloccati. Il tuo daemon reclamerà i {a} nel momento in cui l'altra parte prende i suoi {b}.",
    completed: "Swap completato — i {coin} sono nel tuo wallet.",
    refunded: "Lo swap non si è completato, quindi i tuoi {coin} sono tornati automaticamente. Nulla perso tranne le fee.",
    aborted: "Annullato prima che si muovesse del denaro.",
  },
  progress: {
    awaitingLock: "In attesa del loro blocco",
    awaitingClaim: "In attesa del loro riscatto",
    theirLock: "Conferma del loro blocco",
    securing: "Protezione dei tuoi {coin}",
    blocks: "+{n} blocchi",
    feeBumped: "Commissione aumentata",
    reorg: "Reorg rilevato — nuovo controllo",
  },
  exit: {
    // Exit-gate dialog (fund safety, C6). The engine manages alone, so "keep
    // running" detaches it (it keeps watching timelocks + servicing offers).
    liveTitle: "Uno swap è in corso",
    liveBodyOne:
      "1 swap è a metà strada. È governato da timelock on-chain — l'engine deve restare in esecuzione per redimere o rimborsare prima della scadenza.",
    liveBodyMany:
      "{count} swap sono a metà strada. Sono governati da timelock on-chain — l'engine deve restare in esecuzione per redimere o rimborsare prima della scadenza.",
    keepRunningExplain:
      "Chiudere la finestra mantiene l'engine in esecuzione in background, così completa lo swap in modalità headless. Puoi riaprire Satchel in qualsiasi momento per controllarlo.",
    forceQuitWarn: "Forzare la chiusura ora ferma l'engine e può causare la perdita di fondi.",
    // {word} is the confirm word below; a translation may localize both together.
    typeToConfirm: "Per forzare comunque la chiusura, digita {word} qui sotto.",
    confirmWord: "QUIT",
    keepRunning: "Continua a eseguire, chiudi la finestra",
    keepWithdraw: "Continua a eseguire + ritira le offerte",
    keepLeaveOffers: "Continua a eseguire, lascia le offerte attive",
    forceQuit: "Forza chiusura",
    offersTitle: "Hai offerte pubblicate",
    offersBodyOne:
      "1 tua offerta è ancora sul Corkboard. Le offerte non bloccano nulla, ma lasciarla attiva significa che le controparti possono ancora accettarla mentre Satchel è chiuso — l'engine gestirà l'accettazione.",
    offersBodyMany:
      "{count} tue offerte sono ancora sul Corkboard. Le offerte non bloccano nulla, ma lasciarle attive significa che le controparti possono ancora accettarle mentre Satchel è chiuso — l'engine gestirà le accettazioni.",
    withdrawExit: "Ritira tutte ed esci",
  },
  unlock: {
    title: "Sblocca merchant",
    body:
      "Il seed di questo merchant è cifrato. Inserisci la sua passphrase per sbloccarlo per questa sessione — Satchel lo tiene solo in memoria e lo dimentica all'uscita.",
    switchMerchant: "Cambia merchant",
    unlock: "Sblocca",
  },
  common: {
    cancel: "Annulla",
    confirm: "Conferma",
    save: "Salva",
    done: "Fatto",
    later: "Più tardi",
    retry: "Riprova connessione",
  },
};
