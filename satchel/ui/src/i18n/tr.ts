// The Turkish (Türkçe) string bundle. Mirrors en.ts key-for-key; only values are translated.
import type { Bundle } from "./en";

export const tr: Bundle = {
  app: {
    name: "Satchel",
    tagline: "güvene dayanmayan takaslar",
  },
  // In-app update notifications (sidebar version badge + dialog).
  update: {
    title: "Güncelleme mevcut",
    upToDate: "Güncelsiniz",
    current: "Yüklü",
    latest: "En son",
    notesTitle: "Sürüm notları",
    get: "Güncellemeyi al",
    dismiss: "Yoksay",
    close: "Kapat",
    badgeTooltip: "Güncelleme mevcut — ayrıntılar için tıklayın",
    versionTooltip: "Güncellemeleri kontrol etmek için tıklayın",
  },
  // Risk disclaimer (first-run welcome + Settings → About).
  disclaimer: {
    title: "Kendi varlık koruması — anahtarlar sizin, sorumluluk sizin",
    body: "Satchel saklama gerektirmeyen atomik takaslar gerçekleştirir: anahtarlarınızı yalnızca siz tutarsınız ve bir takas yürürken bir satıcının seed'i sıcak transit anahtarlarını tutar. Takas protokolleri (v1 HTLC ve v2 Taproot/MuSig2) incelenmiş olup mainnet üzerinde canlıdır. MIT lisanslıdır ve hiçbir garanti olmaksızın olduğu gibi sunulur — kurtarma ifadenizi yedekleyin ve riski kendinize ait olmak üzere kullanın.",
  },
  nav: {
    public: "Herkese açık",
    corkboard: "Corkboard",
    postOffer: "Teklif yayınla",
    private: "Özel",
    privateCreate: "Fiş oluştur",
    privateReceive: "Bir fiş al",
    privateSlips: "Fişlerim",
    swaps: "Takaslar",
    relays: "Röleler",
    wallets: "Cüzdanlar",
    settings: "Ayarlar",
    coins: "Coin'ler",
  },
  makeOffer: {
    title: "Teklif yayınla",
    intro:
      "Corkboard'a imzalı bir teklif yayınlayın. Hiçbir şey kilitlenmez — bu yalnızca bir ilandır; istediğiniz zaman geri çekebilirsiniz ve takas yalnızca biri teklifi aldığında ve her iki taraf da fonladığında başlar.",
    give: "Verirsiniz",
    want: "Alırsınız",
    // Canonical pair + direction: pick the pair, choose Sell/Buy the base, enter
    // the base amount and a quote-per-base price (invariant to direction).
    pair: "Parite",
    noPairs: "İşlem yapılabilir parite yok — Ayarlar → Coin'ler bölümünden en az iki coin bağlayın.",
    sell: "{sym} sat",
    buy: "{sym} al",
    amount: "Miktar",
    youGive: "Verirsiniz",
    youGet: "Alırsınız",
    price: "Fiyat",
    priceUnit: "{base} başına {unit}",
    pricePlaceholder: "birim fiyat",
    balance: "Bakiye: {amt} {sym}",
    balanceLoading: "Bakiye: …",
    noCoins: "Yapılandırılmış coin yok",
    sameCoin: "Verilen ve alınan coin'ler farklı olmalıdır.",
    legDown: "Bu coin'lerden birinin düğümü çalışmıyor — yayınlamadan önce başlatın (veya Ayarlar → Coin'ler bölümünü kontrol edin).",
    // Swap-protocol pin (only offered when a pair+network supports more than
    // one). v2 label reuses coins.protoPrivate ("Private (Taproot)").
    protocol: "Takas türü",
    protoStandard: "Standart (HTLC)",
    // Titles for the review/confirm dialog shown before posting (see OfferForm).
    reviewOfferTitle: "Teklifinizi gözden geçirin",
    reviewSlipTitle: "Fişinizi gözden geçirin",
    term: "Güvenlik zaman kilidi",
    termShort: "Kısa",
    termMedium: "Orta",
    termLong: "Uzun",
    termHint: {
      short: "Kısa — işlem takılırsa fonlar en hızlı şekilde otomatik geri ödenir (~12s / 6s), en küçük güvenlik payıyla.",
      medium: "Orta — dengeli geri ödeme penceresi (~24s / 12s).",
      long: "Uzun (en güvenli) — en geniş güvenlik payı; işlem takılırsa ~36s / 18s sonra otomatik geri ödeme.",
    },
    validFor: "Geçerlilik süresi (dakika)",
    validForMins: "{mins} dk",
    validForHint:
      "Teklifin ne kadar süre listede kalacağı. Çevrimiçi olduğunuz sürece otomatik olarak güncel tutulur; bu süreden sonra geçerliliğini yitirir. Uygulamayı kapatmak teklifi geri çeker.",
    note: "Sabit boyutlu teklif — biri alana kadar hiçbir şey kilitlenmez. Miktarlar zincir üzerindedir; üstüne ağ ücretlerini siz ödersiniz ve Corkboard hiçbir ücret almaz. Zaman kilidi, bir takas takılırsa otomatik geri ödeme penceresidir.",
    post: "Teklifi yayınla",
    makeSlip: "Fiş oluştur",
    slipTitle: "Özel teklif fişiniz",
    slipExplainer:
      "Bunu arkadaşınıza gönderin. Almak için Satchel'a yapıştırırlar. Hiçbir şey kilitlenmez; {ttl} içinde geçerliliğini yitirir.",
    copy: "Kopyala",
    copied: "Kopyalandı",
    makeAnother: "Bir tane daha oluştur",
    myPrivateTitle: "Özel tekliflerim",
    myPrivateEmpty: "Bekleyen özel teklif yok.",
    privateExpires: "geçerlilik {when}",
    privateExpired: "süresi doldu",
    cancel: "İptal",
    cancelTip: "Bu fişi geçerli saymayı durdurun — hâlâ elinde tutan bir arkadaşınız artık alamaz.",
  },
  takeSlip: {
    open: "Bir fiş yapıştır",
    title: "Özel bir teklif al",
    intro:
      "Bir arkadaşınız size özel bir teklif fişi gönderdi (pactoffer1: ile başlar). Gözden geçirip almak için buraya yapıştırın — tıpkı panodan gelen bir teklif gibi.",
    placeholder: "pactoffer1:…",
    take: "Gözden geçir ve al",
    invalid: "Bu bir fişe benzemiyor — pactoffer1: ile başlamalı.",
    previewLabel: "Bu fiş şunu sunuyor",
  },
  // PRIVATE nav group (off-market slips) — screen titles + intros. The form,
  // slip output, my-slips list and take flow reuse makeOffer.*/takeSlip.* copy.
  private: {
    createTitle: "Özel bir teklif oluştur",
    createIntro:
      "İmzalı bir teklif oluşturup kendi sohbetiniz üzerinden bir arkadaşınıza fiş olarak verin. Hiçbir yerde listelenmez — ve ikiniz de fonlayana kadar hiçbir şey kilitlenmez.",
    slipsIntro:
      "Oluşturduğunuz fişler. Bir fişi elinde tutan herkes, süresi dolana kadar onu alabilir; süre dolmadan geçerli saymayı durdurmak için birini iptal edin.",
    slipsEmptyBody: "Bir arkadaşınıza gönderebileceğiniz bir fiş almak için özel bir teklif oluşturun.",
    receiveTitle: "Özel bir teklif al",
    received: "Alındı — Takaslar bölümünden takip edin.",
  },
  // Shared take-confirmation dialog (board take + slip take).
  takeConfirm: {
    title: "Bu teklif alınsın mı?",
    confirm: "Teklifi al",
    counterparty: "Karşı taraf",
    youGive: "Verirsiniz",
    youReceive: "Alırsınız",
    safetyRefund: "Güvenlik geri ödemesi",
    offerAge: "Teklif yaşı",
    makerFundsFirst:
      "Maker {sym} miktarını önce kilitler — siz asla önce göndermezsiniz. Kendi tarafınızı fonlamadan önce yine de iptal edebilirsiniz ve takas takılırsa motor, güvenlik zaman kilidinin ardından otomatik geri ödeme yapar.",
  },
  header: {
    activeMerchant: "Etkin satıcı — değiştirmek veya yönetmek için tıklayın",
    manageMerchants: "Satıcıları Yönet…",
    noMerchant: "satıcı yok",
    openMenu: "Menüyü aç",
    collapseMenu: "menüyü daralt",
    settings: "Ayarlar",
    language: "Dil",
    pactConnected: "Motor bağlı",
    pactUnreachable: "Motora ulaşılamıyor",
    liveSwapsOne: "1 takas yürürlükte — görüntülemek için tıklayın",
    liveSwapsMany: "{count} takas yürürlükte — görüntülemek için tıklayın",
    liveSwapsNone: "Yürürlükte takas yok",
    coinOk: "{name} — bağlı · tepe {tip}",
    coinUnconfigured: "{name} — kurulmadı",
    coinError: "{name} — {status}",
    relaysOk: "Nostr röleleri — {up}/{total} bağlı",
    relaysDown: "Nostr röleleri — {total} röleden hiçbiri bağlı değil",
  },
  network: {
    mainnet: "MainNet",
    testnet: "TestNet",
    regtest: "RegTest",
    signet: "Signet",
    notRealFunds: "Gerçek fon değil — bu {network} ağıdır",
  },
  // Watch-only mode: a viewer session with no coins. Browse the board and
  // withdraw your own offers, but no posting / taking / funding.
  watchOnly: {
    badge: "Yalnızca izleme",
    badgeTip:
      "Yalnızca izleme modu — panoya göz atabilir ve kendi tekliflerinizi geri çekebilirsiniz, ancak yayınlayamaz, alamaz veya fonlayamazsınız. İşlem yapmak için Ayarlar'dan coin'leri kurun.",
    coinWizardButton: "Yalnızca izleme modunda göz at",
    coinWizardHint:
      "Coin kurulumunu atlayıp yalnızca panoya göz atın (salt okunur). Yine de kendi tekliflerinizi geri çekebilirsiniz — başka bir oturumun bıraktığı teklifleri kaldırmak için kullanışlıdır. İstediğiniz zaman Ayarlar'dan kapatabilirsiniz.",
    postBlockedTitle: "Yalnızca izleme modu",
    postBlockedBody:
      "Bu yalnızca izleme oturumudur, bu yüzden teklif yayınlayamaz. İşlem yapmak için Ayarlar → Coin'ler bölümünden en az iki coin kurun.",
    takeBlockedBody: "Yalnızca izleme modu — bu teklifi gözden geçirebilirsiniz, ancak almak için coin kurulumu gerekir.",
    takeBlockedTip: "Yalnızca izleme modu — teklif almak için Ayarlar'dan coin'leri kurun.",
  },
  merchants: {
    title: "Satıcılarınız",
    intro:
      "Bir satıcı, tek bir işlem kimliğidir — kendi seed'i ve takas geçmişi vardır. Farklı bir satıcı altında işlem yapmak, bağlamları birbirine bağlanamaz tutar (tek kullanımlık bir kimlik). Ana coin'leriniz burada değil, kendi cüzdanınızda durur.",
    // First-run welcome (empty merchant manager).
    welcomeTitle: "Satchel'a hoş geldiniz",
    welcomeIntro:
      "Satchel, kendi seed'ine sahip tek bir işlem kimliği olan bir “satıcı” altında işlem yapar. Henüz hiçbiriniz yok: yeni bir tane oluşturun veya başlamak için mevcut bir kurtarma ifadesini içe aktarın.",
    importMerchant: "Bir satıcı içe aktar",
    none: "Henüz satıcı yok.",
    active: "etkin",
    switch: "değiştir",
    newMerchant: "Yeni satıcı",
    thisMerchant: "bu satıcı",
    nameLabel: "Satıcı adı",
    namePlaceholder: "örn. Ana",
    introFirst:
      "İlk işlem kimliğinizi (bir “satıcı”) kurun. Yalnızca yürürlükteki takaslar için sıcak transit anahtarlarını tutar — ana coin'leriniz kendi cüzdanınızda kalır.",
    introNew: "Yeni bir satıcı, kendi seed'i ve takas geçmişi olan yeni, ayrı bir kimliktir.",
    // UI-5 merchant selector (phoenix wallet-selector parity).
    createNew: "Yeni oluştur",
    import: "İçe aktar",
    load: "Satıcıyı Yükle",
    loaded: "yüklendi",
    locked: "kilitli",
    lockedTip: "Şifrelenmiş seed — yüklerken parolanızla kilidini açın.",
    close: "Kapat",
    idLabel: "klasör",
    switching: "Satıcı değiştiriliyor…",
    switchingBody: "Motor o klasöre karşı yeniden başlatılıyor.",
  },
  // Seed create/import (SeedForm) + counterparty + status.
  seed: {
    intro: "Yepyeni bir seed oluşturun veya hâlihazırda sahip olduğunuz birini içe aktarın.",
    createNew: "Yeni oluştur",
    createDesc: "Yeni bir seed üretin. Kurtarma ifadesini siz yedeklersiniz.",
    import: "İçe aktar",
    importDesc: "Mevcut bir 12/24 sözcüklü ifadeden geri yükleyin.",
    recoveryLabel: "Kurtarma ifadesi",
    importPlaceholder: "sözcük1 sözcük2 sözcük3 …",
    encrypt: "Şifrele",
    encryptDesc:
      "Bir parola, seed'i beklerken korur. Oturum başına bir kez girersiniz — Satchel onu asla saklamaz. Not: Yeniden başlatmadan sonra, siz onu tekrar girene kadar gözetimsiz otomatik geri ödeme duraklar.",
    noPassphrase: "Parola yok (önerilir)",
    noPassphraseDesc:
      "Otomatik geri ödeme, hiçbir şey girmeden yeniden başlatmalar boyunca çalışmaya devam eder — bu yalnızca sıcak bir transit seed'idir. Bedeli: dosya/ana makine erişimi, bu satıcının transit anahtarlarını + kimliğini açığa çıkarır.",
    passphraseLabel: "Parola",
    passphrasePlaceholder: "bir parola seçin",
    createTitle: "Seed oluştur",
    importTitle: "Seed içe aktar",
    secureTitle: "{label} güvenliğini sağla",
    revealTitle: "Kurtarma ifadenizi yazın",
    revealBody:
      "Bu sözcüklere sahip herkes bu satıcının sıcak anahtarlarını kontrol eder. Satchel hiçbir kopya tutmaz — çevrimdışı saklayın. Sonraki adımda birkaç sözcüğü onaylayacaksınız.",
    ackLabel: "Kurtarma ifademi yazdım.",
    // Multi-step onboarding (create/import -> secret -> confirm -> passphrase).
    chooseTitle: "{label} kurulumu",
    enterTitle: "Kurtarma ifadenizi içe aktarın",
    enterBody:
      "Her sözcüğü yazın — yazdıkça otomatik tamamlanırlar — veya tüm ifadeyi yapıştırın. Devam etmeden önce kontrol ederiz.",
    wordCount: "{n} sözcük",
    wordAria: "Sözcük {n}",
    checkIncomplete: "{n} sözcüğün tamamını girin.",
    checkUnknown: "Bazı sözcükler BIP39 sözcük listesinde yok — vurgulananları kontrol edin.",
    checkBadChecksum: "Sağlama toplamı eşleşmiyor — sözcüklerinizi ve sıralarını yeniden kontrol edin.",
    checkOk: "Kurtarma ifadesi geçerli görünüyor.",
    verifyTitle: "Yedeğinizi onaylayın",
    verifyBody: "İfadeyi yazdığınızı doğrulamak için bu konumlardaki sözcükleri girin.",
    verifyWord: "Sözcük #{n}",
    verifyMismatch: "Bunlar ifadenizle eşleşmiyor — yedeğinizi kontrol edin.",
    passphraseTitle: "Seed'i koruyun",
    passphraseBody:
      "İsteğe bağlı olarak saklanan seed'i bir parolayla şifreleyin. Bunu atlayabilirsiniz — aşağıdaki dengeye bakın.",
  },
  counterparty: {
    you: "Bu sizsiniz",
    youShort: "siz",
    unknown: "bilinmeyen kimlik",
    // Short fingerprint fallback (identity.ts shortId) when no pubkey is known.
    unknownShort: "bilinmiyor",
  },
  status: {
    notConnectedTitle: "Motora bağlı değil",
    disconnectedBody:
      "Satchel motora ulaşamıyor. Hâlâ başlıyor olabilir veya etkin satıcının düğüm bağlantıları kesilmiş olabilir. Yeniden deneyin veya yukarıdaki seçiciden satıcı değiştirin.",
    openInSatchel: "Bunu Satchel'da aç",
    noTauriBody:
      "Bu, Satchel'ın arayüzüdür — motora ulaşmak için Tauri köprüsüne ihtiyaç duyar. Tarayıcı yerine masaüstü uygulamasını (cargo tauri dev) başlatın.",
  },
  settings: {
    title: "Ayarlar",
    subtitle: "Bu kurulum için uygulama genelinde tercihler.",
    // UI-3 Settings tabs.
    tabGeneral: "Genel",
    tabCoins: "Coin'ler",
    tabNetwork: "Ağ",
    tabAbout: "Hakkında",
    appearance: "Görünüm",
    theme: "Tema",
    themeDark: "Koyu",
    themeLight: "Açık",
    themeSystem: "Sistem",
    themeHint: "Satchel'ın görünümünü seçin. Sistem, işletim sisteminizin ayarını izler.",
    language: "Dil",
    languageHint: "Çeviriler katkıda bulundukça daha fazla dil eklenecek.",
    mode: "Mod",
    watchOnly: "Yalnızca izleme modu",
    watchOnlyHint:
      "Coin kurmadan panoya göz atın. Yine de kendi tekliflerinizi geri çekebilirsiniz, ancak yayınlayamaz, alamaz veya fonlayamazsınız. İşlem yapmak için kapatın (en az iki coin bağlı olması gerekir).",
    network: "Ağ",
    boards: "Corkboard'lar",
    boardsDesc:
      "İsteğe bağlı, kendiniz barındırdığınız HTTP panoları. Güvendiğiniz herhangi birini ekleyin; Nostr'a güvenmek için boş bırakın.",
    boardsNone: "Yapılandırılmış yok",
    nostrRelays: "Nostr röleleri",
    nostrRelaysDesc:
      "Röleler, ilan panosunu merkeziyetsiz bir ağ üzerinden taşır — hiçbir operatör tekliflerinizi okuyamaz veya eşleştiremez. Varsayılan bir setle önceden bağlanmıştır; serbestçe düzenleyin.",
    nostrRelaysOff: "Kapalı — Nostr aktarımı devre dışı",
    addUrl: "Ekle",
    removeUrl: "Kaldır",
    relayInvalid: "Bir ws:// veya wss:// röle URL'si girin",
    boardInvalid: "Bir http:// veya https:// pano URL'si girin",
    netSave: "Kaydet ve yeniden bağlan",
    netSaving: "Kaydediliyor ve yeniden bağlanılıyor…",
    netSaved: "Kaydedildi",
    // Fees tab — fee-bump policy (per active merchant).
    tabFees: "Ücretler",
    fees: "Ücret artırma",
    feesScope: "Bu ayarlar etkin satıcı için geçerlidir.",
    feesIntro:
      "Ücret artırmaları için güvenlik/maliyet dengeleri, zorunlu kurulum değildir. Yeni değerler gelecekteki artırmalara uygulanır; zaten fonlanmış takaslar, fonlandıkları politikayı korur.",
    feeMax: "Maks. ücret oranı (sat/vB)",
    feeMaxHint:
      "Her ücret artırması için tavan. Varsayılan 500, aynı zamanda kesin sistem maksimumudur. Maliyetleri sınırlamak için düşürün.",
    feeReservation: "Fonlama artırma rezervasyonu (×)",
    feeReservationHint:
      "Fon kontrolünün artırma payı olarak ayırdığı miktar. Yüksek olması daha büyük ücret sıçramalarını kurtarır ancak daha fazla bakiyeyi bağlar ve daha fazla takası reddeder. Varsayılan 3.",
    feeCommitted: "Geri alma fazla tahsisi (×)",
    feeCommittedHint:
      "Satchel kapalıyken bile onaylansın diye v2 geri alma ücretinin ne kadar fazla önceden ödendiği. Yalnızca yeni takaslara uygulanır. Varsayılan 2.",
    feeSave: "Kaydet",
    feeSaving: "Kaydediliyor…",
    feeSaved: "Kaydedildi",
    feeReset: "Varsayılanlara sıfırla",
    coins: "Coin'ler ve düğümler",
    coinsHint: "Her coin'i kendi düğümünüze bağlayın. Hiçbir şey kaydedilmeden önce genesis kontrol edilir.",
    about: "Hakkında",
    version: "Sürüm {version}",
    updateUpToDate: "Güncel",
    updateCheckPlaceholder: "Güncelleme kontrolü daha sonraki bir sürümde gelecek.",
    trustModel: "Anahtarlarınız nerede yaşar",
    trustModelBody:
      "Gizli bilgiler motorda yaşar, asla Satchel'da değil. Satıcı seed'i, motorun veri klasöründe durur (şifreli veya düz metin — seçim sizin); Satchel hiçbir seed veya parola saklamaz. Seed tasarım gereği sıcaktır (yalnızca transit anahtarları) — kayda değer gelirleri kendi soğuk cüzdanınıza süpürün.",
  },
  coins: {
    intro:
      "Her coin'i kendi düğümünüze bağlayın. İlk URL, düğümünüzün kendi cüzdanıdır — takas bacaklarınızı fonlar ve gelirleri alır. Hiçbir şey kaydedilmeden önce, fonlar asla yanlış zincire gönderilemesin diye Satchel düğümün genesis bloğunu kontrol eder. Bağlantılar tüm satıcılarınız arasında paylaşılır.",
    networkBadge: "{network} ağı için yapılandırılıyor",
    needMerchant:
      "Önce bir satıcı bağlayın — coin kurulumu için motorun çalışıyor olması gerekir. Sağ üstteki satıcı seçicisini kullanın.",
    pairsTitle: "İşlem pariteleri",
    pairsHint:
      "Pariteler, her coin'in yapabildiklerinden türetilir — sabit bir liste yoktur. Bir parite, iki coin'i de bağlandığında açılır.",
    noPairs: "Mevcut parite yok.",
    notSetUp: "Kurulmadı",
    connectedTip: "Bağlı · tepe {tip}",
    connError: "Bağlantı hatası",
    setUp: "Kur",
    editConnection: "Bağlantıyı düzenle",
    remove: "kaldır",
    disconnectTip: "Bu coin'in bağlantısını kes",
    disconnectTitle: "{coin} bağlantısı kesilsin mi?",
    disconnectBody: "Ona ihtiyaç duyan takaslar, yeniden bağlanana kadar kullanılamaz.",
    ready: "İşleme hazır",
    connectMissing: "{coins} bağla",
    notBuildable: "Henüz oluşturulabilir değil",
    // Swap-protocol chips on a pair (pact-htlc-v1 HTLC vs pact-htlc-v2 adaptor).
    protoPrivate: "Özel (Taproot)",
    protoPrivateTip: "Özel takas (Taproot/MuSig2 adaptör) — zincir üzerinde sıradan bir ödeme gibi görünür",
    protoHtlcTip: "Klasik HTLC takası",
    // Coin-setup backend choices (CoinSetup).
    backendCoreTitle: "Core RPC cüzdanı",
    backendCoreDesc: "Düğümünüzün kendi cüzdanı takası fonlar ve gelirleri alır.",
    backendHardwareTitle: "Donanım",
    backendHardwareDesc: "Fonlama bacağı için Ledger / PSBT imzalama.",
    backendLater: "sonra",
    // CoinSetup dialog.
    setupTitle: "{coin} bağla",
    setupIntro:
      "Satchel'ı kendi {sym} düğümünüze yönlendirin. Düğüm bir genesis-bloğu kontrolünden geçene kadar hiçbir şey kaydedilmez — fonlarınız yalnızca gerçek {sym} zincirine dokunur.",
    backendUrlLabel: "Düğüm arka uç URL'leri",
    backendUrlHint:
      "İlk URL = düğümünüzün kendi cüzdanı (takasları fonlar, gelirleri alır). Ekstra, bağımsız zincir görünümleri için virgüllerden sonra Electrum sunucuları (tcp://host:port) ekleyin.",
    fundingWallet: "Fonlama cüzdanı",
    confirmationsLabel: "Kesinleşmeden önceki onay sayısı",
    confirmationsHint:
      "Bu zincirdeki bir fonlama veya geri almanın, bir takas ona göre hareket etmeden önce ne kadar derin olması gerektiği — reorg güvenlik payı. Yüksek olması daha güvenli ama daha yavaştır; önerilen varsayılan ({default}) için boş bırakın.",
    validateNode: "Düğümü doğrula",
    checking: "Düğüm kontrol ediliyor…",
    genesisOk: "Genesis eşleşti — bu doğru zincir",
    genesisDetail: "tepe yükseklik {tip} · genesis {hash}…",
    genesisBad: "Reddedildi — kaydedilmiyor",
    errorShort: "hata",
    // Structured connection form (CoinSetup v2).
    rpcHostLabel: "RPC ana makine",
    rpcPortLabel: "RPC bağlantı noktası",
    authMethodLabel: "Kimlik doğrulama",
    authCookie: ".cookie dosyası",
    authCookieDesc: "Düğümün .cookie dosyasını veri dizininden otomatik oku (varsayılan, parola saklanmaz).",
    authUserPass: "Kullanıcı / parola",
    authUserPassDesc: "Düğümünüzün yapılandırmasından rpcuser / rpcpassword — uzak bir düğüm için gereklidir.",
    rpcUserLabel: "RPC kullanıcı adı",
    rpcPasswordLabel: "RPC parolası",
    datadirLabel: "Düğüm veri dizini",
    cookiePathNote: "Çerez, bu dizin altında {path} konumundan okunur.",
    walletLabel: "Cüzdan adı (isteğe bağlı)",
    walletPlaceholder: "düğümünüzün cüzdanı",
    needPort: "Önce RPC bağlantı noktasını girin.",
    validateFirst: "Kaydetmeden önce düğümü doğrulayın.",
    savingReconnecting: "Kaydediliyor ve yeniden bağlanılıyor…",
    connected: "{coin} bağlandı",
    // Template picker (a coins.toml coin the engine version doesn't support).
    unsupportedByEngine: "Desteklenmiyor",
    unsupportedByEngineTip:
      "Bu coin coins.toml içinde tanımlı ancak motorun bu sürümünde yerleşik değil, bu yüzden işlem yapılamaz.",
  },
  coinWizard: {
    title: "Coin'lerinizi bağlayın",
    intro:
      "En az iki coin seçin ve her birini kendi düğümünüze yönlendirin. Bir takas iki zincire ihtiyaç duyar, bu yüzden iki düğüm bağlanıp canlı olduğunda işlem yapma açılır. Daha sonra Ayarlar'dan coin ekleyebilir veya değiştirebilirsiniz.",
    progress: "{min} coin'den {count} tanesi bağlı",
    continue: "Devam et",
    live: "Canlı",
    nodeDown: "Düğüm çalışmıyor",
  },
  wallets: {
    intro:
      "Bunlar, kendi düğümlerinizin cüzdanlarıdır (motorun takasları fonlamak ve gelirleri almak için kullandıkları) — anahtarlar sizin, makine sizin. Satchel asla coin'lerinizi tutmaz.",
    hotSeedNudge:
      "Bu, bir kasa değil, sıcak bir seed üzerindeki harcama cüzdanıdır — kayda değer bakiyeleri kendi soğuk/core cüzdanınıza süpürün.",
    notConnected: "Bağlı değil",
    notConnectedBody: "Önce bir satıcı bağlayın — cüzdan görünümü için motorun çalışıyor olması gerekir.",
    noCoins: "Henüz coin kurulmadı",
    noCoinsBody: "Ayarlar → Coin'ler bölümünden bir coin bağlayın, cüzdanı burada görünür.",
    goToCoins: "Coin'lere git",
    watchOnlyTitle: "Yalnızca izleme modunda cüzdan yok",
    watchOnlyBody:
      "Bu, bağlı coin'i olmayan yalnızca izleme oturumudur, bu yüzden gösterilecek cüzdan yok. Ayarlar'dan yalnızca izlemeyi kapatın ve takasları fonlamak için bir coin bağlayın.",
    walletName: "cüzdan · {wallet}",
    walletScopedHint: "Bu coin için her RPC, bu düğüm cüzdanına kapsamlandırılmıştır.",
    walletDefault: "varsayılan cüzdan (kapsamlandırılmamış)",
    walletDefaultHint:
      "Bu coin için cüzdan ayarlanmamış, bu yüzden RPC'ler düğümün varsayılan cüzdanını kullanır. Her çağrıyı belirli bir cüzdana kapsamlandırmak için Ayarlar → Coin'ler bölümünden bir tane ayarlayın.",
    balanceLabel: "{symbol} bakiyesi",
    receive: "Al",
    send: "Gönder",
    sendTo: "Adrese gönder",
    amount: "Miktar",
    sendTitle: "{amount} {sym} gönderilsin mi?",
    sendConfirmBody: "Şu adrese: {to}\n\nBu, kendi düğümünüzün cüzdanından harcar ve geri alınamaz.",
  },
  corkboard: {
    noBoardTitle: "Bağlı Corkboard yok",
    noBoardBody:
      "Bir Corkboard, maker'ların teklif iğnelediği paylaşımlı bir ilan panosudur. Asla takasları eşleştirmez veya coin tutmaz — göz atmak ve yayınlamak için Satchel'ı güvendiğiniz birine yönlendirin.",
    noPairs: "Mevcut parite yok",
    board: "Corkboard",
    boardSettings: "Ayarlar'dan yapılandır",
    filterAll: "Tümü",
    filterMine: "Benimkiler",
    offered: "{symbol} sunuldu",
    noOffers: "Şu anda alabileceğiniz teklif yok",
    noOffersBody:
      "Kurduğunuz bir parite için bir maker teklif yayınlar yayınlamaz teklifler burada görünür. Kendi teklifinizi de yayınlayabilirsiniz.",
    hiddenOffers:
      "Bağlamadığınız pariteler için {count} teklif daha var. Onlarla işlem yapmak için her iki coin'i de kurun:",
    yourOffer: "sizin teklifiniz",
    offerStaged: "yayınlanıyor…",
    offerStagedTip:
      "Bu cihazdan yayınlandı ve bir röleden geri onaylanmayı bekliyor. İlan veriyor; bir röle yansıttığında canlı hale gelir.",
    take: "Teklifi al",
    legDown: "Bu paritenin düğümlerinden biri çalışmıyor — almadan önce başlatın (veya Ayarlar → Coin'ler bölümünü kontrol edin).",
    withdraw: "Geri çek",
    withdrawTip: "Anında geri çekin — bir teklif asla fon kilitlemez",
    safetyRefund: "güvenlik geri ödemesi",
    safetyRefundTip:
      "Takas takılırsa, her iki taraf da otomatik geri ödeme alır — taker'ın bacağı önce, sizinki biraz sonra açılır. Kimse takılı kalmaz.",
    activeTitle: "Etkin takaslarınız",
    states: {
      open: "açık",
      takenByUs: "siz aldınız",
      revoked: "geri çekildi",
      expired: "süresi doldu",
    },
    // Two-sided order-book view of the Corkboard. Bids = makers giving the
    // quote coin to get the base; asks = the reverse. The ladder is a way to
    // READ the board — it never matches or prioritises (load-bearing).
    book: {
      bids: "Alışlar",
      asks: "Satışlar",
      bidsHint: "{base} isteniyor · {quote} ödeniyor",
      asksHint: "{base} satılıyor · {quote} karşılığı",
      price: "Fiyat",
      size: "Boyut",
      noBids: "Alış yok",
      noAsks: "Satış yok",
      spread: "Makas {pct}",
      spreadOneSided: "Tek taraflı",
      crossed: "çaprazlanmış",
      crossedTip: "En iyi alış ≥ en iyi satış. Pano asla otomatik eşleştirmez, bu yüzden bu örtüşen teklifler öylece durur — iki taraftan birini alın.",
      mid: "orta {price}",
      levelOffers: "Bu fiyatta {count} teklif — almak için birini seçin",
      depthTip: "{count} ilan boyunca bu fiyatta sunulan toplam {sym}.",
      takerNote: "Onu alırsanız, {give} verir ve {get} alırsınız.",
      selectLevel: "Oradaki teklifleri görmek için yukarıdan bir fiyat seviyesi seçin.",
      paneHeader: "{size} {base} @ {price} {unit}",
      denomTip: "{coin} miktarları için görüntüleme birimi",
      showMore: "{count} tane daha göster",
      showLess: "İlk {count} tanesini göster",
    },
  },
  relays: {
    title: "Röleler",
    subtitle: "Nostr rölelerinize canlı bağlantı — tekliflerinizin ve almalarınızın üzerinden geçtiği ağ. Röleleri Ayarlar → Ağ bölümünden ekleyin veya kaldırın.",
    connectedCount: "{up} / {total} bağlı",
    refresh: "Yenile",
    ms: "{ms} ms",
    up: "açık",
    down: "kapalı",
    statsTip: "{success}/{attempts} başarılı bağlantı · ↓{down} ↑{up}",
    none: "Yapılandırılmış röle yok",
    noneBody: "Ağ üzerinden teklif yayınlamak ve almak için Ayarlar → Ağ bölümünden bir Nostr rölesi ekleyin.",
    goToNetwork: "Ayarlar'a git",
    notConnected: "Bağlı değil",
    notConnectedBody: "Röle görünümü için motorun çalışıyor olması gerekir — önce bir satıcı bağlayın.",
  },
  swaps: {
    title: "Takaslar",
    hint: "Tüm defteriniz — yürürlükteki takaslar üstte, biten işlemler altta. Canlı takaslar üzerinde Corkboard'dan da işlem yapabilirsiniz.",
    activeTitle: "Yürürlükte",
    historyTitle: "Geçmiş",
    none: "Henüz takas yok — Corkboard'da bir teklif alın.",
    // Active-swaps dock action buttons + the refund-time label.
    cancel: "iptal",
    refund: "geri ödeme",
    dump: "günlükleri dök",
    dumpHint: "Bu takas için gizli bilgi içermeyen bir tanılama paketini (durum + günlük satırları) kopyalayarak geliştiricilere yapıştırın.",
    dumpCopied: "Tanılama kopyalandı — geliştiricilere yapıştırın.",
    dumpFailed: "Tanılama paketi kopyalanamadı.",
    refundAt: "geri ödeme {when}",
    // Confirm dialogs for acting on a live swap (ActiveSwaps).
    cancelTitle: "Bu takas iptal edilsin mi?",
    cancelConfirm: "Takası iptal et",
    cancelKeep: "Devam ettir",
    // Abort reason recorded on the swap when cancelled from Satchel.
    cancelReason: "Satchel'da iptal edildi",
    cancelBody:
      "Bu, siz fonlamadan önce takası terk eder. Henüz size ait hiçbir şey kilitli değil, bu yüzden hiçbir şey kaybetmezsiniz — teklif yalnızca tamamlanmaz.",
    refundTitle: "Fonlarınızı geri çekelim mi?",
    refundConfirm: "Geri öde",
    refundBody:
      "Güvenlik zaman kilidi geçti, bu yüzden kilitlediğiniz fonları geri talep edebilirsiniz. Bu, geri ödemenizi şimdi yayınlar; motor ayrıca son tarihten sonra bunu otomatik olarak da yapar.",
    col: {
      swap: "takas",
      role: "rol",
      state: "durum",
      amounts: "verir → alır",
      when: "ne zaman",
      finalTx: "son işlem",
    },
    // Expandable per-leg on-chain detail (the audit trail). We show both
    // funding txs + OUR settlement; never the counterparty's settlement or the
    // swap secret.
    audit: {
      toggle: "Zincir üzeri ayrıntıyı göster",
      title: "Zincir üzeri ayrıntı",
      youLocked: "kilitlediğiniz",
      theyLocked: "kilitledikleri",
      funding: "Fonlama",
      received: "Alındı",
      refunded: "Geri ödendi",
      pending: "henüz zincir üzerinde değil",
      copy: "İşlem kimliğini kopyala",
      copied: "İşlem kimliği kopyalandı",
    },
  },
  fees: {
    title: "Ağ maliyeti önizlemesi",
    estimated: "tahmini",
    provisionalNote: "Bu pactd derlemesi henüz ücret tahminini sunmuyor.",
    summary: "Bir takas, ödediğiniz 2 zincir üzeri işlemdir: verme zincirinde fonlama, alma zincirinde geri alma.",
    fallbackTip: "Bir düğüme ulaşılamadı, bu yüzden tutucu bir varsayılan ücret oranı kullanıldı — bunları bir tahmin olarak değerlendirin.",
    ifItStalls: "(takılırsa)",
  },
  funds: {
    insufficient:
      "Bu takası fonlamak için yeterli {sym} yok — ~{need} {sym} gerekir (miktar + fonlama ücreti), cüzdanda {have} {sym} var.",
  },
  wizard: {
    welcome: "Satchel'a hoş geldiniz",
    connectTitle: "Pact motoruna bağlanın",
    connectIntro:
      "Satchel, Pact motorunun ince bir istemcisidir — anahtarlarınızı tutan ve takasları yürüten çekirdek. Ona nasıl ulaşacağınızı seçin.",
    managed: "Yerleşik Pact motorunu çalıştır",
    managedDesc: "Satchel kendi Pact motorunu başlatır ve denetler. Önerilir.",
    external: "Harici bir Pact motoruna bağlan",
    externalDesc: "Zaten çalıştırdığınız bir Pact motoruna yönlendirin (başlatmadan önce SATCHEL_PACTD_URL + çerezi ayarlayın).",
    externalNote:
      "Harici mod, Satchel başlatılmadan önce ortam değişkenleriyle seçilir. Kullanmak için SATCHEL_PACTD_URL ayarlı olarak yeniden başlatın.",
    coinsTitle: "Coin'lerinizi ekleyin",
    coinsIntro:
      "Satıcınız oluşturulduktan sonra, Ayarlar → Coin'ler bölümünden her coin'i kendi düğümünüze bağlayın. Bir coin ve bir arka uç seçin (sıfır kurulum için herkese açık Electrum veya kendi düğümünüz); hiçbir şey kaydedilmeden önce genesis bu ağa karşı kontrol edilir.",
    coinsTemplatesSoon: "Tek tıkla coin şablonları daha sonraki bir sürümde burada gelir.",
    back: "Geri",
    continue: "Devam et",
    finish: "Kurulumu bitir",
  },
  // UI-4 docked activity log.
  log: {
    title: "Etkinlik",
    empty: "— etkinlik günlüğü —",
    count: "{count} satır",
    collapse: "Günlüğü daralt",
    expand: "Günlüğü genişlet",
    // Activity-log lines emitted by the frontend. `{err}` carries a raw engine
    // message (itself not translated — it comes from pactd over the wire).
    noTauri: "Satchel içinde çalışmıyor — bu arayüz Tauri köprüsüne ihtiyaç duyar",
    startupError: "başlatma: {err}",
    notConnected: "bağlı değil: {err}",
    connected: "pactd {version} öğesine bağlandı ({protocol})",
    listcoinsError: "listcoins: {err}",
    watchOnlyError: "yalnızca izleme: {err}",
    switchedMerchant: "{id} satıcısına geçildi",
    switchMerchantError: "satıcı değiştir: {err}",
    loadMerchantError: "satıcı yükle: {err}",
    merchantCreated: "{id} satıcısı oluşturuldu",
    merchantReady: "satıcı hazır",
    actionOk: "{action} {id}: tamam",
    actionError: "{action} {id}: {err}",
    diagCopied: "{id} için tanılama kopyalandı ({count} günlük satırı) — geliştiricilere yapıştırın",
    dumpError: "dök {id}: {err}",
    coinDisconnected: "{coin} bağlantısı kesildi",
    removeCoinError: "coin kaldır: {err}",
    tookOffer: "{id} teklifi alındı — artık aşağıdaki etkin takaslarınızda görünüyor",
    takeError: "al: {err}",
    offerWithdrawn: "{id} teklifi geri çekildi",
    withdrawError: "geri çek: {err}",
    postedOffer: "{id} teklifi yayınlandı — istediğiniz zaman geri çekin; hiçbir şey kilitli değil",
    createdSlip: "özel bir teklif fişi oluşturuldu — arkadaşınıza gönderin",
    tookPrivateOffer: "{id} özel teklifi alındı — artık etkin takaslarınızda görünüyor",
    cancelledPrivateOffer: "{id} özel teklifi iptal edildi",
    cancelError: "iptal: {err}",
    noticeboardUpdated: "ilan panosu güncellendi",
    feePolicyUpdated: "ücret politikası güncellendi",
  },
  // Relative-time + freshness prose from format.ts (rendered via the tr() mirror,
  // since those are pure non-component helpers). Unit letters stay inside the
  // template so a translation owns the whole phrase.
  format: {
    ageUnknown: "yaş bilinmiyor",
    justNow: "az önce",
    minutesAgo: "{n}dk önce",
    hoursAgo: "{n}s önce",
    daysAgo: "{n}g önce",
    expiryNow: "şimdi",
    expirySoon: "yakında",
    inMinutes: "~{n}dk içinde",
    inHours: "~{n}s içinde",
    inDays: "~{n}g içinde",
    posted: "{age} yayınlandı",
    expires: "geçerlilik {time}",
  },
  // Plain-language swap story per (role, state) — the honest "who is exposed
  // when" framing shown on every active swap. {a}/{b} are coin tickers; {t1}/{t2}
  // are local refund times. Rendered via tr() (narrate() is a pure helper).
  narrate: {
    initiating:
      "Alma gönderildi — maker'ın takası başlatmasını bekliyor. Henüz hiçbir şey kilitli değil; yanıt vermezlerse kendi kendine iptal olur.",
    created: "Teklif gönderildi — diğer tarafın kabul etmesini bekliyor. Hiçbir şey taahhüt edilmedi.",
    acceptedMaker: "Şartlar kabul edildi. Sıradaki: {a} miktarınızı kilitleyin. Fonlamadığınız sürece hâlâ serbestçe iptal edebilirsiniz.",
    acceptedTaker: "Şartlar kabul edildi. Diğer taraf {a} miktarını önce kilitler — siz asla önce göndermezsiniz.",
    noncesExchanged:
      "Özel takas kuruluyor — imzalama materyali değişiliyor. Henüz hiçbir şey kilitli değil.",
    signedMaker:
      "Her iki taraf da imzaladı. Daemon'ınız {a} miktarını kilitler, ardından {b} miktarını otomatik olarak talep eder. Bir şey takılırsa, {a} miktarınız {t1} zamanında geri döner.",
    signedTaker:
      "Her iki taraf da imzaladı. Daemon'ınız {b} miktarını kilitler ve diğer taraf hareket eder etmez {a} miktarını talep eder. Güvenlik ağı: {t2} zamanında geri ödeme.",
    fundedAMaker:
      "{a} miktarınız kilitli. Diğer tarafın {b} miktarını kilitlemesini bekliyor. Hiç yapmazlarsa, {a} miktarınız {t1} zamanında otomatik olarak geri döner.",
    fundedATaker:
      "{a} miktarları kilitli ve doğrulandı. Sıradaki: {b} miktarınızı kilitleyin. Güvenlik ağı: bir şey takılırsa {t2} zamanında otomatik geri ödeme.",
    fundedBMaker: "İkisi de kilitlendi. Daemon'ınız güvenle onaylanır onaylanmaz {b} miktarını talep eder.",
    fundedBTaker: "İkisi de kilitlendi. Daemon'ınız diğer taraf {b} miktarını alır almaz {a} miktarını talep edecek.",
    redeemedB:
      "{b} miktarını talep ettiniz — onaylanmasını bekliyor. Kilitli {a} miktarınız, bu kesinleşene kadar korunmaya devam eder.",
    completed: "Takas tamamlandı — {coin} cüzdanınızda.",
    refunded: "Takas tamamlanmadı, bu yüzden {coin} miktarınız otomatik olarak geri döndü. Ücretler dışında hiçbir kayıp yok.",
    aborted: "Herhangi bir para hareket etmeden önce iptal edildi.",
  },
  progress: {
    awaitingLock: "Onların kilidi bekleniyor",
    awaitingClaim: "Onların talebi bekleniyor",
    theirLock: "Onların kilidi onaylanıyor",
    securing: "{coin} güvenceye alınıyor",
    blocks: "+{n} blok",
    feeBumped: "Ücret artırıldı",
    reorg: "Reorg algılandı — yeniden denetleniyor",
  },
  exit: {
    // Exit-gate dialog (fund safety, C6). The engine manages alone, so "keep
    // running" detaches it (it keeps watching timelocks + servicing offers).
    liveTitle: "Bir takas yürürlükte",
    liveBodyOne:
      "1 takas yürürlükte. Zincir üzeri zaman kilitleriyle yönetilir — motor, son tarihten önce geri almak veya geri ödemek için çalışmaya devam etmelidir.",
    liveBodyMany:
      "{count} takas yürürlükte. Zincir üzeri zaman kilitleriyle yönetilir — motor, son tarihten önce geri almak veya geri ödemek için çalışmaya devam etmelidir.",
    keepRunningExplain:
      "Pencereyi kapatmak motoru arka planda çalışır durumda tutar, böylece takası penceresiz tamamlar. Kontrol etmek için Satchel'ı istediğiniz zaman yeniden açabilirsiniz.",
    forceQuitWarn: "Şimdi zorla çıkmak motoru durdurur ve fon kaybına neden olabilir.",
    // {word} is the confirm word below; a translation may localize both together.
    typeToConfirm: "Yine de zorla çıkmak için aşağıya {word} yazın.",
    confirmWord: "QUIT",
    keepRunning: "Çalışır durumda tut, pencereyi kapat",
    keepWithdraw: "Çalışır durumda tut + teklifleri geri çek",
    keepLeaveOffers: "Çalışır durumda tut, teklifleri bırak",
    forceQuit: "Zorla çık",
    offersTitle: "Yayınlanmış teklifleriniz var",
    offersBodyOne:
      "Size ait 1 teklif hâlâ Corkboard'da. Teklifler hiçbir şey kilitlemez, ancak bırakmak, Satchel kapalıyken karşı tarafların onu yine de alabileceği anlamına gelir — motor almayı işleyecek.",
    offersBodyMany:
      "Size ait {count} teklif hâlâ Corkboard'da. Teklifler hiçbir şey kilitlemez, ancak bırakmak, Satchel kapalıyken karşı tarafların onları yine de alabileceği anlamına gelir — motor almaları işleyecek.",
    withdrawExit: "Tümünü geri çek ve çık",
  },
  unlock: {
    title: "Satıcının kilidini aç",
    body:
      "Bu satıcının seed'i şifrelenmiş. Bu oturum için kilidini açmak üzere parolasını girin — Satchel onu yalnızca bellekte tutar ve çıkışta unutur.",
    switchMerchant: "Satıcı değiştir",
    unlock: "Kilidini aç",
  },
  common: {
    cancel: "İptal",
    confirm: "Onayla",
    save: "Kaydet",
    done: "Bitti",
    later: "Sonra",
    retry: "Bağlantıyı yeniden dene",
  },
};
