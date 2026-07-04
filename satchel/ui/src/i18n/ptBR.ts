// The Brazilian Portuguese (Português, Brasil) string bundle. Mirrors en.ts key-for-key; only values are translated.
import type { Bundle } from "./en";

export const ptBR: Bundle = {
  app: {
    name: "Satchel",
  },
  // In-app update notifications (sidebar version badge + dialog).
  update: {
    title: "Atualização disponível",
    upToDate: "Você está atualizado",
    current: "Instalada",
    latest: "Mais recente",
    notesTitle: "Notas da versão",
    get: "Obter a atualização",
    dismiss: "Dispensar",
    close: "Fechar",
    badgeTooltip: "Atualização disponível — clique para detalhes",
    versionTooltip: "Clique para verificar atualizações",
  },
  // Risk disclaimer (first-run welcome + Settings → About).
  disclaimer: {
    title: "Autocustódia — suas chaves, sua responsabilidade",
    body: "O Satchel realiza atomic swaps sem custódia: somente você detém suas chaves, e a seed de um merchant guarda chaves quentes de trânsito enquanto um swap está em andamento. Os protocolos de swap (v1 HTLC e v2 Taproot/MuSig2) são revisados e estão em produção na mainnet. Licenciado sob MIT e fornecido no estado em que se encontra, sem garantias — faça backup da sua frase de recuperação e use por sua conta e risco.",
  },
  nav: {
    public: "Público",
    corkboard: "Corkboard",
    postOffer: "Publicar uma oferta",
    private: "Privado",
    privateCreate: "Criar comprovante",
    privateReceive: "Aceitar um comprovante",
    privateSlips: "Meus comprovantes",
    swaps: "Swaps",
    relays: "Relays",
    wallets: "Carteiras",
    contacts: "Contacts",
    settings: "Configurações",
    coins: "Moedas",
  },
  makeOffer: {
    title: "Publicar uma oferta",
    intro:
      "Publique uma oferta assinada no Corkboard. Nada fica bloqueado — é apenas um anúncio; retire quando quiser, e um swap só começa quando alguém aceita e os dois lados fazem o funding.",
    give: "Você dá",
    want: "Você recebe",
    // Canonical pair + direction: pick the pair, choose Sell/Buy the base, enter
    // the base amount and a quote-per-base price (invariant to direction).
    pair: "Par",
    noPairs: "Nenhum par negociável — conecte pelo menos duas moedas em Configurações → Moedas.",
    sell: "Vender {sym}",
    buy: "Comprar {sym}",
    amount: "Quantidade",
    youGive: "Você dá",
    youGet: "Você recebe",
    price: "Preço",
    priceUnit: "{unit} por {base}",
    pricePlaceholder: "preço unitário",
    balance: "Saldo: {amt} {sym}",
    balanceLoading: "Saldo: …",
    noCoins: "Nenhuma moeda configurada",
    legDown: "O node de uma dessas moedas está fora do ar — inicie-o (ou verifique Configurações → Moedas) antes de publicar.",
    // Swap-protocol pin (only offered when a pair+network supports more than
    // one). v2 label reuses coins.protoPrivate ("Private (Taproot)").
    protocol: "Tipo de swap",
    protoStandard: "Padrão (HTLC)",
    // Titles for the review/confirm dialog shown before posting (see OfferForm).
    reviewOfferTitle: "Revise sua oferta",
    reviewSlipTitle: "Revise seu comprovante",
    term: "Timelock de segurança",
    termShort: "Curto",
    termMedium: "Médio",
    termLong: "Longo",
    termHint: {
      short: "Curto — o refund automático dos fundos acontece mais rápido se a negociação travar (~12h / 6h), com a menor margem de segurança.",
      medium: "Médio — janela de refund equilibrada (~24h / 12h).",
      long: "Longo (mais seguro) — maior margem de segurança; refund automático após ~36h / 18h se a negociação travar.",
    },
    validFor: "Válida por (minutos)",
    validForMins: "{mins} min",
    validForHint:
      "Por quanto tempo a oferta fica listada. Enquanto você estiver online, ela é mantida atualizada automaticamente; depois disso, expira. Fechar o app a retira.",
    note: "Oferta de tamanho fixo — nada fica bloqueado até que alguém a aceite. As quantias são on-chain; você paga as taxas de rede por cima e o Corkboard não cobra nada. O timelock é a janela de refund automático caso um swap trave.",
    post: "Publicar oferta",
    makeSlip: "Criar comprovante",
    slipTitle: "Seu comprovante de oferta privada",
    slipExplainer:
      "Envie isto ao seu amigo. Ele cola no Satchel para aceitar. Nada fica bloqueado; expira em {ttl}.",
    copy: "Copiar",
    copied: "Copiado",
    makeAnother: "Criar outro",
    myPrivateTitle: "Minhas ofertas privadas",
    myPrivateEmpty: "Nenhuma oferta privada pendente.",
    privateExpires: "expira {when}",
    privateExpired: "expirada",
    cancel: "Cancelar",
    cancelTip: "Deixar de honrar este comprovante — um amigo que ainda o tenha não poderá mais aceitá-lo.",
  },
  takeSlip: {
    intro:
      "Um amigo lhe enviou um comprovante de oferta privada (começa com pactoffer1:). Cole-o aqui para revisar e aceitar — exatamente como uma oferta do board.",
    placeholder: "pactoffer1:…",
    take: "Revisar e aceitar",
    invalid: "Isso não parece um comprovante — deveria começar com pactoffer1:.",
    previewLabel: "Este comprovante oferece",
  },
  // PRIVATE nav group (off-market slips) — screen titles + intros. The form,
  // slip output, my-slips list and take flow reuse makeOffer.*/takeSlip.* copy.
  private: {
    createTitle: "Criar uma oferta privada",
    createIntro:
      "Monte uma oferta assinada e entregue a um amigo como um comprovante pelo seu próprio chat. Nada é listado em lugar nenhum — e nada fica bloqueado até que ambos façam o funding.",
    slipsIntro:
      "Comprovantes que você criou. Qualquer um que tenha um comprovante pode aceitá-lo até expirar; cancele um para deixar de honrá-lo antes disso.",
    slipsEmptyBody: "Crie uma oferta privada para gerar um comprovante que você possa enviar a um amigo.",
    receiveTitle: "Aceitar uma oferta privada",
    received: "Aceita — acompanhe em Swaps.",
  },
  // Shared take-confirmation dialog (board take + slip take).
  takeConfirm: {
    title: "Aceitar esta oferta?",
    confirm: "Aceitar oferta",
    counterparty: "Contraparte",
    youGive: "Você dá",
    youReceive: "Você recebe",
    safetyRefund: "Refund de segurança",
    offerAge: "Idade da oferta",
    makerFundsFirst:
      "O maker bloqueia seu {sym} primeiro — você nunca envia primeiro. Você ainda pode cancelar antes de fazer o funding do seu lado, e a engine faz o refund automático após o timelock de segurança se o swap travar.",
  },
  header: {
    activeMerchant: "Merchant ativo — clique para trocar ou gerenciar",
    manageMerchants: "Gerenciar Merchants…",
    noMerchant: "nenhum merchant",
    openMenu: "Abrir menu",
    collapseMenu: "recolher menu",
    settings: "Configurações",
    language: "Idioma",
    pactConnected: "Engine conectada",
    pactUnreachable: "Engine inacessível",
    liveSwapsOne: "1 swap em andamento — clique para ver",
    liveSwapsMany: "{count} swaps em andamento — clique para ver",
    liveSwapsNone: "Nenhum swap em andamento",
    coinOk: "{name} — conectada · topo {tip}",
    coinUnconfigured: "{name} — não configurada",
    coinError: "{name} — {status}",
    relaysOk: "Relays Nostr — {up}/{total} conectados",
    relaysDown: "Relays Nostr — nenhum dos {total} conectado",
  },
  network: {
    mainnet: "MainNet",
    testnet: "TestNet",
    regtest: "RegTest",
    signet: "Signet",
    notRealFunds: "Não são fundos reais — esta é a rede {network}",
  },
  // Watch-only mode: a viewer session with no coins. Browse the board and
  // withdraw your own offers, but no posting / taking / funding.
  watchOnly: {
    badge: "Somente visualização",
    badgeTip:
      "Modo somente visualização — navegue pelo board e retire suas próprias ofertas, mas você não pode publicar, aceitar ou fazer funding. Configure moedas em Configurações para negociar.",
    coinWizardButton: "Navegar no modo somente visualização",
    coinWizardHint:
      "Pule a configuração de moedas e apenas navegue pelo board (somente leitura). Você ainda pode retirar suas próprias ofertas — útil para remover ofertas deixadas por outra sessão. Desative quando quiser em Configurações.",
    postBlockedTitle: "Modo somente visualização",
    postBlockedBody:
      "Esta é uma sessão somente visualização, então não é possível publicar ofertas. Configure pelo menos duas moedas em Configurações → Moedas para negociar.",
    takeBlockedBody: "Modo somente visualização — você pode revisar esta oferta, mas aceitá-la exige moedas configuradas.",
    takeBlockedTip: "Modo somente visualização — configure moedas em Configurações para aceitar ofertas.",
  },
  merchants: {
    title: "Seus merchants",
    intro:
      "Um merchant é uma identidade de negociação — com sua própria seed e histórico de swaps. Negociar sob um merchant diferente mantém os contextos não vinculáveis (uma identidade descartável). Suas moedas principais ficam na sua própria carteira, não aqui.",
    // First-run welcome (empty merchant manager).
    welcomeTitle: "Bem-vindo ao Satchel",
    welcomeIntro:
      "O Satchel negocia sob um “merchant” — uma identidade de negociação com sua própria seed. Você ainda não tem nenhuma: crie uma nova ou importe uma frase de recuperação existente para começar.",
    importMerchant: "Importar um merchant",
    none: "Nenhum merchant ainda.",
    switch: "trocar",
    newMerchant: "Novo merchant",
    thisMerchant: "este merchant",
    nameLabel: "Nome do merchant",
    namePlaceholder: "ex.: Principal",
    rename: "Renomear",
    introFirst:
      "Configure sua primeira identidade de negociação (um “merchant”). Ela guarda apenas chaves quentes de trânsito para swaps em andamento — suas moedas principais ficam na sua própria carteira.",
    introNew: "Um novo merchant é uma identidade nova e separada, com sua própria seed e histórico de swaps.",
    // UI-5 merchant selector (phoenix wallet-selector parity).
    createNew: "Criar novo",
    import: "Importar",
    load: "Carregar Merchant",
    loaded: "carregado",
    locked: "bloqueado",
    lockedTip: "Seed criptografada — desbloqueie com sua senha ao carregá-la.",
    close: "Fechar",
    idLabel: "pasta",
    switching: "Trocando de merchant…",
    switchingBody: "Reiniciando a engine contra aquela pasta.",
  },
  // Seed create/import (SeedForm) + counterparty + status.
  seed: {
    intro: "Crie uma seed totalmente nova ou importe uma que você já tenha.",
    createNew: "Criar nova",
    createDesc: "Gera uma seed nova. Você faz backup da frase de recuperação.",
    import: "Importar",
    importDesc: "Restaure a partir de uma frase de 12/24 palavras existente.",
    recoveryLabel: "Frase de recuperação",
    encrypt: "Criptografar",
    encryptDesc:
      "Uma senha protege a seed em repouso. Você a digita uma vez por sessão — o Satchel nunca a armazena. Observação: o refund automático sem supervisão fica pausado após um reinício até você digitá-la novamente.",
    noPassphrase: "Sem senha (recomendado)",
    noPassphraseDesc:
      "O refund automático continua funcionando após reinícios, sem nada a digitar — esta é apenas uma seed quente de trânsito. Custo: acesso ao arquivo/host expõe as chaves de trânsito e a identidade deste merchant.",
    passphraseLabel: "Senha",
    passphrasePlaceholder: "escolha uma senha",
    revealTitle: "Anote sua frase de recuperação",
    revealBody:
      "Qualquer um com estas palavras controla as chaves quentes deste merchant. O Satchel não guarda cópia — armazene-a offline. A seguir, você confirmará algumas palavras.",
    ackLabel: "Anotei minha frase de recuperação.",
    // Multi-step onboarding (create/import -> secret -> confirm -> passphrase).
    chooseTitle: "Configurar {label}",
    enterTitle: "Importe sua frase de recuperação",
    enterBody:
      "Digite cada palavra — elas se autocompletam conforme você digita — ou cole a frase inteira. Verificamos antes de você continuar.",
    wordCount: "{n} palavras",
    wordCountHint:
      "12 palavras já bastam — esta é uma carteira quente de trânsito, não armazenamento frio. Escolha 24 se preferir a frase mais longa.",
    wordAria: "Palavra {n}",
    checkIncomplete: "Digite todas as {n} palavras.",
    checkUnknown: "Algumas palavras não estão na lista BIP39 — verifique as destacadas.",
    checkBadChecksum: "O checksum não confere — confira suas palavras e a ordem delas.",
    checkOk: "A frase de recuperação parece válida.",
    verifyTitle: "Confirme seu backup",
    verifyBody: "Digite as palavras nestas posições para confirmar que você anotou a frase.",
    verifyWord: "Palavra nº{n}",
    verifyMismatch: "Essas não correspondem à sua frase — confira seu backup.",
    passphraseTitle: "Proteger a seed",
    passphraseBody:
      "Opcionalmente, criptografe a seed armazenada com uma senha. Você pode pular isto — veja a contrapartida abaixo.",
  },
  counterparty: {
    you: "Este é você",
    youShort: "você",
    unknown: "identidade desconhecida",
    // Short fingerprint fallback (identity.ts shortId) when no pubkey is known.
    unknownShort: "desconhecida",
  },
  contacts: {
    title: "Contatos",
    subtitle: "Seus apelidos privados para as pessoas com quem você negocia.",
    privacyNote:
      "Os contatos são armazenados apenas neste dispositivo e nunca são compartilhados, publicados ou enviados a um relay. Um apelido é o seu rótulo — o identicon e a impressão digital continuam sendo a identidade real.",
    searchPlaceholder: "Buscar apelido, nota ou chave",
    empty: "Nenhum contato ainda. Clique no identicon de uma contraparte em qualquer lugar para adicionar um.",
    emptyFiltered: "Nenhum contato corresponde a este filtro.",
    count: "{n} contatos",
    colWho: "Identidade",
    colNick: "Apelido",
    colNote: "Notas",
    colStatus: "Situação",
    colAdded: "Adicionado",
    colActions: "",
    filterAll: "Todos",
    filterTrusted: "Confiáveis",
    filterBlocked: "Bloqueados",
    // Corkboard toggle: drop blocked makers' offers from the ladder.
    hideBlocked: "Ocultar ofertas bloqueadas",
    statusTrusted: "Confiável",
    statusNeutral: "Neutro",
    statusBlocked: "Bloqueado",
    menuAdd: "Adicionar aos contatos…",
    menuEdit: "Editar contato…",
    menuMarkTrusted: "Marcar como confiável",
    menuMarkNeutral: "Marcar como neutro",
    menuMarkBlocked: "Bloquear",
    menuCopyKey: "Copiar chave pública",
    menuOpen: "Abrir em Contatos",
    keyCopied: "Chave pública copiada",
    editTitle: "Editar contato",
    addTitle: "Adicionar contato",
    nickLabel: "Apelido",
    nickPlaceholder: "ex.: Alice do meetup",
    noteLabel: "Notas",
    notePlaceholder: "Qualquer coisa que você queira lembrar — como falar com a pessoa, negociações passadas…",
    save: "Salvar",
    cancel: "Cancelar",
    remove: "Remover contato",
    removeConfirmTitle: "Remover contato?",
    removeConfirmBody: "Isto exclui seu apelido e notas locais de {who}. Não pode ser desfeito.",
    blockedWarning: "Você bloqueou esta contraparte",
    blockedWarningBody:
      "Você marcou esta pessoa como bloqueada. Bloquear é apenas um lembrete pessoal — não impede a negociação. Continue apenas se for essa sua intenção.",
  },
  status: {
    notConnectedTitle: "Não conectado à engine",
    disconnectedBody:
      "O Satchel não consegue alcançar a engine. Ela pode ainda estar iniciando, ou as conexões de node do merchant ativo podem estar fora do ar. Tente novamente ou troque de merchant pelo seletor no topo.",
    openInSatchel: "Abrir isto no Satchel",
    noTauriBody:
      "Esta é a interface do Satchel — ela precisa da ponte Tauri para alcançar a engine. Inicie o app desktop (cargo tauri dev) em vez de um navegador.",
  },
  settings: {
    title: "Configurações",
    subtitle: "Preferências gerais do app para esta instalação.",
    // UI-3 Settings tabs.
    tabGeneral: "Geral",
    tabCoins: "Moedas",
    tabNetwork: "Rede",
    tabAbout: "Sobre",
    appearance: "Aparência",
    theme: "Tema",
    themeDark: "Escuro",
    themeLight: "Claro",
    themeSystem: "Sistema",
    themeHint: "Escolha a aparência do Satchel. Sistema segue a configuração do seu SO.",
    language: "Idioma",
    languageHint: "Mais idiomas chegam à medida que traduções são contribuídas.",
    mode: "Modo",
    watchOnly: "Modo somente visualização",
    watchOnlyHint:
      "Navegue pelo board sem configurar moedas. Você ainda pode retirar suas próprias ofertas, mas não pode publicar, aceitar nem fazer funding. Desative para negociar (você precisará de pelo menos duas moedas conectadas).",
    network: "Rede",
    boards: "Corkboards",
    boardsDesc:
      "Boards HTTP auto-hospedados opcionais. Adicione os que você confiar; deixe vazio para depender do Nostr.",
    boardsNone: "Nenhum configurado",
    nostrRelays: "Relays Nostr",
    nostrRelaysDesc:
      "Os relays transportam o noticeboard por uma rede descentralizada — nenhum operador pode ler ou cruzar suas ofertas. Já vem com um conjunto padrão; edite à vontade.",
    nostrRelaysOff: "Desligado — transporte Nostr desativado",
    addUrl: "Adicionar",
    removeUrl: "Remover",
    relayInvalid: "Informe uma URL de relay ws:// ou wss://",
    boardInvalid: "Informe uma URL de board http:// ou https://",
    netSave: "Salvar e reconectar",
    netSaving: "Salvando e reconectando…",
    netSaved: "Salvo",
    // Fees tab — fee-bump policy (per active merchant).
    tabFees: "Taxas",
    fees: "Aumento de taxa (fee bumping)",
    feesScope: "Estas configurações se aplicam ao merchant ativo.",
    feesIntro:
      "Contrapartidas de segurança/custo para fee bumps, não é configuração obrigatória. Novos valores se aplicam a bumps futuros; swaps já financiados mantêm a política sob a qual foram financiados.",
    feeMax: "Feerate máxima (sat/vB)",
    feeMaxHint:
      "Teto para todo fee bump. Padrão 500, também o máximo absoluto do sistema. Reduza para limitar custos.",
    feeReservation: "Reserva para bump de funding (×)",
    feeReservationHint:
      "Saldo que a verificação de fundos reserva como folga para bumps. Mais alto resgata picos de taxa maiores, mas imobiliza mais saldo e rejeita mais swaps. Padrão 3.",
    feeCommitted: "Sobreprovisão do redeem (×)",
    feeCommittedHint:
      "Quanto a mais a taxa de redeem v2 é pré-paga para que confirme mesmo com o Satchel fechado. Aplica-se apenas a novos swaps. Padrão 2.",
    feeSave: "Salvar",
    feeSaving: "Salvando…",
    feeSaved: "Salvo",
    feeReset: "Restaurar padrões",
    coins: "Moedas e nodes",
    coinsHint: "Conecte cada moeda ao seu próprio node. O bloco gênese é verificado antes de qualquer coisa ser salva.",
    about: "Sobre",
    version: "Versão {version}",
    updateUpToDate: "Atualizado",
    updateCheckPlaceholder: "A verificação de atualização chega em uma versão futura.",
    trustModel: "Onde suas chaves ficam",
    trustModelBody:
      "Os segredos ficam na engine, nunca no Satchel. A seed do merchant fica na pasta de dados da engine (criptografada ou em texto puro — sua escolha); o Satchel não armazena seed nem senha. A seed é quente por design (apenas chaves de trânsito) — varra ganhos consideráveis para sua própria carteira fria (cold wallet).",
  },
  coins: {
    intro:
      "Conecte cada moeda ao seu próprio node. A primeira URL é a carteira do seu próprio node — ela financia as pernas do seu swap e recebe os ganhos. Antes de salvar qualquer coisa, o Satchel verifica o bloco gênese do node para que os fundos nunca sejam enviados à chain errada. As conexões são compartilhadas entre todos os seus merchants.",
    networkBadge: "Configurando para a rede {network}",
    needMerchant:
      "Conecte um merchant primeiro — a configuração de moedas precisa da engine rodando. Use o seletor de merchant no canto superior direito.",
    pairsTitle: "Pares de negociação",
    pairsHint:
      "Os pares derivam do que cada moeda consegue fazer — não há lista fixa. Um par abre assim que ambas as suas moedas estiverem conectadas.",
    noPairs: "Nenhum par disponível.",
    notSetUp: "Não configurada",
    connectedTip: "Conectada · topo {tip}",
    connError: "Erro de conexão",
    setUp: "Configurar",
    editConnection: "Editar conexão",
    remove: "remover",
    disconnectTip: "Desconectar esta moeda",
    disconnectTitle: "Desconectar {coin}?",
    disconnectBody: "Swaps que precisam dela não estarão disponíveis até você reconectar.",
    ready: "Pronta para negociar",
    connectMissing: "Conectar {coins}",
    notBuildable: "Ainda não construível",
    // Swap-protocol chips on a pair (pact-htlc-v1 HTLC vs pact-htlc-v2 adaptor).
    protoPrivate: "Privado (Taproot)",
    protoPrivateTip: "Swap privado (adaptor Taproot/MuSig2) — parece um pagamento comum on-chain",
    protoHtlcTip: "Swap HTLC clássico",
    // Coin-setup backend choices (CoinSetup).
    // CoinSetup dialog.
    setupTitle: "Conectar {coin}",
    setupIntro:
      "Aponte o Satchel para o seu próprio node {sym}. Nada é salvo até o node passar na verificação do bloco gênese — seus fundos só tocam a chain real do {sym}.",
    confirmationsLabel: "Confirmações antes de final",
    confirmationsHint:
      "Quão profundo um funding ou redeem nesta chain deve estar antes de um swap agir sobre ele — a margem de segurança contra reorg. Mais alto é mais seguro, porém mais lento; deixe em branco para o padrão recomendado ({default}).",
    validateNode: "Validar node",
    checking: "Verificando o node…",
    genesisOk: "Gênese conferido — esta é a chain correta",
    genesisDetail: "altura do topo {tip} · gênese {hash}…",
    genesisBad: "Rejeitado — não salvando",
    errorShort: "erro",
    // Structured connection form (CoinSetup v2).
    rpcHostLabel: "Host RPC",
    rpcPortLabel: "Porta RPC",
    authMethodLabel: "Autenticação",
    authCookie: "Arquivo cookie",
    authCookieDesc: "Lê automaticamente o .cookie do node a partir do diretório de dados (o padrão, sem senha armazenada).",
    authUserPass: "Usuário / senha",
    authUserPassDesc: "O rpcuser / rpcpassword da configuração do seu node — necessário para um node remoto.",
    rpcUserLabel: "Usuário RPC",
    rpcPasswordLabel: "Senha RPC",
    datadirLabel: "Diretório de dados do node",
    cookiePathNote: "O cookie é lido de {path} dentro deste diretório.",
    walletLabel: "Nome da carteira (opcional)",
    walletPlaceholder: "a carteira do seu node",
    needPort: "Informe primeiro a porta RPC.",
    validateFirst: "Valide o node antes de salvar.",
    savingReconnecting: "Salvando e reconectando…",
    connected: "{coin} conectada",
    // Nodeless (Electrum) connection mode (epic #58).
    modeLabel: "Tipo de conexão",
    modeNode: "Seu próprio node",
    modeNodeDesc: "Core RPC — a carteira do node financia os swaps. Soberania máxima.",
    modeNodeless: "Electrum",
    modeNodelessDesc:
      "Nenhum node necessário: os dados da chain vêm de servidores Electrum e a carteira vive na sua seed do Pact.",
    electrumUrlsLabel: "Servidores Electrum",
    electrumUrlsHelp:
      "Um por linha: tcp://host:port ou ssl://host:port. A mainnet exige pelo menos dois servidores independentes, para cruzar as visões da chain entre si.",
    electrumNeedUrl: "Informe pelo menos uma URL de servidor Electrum (tcp:// ou ssl://).",
    electrumBadUrl: "URLs Electrum devem começar com tcp:// ou ssl:// — recebido: {url}",
    validateServers: "Validar servidores",
    connRpcLocal: "RPC (local)",
    connRpcRemote: "RPC (remoto)",
    connElectrumLocal: "Electrum (local)",
    connElectrumRemote: "Electrum (remoto)",
    connRpcTip:
      "Esta moeda fala por RPC com um node no estilo Bitcoin Core; a carteira do node financia os swaps.",
    connElectrumTip:
      "Esta moeda se conecta a servidores Electrum — sem node. A carteira vive na sua seed do Pact.",
    // Template picker (a coins.toml coin the engine version doesn't support).
    unsupportedByEngine: "Não suportada",
    unsupportedByEngineTip:
      "Esta moeda está definida em coins.toml, mas não foi compilada nesta versão da engine, então não pode ser negociada.",
  },
  coinWizard: {
    title: "Conecte suas moedas",
    intro:
      "Escolha pelo menos duas moedas e aponte cada uma para o seu próprio node. Um swap precisa de duas chains, então a negociação é liberada assim que dois nodes estiverem conectados e ativos. Você pode adicionar ou alterar moedas depois em Configurações.",
    progress: "{count} de {min} moedas conectadas",
    continue: "Continuar",
    live: "Ativo",
    nodeDown: "Node fora do ar",
  },
  wallets: {
    intro:
      "Estas são as carteiras dos seus próprios nodes (as que a engine usa para financiar swaps e receber ganhos) — suas chaves, sua máquina. O Satchel nunca guarda suas moedas.",
    hotSeedNudge:
      "Esta é uma carteira de gastos em uma seed quente, não um cofre — varra saldos consideráveis para sua própria carteira fria/core.",
    notConnected: "Não conectado",
    notConnectedBody: "Conecte um merchant primeiro — a visão da carteira precisa da engine rodando.",
    noCoins: "Nenhuma moeda configurada ainda",
    noCoinsBody: "Conecte uma moeda em Configurações → Moedas e a carteira dela aparece aqui.",
    goToCoins: "Ir para Moedas",
    watchOnlyTitle: "Sem carteiras no modo somente visualização",
    watchOnlyBody:
      "Esta é uma sessão somente visualização sem moedas conectadas, então não há carteiras para exibir. Desative o modo somente visualização em Configurações e conecte uma moeda para financiar swaps.",
    walletName: "carteira · {wallet}",
    walletScopedHint: "Toda RPC para esta moeda é direcionada a esta carteira do node.",
    walletDefault: "carteira padrão (não direcionada)",
    walletDefaultHint:
      "Nenhuma carteira definida para esta moeda, então as RPCs usam a carteira padrão do node. Defina uma em Configurações → Moedas para direcionar cada chamada a uma carteira específica.",
    balanceLabel: "saldo de {symbol}",
    // ---- nodeless (pact-seed bdk) wallet: send / receive / activity --------
    pactSeed: "carteira da seed do Pact",
    pactSeedHint:
      "Esta moeda roda sem node: a carteira dela vive na sua seed do Pact, sincronizada a partir de servidores Electrum — nenhum node necessário. Enviar, receber e o histórico ficam bem aqui.",
    receive: "Receber",
    send: "Enviar",
    activity: "Atividade",
    copy: "Copiar",
    copied: "Copiado",
    close: "Fechar",
    refresh: "Atualizar",
    receiveTitle: "Receber {sym}",
    receiveIntro:
      "Um endereço novo da sua carteira da seed do Pact. Moedas enviadas para cá aparecem no saldo assim que confirmadas.",
    receiveIntroRpc:
      "Um endereço novo da carteira do seu node. Moedas enviadas para cá aparecem no saldo assim que confirmadas.",
    receiveFreshNote:
      "Toda vez que você abre este diálogo, recebe um endereço novo. Os endereços antigos continuam funcionando — os novos são apenas melhores para a privacidade.",
    sendTitle: "Enviar {sym}",
    sendIntro: "Disponível para gastar: {balance} {sym}.",
    sendAddressLabel: "Endereço {sym} do destinatário",
    sendAmountLabel: "Quantia",
    sendNeedAddress: "Informe o endereço do destinatário.",
    sendNeedAmount: "Informe uma quantia.",
    sendOverBalance: "Mais do que o saldo disponível para gastar.",
    sendFeeNote:
      "A taxa de rede é adicionada por fora, escolhida automaticamente do mercado de taxas ao vivo.",
    sendBroadcast: "Enviado — {txid}… está a caminho ({sym}).",
    sendConfirm: "Enviar",
    activityTitle: "Atividade de {sym}",
    activityEmpty: "Nada ainda — receba moedas ou conclua um swap e isso aparece aqui.",
    activityWhen: "Quando",
    activityDirection: "Direção",
    activityAmount: "Quantia ({sym})",
    activityFee: "Taxa",
    activityConfs: "Confs",
    activityTxid: "Transação",
    activityPending: "pendente",
    activitySent: "Enviado",
    activityReceived: "Recebido",
  },
  corkboard: {
    noBoardTitle: "Nenhum Corkboard conectado",
    noBoardBody:
      "Um Corkboard é um quadro de avisos compartilhado onde makers fixam ofertas. Ele nunca cruza negociações nem guarda moedas — aponte o Satchel para um em que você confie para navegar e publicar.",
    noPairs: "Nenhum par disponível",
    board: "Corkboard",
    boardSettings: "Configurar em Configurações",
    filterAll: "Todas",
    filterMine: "Minhas",
    noOffers: "Nenhuma oferta que você possa aceitar agora",
    noOffersBody:
      "As ofertas aparecem aqui assim que um maker publicar uma para um par que você configurou. Você também pode publicar a sua.",
    yourOffer: "sua oferta",
    offerStaged: "publicando…",
    offerStagedTip:
      "Publicada deste dispositivo e aguardando confirmação de retorno de um relay. Já está anunciando; torna-se ativa assim que um relay a ecoar.",
    take: "Aceitar oferta",
    legDown: "O node de uma das moedas deste par está fora do ar — inicie-o (ou verifique Configurações → Moedas) antes de aceitar.",
    withdraw: "Retirar",
    withdrawTip: "Retire na hora — uma oferta nunca bloqueia fundos",
    safetyRefund: "refund de segurança",
    safetyRefundTip:
      "Se o swap travar, ambos os lados recebem refund automático — a perna do taker é desbloqueada primeiro, a sua um pouco depois. Ninguém fica preso.",
    activeTitle: "Seus swaps ativos",
    states: {
      takenByUs: "aceita por você",
      revoked: "retirada",
      expired: "expirada",
    },
    // Two-sided order-book view of the Corkboard. Bids = makers giving the
    // quote coin to get the base; asks = the reverse. The ladder is a way to
    // READ the board — it never matches or prioritises (load-bearing).
    book: {
      bids: "Compras",
      asks: "Vendas",
      bidsHint: "querem {base} · pagando {quote}",
      asksHint: "vendendo {base} · por {quote}",
      price: "Preço",
      size: "Tamanho",
      noBids: "Nenhuma compra",
      noAsks: "Nenhuma venda",
      spread: "Spread {pct}",
      spreadOneSided: "Unilateral",
      crossed: "cruzado",
      crossedTip: "Maior compra ≥ menor venda. O board nunca cruza automaticamente, então essas ofertas sobrepostas apenas ficam ali — aceite qualquer um dos lados.",
      mid: "médio {price}",
      levelOffers: "{count} oferta(s) a este preço — escolha uma para aceitar",
      depthTip: "Total de {sym} ofertado a este preço em {count} aviso(s).",
      selectLevel: "Escolha um nível de preço acima para ver as ofertas ali.",
      paneHeader: "{size} {base} @ {price} {unit}",
      denomTip: "Unidade de exibição para quantias de {coin}",
      showMore: "Mostrar mais {count}",
      showLess: "Mostrar os {count} principais",
    },
  },
  relays: {
    title: "Relays",
    subtitle: "Conectividade ao vivo com seus relays Nostr — a rede por onde suas ofertas e aceites trafegam. Adicione ou remova relays em Configurações → Rede.",
    connectedCount: "{up} / {total} conectados",
    refresh: "Atualizar",
    ms: "{ms} ms",
    up: "ativo",
    down: "fora",
    statsTip: "{success}/{attempts} conexões bem-sucedidas · ↓{down} ↑{up}",
    none: "Nenhum relay configurado",
    noneBody: "Adicione um relay Nostr em Configurações → Rede para publicar e receber ofertas pela rede.",
    goToNetwork: "Ir para Configurações",
    notConnected: "Não conectado",
    notConnectedBody: "A visão de relays precisa da engine rodando — conecte um merchant primeiro.",
  },
  swaps: {
    maker: "Maker",
    taker: "Taker",
    title: "Swaps",
    hint: "Seu registro completo — swaps em andamento no topo, negociações concluídas abaixo. Você também pode agir sobre swaps ativos pelo Corkboard.",
    activeTitle: "Em andamento",
    historyTitle: "Histórico",
    none: "Nenhum swap ainda — aceite uma oferta no Corkboard.",
    // Active-swaps dock action buttons + the refund-time label.
    cancel: "cancelar",
    dump: "exportar logs",
    dumpHint: "Copia um pacote de diagnóstico sem segredos (estado + linhas de log) deste swap, para colar aos desenvolvedores.",
    dumpCopied: "Diagnóstico copiado — cole aos desenvolvedores.",
    dumpFailed: "Não foi possível copiar o pacote de diagnóstico.",
    refundAt: "refund {when}",
    // Confirm dialogs for acting on a live swap (ActiveSwaps).
    cancelTitle: "Cancelar este swap?",
    cancelConfirm: "Cancelar swap",
    cancelKeep: "Manter",
    // Abort reason recorded on the swap when cancelled from Satchel.
    cancelReason: "cancelado no Satchel",
    cancelBody:
      "Isto abandona o swap antes de você ter feito o funding. Nada seu está bloqueado ainda, então você não perde nada — a oferta apenas não será concluída.",
    col: {
      swap: "swap",
      role: "papel",
      state: "estado",
      amounts: "dá → recebe",
      when: "quando",
      finalTx: "tx final",
    },
    // Expandable per-leg on-chain detail (the audit trail). We show both
    // funding txs + OUR settlement; never the counterparty's settlement or the
    // swap secret.
    audit: {
      toggle: "Mostrar detalhe on-chain",
      title: "Detalhe on-chain",
      youLocked: "você bloqueou",
      theyLocked: "eles bloquearam",
      funding: "Funding",
      received: "Recebido",
      refunded: "Devolvido (refund)",
      pending: "ainda não está on-chain",
      copy: "Copiar id da transação",
      copied: "Id da transação copiado",
    },
  },
  fees: {
    title: "Prévia do custo de rede",
    estimated: "estimado",
    provisionalNote: "Esta build do pactd ainda não expõe estimativa de taxa.",
    summary: "Um swap são 2 transações on-chain que você paga: o funding na chain de origem, o redeem na chain de destino.",
    fallbackTip: "Um node estava inacessível, então uma feerate padrão conservadora foi usada — trate isto como uma estimativa.",
    ifItStalls: "(se travar)",
  },
  funds: {
    insufficient:
      "{sym} insuficiente para financiar este swap — necessário ~{need} {sym} (quantia + taxa de funding), a carteira tem {have} {sym}.",
  },
  wizard: {
    back: "Voltar",
    continue: "Continuar",
  },
  // UI-4 docked activity log.
  log: {
    title: "Atividade",
    empty: "— registro de atividade —",
    count: "{count} linhas",
    collapse: "Recolher registro",
    expand: "Expandir registro",
    // Activity-log lines emitted by the frontend. `{err}` carries a raw engine
    // message (itself not translated — it comes from pactd over the wire).
    noTauri: "não rodando dentro do Satchel — esta interface precisa da ponte Tauri",
    startupError: "inicialização: {err}",
    notConnected: "não conectado: {err}",
    connected: "conectado ao pactd {version} ({protocol})",
    listcoinsError: "listcoins: {err}",
    watchOnlyError: "somente visualização: {err}",
    switchedMerchant: "trocado para o merchant {id}",
    renamedMerchant: "merchant renomeado para {name}",
    renameMerchantError: "renomear merchant: {err}",
    switchMerchantError: "trocar merchant: {err}",
    loadMerchantError: "carregar merchant: {err}",
    merchantCreated: "merchant {id} criado",
    merchantReady: "merchant pronto",
    actionOk: "{action} {id}: ok",
    actionError: "{action} {id}: {err}",
    diagCopied: "diagnóstico de {id} copiado ({count} linhas de log) — cole aos devs",
    dumpError: "dump {id}: {err}",
    coinDisconnected: "{coin} desconectada",
    removeCoinError: "remover moeda: {err}",
    tookOffer: "oferta {id} aceita — agora aparece nos seus swaps ativos abaixo",
    takeError: "aceitar: {err}",
    offerWithdrawn: "oferta {id} retirada",
    withdrawError: "retirar: {err}",
    postedOffer: "oferta {id} publicada — retire quando quiser; nada fica bloqueado",
    createdSlip: "comprovante de oferta privada criado — envie ao seu amigo",
    tookPrivateOffer: "oferta privada {id} aceita — agora aparece nos seus swaps ativos",
    cancelledPrivateOffer: "oferta privada {id} cancelada",
    cancelError: "cancelar: {err}",
    noticeboardUpdated: "noticeboard atualizado",
    feePolicyUpdated: "política de taxas atualizada",
  },
  // Relative-time + freshness prose from format.ts (rendered via the tr() mirror,
  // since those are pure non-component helpers). Unit letters stay inside the
  // template so a translation owns the whole phrase.
  format: {
    ageUnknown: "idade desconhecida",
    justNow: "agora mesmo",
    minutesAgo: "há {n}min",
    hoursAgo: "há {n}h",
    daysAgo: "há {n}d",
    expiryNow: "agora",
    expirySoon: "em breve",
    inMinutes: "em ~{n}min",
    inHours: "em ~{n}h",
    inDays: "em ~{n}d",
    posted: "publicada {age}",
    expires: "expira {time}",
  },
  // Plain-language swap story per (role, state) — the honest "who is exposed
  // when" framing shown on every active swap. {a}/{b} are coin tickers; {t1}/{t2}
  // are local refund times. Rendered via tr() (narrate() is a pure helper).
  narrate: {
    finalizing: "Você reivindicou seus {got} — confirmações finais. Mantenha o app aberto até enterrar; seus {gave} ficam protegidos até lá.",
    initiating:
      "Aceite enviado — aguardando o maker iniciar o swap. Nada está bloqueado ainda; cancela sozinho se ele não responder.",
    created: "Oferta enviada — aguardando o outro lado concordar. Nada está comprometido.",
    acceptedMaker: "Termos acordados. A seguir: bloqueie seu {a}. Até fazer o funding, você ainda pode cancelar livremente.",
    acceptedTaker: "Termos acordados. O outro lado bloqueia o {a} dele primeiro — você nunca envia primeiro.",
    noncesExchanged:
      "Configurando o swap privado — trocando material de assinatura. Nada está bloqueado ainda.",
    signedMaker:
      "Ambos os lados assinaram e seu {a} está bloqueado. Seu daemon reivindica o {b} automaticamente assim que o outro lado bloqueia e confirma. Se algo travar, seu {a} retorna às {t1}.",
    signedTaker:
      "Ambos os lados assinaram. Assim que o {a} deles for confirmado, seu daemon bloqueia seu {b} e depois reivindica o {a} automaticamente. Assim que seu {b} estiver bloqueado, ele retorna às {t2} se algo travar.",
    fundedAMaker:
      "Seu {a} está bloqueado. Aguardando o outro lado bloquear o {b} dele. Se ele nunca o fizer, seu {a} retorna automaticamente às {t1}.",
    fundedATaker:
      "O {a} dele está bloqueado e verificado. A seguir: bloqueie seu {b}. Rede de segurança: refund automático às {t2} se algo travar.",
    fundedBMaker: "Ambos bloquearam. Seu daemon reivindica o {b} assim que estiver confirmado com segurança.",
    fundedBTaker: "Ambos bloquearam. Seu daemon reivindicará o {a} no instante em que o outro lado pegar o {b} dele.",
    completed: "Swap concluído — o {coin} está na sua carteira.",
    refunded: "O swap não foi concluído, então seu {coin} voltou automaticamente. Nada perdido, exceto as taxas.",
    aborted: "Cancelado antes de qualquer dinheiro se mover.",
  },
  progress: {
    awaitingLock: "Aguardando o bloqueio deles",
    awaitingClaim: "Aguardando o resgate deles",
    theirLock: "Confirmando o bloqueio deles",
    ourLock: "Confirmando o seu bloqueio",
    securing: "Protegendo seus {coin}",
    funding: "Bloqueando seus {coin} — desbloqueie a carteira se travar",
    blocks: "+{n} blocos",
    feeBumped: "Taxa aumentada",
    reorg: "Reorg detectada — verificando novamente",
  },
  exit: {
    // Exit-gate dialog (fund safety, C6). The engine manages alone, so "keep
    // running" detaches it (it keeps watching timelocks + servicing offers).
    liveTitle: "Um swap está em andamento",
    liveBodyOne:
      "1 swap está em andamento. Ele é governado por timelocks on-chain — a engine precisa continuar rodando para fazer o redeem ou refund antes do prazo.",
    liveBodyMany:
      "{count} swaps estão em andamento. Eles são governados por timelocks on-chain — a engine precisa continuar rodando para fazer o redeem ou refund antes do prazo.",
    keepRunningExplain:
      "Fechar a janela mantém a engine rodando em segundo plano, então ela conclui o swap sem interface. Você pode reabrir o Satchel a qualquer momento para conferir.",
    forceQuitWarn: "Forçar o encerramento agora para a engine e pode causar perda de fundos.",
    // {word} is the confirm word below; a translation may localize both together.
    typeToConfirm: "Para forçar o encerramento mesmo assim, digite {word} abaixo.",
    confirmWord: "QUIT",
    keepRunning: "Manter rodando, fechar janela",
    keepWithdraw: "Manter rodando + retirar ofertas",
    keepLeaveOffers: "Manter rodando, deixar ofertas no ar",
    forceQuit: "Forçar encerramento",
    offersTitle: "Você tem ofertas publicadas",
    offersBodyOne:
      "1 oferta sua ainda está no Corkboard. Ofertas não bloqueiam nada, mas deixá-la no ar significa que contrapartes ainda podem aceitá-la enquanto o Satchel estiver fechado — a engine atenderá o aceite.",
    offersBodyMany:
      "{count} ofertas suas ainda estão no Corkboard. Ofertas não bloqueiam nada, mas deixá-las no ar significa que contrapartes ainda podem aceitá-las enquanto o Satchel estiver fechado — a engine atenderá os aceites.",
    withdrawExit: "Retirar todas e sair",
  },
  unlock: {
    title: "Desbloquear merchant",
    body:
      "A seed deste merchant está criptografada. Digite a senha dela para desbloqueá-la nesta sessão — o Satchel a mantém apenas em memória e a esquece ao sair.",
    switchMerchant: "Trocar merchant",
    unlock: "Desbloquear",
  },
  common: {
    cancel: "Cancelar",
    confirm: "Confirmar",
    save: "Salvar",
    done: "Concluído",
    later: "Depois",
    retry: "Tentar reconectar",
  },
};
