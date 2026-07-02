// The Catalan (Català) string bundle. Mirrors en.ts key-for-key; only values are translated.
import type { Bundle } from "./en";

export const ca: Bundle = {
  app: {
    name: "Satchel",
  },
  // In-app update notifications (sidebar version badge + dialog).
  update: {
    title: "Actualització disponible",
    upToDate: "Tens la versió més recent",
    current: "Instal·lada",
    latest: "Més recent",
    notesTitle: "Notes de la versió",
    get: "Obtén l'actualització",
    dismiss: "Descarta",
    close: "Tanca",
    badgeTooltip: "Actualització disponible — fes clic per veure'n els detalls",
    versionTooltip: "Fes clic per comprovar si hi ha actualitzacions",
  },
  // Risk disclaimer (first-run welcome + Settings → About).
  disclaimer: {
    title: "Autocustòdia — les teves claus, la teva responsabilitat",
    body: "Satchel fa intercanvis atòmics no custodiats: només tu tens les teves claus, i la llavor d'un comerciant conté claus de trànsit calentes mentre un intercanvi és en curs. Els protocols d'intercanvi (v1 HTLC i v2 Taproot/MuSig2) estan revisats i actius a la mainnet. Amb llicència MIT i proporcionat tal com és, sense cap garantia — fes una còpia de seguretat de la teva frase de recuperació i fes-lo servir sota la teva responsabilitat.",
  },
  nav: {
    public: "Públic",
    corkboard: "Corkboard",
    postOffer: "Publica una oferta",
    private: "Privat",
    privateCreate: "Crea un val",
    privateReceive: "Accepta un val",
    privateSlips: "Els meus vals",
    swaps: "Intercanvis",
    relays: "Relés",
    wallets: "Carteres",
    contacts: "Contacts",
    settings: "Configuració",
    coins: "Monedes",
  },
  makeOffer: {
    title: "Publica una oferta",
    intro:
      "Publica una oferta signada al Corkboard. No es bloqueja res — només és un anunci; retira-la quan vulguis, i un intercanvi només comença quan algú l'accepta i les dues parts financen.",
    give: "Tu dones",
    want: "Tu reps",
    // Canonical pair + direction: pick the pair, choose Sell/Buy the base, enter
    // the base amount and a quote-per-base price (invariant to direction).
    pair: "Parell",
    noPairs: "No hi ha cap parell negociable — connecta almenys dues monedes a Configuració → Monedes.",
    sell: "Ven {sym}",
    buy: "Compra {sym}",
    amount: "Quantitat",
    youGive: "Tu dones",
    youGet: "Tu reps",
    price: "Preu",
    priceUnit: "{unit} per {base}",
    pricePlaceholder: "preu unitari",
    balance: "Saldo: {amt} {sym}",
    balanceLoading: "Saldo: …",
    noCoins: "No hi ha cap moneda configurada",
    legDown: "El node d'una d'aquestes monedes no funciona — inicia'l (o revisa Configuració → Monedes) abans de publicar.",
    // Swap-protocol pin (only offered when a pair+network supports more than
    // one). v2 label reuses coins.protoPrivate ("Private (Taproot)").
    protocol: "Tipus d'intercanvi",
    protoStandard: "Estàndard (HTLC)",
    // Titles for the review/confirm dialog shown before posting (see OfferForm).
    reviewOfferTitle: "Revisa la teva oferta",
    reviewSlipTitle: "Revisa el teu val",
    term: "Timelock de seguretat",
    termShort: "Curt",
    termMedium: "Mitjà",
    termLong: "Llarg",
    termHint: {
      short: "Curt — els fons es reemborsen automàticament més ràpid si el tracte s'encalla (~12h / 6h), amb el marge de seguretat més petit.",
      medium: "Mitjà — finestra de reemborsament equilibrada (~24h / 12h).",
      long: "Llarg (el més segur) — marge de seguretat més ampli; reemborsament automàtic després de ~36h / 18h si el tracte s'encalla.",
    },
    validFor: "Vàlida durant (minuts)",
    validForMins: "{mins} min",
    validForHint:
      "Quant de temps roman publicada l'oferta. Mentre estiguis en línia es manté actualitzada automàticament; després caduca. Tancar l'aplicació la retira.",
    note: "Oferta de mida fixa — no es bloqueja res fins que algú l'accepta. Les quantitats són on-chain; pagues les comissions de xarxa a part i el Corkboard no cobra res. El timelock és la finestra de reemborsament automàtic si un intercanvi s'encalla.",
    post: "Publica l'oferta",
    makeSlip: "Crea un val",
    slipTitle: "El teu val d'oferta privada",
    slipExplainer:
      "Envia això al teu amic. Ell l'enganxa a Satchel per acceptar-lo. No es bloqueja res; caduca en {ttl}.",
    copy: "Copia",
    copied: "Copiat",
    makeAnother: "Crea'n un altre",
    myPrivateTitle: "Les meves ofertes privades",
    myPrivateEmpty: "No hi ha ofertes privades pendents.",
    privateExpires: "caduca {when}",
    privateExpired: "caducada",
    cancel: "Cancel·la",
    cancelTip: "Deixa d'honrar aquest val — un amic que encara el tingui ja no podrà acceptar-lo.",
  },
  takeSlip: {
    intro:
      "Un amic t'ha enviat un val d'oferta privada (comença amb pactoffer1:). Enganxa'l aquí per revisar-lo i acceptar-lo — exactament com una oferta del tauler.",
    placeholder: "pactoffer1:…",
    take: "Revisa i accepta",
    invalid: "Això no sembla un val — hauria de començar amb pactoffer1:.",
    previewLabel: "Aquest val ofereix",
  },
  // PRIVATE nav group (off-market slips) — screen titles + intros. The form,
  // slip output, my-slips list and take flow reuse makeOffer.*/takeSlip.* copy.
  private: {
    createTitle: "Crea una oferta privada",
    createIntro:
      "Construeix una oferta signada i passa-la a un amic com a val pel teu propi xat. No es publica enlloc — i no es bloqueja res fins que tots dos financeu.",
    slipsIntro:
      "Vals que has creat. Qualsevol que tingui un val pot acceptar-lo fins que caduqui; cancel·la'n un per deixar d'honrar-lo abans.",
    slipsEmptyBody: "Crea una oferta privada per obtenir un val que puguis enviar a un amic.",
    receiveTitle: "Accepta una oferta privada",
    received: "Acceptada — segueix-la a Intercanvis.",
  },
  // Shared take-confirmation dialog (board take + slip take).
  takeConfirm: {
    title: "Vols acceptar aquesta oferta?",
    confirm: "Accepta l'oferta",
    counterparty: "Contrapart",
    youGive: "Tu dones",
    youReceive: "Tu reps",
    safetyRefund: "Reemborsament de seguretat",
    offerAge: "Antiguitat de l'oferta",
    makerFundsFirst:
      "El maker bloqueja els seus {sym} primer — tu mai envies primer. Encara pots cancel·lar abans de finançar la teva part, i el motor reemborsa automàticament després del timelock de seguretat si l'intercanvi s'encalla.",
  },
  header: {
    activeMerchant: "Comerciant actiu — fes clic per canviar o gestionar",
    manageMerchants: "Gestiona els comerciants…",
    noMerchant: "cap comerciant",
    openMenu: "Obre el menú",
    collapseMenu: "replega el menú",
    settings: "Configuració",
    language: "Idioma",
    pactConnected: "Motor connectat",
    pactUnreachable: "Motor inaccessible",
    liveSwapsOne: "1 intercanvi en curs — fes clic per veure'l",
    liveSwapsMany: "{count} intercanvis en curs — fes clic per veure'ls",
    liveSwapsNone: "Cap intercanvi en curs",
    coinOk: "{name} — connectat · cim {tip}",
    coinUnconfigured: "{name} — no configurat",
    coinError: "{name} — {status}",
    relaysOk: "Relés Nostr — {up}/{total} connectats",
    relaysDown: "Relés Nostr — cap de {total} connectat",
  },
  network: {
    mainnet: "MainNet",
    testnet: "TestNet",
    regtest: "RegTest",
    signet: "Signet",
    notRealFunds: "No són fons reals — això és la xarxa {network}",
  },
  // Watch-only mode: a viewer session with no coins. Browse the board and
  // withdraw your own offers, but no posting / taking / funding.
  watchOnly: {
    badge: "Només visualització",
    badgeTip:
      "Mode només visualització — navega pel tauler i retira les teves pròpies ofertes, però no pots publicar, acceptar ni finançar. Configura monedes a Configuració per negociar.",
    coinWizardButton: "Navega en mode només visualització",
    coinWizardHint:
      "Omet la configuració de monedes i simplement navega pel tauler (només lectura). Encara pots retirar les teves pròpies ofertes — pràctic per retirar ofertes deixades per una altra sessió. Desactiva-ho quan vulguis a Configuració.",
    postBlockedTitle: "Mode només visualització",
    postBlockedBody:
      "Aquesta és una sessió només de visualització, per tant no pot publicar ofertes. Configura almenys dues monedes a Configuració → Monedes per negociar.",
    takeBlockedBody: "Mode només visualització — pots revisar aquesta oferta, però per acceptar-la cal tenir monedes configurades.",
    takeBlockedTip: "Mode només visualització — configura monedes a Configuració per acceptar ofertes.",
  },
  merchants: {
    title: "Els teus comerciants",
    intro:
      "Un comerciant és una identitat de negociació — amb la seva pròpia llavor i historial d'intercanvis. Negociar sota un comerciant diferent manté els contextos no vinculables (una identitat d'usar i llençar). Les teves monedes principals viuen a la teva pròpia cartera, no aquí.",
    // First-run welcome (empty merchant manager).
    welcomeTitle: "Benvingut a Satchel",
    welcomeIntro:
      "Satchel negocia sota un «comerciant» — una identitat de negociació amb la seva pròpia llavor. Encara no en tens cap: crea'n una de nova, o importa una frase de recuperació existent per començar.",
    importMerchant: "Importa un comerciant",
    none: "Encara no hi ha comerciants.",
    switch: "canvia",
    newMerchant: "Comerciant nou",
    thisMerchant: "aquest comerciant",
    nameLabel: "Nom del comerciant",
    namePlaceholder: "p. ex. Principal",
    rename: "Reanomena",
    introFirst:
      "Configura la teva primera identitat de negociació (un «comerciant»). Només conté claus de trànsit calentes per a intercanvis en curs — les teves monedes principals queden a la teva pròpia cartera.",
    introNew: "Un comerciant nou és una identitat fresca i separada amb la seva pròpia llavor i historial d'intercanvis.",
    // UI-5 merchant selector (phoenix wallet-selector parity).
    createNew: "Crea'n un de nou",
    import: "Importa",
    load: "Carrega el comerciant",
    loaded: "carregat",
    locked: "bloquejat",
    lockedTip: "Llavor xifrada — desbloqueja-la amb la teva contrasenya quan la carreguis.",
    close: "Tanca",
    idLabel: "carpeta",
    switching: "Canviant de comerciant…",
    switchingBody: "Reiniciant el motor contra aquesta carpeta.",
  },
  // Seed create/import (SeedForm) + counterparty + status.
  seed: {
    intro: "Crea una llavor totalment nova, o importa'n una que ja tinguis.",
    createNew: "Crea'n una de nova",
    createDesc: "Genera una llavor fresca. Tu fas còpia de seguretat de la frase de recuperació.",
    import: "Importa",
    importDesc: "Restaura des d'una frase existent de 12/24 paraules.",
    recoveryLabel: "Frase de recuperació",
    encrypt: "Xifra",
    encryptDesc:
      "Una contrasenya protegeix la llavor en repòs. La introdueixes un cop per sessió — Satchel mai l'emmagatzema. Nota: el reemborsament automàtic sense vigilància es pausa després d'un reinici fins que la tornis a introduir.",
    noPassphrase: "Sense contrasenya (recomanat)",
    noPassphraseDesc:
      "El reemborsament automàtic continua funcionant entre reinicis sense haver d'introduir res — això només és una llavor de trànsit calenta. Cost: l'accés al fitxer/host exposa les claus de trànsit i la identitat d'aquest comerciant.",
    passphraseLabel: "Contrasenya",
    passphrasePlaceholder: "tria una contrasenya",
    revealTitle: "Anota la teva frase de recuperació",
    revealBody:
      "Qualsevol que tingui aquestes paraules controla les claus calentes d'aquest comerciant. Satchel no en guarda cap còpia — desa-la fora de línia. A continuació en confirmaràs unes quantes paraules.",
    ackLabel: "He anotat la meva frase de recuperació.",
    // Multi-step onboarding (create/import -> secret -> confirm -> passphrase).
    chooseTitle: "Configura {label}",
    enterTitle: "Importa la teva frase de recuperació",
    enterBody:
      "Escriu cada paraula — s'autocompleten a mesura que avances — o enganxa la frase sencera. La comprovem abans de continuar.",
    wordCount: "{n} paraules",
    wordAria: "Paraula {n}",
    checkIncomplete: "Introdueix les {n} paraules.",
    checkUnknown: "Algunes paraules no són a la llista de paraules BIP39 — revisa les marcades.",
    checkBadChecksum: "La suma de verificació no coincideix — torna a revisar les paraules i el seu ordre.",
    checkOk: "La frase de recuperació sembla vàlida.",
    verifyTitle: "Confirma la teva còpia de seguretat",
    verifyBody: "Escriu les paraules en aquestes posicions per confirmar que has anotat la frase.",
    verifyWord: "Paraula #{n}",
    verifyMismatch: "Aquestes no coincideixen amb la teva frase — revisa la teva còpia de seguretat.",
    passphraseTitle: "Protegeix la llavor",
    passphraseBody:
      "Opcionalment xifra la llavor emmagatzemada amb una contrasenya. Pots ometre-ho — vegeu el compromís a continuació.",
  },
  counterparty: {
    you: "Aquest ets tu",
    youShort: "tu",
    unknown: "identitat desconeguda",
    // Short fingerprint fallback (identity.ts shortId) when no pubkey is known.
    unknownShort: "desconegut",
  },
  contacts: {
    title: "Contactes",
    subtitle: "Els teus sobrenoms privats per a les persones amb qui negocies.",
    privacyNote:
      "Els contactes s'emmagatzemen només en aquest dispositiu i mai es comparteixen, publiquen ni s'envien a un relé. Un sobrenom és la teva etiqueta — l'identicon i l'empremta segueixen sent la identitat real.",
    searchPlaceholder: "Cerca sobrenom, nota o clau",
    empty: "Encara no hi ha contactes. Fes clic a l'identicon d'una contrapart a qualsevol lloc per afegir-ne un.",
    emptyFiltered: "Cap contacte coincideix amb aquest filtre.",
    count: "{n} contactes",
    colWho: "Identitat",
    colNick: "Sobrenom",
    colNote: "Notes",
    colStatus: "Estat",
    colAdded: "Afegit",
    colActions: "",
    filterAll: "Tots",
    filterTrusted: "De confiança",
    filterBlocked: "Bloquejats",
    // Corkboard toggle: drop blocked makers' offers from the ladder.
    hideBlocked: "Amaga les ofertes bloquejades",
    statusTrusted: "De confiança",
    statusNeutral: "Neutral",
    statusBlocked: "Bloquejat",
    menuAdd: "Afegeix als contactes…",
    menuEdit: "Edita el contacte…",
    menuMarkTrusted: "Marca com a de confiança",
    menuMarkNeutral: "Marca com a neutral",
    menuMarkBlocked: "Bloqueja",
    menuCopyKey: "Copia la clau pública",
    menuOpen: "Obre a Contactes",
    keyCopied: "Clau pública copiada",
    editTitle: "Edita el contacte",
    addTitle: "Afegeix un contacte",
    nickLabel: "Sobrenom",
    nickPlaceholder: "p. ex. l'Alícia de la trobada",
    noteLabel: "Notes",
    notePlaceholder: "Qualsevol cosa que vulguis recordar — com contactar-hi, intercanvis anteriors…",
    save: "Desa",
    cancel: "Cancel·la",
    remove: "Elimina el contacte",
    removeConfirmTitle: "Vols eliminar el contacte?",
    removeConfirmBody: "Això elimina el teu sobrenom i les notes locals de {who}. No es pot desfer.",
    blockedWarning: "Has bloquejat aquesta contrapart",
    blockedWarningBody:
      "Has marcat aquesta persona com a bloquejada. Bloquejar només és un recordatori personal — no atura l'intercanvi. Continua només si és la teva intenció.",
  },
  status: {
    notConnectedTitle: "No connectat al motor",
    disconnectedBody:
      "Satchel no pot arribar al motor. Pot ser que encara s'estigui iniciant, o que les connexions de node del comerciant actiu estiguin caigudes. Torna-ho a provar, o canvia de comerciant des del selector de dalt.",
    openInSatchel: "Obre això a Satchel",
    noTauriBody:
      "Aquesta és la interfície de Satchel — necessita el pont Tauri per arribar al motor. Inicia l'aplicació d'escriptori (cargo tauri dev) en lloc d'un navegador.",
  },
  settings: {
    title: "Configuració",
    subtitle: "Preferències globals de l'aplicació per a aquesta instal·lació.",
    // UI-3 Settings tabs.
    tabGeneral: "General",
    tabCoins: "Monedes",
    tabNetwork: "Xarxa",
    tabAbout: "Quant a",
    appearance: "Aparença",
    theme: "Tema",
    themeDark: "Fosc",
    themeLight: "Clar",
    themeSystem: "Sistema",
    themeHint: "Tria com es veu Satchel. Sistema segueix la configuració del teu SO.",
    language: "Idioma",
    languageHint: "S'afegeixen més idiomes a mesura que es contribueixen traduccions.",
    mode: "Mode",
    watchOnly: "Mode només visualització",
    watchOnlyHint:
      "Navega pel tauler sense configurar monedes. Encara pots retirar les teves pròpies ofertes, però no pots publicar, acceptar ni finançar. Desactiva'l per negociar (necessitaràs almenys dues monedes connectades).",
    network: "Xarxa",
    boards: "Corkboards",
    boardsDesc:
      "Taulers HTTP autoallotjats opcionals. Afegeix els que confiïs; deixa-ho buit per dependre de Nostr.",
    boardsNone: "Cap configurat",
    nostrRelays: "Relés Nostr",
    nostrRelaysDesc:
      "Els relés transporten el tauler d'anuncis per una xarxa descentralitzada — cap operador pot llegir ni emparellar les teves ofertes. Precablejat amb un conjunt per defecte; edita'l lliurement.",
    nostrRelaysOff: "Desactivat — transport Nostr deshabilitat",
    addUrl: "Afegeix",
    removeUrl: "Elimina",
    relayInvalid: "Introdueix una URL de relé ws:// o wss://",
    boardInvalid: "Introdueix una URL de tauler http:// o https://",
    netSave: "Desa i torna a connectar",
    netSaving: "Desant i tornant a connectar…",
    netSaved: "Desat",
    // Fees tab — fee-bump policy (per active merchant).
    tabFees: "Comissions",
    fees: "Increment de comissions",
    feesScope: "Aquesta configuració s'aplica al comerciant actiu.",
    feesIntro:
      "Compromisos de seguretat/cost per als increments de comissions, no és una configuració obligatòria. Els valors nous s'apliquen a futurs increments; els intercanvis ja finançats mantenen la política amb què es van finançar.",
    feeMax: "Feerate màxim (sat/vB)",
    feeMaxHint:
      "Límit per a cada increment de comissió. Per defecte 500, també el màxim absolut del sistema. Baixa'l per limitar els costos.",
    feeReservation: "Reserva per a increment de finançament (×)",
    feeReservationHint:
      "El saldo que la comprovació de fons reserva com a marge per a increments. Més alt rescata pics de comissions més grans però immobilitza més saldo i rebutja més intercanvis. Per defecte 3.",
    feeCommitted: "Sobreprovisió de bescanvi (×)",
    feeCommittedHint:
      "Quant extra es paga per avançat la comissió de bescanvi v2 perquè es confirmi fins i tot amb Satchel tancat. S'aplica només a intercanvis nous. Per defecte 2.",
    feeSave: "Desa",
    feeSaving: "Desant…",
    feeSaved: "Desat",
    feeReset: "Restableix els valors per defecte",
    coins: "Monedes i nodes",
    coinsHint: "Connecta cada moneda al teu propi node. El bloc gènesi es comprova abans de desar res.",
    about: "Quant a",
    version: "Versió {version}",
    updateUpToDate: "Al dia",
    updateCheckPlaceholder: "La comprovació d'actualitzacions arribarà en una versió posterior.",
    trustModel: "On viuen les teves claus",
    trustModelBody:
      "Els secrets viuen al motor, mai a Satchel. La llavor del comerciant es troba a la carpeta de dades del motor (xifrada o en text pla — tu tries); Satchel no emmagatzema cap llavor ni contrasenya. La llavor és calenta per disseny (només claus de trànsit) — escombra els guanys considerables a la teva pròpia cartera freda.",
  },
  coins: {
    intro:
      "Connecta cada moneda al teu propi node. La primera URL és la cartera del teu propi node — finança les teves potes d'intercanvi i rep els guanys. Abans de desar res, Satchel comprova el bloc gènesi del node perquè els fons no es puguin enviar mai a la cadena equivocada. Les connexions es comparteixen entre tots els teus comerciants.",
    networkBadge: "Configurant per a la xarxa {network}",
    needMerchant:
      "Connecta primer un comerciant — la configuració de monedes necessita el motor en marxa. Fes servir el selector de comerciant de dalt a la dreta.",
    pairsTitle: "Parells de negociació",
    pairsHint:
      "Els parells es deriven del que pot fer cada moneda — no hi ha cap llista fixa. Un parell s'obre quan les seves dues monedes estan connectades.",
    noPairs: "No hi ha cap parell disponible.",
    notSetUp: "No configurat",
    connectedTip: "Connectat · cim {tip}",
    connError: "Error de connexió",
    setUp: "Configura",
    editConnection: "Edita la connexió",
    remove: "elimina",
    disconnectTip: "Desconnecta aquesta moneda",
    disconnectTitle: "Vols desconnectar {coin}?",
    disconnectBody: "Els intercanvis que la necessitin no estaran disponibles fins que la tornis a connectar.",
    ready: "A punt per negociar",
    connectMissing: "Connecta {coins}",
    notBuildable: "Encara no construïble",
    // Swap-protocol chips on a pair (pact-htlc-v1 HTLC vs pact-htlc-v2 adaptor).
    protoPrivate: "Privat (Taproot)",
    protoPrivateTip: "Intercanvi privat (adaptador Taproot/MuSig2) — sembla un pagament ordinari on-chain",
    protoHtlcTip: "Intercanvi HTLC clàssic",
    // Coin-setup backend choices (CoinSetup).
    // CoinSetup dialog.
    setupTitle: "Connecta {coin}",
    setupIntro:
      "Apunta Satchel al teu propi node {sym}. No es desa res fins que el node passa una comprovació del bloc gènesi — els teus fons només toquen mai la cadena {sym} real.",
    confirmationsLabel: "Confirmacions abans de final",
    confirmationsHint:
      "Com de profund ha de ser un finançament o bescanvi en aquesta cadena abans que un intercanvi hi actuï — el marge de seguretat davant reorganitzacions. Més alt és més segur però més lent; deixa-ho en blanc per al valor per defecte recomanat ({default}).",
    validateNode: "Valida el node",
    checking: "Comprovant el node…",
    genesisOk: "Gènesi coincident — aquesta és la cadena correcta",
    genesisDetail: "alçada del cim {tip} · gènesi {hash}…",
    genesisBad: "Rebutjat — no es desa",
    errorShort: "error",
    // Structured connection form (CoinSetup v2).
    rpcHostLabel: "Host RPC",
    rpcPortLabel: "Port RPC",
    authMethodLabel: "Autenticació",
    authCookie: "Fitxer cookie",
    authCookieDesc: "Llegeix automàticament el .cookie del node des del seu directori de dades (per defecte, sense desar contrasenya).",
    authUserPass: "Usuari / contrasenya",
    authUserPassDesc: "El rpcuser / rpcpassword de la configuració del teu node — necessari per a un node remot.",
    rpcUserLabel: "Nom d'usuari RPC",
    rpcPasswordLabel: "Contrasenya RPC",
    datadirLabel: "Directori de dades del node",
    cookiePathNote: "El cookie es llegeix de {path} sota aquest directori.",
    walletLabel: "Nom de la cartera (opcional)",
    walletPlaceholder: "la cartera del teu node",
    needPort: "Introdueix primer el port RPC.",
    validateFirst: "Valida el node abans de desar.",
    savingReconnecting: "Desant i tornant a connectar…",
    connected: "{coin} connectat",
    // Template picker (a coins.toml coin the engine version doesn't support).
    unsupportedByEngine: "No compatible",
    unsupportedByEngineTip:
      "Aquesta moneda està definida a coins.toml però no està integrada en aquesta versió del motor, per tant no es pot negociar.",
  },
  coinWizard: {
    title: "Connecta les teves monedes",
    intro:
      "Tria almenys dues monedes i apunta cadascuna al teu propi node. Un intercanvi necessita dues cadenes, així que la negociació es desbloqueja quan dos nodes estan connectats i actius. Pots afegir o canviar monedes més tard a Configuració.",
    progress: "{count} de {min} monedes connectades",
    continue: "Continua",
    live: "Actiu",
    nodeDown: "Node caigut",
  },
  wallets: {
    intro:
      "Aquestes són les carteres dels teus propis nodes (les que el motor fa servir per finançar intercanvis i rebre guanys) — les teves claus, la teva màquina. Satchel mai conté les teves monedes.",
    hotSeedNudge:
      "Aquesta és una cartera de despesa sobre una llavor calenta, no una caixa forta — escombra els saldos considerables a la teva pròpia cartera freda/core.",
    notConnected: "No connectat",
    notConnectedBody: "Connecta primer un comerciant — la vista de cartera necessita el motor en marxa.",
    noCoins: "Encara no hi ha monedes configurades",
    noCoinsBody: "Connecta una moneda a Configuració → Monedes i la seva cartera apareixerà aquí.",
    goToCoins: "Vés a Monedes",
    watchOnlyTitle: "No hi ha carteres en mode només visualització",
    watchOnlyBody:
      "Aquesta és una sessió només de visualització sense monedes connectades, per tant no hi ha cap cartera per mostrar. Desactiva el mode només visualització a Configuració i connecta una moneda per finançar intercanvis.",
    walletName: "cartera · {wallet}",
    walletScopedHint: "Cada RPC d'aquesta moneda està limitat a aquesta cartera del node.",
    walletDefault: "cartera per defecte (sense àmbit)",
    walletDefaultHint:
      "No hi ha cap cartera definida per a aquesta moneda, així que els RPC fan servir la cartera per defecte del node. Defineix-ne una a Configuració → Monedes per limitar cada crida a una cartera concreta.",
    balanceLabel: "saldo {symbol}",
  },
  corkboard: {
    noBoardTitle: "Cap Corkboard connectat",
    noBoardBody:
      "Un Corkboard és un tauler d'anuncis compartit on els makers fixen ofertes. Mai emparella tractes ni conté monedes — apunta Satchel a un en què confiïs per navegar i publicar.",
    noPairs: "No hi ha cap parell disponible",
    board: "Corkboard",
    boardSettings: "Configura a Configuració",
    filterAll: "Totes",
    filterMine: "Meves",
    noOffers: "No hi ha cap oferta que puguis acceptar ara mateix",
    noOffersBody:
      "Les ofertes apareixen aquí tan bon punt un maker en publica una per a un parell que hagis configurat. També pots publicar les teves.",
    yourOffer: "la teva oferta",
    offerStaged: "publicant…",
    offerStagedTip:
      "Publicada des d'aquest dispositiu i a l'espera de ser confirmada de tornada per un relé. S'està anunciant; passa a estar activa quan un relé la repeteix.",
    take: "Accepta l'oferta",
    legDown: "El node d'una moneda d'aquest parell no funciona — inicia'l (o revisa Configuració → Monedes) abans d'acceptar.",
    withdraw: "Retira",
    withdrawTip: "Retira a l'instant — una oferta mai bloqueja fons",
    safetyRefund: "reemborsament de seguretat",
    safetyRefundTip:
      "Si l'intercanvi s'encalla, les dues parts es reemborsen automàticament — la pota del taker es desbloqueja primer, la teva una mica després. Ningú no acaba encallat.",
    activeTitle: "Els teus intercanvis actius",
    states: {
      takenByUs: "acceptada per tu",
      revoked: "retirada",
      expired: "caducada",
    },
    // Two-sided order-book view of the Corkboard. Bids = makers giving the
    // quote coin to get the base; asks = the reverse. The ladder is a way to
    // READ the board — it never matches or prioritises (load-bearing).
    book: {
      bids: "Demandes",
      asks: "Ofertes",
      bidsHint: "volen {base} · pagant {quote}",
      asksHint: "venent {base} · per {quote}",
      price: "Preu",
      size: "Mida",
      noBids: "Cap demanda",
      noAsks: "Cap oferta",
      spread: "Marge {pct}",
      spreadOneSided: "Una sola banda",
      crossed: "creuat",
      crossedTip: "Demanda màxima ≥ oferta mínima. El tauler mai emparella automàticament, així que aquestes ofertes superposades simplement hi queden — accepta qualsevol banda.",
      mid: "mitjana {price}",
      levelOffers: "{count} oferta/es a aquest preu — tria'n una per acceptar",
      depthTip: "Total de {sym} en oferta a aquest preu repartit en {count} anunci(s).",
      selectLevel: "Tria un nivell de preu a dalt per veure'n les ofertes.",
      paneHeader: "{size} {base} @ {price} {unit}",
      denomTip: "Unitat de visualització per a les quantitats de {coin}",
      showMore: "Mostra'n {count} més",
      showLess: "Mostra els {count} primers",
    },
  },
  relays: {
    title: "Relés",
    subtitle: "Connectivitat en directe amb els teus relés Nostr — la xarxa per on viatgen les teves ofertes i acceptacions. Afegeix o elimina relés a Configuració → Xarxa.",
    connectedCount: "{up} / {total} connectats",
    refresh: "Actualitza",
    ms: "{ms} ms",
    up: "actiu",
    down: "caigut",
    statsTip: "{success}/{attempts} connexions exitoses · ↓{down} ↑{up}",
    none: "Cap relé configurat",
    noneBody: "Afegeix un relé Nostr a Configuració → Xarxa per publicar i rebre ofertes per la xarxa.",
    goToNetwork: "Vés a Configuració",
    notConnected: "No connectat",
    notConnectedBody: "La vista de relés necessita el motor en marxa — connecta primer un comerciant.",
  },
  swaps: {
    maker: "Maker",
    taker: "Taker",
    title: "Intercanvis",
    hint: "El teu llibre complet — intercanvis en curs a dalt, tractes finalitzats a sota. També pots actuar sobre intercanvis actius des del Corkboard.",
    activeTitle: "En curs",
    historyTitle: "Historial",
    none: "Encara no hi ha intercanvis — accepta una oferta al Corkboard.",
    // Active-swaps dock action buttons + the refund-time label.
    cancel: "cancel·la",
    dump: "bolca registres",
    dumpHint: "Copia un paquet de diagnòstic sense secrets (estat + línies de registre) d'aquest intercanvi, per enganxar-lo als desenvolupadors.",
    dumpCopied: "Diagnòstic copiat — enganxa'l als desenvolupadors.",
    dumpFailed: "No s'ha pogut copiar el paquet de diagnòstic.",
    refundAt: "reemborsament {when}",
    // Confirm dialogs for acting on a live swap (ActiveSwaps).
    cancelTitle: "Vols cancel·lar aquest intercanvi?",
    cancelConfirm: "Cancel·la l'intercanvi",
    cancelKeep: "Mantén-lo",
    // Abort reason recorded on the swap when cancelled from Satchel.
    cancelReason: "cancel·lat a Satchel",
    cancelBody:
      "Això abandona l'intercanvi abans que hagis finançat. Encara no es bloqueja res teu, així que no perds res — simplement l'oferta no es completarà.",
    col: {
      swap: "intercanvi",
      role: "rol",
      state: "estat",
      amounts: "dona → rep",
      when: "quan",
      finalTx: "tx final",
    },
    // Expandable per-leg on-chain detail (the audit trail). We show both
    // funding txs + OUR settlement; never the counterparty's settlement or the
    // swap secret.
    audit: {
      toggle: "Mostra el detall on-chain",
      title: "Detall on-chain",
      youLocked: "tu vas bloquejar",
      theyLocked: "ells van bloquejar",
      funding: "Finançament",
      received: "Rebut",
      refunded: "Reemborsat",
      pending: "encara no on-chain",
      copy: "Copia l'id de la transacció",
      copied: "Id de la transacció copiat",
    },
  },
  fees: {
    title: "Previsualització del cost de xarxa",
    estimated: "estimat",
    provisionalNote: "Aquesta versió de pactd encara no exposa l'estimació de comissions.",
    summary: "Un intercanvi són 2 transaccions on-chain que pagues: el finançament a la cadena que dones, el bescanvi a la cadena que reps.",
    fallbackTip: "Un node era inaccessible, així que s'ha fet servir un feerate per defecte conservador — tracta-ho com una estimació.",
    ifItStalls: "(si s'encalla)",
  },
  funds: {
    insufficient:
      "No hi ha prou {sym} per finançar aquest intercanvi — calen ~{need} {sym} (quantitat + comissió de finançament), la cartera té {have} {sym}.",
  },
  wizard: {
    back: "Enrere",
    continue: "Continua",
  },
  // UI-4 docked activity log.
  log: {
    title: "Activitat",
    empty: "— registre d'activitat —",
    count: "{count} línies",
    collapse: "Replega el registre",
    expand: "Desplega el registre",
    // Activity-log lines emitted by the frontend. `{err}` carries a raw engine
    // message (itself not translated — it comes from pactd over the wire).
    noTauri: "no s'està executant dins de Satchel — aquesta interfície necessita el pont Tauri",
    startupError: "inici: {err}",
    notConnected: "no connectat: {err}",
    connected: "connectat a pactd {version} ({protocol})",
    listcoinsError: "listcoins: {err}",
    watchOnlyError: "només visualització: {err}",
    switchedMerchant: "canviat al comerciant {id}",
    renamedMerchant: "comerciant reanomenat a {name}",
    renameMerchantError: "reanomena comerciant: {err}",
    switchMerchantError: "canvi de comerciant: {err}",
    loadMerchantError: "càrrega de comerciant: {err}",
    merchantCreated: "comerciant {id} creat",
    merchantReady: "comerciant a punt",
    actionOk: "{action} {id}: correcte",
    actionError: "{action} {id}: {err}",
    diagCopied: "diagnòstic de {id} copiat ({count} línies de registre) — enganxa'l als desenvolupadors",
    dumpError: "bolcat {id}: {err}",
    coinDisconnected: "{coin} desconnectat",
    removeCoinError: "elimina moneda: {err}",
    tookOffer: "oferta {id} acceptada — ara apareix als teus intercanvis actius a sota",
    takeError: "acceptació: {err}",
    offerWithdrawn: "oferta {id} retirada",
    withdrawError: "retirada: {err}",
    postedOffer: "oferta {id} publicada — retira-la quan vulguis; no es bloqueja res",
    createdSlip: "val d'oferta privada creat — envia'l al teu amic",
    tookPrivateOffer: "oferta privada {id} acceptada — ara apareix als teus intercanvis actius",
    cancelledPrivateOffer: "oferta privada {id} cancel·lada",
    cancelError: "cancel·lació: {err}",
    noticeboardUpdated: "tauler d'anuncis actualitzat",
    feePolicyUpdated: "política de comissions actualitzada",
  },
  // Relative-time + freshness prose from format.ts (rendered via the tr() mirror,
  // since those are pure non-component helpers). Unit letters stay inside the
  // template so a translation owns the whole phrase.
  format: {
    ageUnknown: "antiguitat desconeguda",
    justNow: "ara mateix",
    minutesAgo: "fa {n}m",
    hoursAgo: "fa {n}h",
    daysAgo: "fa {n}d",
    expiryNow: "ara",
    expirySoon: "aviat",
    inMinutes: "en ~{n}m",
    inHours: "en ~{n}h",
    inDays: "en ~{n}d",
    posted: "publicada {age}",
    expires: "caduca {time}",
  },
  // Plain-language swap story per (role, state) — the honest "who is exposed
  // when" framing shown on every active swap. {a}/{b} are coin tickers; {t1}/{t2}
  // are local refund times. Rendered via tr() (narrate() is a pure helper).
  narrate: {
    finalizing: "Has reclamat els teus {got} — confirmacions finals. Mantén l'aplicació oberta fins que s'enterri; els teus {gave} queden protegits fins llavors.",
    initiating:
      "Acceptació enviada — esperant que el maker iniciï l'intercanvi. Encara no es bloqueja res; es cancel·la sol si no responen.",
    created: "Oferta enviada — esperant que l'altra part hi accedeixi. No s'ha compromès res.",
    acceptedMaker: "Termes acordats. Següent: bloqueja els teus {a}. Fins que no financis, encara pots cancel·lar lliurement.",
    acceptedTaker: "Termes acordats. L'altra part bloqueja els seus {a} primer — tu mai envies primer.",
    noncesExchanged:
      "Preparant l'intercanvi privat — intercanviant material de signatura. Encara no es bloqueja res.",
    signedMaker:
      "Les dues parts han signat i els teus {a} estan bloquejats. El teu dimoni reclama els {b} automàticament tan bon punt l'altra part bloqueja i ho confirma. Si alguna cosa s'encalla, els teus {a} tornen a {t1}.",
    signedTaker:
      "Les dues parts han signat. Un cop confirmats els seus {a}, el teu dimoni bloqueja els teus {b} i després reclama els {a} automàticament. Un cop bloquejats els teus {b}, tornen a {t2} si alguna cosa s'encalla.",
    fundedAMaker:
      "Els teus {a} estan bloquejats. Esperant que l'altra part bloquegi els seus {b}. Si no ho fan mai, els teus {a} tornen automàticament a {t1}.",
    fundedATaker:
      "Els seus {a} estan bloquejats i verificats. Següent: bloqueja els teus {b}. Xarxa de seguretat: reemborsament automàtic a {t2} si alguna cosa s'encalla.",
    fundedBMaker: "Tots dos bloquejats. El teu dimoni reclama els {b} tan bon punt es confirmin amb seguretat.",
    fundedBTaker: "Tots dos bloquejats. El teu dimoni reclamarà els {a} en el moment que l'altra part agafi els seus {b}.",
    completed: "Intercanvi complet — els {coin} són a la teva cartera.",
    refunded: "L'intercanvi no s'ha completat, així que els teus {coin} han tornat automàticament. No s'ha perdut res excepte comissions.",
    aborted: "Cancel·lat abans que es mogués cap diner.",
  },
  progress: {
    awaitingLock: "A l'espera del seu bloqueig",
    awaitingClaim: "A l'espera de la seva reclamació",
    theirLock: "Confirmant el seu bloqueig",
    ourLock: "Confirmant el teu bloqueig",
    securing: "Assegurant els teus {coin}",
    funding: "Bloquejant els teus {coin} — desbloqueja la cartera si s'encalla",
    blocks: "+{n} blocs",
    feeBumped: "Comissió apujada",
    reorg: "Reorganització detectada — recomprovant",
  },
  exit: {
    // Exit-gate dialog (fund safety, C6). The engine manages alone, so "keep
    // running" detaches it (it keeps watching timelocks + servicing offers).
    liveTitle: "Hi ha un intercanvi en curs",
    liveBodyOne:
      "1 intercanvi és a mig camí. Està governat per timelocks on-chain — el motor ha de continuar en marxa per bescanviar o reemborsar abans del termini.",
    liveBodyMany:
      "{count} intercanvis són a mig camí. Estan governats per timelocks on-chain — el motor ha de continuar en marxa per bescanviar o reemborsar abans del termini.",
    keepRunningExplain:
      "Tancar la finestra manté el motor en marxa en segon pla, així que acaba l'intercanvi sense interfície. Pots tornar a obrir Satchel quan vulguis per comprovar-lo.",
    forceQuitWarn: "Forçar la sortida ara atura el motor i pot fer perdre fons.",
    // {word} is the confirm word below; a translation may localize both together.
    typeToConfirm: "Per forçar la sortida igualment, escriu {word} a sota.",
    confirmWord: "QUIT",
    keepRunning: "Mantén en marxa, tanca la finestra",
    keepWithdraw: "Mantén en marxa + retira les ofertes",
    keepLeaveOffers: "Mantén en marxa, deixa les ofertes publicades",
    forceQuit: "Força la sortida",
    offersTitle: "Tens ofertes publicades",
    offersBodyOne:
      "1 oferta teva encara és al Corkboard. Les ofertes no bloquegen res, però deixar-la publicada vol dir que les contraparts encara poden acceptar-la mentre Satchel està tancat — el motor atendrà l'acceptació.",
    offersBodyMany:
      "{count} ofertes teves encara són al Corkboard. Les ofertes no bloquegen res, però deixar-les publicades vol dir que les contraparts encara poden acceptar-les mentre Satchel està tancat — el motor atendrà les acceptacions.",
    withdrawExit: "Retira-ho tot i surt",
  },
  unlock: {
    title: "Desbloqueja el comerciant",
    body:
      "La llavor d'aquest comerciant està xifrada. Introdueix la seva contrasenya per desbloquejar-la durant aquesta sessió — Satchel la manté només en memòria i l'oblida en sortir.",
    switchMerchant: "Canvia de comerciant",
    unlock: "Desbloqueja",
  },
  common: {
    cancel: "Cancel·la",
    confirm: "Confirma",
    save: "Desa",
    done: "Fet",
    later: "Més tard",
    retry: "Torna a connectar",
  },
};
