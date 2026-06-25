// The Spanish (Español) string bundle. Mirrors en.ts key-for-key; only values are translated.
import type { Bundle } from "./en";

export const es: Bundle = {
  app: {
    name: "Satchel",
    tagline: "swaps sin custodia",
  },
  // In-app update notifications (sidebar version badge + dialog).
  update: {
    title: "Actualización disponible",
    upToDate: "Estás al día",
    current: "Instalada",
    latest: "Última",
    notesTitle: "Notas de la versión",
    get: "Obtener la actualización",
    dismiss: "Descartar",
    close: "Cerrar",
    badgeTooltip: "Actualización disponible — haz clic para ver detalles",
    versionTooltip: "Haz clic para buscar actualizaciones",
  },
  // Risk disclaimer (first-run welcome + Settings → About).
  disclaimer: {
    title: "Autocustodia — tus claves, tu responsabilidad",
    body: "Satchel realiza atomic swaps sin custodia: solo tú tienes tus claves, y la semilla de un merchant guarda claves de tránsito en caliente mientras un swap está en curso. Los protocolos de swap (HTLC v1 y Taproot/MuSig2 v2) están revisados y operativos en mainnet. Con licencia MIT y proporcionado tal cual, sin garantía — haz una copia de seguridad de tu frase de recuperación y úsalo bajo tu propia responsabilidad.",
  },
  nav: {
    public: "Público",
    corkboard: "Corkboard",
    postOffer: "Publicar una oferta",
    private: "Privado",
    privateCreate: "Crear cupón",
    privateReceive: "Tomar un cupón",
    privateSlips: "Mis cupones",
    swaps: "Swaps",
    relays: "Relays",
    wallets: "Carteras",
    settings: "Ajustes",
    coins: "Monedas",
  },
  makeOffer: {
    title: "Publicar una oferta",
    intro:
      "Publica una oferta firmada en el Corkboard. No se bloquea nada — es solo un anuncio; retírala cuando quieras, y un swap solo empieza cuando alguien la toma y ambas partes financian.",
    give: "Tú das",
    want: "Tú recibes",
    // Canonical pair + direction: pick the pair, choose Sell/Buy the base, enter
    // the base amount and a quote-per-base price (invariant to direction).
    pair: "Par",
    noPairs: "No hay pares negociables — conecta al menos dos monedas en Ajustes → Monedas.",
    sell: "Vender {sym}",
    buy: "Comprar {sym}",
    amount: "Cantidad",
    youGive: "Tú das",
    youGet: "Tú obtienes",
    price: "Precio",
    priceUnit: "{unit} por {base}",
    pricePlaceholder: "precio unitario",
    balance: "Saldo: {amt} {sym}",
    balanceLoading: "Saldo: …",
    noCoins: "No hay monedas configuradas",
    sameCoin: "Las monedas que das y recibes deben ser diferentes.",
    legDown: "El nodo de una de estas monedas está caído — inícialo (o revisa Ajustes → Monedas) antes de publicar.",
    // Swap-protocol pin (only offered when a pair+network supports more than
    // one). v2 label reuses coins.protoPrivate ("Private (Taproot)").
    protocol: "Tipo de swap",
    protoStandard: "Estándar (HTLC)",
    // Titles for the review/confirm dialog shown before posting (see OfferForm).
    reviewOfferTitle: "Revisa tu oferta",
    reviewSlipTitle: "Revisa tu cupón",
    term: "Timelock de seguridad",
    termShort: "Corto",
    termMedium: "Medio",
    termLong: "Largo",
    termHint: {
      short: "Corto — el reembolso automático es el más rápido si la operación se atasca (~12h / 6h), con el menor margen de seguridad.",
      medium: "Medio — ventana de reembolso equilibrada (~24h / 12h).",
      long: "Largo (el más seguro) — el mayor margen de seguridad; reembolso automático tras ~36h / 18h si la operación se atasca.",
    },
    validFor: "Válida durante (minutos)",
    validForMins: "{mins} min",
    validForHint:
      "Cuánto tiempo permanece listada la oferta. Mientras estás en línea se mantiene actualizada automáticamente; pasado este tiempo caduca. Cerrar la app la retira.",
    note: "Oferta de tamaño fijo — no se bloquea nada hasta que alguien la toma. Las cantidades son on-chain; tú pagas las comisiones de red aparte y el Corkboard no cobra nada. El timelock es la ventana de reembolso automático si un swap se atasca.",
    post: "Publicar oferta",
    makeSlip: "Crear cupón",
    slipTitle: "Tu cupón de oferta privada",
    slipExplainer:
      "Envíalo a tu contacto. Lo pega en Satchel para tomarlo. No se bloquea nada; caduca en {ttl}.",
    copy: "Copiar",
    copied: "Copiado",
    makeAnother: "Crear otro",
    myPrivateTitle: "Mis ofertas privadas",
    myPrivateEmpty: "No hay ofertas privadas pendientes.",
    privateExpires: "caduca {when}",
    privateExpired: "caducada",
    cancel: "Cancelar",
    cancelTip: "Dejar de honrar este cupón — un contacto que todavía lo tenga ya no podrá tomarlo.",
  },
  takeSlip: {
    open: "Pegar un cupón",
    title: "Tomar una oferta privada",
    intro:
      "Un contacto te ha enviado un cupón de oferta privada (empieza por pactoffer1:). Pégalo aquí para revisarlo y tomarlo — exactamente igual que una oferta del tablón.",
    placeholder: "pactoffer1:…",
    take: "Revisar y tomar",
    invalid: "Eso no parece un cupón — debería empezar por pactoffer1:.",
    previewLabel: "Este cupón ofrece",
  },
  // PRIVATE nav group (off-market slips) — screen titles + intros. The form,
  // slip output, my-slips list and take flow reuse makeOffer.*/takeSlip.* copy.
  private: {
    createTitle: "Crear una oferta privada",
    createIntro:
      "Crea una oferta firmada y entrégala a un contacto como un cupón por tu propio chat. No se lista en ningún sitio — y no se bloquea nada hasta que ambos financiáis.",
    slipsIntro:
      "Cupones que has creado. Cualquiera que tenga un cupón puede tomarlo hasta que caduque; cancela uno para dejar de honrarlo antes de entonces.",
    slipsEmptyBody: "Crea una oferta privada para obtener un cupón que puedas enviar a un contacto.",
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
      "El maker bloquea sus {sym} primero — tú nunca envías primero. Aún puedes cancelar antes de financiar tu parte, y el motor reembolsa automáticamente tras el timelock de seguridad si el swap se atasca.",
  },
  header: {
    activeMerchant: "Merchant activo — haz clic para cambiar o gestionar",
    manageMerchants: "Gestionar merchants…",
    noMerchant: "sin merchant",
    openMenu: "Abrir menú",
    collapseMenu: "contraer menú",
    settings: "Ajustes",
    language: "Idioma",
    pactConnected: "Motor conectado",
    pactUnreachable: "Motor inaccesible",
    liveSwapsOne: "1 swap en curso — haz clic para ver",
    liveSwapsMany: "{count} swaps en curso — haz clic para ver",
    liveSwapsNone: "No hay swaps en curso",
    coinOk: "{name} — conectada · tip {tip}",
    coinUnconfigured: "{name} — sin configurar",
    coinError: "{name} — {status}",
    relaysOk: "Relays Nostr — {up}/{total} conectados",
    relaysDown: "Relays Nostr — ninguno de {total} conectado",
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
    badge: "Solo lectura",
    badgeTip:
      "Modo solo lectura — explora el tablón y retira tus propias ofertas, pero no puedes publicar, tomar ni financiar. Configura monedas en Ajustes para operar.",
    coinWizardButton: "Explorar en modo solo lectura",
    coinWizardHint:
      "Omite la configuración de monedas y solo explora el tablón (solo lectura). Aún puedes retirar tus propias ofertas — útil para retirar ofertas dejadas por otra sesión. Desactívalo cuando quieras en Ajustes.",
    postBlockedTitle: "Modo solo lectura",
    postBlockedBody:
      "Esta es una sesión de solo lectura, así que no puede publicar ofertas. Configura al menos dos monedas en Ajustes → Monedas para operar.",
    takeBlockedBody: "Modo solo lectura — puedes revisar esta oferta, pero tomarla requiere tener monedas configuradas.",
    takeBlockedTip: "Modo solo lectura — configura monedas en Ajustes para tomar ofertas.",
  },
  merchants: {
    title: "Tus merchants",
    intro:
      "Un merchant es una identidad de trading — con su propia semilla e historial de swaps. Operar bajo un merchant distinto mantiene los contextos no vinculables (una identidad desechable). Tus monedas principales viven en tu propia cartera, no aquí.",
    // First-run welcome (empty merchant manager).
    welcomeTitle: "Te damos la bienvenida a Satchel",
    welcomeIntro:
      "Satchel opera bajo un «merchant» — una identidad de trading con su propia semilla. Todavía no tienes ninguna: crea una nueva, o importa una frase de recuperación existente para empezar.",
    importMerchant: "Importar un merchant",
    none: "Aún no hay merchants.",
    active: "activo",
    switch: "cambiar",
    newMerchant: "Nuevo merchant",
    thisMerchant: "este merchant",
    nameLabel: "Nombre del merchant",
    namePlaceholder: "p. ej. Principal",
    introFirst:
      "Configura tu primera identidad de trading (un «merchant»). Solo guarda claves de tránsito en caliente para los swaps en curso — tus monedas principales se quedan en tu propia cartera.",
    introNew: "Un nuevo merchant es una identidad fresca e independiente con su propia semilla e historial de swaps.",
    // UI-5 merchant selector (phoenix wallet-selector parity).
    createNew: "Crear nuevo",
    import: "Importar",
    load: "Cargar merchant",
    loaded: "cargado",
    locked: "bloqueado",
    lockedTip: "Semilla cifrada — desbloquéala con tu frase de contraseña al cargarla.",
    close: "Cerrar",
    idLabel: "carpeta",
    switching: "Cambiando de merchant…",
    switchingBody: "Reiniciando el motor contra esa carpeta.",
  },
  // Seed create/import (SeedForm) + counterparty + status.
  seed: {
    intro: "Crea una semilla totalmente nueva, o importa una que ya tengas.",
    createNew: "Crear nueva",
    createDesc: "Genera una semilla nueva. Tú haces la copia de seguridad de la frase de recuperación.",
    import: "Importar",
    importDesc: "Restaura desde una frase existente de 12/24 palabras.",
    recoveryLabel: "Frase de recuperación",
    importPlaceholder: "palabra1 palabra2 palabra3 …",
    encrypt: "Cifrar",
    encryptDesc:
      "Una frase de contraseña protege la semilla en reposo. La introduces una vez por sesión — Satchel nunca la almacena. Nota: el reembolso automático desatendido se pausa tras un reinicio hasta que la vuelvas a introducir.",
    noPassphrase: "Sin frase de contraseña (recomendado)",
    noPassphraseDesc:
      "El reembolso automático sigue funcionando tras los reinicios sin tener que introducir nada — esto es solo una semilla de tránsito en caliente. Coste: el acceso al archivo/host expone las claves de tránsito + la identidad de este merchant.",
    passphraseLabel: "Frase de contraseña",
    passphrasePlaceholder: "elige una frase de contraseña",
    createTitle: "Crear semilla",
    importTitle: "Importar semilla",
    secureTitle: "Asegurar {label}",
    revealTitle: "Anota tu frase de recuperación",
    revealBody:
      "Cualquiera con estas palabras controla las claves en caliente de este merchant. Satchel no guarda ninguna copia — almacénala sin conexión. A continuación confirmarás algunas palabras.",
    ackLabel: "He anotado mi frase de recuperación.",
    // Multi-step onboarding (create/import -> secret -> confirm -> passphrase).
    chooseTitle: "Configurar {label}",
    enterTitle: "Importa tu frase de recuperación",
    enterBody:
      "Escribe cada palabra — se autocompletan a medida que avanzas — o pega la frase completa. La comprobamos antes de que continúes.",
    wordCount: "{n} palabras",
    wordAria: "Palabra {n}",
    checkIncomplete: "Introduce las {n} palabras.",
    checkUnknown: "Algunas palabras no están en la lista BIP39 — revisa las resaltadas.",
    checkBadChecksum: "El checksum no coincide — vuelve a revisar tus palabras y su orden.",
    checkOk: "La frase de recuperación parece válida.",
    verifyTitle: "Confirma tu copia de seguridad",
    verifyBody: "Escribe las palabras de estas posiciones para confirmar que has anotado la frase.",
    verifyWord: "Palabra n.º {n}",
    verifyMismatch: "Esas no coinciden con tu frase — revisa tu copia de seguridad.",
    passphraseTitle: "Proteger la semilla",
    passphraseBody:
      "Opcionalmente cifra la semilla almacenada con una frase de contraseña. Puedes omitirlo — consulta el compromiso a continuación.",
  },
  counterparty: {
    you: "Este eres tú",
    youShort: "tú",
    unknown: "identidad desconocida",
    // Short fingerprint fallback (identity.ts shortId) when no pubkey is known.
    unknownShort: "desconocido",
  },
  status: {
    notConnectedTitle: "Sin conexión con el motor",
    disconnectedBody:
      "Satchel no puede alcanzar el motor. Puede que aún esté arrancando, o que las conexiones de nodos del merchant activo estén caídas. Reintenta, o cambia de merchant desde el selector de arriba.",
    openInSatchel: "Abre esto en Satchel",
    noTauriBody:
      "Esta es la interfaz de Satchel — necesita el puente Tauri para alcanzar el motor. Lanza la app de escritorio (cargo tauri dev) en lugar de un navegador.",
  },
  settings: {
    title: "Ajustes",
    subtitle: "Preferencias de toda la app para esta instalación.",
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
    themeHint: "Elige cómo se ve Satchel. Sistema sigue la configuración de tu sistema operativo.",
    language: "Idioma",
    languageHint: "Se irán añadiendo más idiomas a medida que se aporten traducciones.",
    mode: "Modo",
    watchOnly: "Modo solo lectura",
    watchOnlyHint:
      "Explora el tablón sin configurar monedas. Aún puedes retirar tus propias ofertas, pero no puedes publicar, tomar ni financiar. Desactívalo para operar (necesitarás al menos dos monedas conectadas).",
    network: "Red",
    boards: "Corkboards",
    boardsDesc:
      "Tablones HTTP autoalojados opcionales. Añade los que te merezcan confianza; déjalo vacío para depender de Nostr.",
    boardsNone: "Ninguno configurado",
    nostrRelays: "Relays Nostr",
    nostrRelaysDesc:
      "Los relays transportan el tablón de anuncios por una red descentralizada — ningún operador puede leer ni emparejar tus ofertas. Vienen precargados con un conjunto por defecto; edítalo libremente.",
    nostrRelaysOff: "Desactivado — transporte Nostr deshabilitado",
    addUrl: "Añadir",
    removeUrl: "Quitar",
    relayInvalid: "Introduce una URL de relay ws:// o wss://",
    boardInvalid: "Introduce una URL de tablón http:// o https://",
    netSave: "Guardar y reconectar",
    netSaving: "Guardando y reconectando…",
    netSaved: "Guardado",
    // Fees tab — fee-bump policy (per active merchant).
    tabFees: "Comisiones",
    fees: "Aumento de comisiones",
    feesScope: "Estos ajustes se aplican al merchant activo.",
    feesIntro:
      "Compromisos de seguridad/coste para los aumentos de comisión, no es configuración obligatoria. Los nuevos valores se aplican a futuros aumentos; los swaps ya financiados conservan la política con la que se financiaron.",
    feeMax: "Feerate máximo (sat/vB)",
    feeMaxHint:
      "Techo para cada aumento de comisión. Por defecto 500, también el máximo absoluto del sistema. Bájalo para limitar costes.",
    feeReservation: "Reserva para aumento de financiación (×)",
    feeReservationHint:
      "El saldo que la comprobación de fondos aparta como margen para aumentos. Más alto rescata picos de comisión mayores pero inmoviliza más saldo y rechaza más swaps. Por defecto 3.",
    feeCommitted: "Sobreprovisión de redención (×)",
    feeCommittedHint:
      "Cuánto se prepaga de más la comisión de redención v2 para que confirme incluso con Satchel cerrado. Solo se aplica a swaps nuevos. Por defecto 2.",
    feeSave: "Guardar",
    feeSaving: "Guardando…",
    feeSaved: "Guardado",
    feeReset: "Restablecer valores por defecto",
    coins: "Monedas y nodos",
    coinsHint: "Conecta cada moneda a tu propio nodo. Se comprueba el génesis antes de guardar nada.",
    about: "Acerca de",
    version: "Versión {version}",
    updateUpToDate: "Al día",
    updateCheckPlaceholder: "La comprobación de actualizaciones llegará en una versión posterior.",
    trustModel: "Dónde viven tus claves",
    trustModelBody:
      "Los secretos viven en el motor, nunca en Satchel. La semilla del merchant reside en la carpeta de datos del motor (cifrada o en texto plano — tú decides); Satchel no almacena ninguna semilla ni frase de contraseña. La semilla es caliente por diseño (solo claves de tránsito) — barre los ingresos cuantiosos a tu propia cartera fría.",
  },
  coins: {
    intro:
      "Conecta cada moneda a tu propio nodo. La primera URL es la cartera de tu propio nodo — financia las patas de tus swaps y recibe los ingresos. Antes de guardar nada, Satchel comprueba el bloque génesis del nodo para que los fondos nunca puedan enviarse a la cadena equivocada. Las conexiones se comparten entre todos tus merchants.",
    networkBadge: "Configurando para la red {network}",
    needMerchant:
      "Conecta primero un merchant — la configuración de monedas necesita que el motor esté en marcha. Usa el selector de merchants de arriba a la derecha.",
    pairsTitle: "Pares de trading",
    pairsHint:
      "Los pares se derivan de lo que cada moneda puede hacer — no hay una lista fija. Un par se abre en cuanto sus dos monedas están conectadas.",
    noPairs: "No hay pares disponibles.",
    notSetUp: "Sin configurar",
    connectedTip: "Conectada · tip {tip}",
    connError: "Error de conexión",
    setUp: "Configurar",
    editConnection: "Editar conexión",
    remove: "quitar",
    disconnectTip: "Desconectar esta moneda",
    disconnectTitle: "¿Desconectar {coin}?",
    disconnectBody: "Los swaps que la necesiten no estarán disponibles hasta que reconectes.",
    ready: "Lista para operar",
    connectMissing: "Conecta {coins}",
    notBuildable: "Aún no construible",
    // Swap-protocol chips on a pair (pact-htlc-v1 HTLC vs pact-htlc-v2 adaptor).
    protoPrivate: "Privado (Taproot)",
    protoPrivateTip: "Swap privado (adaptador Taproot/MuSig2) — parece un pago corriente on-chain",
    protoHtlcTip: "Swap HTLC clásico",
    // Coin-setup backend choices (CoinSetup).
    backendCoreTitle: "Cartera RPC del Core",
    backendCoreDesc: "La cartera de tu propio nodo financia el swap y recibe los ingresos.",
    backendHardwareTitle: "Hardware",
    backendHardwareDesc: "Firma con Ledger / PSBT para la pata de financiación.",
    backendLater: "más tarde",
    // CoinSetup dialog.
    setupTitle: "Conectar {coin}",
    setupIntro:
      "Apunta Satchel a tu propio nodo de {sym}. No se guarda nada hasta que el nodo supera una comprobación del bloque génesis — tus fondos solo tocan la cadena real de {sym}.",
    backendUrlLabel: "URL(s) del backend del nodo",
    backendUrlHint:
      "Primera URL = la cartera de tu propio nodo (financia swaps, recibe ingresos). Añade servidores Electrum (tcp://host:puerto) tras comas para vistas adicionales e independientes de la cadena.",
    fundingWallet: "Cartera de financiación",
    confirmationsLabel: "Confirmaciones antes de definitivo",
    confirmationsHint:
      "Cuán profunda debe ser una financiación o redención en esta cadena antes de que un swap actúe sobre ella — el margen de seguridad ante reorgs. Más alto es más seguro pero más lento; déjalo en blanco para el valor por defecto recomendado ({default}).",
    validateNode: "Validar nodo",
    checking: "Comprobando el nodo…",
    genesisOk: "Génesis coincide — esta es la cadena correcta",
    genesisDetail: "altura del tip {tip} · génesis {hash}…",
    genesisBad: "Rechazado — no se guarda",
    errorShort: "error",
    // Structured connection form (CoinSetup v2).
    rpcHostLabel: "Host RPC",
    rpcPortLabel: "Puerto RPC",
    authMethodLabel: "Autenticación",
    authCookie: "Archivo cookie",
    authCookieDesc: "Lee automáticamente el .cookie del nodo desde su directorio de datos (por defecto, no se almacena contraseña).",
    authUserPass: "Usuario / contraseña",
    authUserPassDesc: "El rpcuser / rpcpassword de la configuración de tu nodo — necesario para un nodo remoto.",
    rpcUserLabel: "Usuario RPC",
    rpcPasswordLabel: "Contraseña RPC",
    datadirLabel: "Directorio de datos del nodo",
    cookiePathNote: "La cookie se lee de {path} bajo este directorio.",
    walletLabel: "Nombre de la cartera (opcional)",
    walletPlaceholder: "la cartera de tu nodo",
    needPort: "Introduce primero el puerto RPC.",
    validateFirst: "Valida el nodo antes de guardar.",
    savingReconnecting: "Guardando y reconectando…",
    connected: "{coin} conectada",
    // Template picker (a coins.toml coin the engine version doesn't support).
    unsupportedByEngine: "No soportada",
    unsupportedByEngineTip:
      "Esta moneda está definida en coins.toml pero no está integrada en esta versión del motor, así que no se puede operar.",
  },
  coinWizard: {
    title: "Conecta tus monedas",
    intro:
      "Elige al menos dos monedas y apunta cada una a tu propio nodo. Un swap necesita dos cadenas, así que el trading se desbloquea una vez que dos nodos están conectados y activos. Puedes añadir o cambiar monedas más adelante en Ajustes.",
    progress: "{count} de {min} monedas conectadas",
    continue: "Continuar",
    live: "Activo",
    nodeDown: "Nodo caído",
  },
  wallets: {
    intro:
      "Estas son las carteras de tus propios nodos (las que el motor usa para financiar swaps y recibir ingresos) — tus claves, tu máquina. Satchel nunca guarda tus monedas.",
    hotSeedNudge:
      "Esta es una cartera de gasto sobre una semilla en caliente, no una bóveda — barre los saldos cuantiosos a tu propia cartera fría/core.",
    notConnected: "Sin conexión",
    notConnectedBody: "Conecta primero un merchant — la vista de cartera necesita que el motor esté en marcha.",
    noCoins: "Aún no hay monedas configuradas",
    noCoinsBody: "Conecta una moneda en Ajustes → Monedas y su cartera aparecerá aquí.",
    goToCoins: "Ir a Monedas",
    watchOnlyTitle: "No hay carteras en modo solo lectura",
    watchOnlyBody:
      "Esta es una sesión de solo lectura sin monedas conectadas, así que no hay carteras que mostrar. Desactiva solo lectura en Ajustes y conecta una moneda para financiar swaps.",
    walletName: "cartera · {wallet}",
    walletScopedHint: "Cada RPC de esta moneda se acota a esta cartera del nodo.",
    walletDefault: "cartera por defecto (sin acotar)",
    walletDefaultHint:
      "No hay cartera definida para esta moneda, así que los RPC usan la cartera por defecto del nodo. Define una en Ajustes → Monedas para acotar cada llamada a una cartera concreta.",
    balanceLabel: "Saldo {symbol}",
    receive: "Recibir",
    send: "Enviar",
    sendTo: "Enviar a la dirección",
    amount: "Cantidad",
    sendTitle: "¿Enviar {amount} {sym}?",
    sendConfirmBody: "A {to}\n\nEsto gasta desde la cartera de tu propio nodo y no se puede deshacer.",
  },
  corkboard: {
    noBoardTitle: "Ningún Corkboard conectado",
    noBoardBody:
      "Un Corkboard es un tablón de anuncios compartido donde los makers fijan ofertas. Nunca empareja operaciones ni guarda monedas — apunta Satchel a uno en el que confíes para explorar y publicar.",
    noPairs: "No hay pares disponibles",
    board: "Corkboard",
    boardSettings: "Configurar en Ajustes",
    filterAll: "Todas",
    filterMine: "Mías",
    offered: "{symbol} ofrecidos",
    noOffers: "No hay ofertas que puedas tomar ahora mismo",
    noOffersBody:
      "Las ofertas aparecen aquí en cuanto un maker publica una para un par que hayas configurado. También puedes publicar la tuya.",
    hiddenOffers:
      "{count} oferta(s) más para pares que no has conectado. Configura ambas monedas para operarlas:",
    yourOffer: "tu oferta",
    offerStaged: "publicando…",
    offerStagedTip:
      "Publicada desde este dispositivo y esperando confirmación de vuelta de un relay. Se está anunciando; pasa a estar activa una vez que un relay la repita.",
    take: "Tomar oferta",
    legDown: "Uno de los nodos de este par está caído — inícialo (o revisa Ajustes → Monedas) antes de tomar.",
    withdraw: "Retirar",
    withdrawTip: "Retira al instante — una oferta nunca bloquea fondos",
    safetyRefund: "reembolso de seguridad",
    safetyRefundTip:
      "Si el swap se atasca, ambas partes reembolsan automáticamente — la pata del taker se desbloquea primero, la tuya un poco después. Nadie acaba atascado.",
    activeTitle: "Tus swaps activos",
    states: {
      open: "abierta",
      takenByUs: "tomada por ti",
      revoked: "retirada",
      expired: "caducada",
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
      spreadOneSided: "Unilateral",
      crossed: "cruzado",
      crossedTip: "Compra máxima ≥ venta mínima. El tablón nunca empareja automáticamente, así que estas ofertas solapadas simplemente quedan ahí — toma cualquiera de los dos lados.",
      mid: "medio {price}",
      levelOffers: "{count} oferta(s) a este precio — elige una para tomarla",
      depthTip: "Total de {sym} en oferta a este precio entre {count} anuncio(s).",
      takerNote: "Al tomarla, das {give} y recibes {get}.",
      selectLevel: "Elige un nivel de precio arriba para ver las ofertas que hay.",
      paneHeader: "{size} {base} @ {price} {unit}",
      denomTip: "Unidad de visualización para las cantidades de {coin}",
      showMore: "Mostrar {count} más",
      showLess: "Mostrar las {count} principales",
    },
  },
  relays: {
    title: "Relays",
    subtitle: "Conectividad en vivo con tus relays Nostr — la red por la que viajan tus ofertas y tomas. Añade o quita relays en Ajustes → Red.",
    connectedCount: "{up} / {total} conectados",
    refresh: "Actualizar",
    ms: "{ms} ms",
    up: "activo",
    down: "caído",
    statsTip: "{success}/{attempts} conexiones con éxito · ↓{down} ↑{up}",
    none: "No hay relays configurados",
    noneBody: "Añade un relay Nostr en Ajustes → Red para publicar y recibir ofertas por la red.",
    goToNetwork: "Ir a Ajustes",
    notConnected: "Sin conexión",
    notConnectedBody: "La vista de relays necesita que el motor esté en marcha — conecta primero un merchant.",
  },
  swaps: {
    title: "Swaps",
    hint: "Tu registro completo — los swaps en curso arriba, las operaciones finalizadas abajo. También puedes actuar sobre swaps en vivo desde el Corkboard.",
    activeTitle: "En curso",
    historyTitle: "Historial",
    none: "Aún no hay swaps — toma una oferta en el Corkboard.",
    // Active-swaps dock action buttons + the refund-time label.
    cancel: "cancelar",
    refund: "reembolsar",
    dump: "volcar registros",
    dumpHint: "Copia un paquete de diagnóstico sin secretos (estado + líneas de registro) de este swap, para pegarlo a los desarrolladores.",
    dumpCopied: "Diagnóstico copiado — pégalo a los desarrolladores.",
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
      "El timelock de seguridad ha pasado, así que puedes reclamar los fondos que bloqueaste. Esto difunde tu reembolso ahora; el motor también lo hace automáticamente tras el plazo límite.",
    col: {
      swap: "swap",
      role: "rol",
      state: "estado",
      amounts: "da → recibe",
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
      copy: "Copiar id de la transacción",
      copied: "Id de la transacción copiado",
    },
  },
  fees: {
    title: "Vista previa del coste de red",
    estimated: "estimado",
    provisionalNote: "Esta build de pactd aún no expone la estimación de comisiones.",
    summary: "Un swap son 2 transacciones on-chain que pagas: la financiación en la cadena que das, la redención en la cadena que recibes.",
    fallbackTip: "Un nodo estaba inaccesible, así que se usó un feerate por defecto conservador — trátalos como una estimación.",
    ifItStalls: "(si se atasca)",
  },
  funds: {
    insufficient:
      "No hay suficientes {sym} para financiar este swap — se necesitan ~{need} {sym} (cantidad + comisión de financiación), la cartera tiene {have} {sym}.",
  },
  wizard: {
    welcome: "Te damos la bienvenida a Satchel",
    connectTitle: "Conecta el motor Pact",
    connectIntro:
      "Satchel es un cliente ligero del motor Pact — el núcleo que guarda tus claves y ejecuta los swaps. Elige cómo alcanzarlo.",
    managed: "Ejecutar el motor Pact integrado",
    managedDesc: "Satchel lanza y supervisa su propio motor Pact. Recomendado.",
    external: "Conectar a un motor Pact externo",
    externalDesc: "Apunta a un motor Pact que ya ejecutes (define SATCHEL_PACTD_URL + cookie antes de lanzar).",
    externalNote:
      "El modo externo se selecciona mediante variables de entorno antes de lanzar Satchel. Relanza con SATCHEL_PACTD_URL definido para usarlo.",
    coinsTitle: "Añade tus monedas",
    coinsIntro:
      "Una vez creado tu merchant, conecta cada moneda a tu propio nodo en Ajustes → Monedas. Elige una moneda y un backend (Electrum público para configuración cero, o tu propio nodo); el génesis se comprueba contra esta red antes de guardar nada.",
    coinsTemplatesSoon: "Las plantillas de monedas de un clic llegarán aquí en una versión posterior.",
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
    noTauri: "no se está ejecutando dentro de Satchel — esta interfaz necesita el puente Tauri",
    startupError: "arranque: {err}",
    notConnected: "sin conexión: {err}",
    connected: "conectado a pactd {version} ({protocol})",
    listcoinsError: "listcoins: {err}",
    watchOnlyError: "solo lectura: {err}",
    switchedMerchant: "cambiado al merchant {id}",
    switchMerchantError: "cambiar merchant: {err}",
    loadMerchantError: "cargar merchant: {err}",
    merchantCreated: "merchant {id} creado",
    merchantReady: "merchant listo",
    actionOk: "{action} {id}: ok",
    actionError: "{action} {id}: {err}",
    diagCopied: "diagnóstico de {id} copiado ({count} líneas de registro) — pégalo a los desarrolladores",
    dumpError: "volcado {id}: {err}",
    coinDisconnected: "{coin} desconectada",
    removeCoinError: "quitar moneda: {err}",
    tookOffer: "oferta {id} tomada — ahora aparece en tus swaps activos abajo",
    takeError: "tomar: {err}",
    offerWithdrawn: "oferta {id} retirada",
    withdrawError: "retirar: {err}",
    postedOffer: "oferta {id} publicada — retírala cuando quieras; no se bloquea nada",
    createdSlip: "cupón de oferta privada creado — envíalo a tu contacto",
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
    justNow: "justo ahora",
    minutesAgo: "hace {n} min",
    hoursAgo: "hace {n} h",
    daysAgo: "hace {n} d",
    expiryNow: "ahora",
    expirySoon: "pronto",
    inMinutes: "en ~{n} min",
    inHours: "en ~{n} h",
    inDays: "en ~{n} d",
    posted: "publicada {age}",
    expires: "caduca {time}",
  },
  // Plain-language swap story per (role, state) — the honest "who is exposed
  // when" framing shown on every active swap. {a}/{b} are coin tickers; {t1}/{t2}
  // are local refund times. Rendered via tr() (narrate() is a pure helper).
  narrate: {
    initiating:
      "Toma enviada — esperando a que el maker inicie el swap. Aún no se bloquea nada; se cancela solo si no responde.",
    created: "Oferta enviada — esperando a que la otra parte acepte. No se compromete nada.",
    acceptedMaker: "Términos acordados. Siguiente: bloquea tus {a}. Hasta que financies, aún puedes cancelar libremente.",
    acceptedTaker: "Términos acordados. La otra parte bloquea sus {a} primero — tú nunca envías primero.",
    noncesExchanged:
      "Configurando el swap privado — intercambiando material de firma. Aún no se bloquea nada.",
    signedMaker:
      "Ambas partes han firmado. Tu daemon bloquea los {a}, luego reclama los {b} automáticamente. Si algo se atasca, tus {a} vuelven a las {t1}.",
    signedTaker:
      "Ambas partes han firmado. Tu daemon bloquea los {b} y reclama los {a} en cuanto la otra parte se mueve. Red de seguridad: reembolso a las {t2}.",
    fundedAMaker:
      "Tus {a} están bloqueados. Esperando a que la otra parte bloquee sus {b}. Si nunca lo hace, tus {a} vuelven automáticamente a las {t1}.",
    fundedATaker:
      "Sus {a} están bloqueados y verificados. Siguiente: bloquea tus {b}. Red de seguridad: reembolso automático a las {t2} si algo se atasca.",
    fundedBMaker: "Ambos bloqueados. Tu daemon reclama los {b} en cuanto estén confirmados con seguridad.",
    fundedBTaker: "Ambos bloqueados. Tu daemon reclamará los {a} en cuanto la otra parte tome sus {b}.",
    redeemedB:
      "Has reclamado los {b} — esperando a que confirmen. Tus {a} bloqueados siguen protegidos hasta que esto sea definitivo.",
    completed: "Swap completado — los {coin} están en tu cartera.",
    refunded: "El swap no se completó, así que tus {coin} volvieron automáticamente. No se pierde nada salvo las comisiones.",
    aborted: "Cancelado antes de que se moviera ningún dinero.",
  },
  exit: {
    // Exit-gate dialog (fund safety, C6). The engine manages alone, so "keep
    // running" detaches it (it keeps watching timelocks + servicing offers).
    liveTitle: "Hay un swap en curso",
    liveBodyOne:
      "1 swap está en curso. Se rige por timelocks on-chain — el motor debe seguir en marcha para redimir o reembolsar antes del plazo límite.",
    liveBodyMany:
      "{count} swaps están en curso. Se rigen por timelocks on-chain — el motor debe seguir en marcha para redimir o reembolsar antes del plazo límite.",
    keepRunningExplain:
      "Cerrar la ventana mantiene el motor en marcha en segundo plano, así que termina el swap sin interfaz. Puedes reabrir Satchel cuando quieras para comprobarlo.",
    forceQuitWarn: "Forzar la salida ahora detiene el motor y puede perder fondos.",
    // {word} is the confirm word below; a translation may localize both together.
    typeToConfirm: "Para forzar la salida de todos modos, escribe {word} abajo.",
    confirmWord: "QUIT",
    keepRunning: "Seguir en marcha, cerrar ventana",
    keepWithdraw: "Seguir en marcha + retirar ofertas",
    keepLeaveOffers: "Seguir en marcha, dejar ofertas publicadas",
    forceQuit: "Forzar salida",
    offersTitle: "Tienes ofertas publicadas",
    offersBodyOne:
      "1 oferta tuya sigue en el Corkboard. Las ofertas no bloquean nada, pero dejarla publicada significa que las contrapartes aún pueden tomarla mientras Satchel está cerrado — el motor atenderá la toma.",
    offersBodyMany:
      "{count} ofertas tuyas siguen en el Corkboard. Las ofertas no bloquean nada, pero dejarlas publicadas significa que las contrapartes aún pueden tomarlas mientras Satchel está cerrado — el motor atenderá las tomas.",
    withdrawExit: "Retirar todas y salir",
  },
  unlock: {
    title: "Desbloquear merchant",
    body:
      "La semilla de este merchant está cifrada. Introduce su frase de contraseña para desbloquearla en esta sesión — Satchel la guarda solo en memoria y la olvida al salir.",
    switchMerchant: "Cambiar de merchant",
    unlock: "Desbloquear",
  },
  common: {
    cancel: "Cancelar",
    confirm: "Confirmar",
    save: "Guardar",
    done: "Hecho",
    later: "Más tarde",
    retry: "Reintentar conexión",
  },
};
