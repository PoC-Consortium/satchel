// The Latin American Spanish (Español, Latinoamérica) string bundle. Mirrors en.ts key-for-key; only values are translated.
import type { Bundle } from "./en";

export const es419: Bundle = {
  app: {
    name: "Satchel",
    tagline: "swaps sin custodia",
  },
  // In-app update notifications (sidebar version badge + dialog).
  update: {
    title: "Actualización disponible",
    upToDate: "Estás al día",
    current: "Instalada",
    latest: "Más reciente",
    notesTitle: "Notas de la versión",
    get: "Obtener la actualización",
    dismiss: "Descartar",
    close: "Cerrar",
    badgeTooltip: "Actualización disponible — haz clic para ver detalles",
    versionTooltip: "Haz clic para buscar actualizaciones",
  },
  // Risk disclaimer (first-run welcome + Settings → About).
  disclaimer: {
    title: "Autocustodia — tus llaves, tu responsabilidad",
    body: "Satchel realiza swaps atómicos sin custodia: solo tú tienes tus llaves, y la seed de un merchant guarda llaves de tránsito activas mientras un swap está en curso. Los protocolos de swap (v1 HTLC y v2 Taproot/MuSig2) están revisados y en vivo en mainnet. Con licencia MIT y provisto tal cual, sin garantía — respalda tu frase de recuperación y úsalo bajo tu propio riesgo.",
  },
  nav: {
    public: "Público",
    corkboard: "Corkboard",
    postOffer: "Publicar una oferta",
    private: "Privado",
    privateCreate: "Crear papeleta",
    privateReceive: "Tomar una papeleta",
    privateSlips: "Mis papeletas",
    swaps: "Swaps",
    relays: "Relays",
    wallets: "Wallets",
    settings: "Ajustes",
    coins: "Monedas",
  },
  makeOffer: {
    title: "Publicar una oferta",
    intro:
      "Publica una oferta firmada en el Corkboard. Nada queda bloqueado — es solo un anuncio; retíralo cuando quieras, y un swap solo comienza cuando alguien lo toma y ambas partes financian.",
    give: "Tú das",
    want: "Tú recibes",
    // Canonical pair + direction: pick the pair, choose Sell/Buy the base, enter
    // the base amount and a quote-per-base price (invariant to direction).
    pair: "Par",
    noPairs: "No hay pares negociables — conecta al menos dos monedas en Ajustes → Monedas.",
    sell: "Vender {sym}",
    buy: "Comprar {sym}",
    amount: "Monto",
    youGive: "Tú das",
    youGet: "Tú recibes",
    price: "Precio",
    priceUnit: "{unit} por {base}",
    pricePlaceholder: "precio unitario",
    balance: "Saldo: {amt} {sym}",
    balanceLoading: "Saldo: …",
    noCoins: "No hay monedas configuradas",
    sameCoin: "Lo que das y lo que recibes deben ser monedas distintas.",
    legDown: "El nodo de una de estas monedas está caído — inícialo (o revisa Ajustes → Monedas) antes de publicar.",
    // Swap-protocol pin (only offered when a pair+network supports more than
    // one). v2 label reuses coins.protoPrivate ("Private (Taproot)").
    protocol: "Tipo de swap",
    protoStandard: "Estándar (HTLC)",
    // Titles for the review/confirm dialog shown before posting (see OfferForm).
    reviewOfferTitle: "Revisa tu oferta",
    reviewSlipTitle: "Revisa tu papeleta",
    term: "Timelock de seguridad",
    termShort: "Corto",
    termMedium: "Medio",
    termLong: "Largo",
    termHint: {
      short: "Corto — los fondos se auto-reembolsan más rápido si el trade se atasca (~12 h / 6 h), con el menor margen de seguridad.",
      medium: "Medio — ventana de reembolso equilibrada (~24 h / 12 h).",
      long: "Largo (más seguro) — el mayor margen de seguridad; auto-reembolso después de ~36 h / 18 h si el trade se atasca.",
    },
    validFor: "Válida por (minutos)",
    validForMins: "{mins} min",
    validForHint:
      "Cuánto tiempo permanece listada la oferta. Mientras estás en línea se mantiene actualizada automáticamente; después de esto expira. Cerrar la app la retira.",
    note: "Oferta de tamaño fijo — nada queda bloqueado hasta que alguien la toma. Los montos son on-chain; pagas las comisiones de red aparte y el Corkboard no cobra nada. El timelock es la ventana de auto-reembolso si un swap se atasca.",
    post: "Publicar oferta",
    makeSlip: "Crear papeleta",
    slipTitle: "Tu papeleta de oferta privada",
    slipExplainer:
      "Envíasela a tu amigo. La pega en Satchel para tomarla. Nada queda bloqueado; expira en {ttl}.",
    copy: "Copiar",
    copied: "Copiado",
    makeAnother: "Crear otra",
    myPrivateTitle: "Mis ofertas privadas",
    myPrivateEmpty: "No hay ofertas privadas pendientes.",
    privateExpires: "expira {when}",
    privateExpired: "expirada",
    cancel: "Cancelar",
    cancelTip: "Dejar de honrar esta papeleta — un amigo que aún la tenga ya no podrá tomarla.",
  },
  takeSlip: {
    open: "Pegar una papeleta",
    title: "Tomar una oferta privada",
    intro:
      "Un amigo te envió una papeleta de oferta privada (empieza con pactoffer1:). Pégala aquí para revisarla y tomarla — igual que una oferta del board.",
    placeholder: "pactoffer1:…",
    take: "Revisar y tomar",
    invalid: "Eso no parece una papeleta — debería empezar con pactoffer1:.",
    previewLabel: "Esta papeleta ofrece",
  },
  // PRIVATE nav group (off-market slips) — screen titles + intros. The form,
  // slip output, my-slips list and take flow reuse makeOffer.*/takeSlip.* copy.
  private: {
    createTitle: "Crear una oferta privada",
    createIntro:
      "Crea una oferta firmada y entrégasela a un amigo como papeleta por tu propio chat. No se lista en ningún lado — y nada queda bloqueado hasta que ambos financien.",
    slipsIntro:
      "Papeletas que has creado. Cualquiera que tenga una papeleta puede tomarla hasta que expire; cancela una para dejar de honrarla antes de eso.",
    slipsEmptyBody: "Crea una oferta privada para obtener una papeleta que puedas enviarle a un amigo.",
    receiveTitle: "Tomar una oferta privada",
    received: "Tomada — síguela en Swaps.",
  },
  // Shared take-confirmation dialog (board take + slip take).
  takeConfirm: {
    title: "¿Tomar esta oferta?",
    confirm: "Tomar oferta",
    counterparty: "Contraparte",
    youGive: "Tú das",
    youReceive: "Tú recibes",
    safetyRefund: "Reembolso de seguridad",
    offerAge: "Antigüedad de la oferta",
    makerFundsFirst:
      "El maker bloquea sus {sym} primero — tú nunca envías primero. Aún puedes cancelar antes de financiar tu lado, y el engine auto-reembolsa después del timelock de seguridad si el swap se atasca.",
  },
  header: {
    activeMerchant: "Merchant activo — haz clic para cambiar o gestionar",
    manageMerchants: "Gestionar Merchants…",
    noMerchant: "sin merchant",
    openMenu: "Abrir menú",
    collapseMenu: "contraer menú",
    settings: "Ajustes",
    language: "Idioma",
    pactConnected: "Engine conectado",
    pactUnreachable: "Engine inalcanzable",
    liveSwapsOne: "1 swap en curso — haz clic para ver",
    liveSwapsMany: "{count} swaps en curso — haz clic para ver",
    liveSwapsNone: "Sin swaps en curso",
    coinOk: "{name} — conectado · tip {tip}",
    coinUnconfigured: "{name} — sin configurar",
    coinError: "{name} — {status}",
    relaysOk: "Relays de Nostr — {up}/{total} conectados",
    relaysDown: "Relays de Nostr — ninguno de {total} conectado",
  },
  network: {
    mainnet: "MainNet",
    testnet: "TestNet",
    regtest: "RegTest",
    signet: "Signet",
    notRealFunds: "No son fondos reales — esta es la red {network}",
  },
  // Watch-only mode: a viewer session with no coins. Browse the board and
  // withdraw your own offers, but no posting / taking / funding.
  watchOnly: {
    badge: "Solo observación",
    badgeTip:
      "Modo solo observación — navega el board y retira tus propias ofertas, pero no puedes publicar, tomar ni financiar. Configura monedas en Ajustes para operar.",
    coinWizardButton: "Navegar en modo solo observación",
    coinWizardHint:
      "Omite la configuración de monedas y solo navega el board (solo lectura). Aún puedes retirar tus propias ofertas — útil para retirar ofertas dejadas por otra sesión. Desactívalo cuando quieras en Ajustes.",
    postBlockedTitle: "Modo solo observación",
    postBlockedBody:
      "Esta es una sesión de solo observación, así que no puede publicar ofertas. Configura al menos dos monedas en Ajustes → Monedas para operar.",
    takeBlockedBody: "Modo solo observación — puedes revisar esta oferta, pero tomarla requiere monedas configuradas.",
    takeBlockedTip: "Modo solo observación — configura monedas en Ajustes para tomar ofertas.",
  },
  merchants: {
    title: "Tus merchants",
    intro:
      "Un merchant es una identidad de trading — con su propia seed e historial de swaps. Operar bajo un merchant distinto mantiene los contextos no vinculables (una identidad desechable). Tus monedas principales viven en tu propia wallet, no aquí.",
    // First-run welcome (empty merchant manager).
    welcomeTitle: "Te damos la bienvenida a Satchel",
    welcomeIntro:
      "Satchel opera bajo un “merchant” — una identidad de trading con su propia seed. Aún no tienes ninguna: crea una nueva, o importa una frase de recuperación existente para empezar.",
    importMerchant: "Importar un merchant",
    none: "Aún no hay merchants.",
    active: "activo",
    switch: "cambiar",
    newMerchant: "Nuevo merchant",
    thisMerchant: "este merchant",
    nameLabel: "Nombre del merchant",
    namePlaceholder: "p. ej. Principal",
    introFirst:
      "Configura tu primera identidad de trading (un “merchant”). Solo guarda llaves de tránsito activas para swaps en curso — tus monedas principales quedan en tu propia wallet.",
    introNew: "Un nuevo merchant es una identidad fresca y separada, con su propia seed e historial de swaps.",
    // UI-5 merchant selector (phoenix wallet-selector parity).
    createNew: "Crear nuevo",
    import: "Importar",
    load: "Cargar Merchant",
    loaded: "cargado",
    locked: "bloqueado",
    lockedTip: "Seed cifrada — desbloquéala con tu passphrase al cargarla.",
    close: "Cerrar",
    idLabel: "carpeta",
    switching: "Cambiando de merchant…",
    switchingBody: "Relanzando el engine contra esa carpeta.",
  },
  // Seed create/import (SeedForm) + counterparty + status.
  seed: {
    intro: "Crea una seed totalmente nueva, o importa una que ya tengas.",
    createNew: "Crear nueva",
    createDesc: "Genera una seed nueva. Tú respaldas la frase de recuperación.",
    import: "Importar",
    importDesc: "Restaura desde una frase existente de 12/24 palabras.",
    recoveryLabel: "Frase de recuperación",
    importPlaceholder: "palabra1 palabra2 palabra3 …",
    encrypt: "Cifrar",
    encryptDesc:
      "Una passphrase protege la seed en reposo. La ingresas una vez por sesión — Satchel nunca la guarda. Nota: el auto-reembolso desatendido se pausa tras un reinicio hasta que la vuelvas a ingresar.",
    noPassphrase: "Sin passphrase (recomendado)",
    noPassphraseDesc:
      "El auto-reembolso sigue funcionando entre reinicios sin nada que ingresar — esta es solo una seed de tránsito activa. Costo: el acceso al archivo/host expone las llaves de tránsito e identidad de este merchant.",
    passphraseLabel: "Passphrase",
    passphrasePlaceholder: "elige una passphrase",
    createTitle: "Crear seed",
    importTitle: "Importar seed",
    secureTitle: "Asegurar {label}",
    revealTitle: "Anota tu frase de recuperación",
    revealBody:
      "Cualquiera con estas palabras controla las llaves activas de este merchant. Satchel no guarda ninguna copia — guárdala fuera de línea. A continuación confirmarás algunas palabras.",
    ackLabel: "He anotado mi frase de recuperación.",
    // Multi-step onboarding (create/import -> secret -> confirm -> passphrase).
    chooseTitle: "Configurar {label}",
    enterTitle: "Importa tu frase de recuperación",
    enterBody:
      "Escribe cada palabra — se autocompletan mientras escribes — o pega la frase completa. La verificamos antes de continuar.",
    wordCount: "{n} palabras",
    wordAria: "Palabra {n}",
    checkIncomplete: "Ingresa las {n} palabras.",
    checkUnknown: "Algunas palabras no están en la lista BIP39 — revisa las resaltadas.",
    checkBadChecksum: "El checksum no coincide — revisa tus palabras y su orden.",
    checkOk: "La frase de recuperación parece válida.",
    verifyTitle: "Confirma tu respaldo",
    verifyBody: "Escribe las palabras en estas posiciones para confirmar que anotaste la frase.",
    verifyWord: "Palabra #{n}",
    verifyMismatch: "Esas no coinciden con tu frase — revisa tu respaldo.",
    passphraseTitle: "Protege la seed",
    passphraseBody:
      "Opcionalmente cifra la seed guardada con una passphrase. Puedes omitir esto — mira el compromiso más abajo.",
  },
  counterparty: {
    you: "Eres tú",
    youShort: "tú",
    unknown: "identidad desconocida",
    // Short fingerprint fallback (identity.ts shortId) when no pubkey is known.
    unknownShort: "desconocida",
  },
  status: {
    notConnectedTitle: "Sin conexión al engine",
    disconnectedBody:
      "Satchel no puede alcanzar el engine. Puede que aún esté iniciando, o que las conexiones de nodo del merchant activo estén caídas. Reintenta, o cambia de merchant desde el selector de arriba.",
    openInSatchel: "Abrir esto en Satchel",
    noTauriBody:
      "Esta es la UI de Satchel — necesita el puente de Tauri para alcanzar el engine. Inicia la app de escritorio (cargo tauri dev) en lugar de un navegador.",
  },
  settings: {
    title: "Ajustes",
    subtitle: "Preferencias generales de la app para esta instalación.",
    // UI-3 Settings tabs.
    tabGeneral: "General",
    tabCoins: "Monedas",
    tabNetwork: "Red",
    tabAbout: "Acerca de",
    appearance: "Apariencia",
    theme: "Tema",
    themeDark: "Oscuro",
    themeLight: "Claro",
    themeSystem: "Sistema",
    themeHint: "Elige cómo se ve Satchel. Sistema sigue el ajuste de tu SO.",
    language: "Idioma",
    languageHint: "Se agregarán más idiomas a medida que se aporten traducciones.",
    mode: "Modo",
    watchOnly: "Modo solo observación",
    watchOnlyHint:
      "Navega el board sin configurar monedas. Aún puedes retirar tus propias ofertas, pero no puedes publicar, tomar ni financiar. Desactívalo para operar (necesitarás al menos dos monedas conectadas).",
    network: "Red",
    boards: "Corkboards",
    boardsDesc:
      "Boards HTTP autoalojados opcionales. Agrega los que confíes; déjalo vacío para depender de Nostr.",
    boardsNone: "Ninguno configurado",
    nostrRelays: "Relays de Nostr",
    nostrRelaysDesc:
      "Los relays transportan el tablón de anuncios sobre una red descentralizada — ningún operador puede leer ni emparejar tus ofertas. Vienen preconfigurados con un conjunto por defecto; edítalos libremente.",
    nostrRelaysOff: "Desactivado — transporte por Nostr deshabilitado",
    addUrl: "Agregar",
    removeUrl: "Quitar",
    relayInvalid: "Ingresa una URL de relay ws:// o wss://",
    boardInvalid: "Ingresa una URL de board http:// o https://",
    netSave: "Guardar y reconectar",
    netSaving: "Guardando y reconectando…",
    netSaved: "Guardado",
    // Fees tab — fee-bump policy (per active merchant).
    tabFees: "Comisiones",
    fees: "Bump de comisiones",
    feesScope: "Estos ajustes aplican al merchant activo.",
    feesIntro:
      "Compromisos de seguridad/costo para los bumps de comisión, no configuración obligatoria. Los nuevos valores aplican a bumps futuros; los swaps ya financiados conservan la política con que fueron financiados.",
    feeMax: "Feerate máximo (sat/vB)",
    feeMaxHint:
      "Tope para cada bump de comisión. Predeterminado 500, también el máximo absoluto del sistema. Bájalo para limitar costos.",
    feeReservation: "Reserva del bump de financiación (×)",
    feeReservationHint:
      "El saldo que la verificación de fondos aparta como margen para bumps. Más alto rescata picos de comisión mayores, pero inmoviliza más saldo y rechaza más swaps. Predeterminado 3.",
    feeCommitted: "Sobreaprovisionamiento del redeem (×)",
    feeCommittedHint:
      "Cuánto extra se paga por adelantado la comisión del redeem v2 para que confirme incluso con Satchel cerrado. Aplica solo a swaps nuevos. Predeterminado 2.",
    feeStep: "Paso de escalada RBF (%)",
    feeStepHint: "Qué tan agresivamente sube la comisión de un gasto atascado en cada pasada del scheduler. Predeterminado 50.",
    feeSave: "Guardar",
    feeSaving: "Guardando…",
    feeSaved: "Guardado",
    feeReset: "Restablecer a predeterminados",
    coins: "Monedas y nodos",
    coinsHint: "Conecta cada moneda a tu propio nodo. Se verifica el génesis antes de guardar nada.",
    about: "Acerca de",
    version: "Versión {version}",
    updateUpToDate: "Al día",
    updateCheckPlaceholder: "La verificación de actualizaciones llega en una versión posterior.",
    trustModel: "Dónde viven tus llaves",
    trustModelBody:
      "Los secretos viven en el engine, nunca en Satchel. La seed del merchant reside en la carpeta de datos del engine (cifrada o en texto plano — tú eliges); Satchel no guarda ninguna seed ni passphrase. La seed es activa por diseño (solo llaves de tránsito) — barre ganancias considerables a tu propia wallet fría.",
  },
  coins: {
    intro:
      "Conecta cada moneda a tu propio nodo. La primera URL es la wallet de tu propio nodo — financia tus piernas de swap y recibe las ganancias. Antes de guardar nada, Satchel verifica el bloque génesis del nodo para que los fondos nunca puedan enviarse a la cadena equivocada. Las conexiones se comparten entre todos tus merchants.",
    networkBadge: "Configurando para la red {network}",
    needMerchant:
      "Conecta primero un merchant — la configuración de monedas necesita el engine en ejecución. Usa el selector de merchant arriba a la derecha.",
    pairsTitle: "Pares de trading",
    pairsHint:
      "Los pares se derivan de lo que cada moneda puede hacer — no hay una lista fija. Un par se abre una vez que ambas monedas están conectadas.",
    noPairs: "No hay pares disponibles.",
    notSetUp: "Sin configurar",
    connectedTip: "Conectado · tip {tip}",
    connError: "Error de conexión",
    setUp: "Configurar",
    editConnection: "Editar conexión",
    remove: "quitar",
    disconnectTip: "Desconectar esta moneda",
    disconnectTitle: "¿Desconectar {coin}?",
    disconnectBody: "Los swaps que la necesiten no estarán disponibles hasta que la reconectes.",
    ready: "Listo para operar",
    connectMissing: "Conectar {coins}",
    notBuildable: "Aún no construible",
    // Swap-protocol chips on a pair (pact-htlc-v1 HTLC vs pact-htlc-v2 adaptor).
    protoPrivate: "Privado (Taproot)",
    protoPrivateTip: "Swap privado (adaptador Taproot/MuSig2) — parece un pago común on-chain",
    protoHtlcTip: "Swap HTLC clásico",
    // Coin-setup backend choices (CoinSetup).
    backendCoreTitle: "Wallet RPC del Core",
    backendCoreDesc: "La wallet de tu propio nodo financia el swap y recibe las ganancias.",
    backendHardwareTitle: "Hardware",
    backendHardwareDesc: "Firma con Ledger / PSBT para la pierna de financiación.",
    backendLater: "después",
    // CoinSetup dialog.
    setupTitle: "Conectar {coin}",
    setupIntro:
      "Apunta Satchel a tu propio nodo {sym}. No se guarda nada hasta que el nodo pase una verificación del bloque génesis — tus fondos solo tocan la cadena {sym} real.",
    backendUrlLabel: "URL(s) del backend del nodo",
    backendUrlHint:
      "Primera URL = la wallet de tu propio nodo (financia swaps, recibe ganancias). Agrega servidores Electrum (tcp://host:port) después de comas para vistas de cadena adicionales e independientes.",
    fundingWallet: "Wallet de financiación",
    confirmationsLabel: "Confirmaciones antes de definitivo",
    confirmationsHint:
      "Qué tan profunda debe estar una financiación o redeem en esta cadena antes de que un swap actúe sobre ella — el margen de seguridad ante reorgs. Más alto es más seguro pero más lento; déjalo en blanco para el predeterminado recomendado ({default}).",
    validateNode: "Validar nodo",
    checking: "Verificando el nodo…",
    genesisOk: "Génesis coincide — esta es la cadena correcta",
    genesisDetail: "altura del tip {tip} · génesis {hash}…",
    genesisBad: "Rechazado — no se guarda",
    errorShort: "error",
    // Structured connection form (CoinSetup v2).
    rpcHostLabel: "Host RPC",
    rpcPortLabel: "Puerto RPC",
    authMethodLabel: "Autenticación",
    authCookie: "Archivo cookie",
    authCookieDesc: "Lee automáticamente el .cookie del nodo desde su directorio de datos (el predeterminado, sin guardar contraseña).",
    authUserPass: "Usuario / contraseña",
    authUserPassDesc: "El rpcuser / rpcpassword de la configuración de tu nodo — necesario para un nodo remoto.",
    rpcUserLabel: "Usuario RPC",
    rpcPasswordLabel: "Contraseña RPC",
    datadirLabel: "Directorio de datos del nodo",
    cookiePathNote: "La cookie se lee desde {path} bajo este directorio.",
    walletLabel: "Nombre de wallet (opcional)",
    walletPlaceholder: "la wallet de tu nodo",
    needPort: "Ingresa primero el puerto RPC.",
    validateFirst: "Valida el nodo antes de guardar.",
    savingReconnecting: "Guardando y reconectando…",
    connected: "{coin} conectado",
    // Template picker (a coins.toml coin the engine version doesn't support).
    unsupportedByEngine: "No compatible",
    unsupportedByEngineTip:
      "Esta moneda está definida en coins.toml pero no está integrada en esta versión del engine, así que no se puede operar.",
  },
  coinWizard: {
    title: "Conecta tus monedas",
    intro:
      "Elige al menos dos monedas y apunta cada una a tu propio nodo. Un swap necesita dos cadenas, así que operar se desbloquea una vez que dos nodos estén conectados y en vivo. Puedes agregar o cambiar monedas más tarde en Ajustes.",
    progress: "{count} de {min} monedas conectadas",
    continue: "Continuar",
    live: "En vivo",
    nodeDown: "Nodo caído",
  },
  wallets: {
    intro:
      "Estas son las wallets de tus propios nodos (los que el engine usa para financiar swaps y recibir ganancias) — tus llaves, tu máquina. Satchel nunca tiene tus monedas.",
    hotSeedNudge:
      "Esta es una wallet de gasto sobre una seed activa, no una bóveda — barre saldos considerables a tu propia wallet fría/core.",
    notConnected: "Sin conexión",
    notConnectedBody: "Conecta primero un merchant — la vista de wallet necesita el engine en ejecución.",
    noCoins: "Aún no hay monedas configuradas",
    noCoinsBody: "Conecta una moneda en Ajustes → Monedas y su wallet aparece aquí.",
    goToCoins: "Ir a Monedas",
    watchOnlyTitle: "No hay wallets en modo solo observación",
    watchOnlyBody:
      "Esta es una sesión de solo observación sin monedas conectadas, así que no hay wallets que mostrar. Desactiva el modo solo observación en Ajustes y conecta una moneda para financiar swaps.",
    walletName: "wallet · {wallet}",
    walletScopedHint: "Cada RPC de esta moneda se acota a esta wallet del nodo.",
    walletDefault: "wallet predeterminada (sin acotar)",
    walletDefaultHint:
      "No hay wallet configurada para esta moneda, así que las RPC usan la wallet predeterminada del nodo. Configura una en Ajustes → Monedas para acotar cada llamada a una wallet específica.",
    balanceLabel: "Saldo de {symbol}",
    receive: "Recibir",
    send: "Enviar",
    sendTo: "Enviar a la dirección",
    amount: "Monto",
    sendTitle: "¿Enviar {amount} {sym}?",
    sendConfirmBody: "A {to}\n\nEsto gasta desde la wallet de tu propio nodo y no se puede deshacer.",
  },
  corkboard: {
    noBoardTitle: "Ningún Corkboard conectado",
    noBoardBody:
      "Un Corkboard es un tablón de anuncios compartido donde los makers fijan ofertas. Nunca empareja trades ni guarda monedas — apunta Satchel a uno en el que confíes para navegar y publicar.",
    noPairs: "No hay pares disponibles",
    board: "Corkboard",
    boardSettings: "Configurar en Ajustes",
    filterAll: "Todas",
    filterMine: "Mías",
    offered: "{symbol} en oferta",
    noOffers: "No hay ofertas que puedas tomar ahora mismo",
    noOffersBody:
      "Las ofertas aparecen aquí en cuanto un maker publica una para un par que hayas configurado. También puedes publicar las tuyas.",
    hiddenOffers:
      "{count} oferta(s) más para pares que no has conectado. Configura ambas monedas para operarlas:",
    yourOffer: "tu oferta",
    offerStaged: "publicando…",
    offerStagedTip:
      "Publicada desde este dispositivo y esperando confirmación de vuelta desde un relay. Se está anunciando; queda en vivo una vez que un relay la replica.",
    take: "Tomar oferta",
    legDown: "El nodo de una de las monedas de este par está caído — inícialo (o revisa Ajustes → Monedas) antes de tomar.",
    withdraw: "Retirar",
    withdrawTip: "Retira al instante — una oferta nunca bloquea fondos",
    safetyRefund: "reembolso de seguridad",
    safetyRefundTip:
      "Si el swap se atasca, ambas partes se auto-reembolsan — la pierna del taker se desbloquea primero, la tuya un poco después. Nadie queda atrapado.",
    activeTitle: "Tus swaps activos",
    states: {
      open: "abierta",
      takenByUs: "tomada por ti",
      revoked: "retirada",
      expired: "expirada",
    },
    // Two-sided order-book view of the Corkboard. Bids = makers giving the
    // quote coin to get the base; asks = the reverse. The ladder is a way to
    // READ the board — it never matches or prioritises (load-bearing).
    book: {
      bids: "Compras",
      asks: "Ventas",
      bidsHint: "quieren {base} · pagando {quote}",
      asksHint: "vendiendo {base} · por {quote}",
      price: "Precio",
      size: "Tamaño",
      noBids: "Sin compras",
      noAsks: "Sin ventas",
      spread: "Spread {pct}",
      spreadOneSided: "Un solo lado",
      crossed: "cruzado",
      crossedTip: "Mejor compra ≥ mejor venta. El board nunca empareja automáticamente, así que estas ofertas superpuestas simplemente quedan ahí — toma cualquier lado.",
      mid: "medio {price}",
      levelOffers: "{count} oferta(s) a este precio — elige una para tomar",
      depthTip: "Total de {sym} en oferta a este precio en {count} anuncio(s).",
      takerNote: "Al tomarla, das {give} y recibes {get}.",
      selectLevel: "Elige un nivel de precio arriba para ver las ofertas allí.",
      paneHeader: "{size} {base} @ {price} {unit}",
      denomTip: "Unidad de visualización para montos de {coin}",
      showMore: "Mostrar {count} más",
      showLess: "Mostrar las {count} principales",
    },
  },
  relays: {
    title: "Relays",
    subtitle: "Conectividad en vivo a tus relays de Nostr — la red por la que viajan tus ofertas y tomas. Agrega o quita relays en Ajustes → Red.",
    connectedCount: "{up} / {total} conectados",
    refresh: "Actualizar",
    ms: "{ms} ms",
    up: "activo",
    down: "caído",
    statsTip: "{success}/{attempts} conexiones exitosas · ↓{down} ↑{up}",
    none: "No hay relays configurados",
    noneBody: "Agrega un relay de Nostr en Ajustes → Red para publicar y recibir ofertas por la red.",
    goToNetwork: "Ir a Ajustes",
    notConnected: "Sin conexión",
    notConnectedBody: "La vista de relays necesita el engine en ejecución — conecta primero un merchant.",
  },
  swaps: {
    title: "Swaps",
    hint: "Tu libro mayor completo — swaps en curso arriba, trades finalizados abajo. También puedes actuar sobre swaps en vivo desde el Corkboard.",
    activeTitle: "En curso",
    historyTitle: "Historial",
    none: "Aún no hay swaps — toma una oferta en el Corkboard.",
    // Active-swaps dock action buttons + the refund-time label.
    cancel: "cancelar",
    refund: "reembolsar",
    dump: "volcar logs",
    dumpHint: "Copia un paquete de diagnóstico sin secretos (estado + líneas de log) de este swap, para pegárselo a los desarrolladores.",
    dumpCopied: "Diagnóstico copiado — pégaselo a los desarrolladores.",
    dumpFailed: "No se pudo copiar el paquete de diagnóstico.",
    refundAt: "reembolso {when}",
    // Confirm dialogs for acting on a live swap (ActiveSwaps).
    cancelTitle: "¿Cancelar este swap?",
    cancelConfirm: "Cancelar swap",
    cancelKeep: "Mantenerlo",
    // Abort reason recorded on the swap when cancelled from Satchel.
    cancelReason: "cancelado en Satchel",
    cancelBody:
      "Esto abandona el swap antes de que hayas financiado. Nada tuyo está bloqueado todavía, así que no pierdes nada — la oferta simplemente no se completará.",
    refundTitle: "¿Recuperar tus fondos?",
    refundConfirm: "Reembolsar",
    refundBody:
      "El timelock de seguridad ha pasado, así que puedes reclamar los fondos que bloqueaste. Esto transmite tu reembolso ahora; el engine también lo hace automáticamente después de la fecha límite.",
    col: {
      swap: "swap",
      role: "rol",
      state: "estado",
      amounts: "das → recibes",
      when: "cuándo",
      finalTx: "tx final",
    },
    // Expandable per-leg on-chain detail (the audit trail). We show both
    // funding txs + OUR settlement; never the counterparty's settlement or the
    // swap secret.
    audit: {
      toggle: "Mostrar detalle on-chain",
      title: "Detalle on-chain",
      youLocked: "tú bloqueaste",
      theyLocked: "ellos bloquearon",
      funding: "Financiación",
      received: "Recibido",
      refunded: "Reembolsado",
      pending: "aún no on-chain",
      copy: "Copiar id de transacción",
      copied: "Id de transacción copiado",
    },
  },
  fees: {
    title: "Vista previa del costo de red",
    estimated: "estimado",
    provisionalNote: "Este build de pactd aún no expone estimación de comisiones.",
    summary: "Un swap son 2 transacciones on-chain que pagas: financiación en la cadena que das, redeem en la cadena que recibes.",
    fallbackTip: "Un nodo estaba inalcanzable, así que se usó una tasa de comisión conservadora por defecto — tómalo como una estimación.",
    ifItStalls: "(si se atasca)",
  },
  funds: {
    insufficient:
      "No hay suficiente {sym} para financiar este swap — se necesitan ~{need} {sym} (monto + comisión de financiación), la wallet tiene {have} {sym}.",
  },
  wizard: {
    welcome: "Te damos la bienvenida a Satchel",
    connectTitle: "Conecta el engine Pact",
    connectIntro:
      "Satchel es un cliente ligero del engine Pact — el núcleo que guarda tus llaves y ejecuta los swaps. Elige cómo alcanzarlo.",
    managed: "Ejecutar el engine Pact integrado",
    managedDesc: "Satchel inicia y supervisa su propio engine Pact. Recomendado.",
    external: "Conectar a un engine Pact externo",
    externalDesc: "Apunta a un engine Pact que ya ejecutes (configura SATCHEL_PACTD_URL + cookie antes de iniciar).",
    externalNote:
      "El modo externo se selecciona mediante variables de entorno antes de iniciar Satchel. Relanza con SATCHEL_PACTD_URL configurada para usarlo.",
    coinsTitle: "Agrega tus monedas",
    coinsIntro:
      "Después de crear tu merchant, conecta cada moneda a tu propio nodo en Ajustes → Monedas. Elige una moneda y un backend (Electrum público para cero configuración, o tu propio nodo); el génesis se verifica contra esta red antes de guardar nada.",
    coinsTemplatesSoon: "Las plantillas de monedas de un clic llegan aquí en una versión posterior.",
    back: "Atrás",
    continue: "Continuar",
    finish: "Finalizar configuración",
  },
  // UI-4 docked activity log.
  log: {
    title: "Actividad",
    empty: "— registro de actividad —",
    count: "{count} líneas",
    collapse: "Contraer registro",
    expand: "Expandir registro",
    // Activity-log lines emitted by the frontend. `{err}` carries a raw engine
    // message (itself not translated — it comes from pactd over the wire).
    noTauri: "no se está ejecutando dentro de Satchel — esta UI necesita el puente de Tauri",
    startupError: "inicio: {err}",
    notConnected: "sin conexión: {err}",
    connected: "conectado a pactd {version} ({protocol})",
    listcoinsError: "listcoins: {err}",
    watchOnlyError: "solo observación: {err}",
    switchedMerchant: "cambiado al merchant {id}",
    switchMerchantError: "cambiar merchant: {err}",
    loadMerchantError: "cargar merchant: {err}",
    merchantCreated: "merchant {id} creado",
    merchantReady: "merchant listo",
    actionOk: "{action} {id}: ok",
    actionError: "{action} {id}: {err}",
    diagCopied: "diagnóstico de {id} copiado ({count} líneas de log) — pégaselo a los devs",
    dumpError: "volcar {id}: {err}",
    coinDisconnected: "{coin} desconectado",
    removeCoinError: "quitar moneda: {err}",
    tookOffer: "oferta {id} tomada — ahora aparece en tus swaps activos abajo",
    takeError: "tomar: {err}",
    offerWithdrawn: "oferta {id} retirada",
    withdrawError: "retirar: {err}",
    postedOffer: "oferta {id} publicada — retírala cuando quieras; nada queda bloqueado",
    createdSlip: "papeleta de oferta privada creada — envíasela a tu amigo",
    tookPrivateOffer: "oferta privada {id} tomada — ahora aparece en tus swaps activos",
    cancelledPrivateOffer: "oferta privada {id} cancelada",
    cancelError: "cancelar: {err}",
    noticeboardUpdated: "tablón de anuncios actualizado",
    feePolicyUpdated: "política de comisiones actualizada",
  },
  // Relative-time + freshness prose from format.ts (rendered via the tr() mirror,
  // since those are pure non-component helpers). Unit letters stay inside the
  // template so a translation owns the whole phrase.
  format: {
    ageUnknown: "antigüedad desconocida",
    justNow: "ahora mismo",
    minutesAgo: "hace {n} min",
    hoursAgo: "hace {n} h",
    daysAgo: "hace {n} d",
    expiryNow: "ahora",
    expirySoon: "pronto",
    inMinutes: "en ~{n} min",
    inHours: "en ~{n} h",
    inDays: "en ~{n} d",
    posted: "publicada {age}",
    expires: "expira {time}",
  },
  // Plain-language swap story per (role, state) — the honest "who is exposed
  // when" framing shown on every active swap. {a}/{b} are coin tickers; {t1}/{t2}
  // are local refund times. Rendered via tr() (narrate() is a pure helper).
  narrate: {
    initiating:
      "Toma enviada — esperando a que el maker inicie el swap. Nada queda bloqueado todavía; se cancela por sí solo si no responden.",
    created: "Oferta enviada — esperando a que la otra parte acepte. Nada queda comprometido.",
    acceptedMaker: "Términos acordados. Siguiente: bloquea tus {a}. Hasta que financies, aún puedes cancelar libremente.",
    acceptedTaker: "Términos acordados. La otra parte bloquea sus {a} primero — tú nunca envías primero.",
    noncesExchanged:
      "Configurando el swap privado — intercambiando material de firma. Nada queda bloqueado todavía.",
    signedMaker:
      "Ambas partes firmaron. Tu daemon bloquea los {a}, luego reclama los {b} automáticamente. Si algo se atasca, tus {a} regresan a las {t1}.",
    signedTaker:
      "Ambas partes firmaron. Tu daemon bloquea los {b} y reclama los {a} en cuanto la otra parte se mueva. Red de seguridad: reembolso a las {t2}.",
    fundedAMaker:
      "Tus {a} están bloqueados. Esperando a que la otra parte bloquee sus {b}. Si nunca lo hacen, tus {a} regresan automáticamente a las {t1}.",
    fundedATaker:
      "Sus {a} están bloqueados y verificados. Siguiente: bloquea tus {b}. Red de seguridad: reembolso automático a las {t2} si algo se atasca.",
    fundedBMaker: "Ambos bloqueados. Tu daemon reclama los {b} en cuanto estén confirmados de forma segura.",
    fundedBTaker: "Ambos bloqueados. Tu daemon reclamará los {a} en el momento en que la otra parte tome sus {b}.",
    redeemedB:
      "Reclamaste los {b} — esperando a que confirmen. Tus {a} bloqueados siguen protegidos hasta que esto sea definitivo.",
    completed: "Swap completo — los {coin} están en tu wallet.",
    refunded: "El swap no se completó, así que tus {coin} regresaron automáticamente. No se perdió nada salvo comisiones.",
    aborted: "Cancelado antes de que se moviera dinero.",
  },
  exit: {
    // Exit-gate dialog (fund safety, C6). The engine manages alone, so "keep
    // running" detaches it (it keeps watching timelocks + servicing offers).
    liveTitle: "Hay un swap en curso",
    liveBodyOne:
      "1 swap está en curso. Está gobernado por timelocks on-chain — el engine debe seguir ejecutándose para hacer el redeem o reembolsar antes de la fecha límite.",
    liveBodyMany:
      "{count} swaps están en curso. Están gobernados por timelocks on-chain — el engine debe seguir ejecutándose para hacer el redeem o reembolsar antes de la fecha límite.",
    keepRunningExplain:
      "Cerrar la ventana mantiene el engine ejecutándose en segundo plano, así que termina el swap sin interfaz. Puedes reabrir Satchel cuando quieras para revisarlo.",
    forceQuitWarn: "Forzar el cierre ahora detiene el engine y puede perder fondos.",
    // {word} is the confirm word below; a translation may localize both together.
    typeToConfirm: "Para forzar el cierre de todos modos, escribe {word} abajo.",
    confirmWord: "QUIT",
    keepRunning: "Seguir ejecutando, cerrar ventana",
    keepWithdraw: "Seguir ejecutando + retirar ofertas",
    keepLeaveOffers: "Seguir ejecutando, dejar las ofertas",
    forceQuit: "Forzar cierre",
    offersTitle: "Tienes ofertas publicadas",
    offersBodyOne:
      "1 oferta tuya sigue en el Corkboard. Las ofertas no bloquean nada, pero dejarla puesta significa que las contrapartes aún pueden tomarla mientras Satchel está cerrado — el engine atenderá la toma.",
    offersBodyMany:
      "{count} ofertas tuyas siguen en el Corkboard. Las ofertas no bloquean nada, pero dejarlas puestas significa que las contrapartes aún pueden tomarlas mientras Satchel está cerrado — el engine atenderá las tomas.",
    withdrawExit: "Retirar todo y salir",
  },
  unlock: {
    title: "Desbloquear merchant",
    body:
      "La seed de este merchant está cifrada. Ingresa su passphrase para desbloquearla en esta sesión — Satchel la mantiene solo en memoria y la olvida al salir.",
    switchMerchant: "Cambiar merchant",
    unlock: "Desbloquear",
  },
  common: {
    cancel: "Cancelar",
    confirm: "Confirmar",
    save: "Guardar",
    done: "Listo",
    later: "Después",
    retry: "Reintentar conexión",
  },
};
