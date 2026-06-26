// The Indonesian (Bahasa Indonesia) string bundle. Mirrors en.ts key-for-key; only values are translated.
import type { Bundle } from "./en";

export const id: Bundle = {
  app: {
    name: "Satchel",
  },
  // In-app update notifications (sidebar version badge + dialog).
  update: {
    title: "Pembaruan tersedia",
    upToDate: "Anda sudah memakai versi terbaru",
    current: "Terpasang",
    latest: "Terbaru",
    notesTitle: "Catatan rilis",
    get: "Ambil pembaruan",
    dismiss: "Abaikan",
    close: "Tutup",
    badgeTooltip: "Pembaruan tersedia — klik untuk detail",
    versionTooltip: "Klik untuk memeriksa pembaruan",
  },
  // Risk disclaimer (first-run welcome + Settings → About).
  disclaimer: {
    title: "Self-custody — kunci Anda, tanggung jawab Anda",
    body: "Satchel melakukan atomic swap non-kustodian: hanya Anda yang memegang kunci Anda, dan seed seorang merchant memegang kunci transit panas selama swap berlangsung. Protokol swap (v1 HTLC dan v2 Taproot/MuSig2) sudah ditinjau dan berjalan di mainnet. Dilisensikan MIT dan disediakan apa adanya, tanpa jaminan — cadangkan frasa pemulihan Anda dan gunakan dengan risiko Anda sendiri.",
  },
  nav: {
    public: "Publik",
    corkboard: "Corkboard",
    postOffer: "Pasang penawaran",
    private: "Privat",
    privateCreate: "Buat slip",
    privateReceive: "Ambil slip",
    privateSlips: "Slip saya",
    swaps: "Swap",
    relays: "Relay",
    wallets: "Dompet",
    settings: "Pengaturan",
    coins: "Koin",
  },
  makeOffer: {
    title: "Pasang penawaran",
    intro:
      "Pasang penawaran bertanda tangan ke Corkboard. Tidak ada yang dikunci — ini hanya iklan; tarik kapan saja, dan swap baru dimulai saat seseorang mengambilnya dan kedua sisi mendanai.",
    give: "Anda berikan",
    want: "Anda terima",
    // Canonical pair + direction: pick the pair, choose Sell/Buy the base, enter
    // the base amount and a quote-per-base price (invariant to direction).
    pair: "Pasangan",
    noPairs: "Tidak ada pasangan yang bisa diperdagangkan — hubungkan setidaknya dua koin di Pengaturan → Koin.",
    sell: "Jual {sym}",
    buy: "Beli {sym}",
    amount: "Jumlah",
    youGive: "Anda berikan",
    youGet: "Anda dapat",
    price: "Harga",
    priceUnit: "{unit} per {base}",
    pricePlaceholder: "harga per unit",
    balance: "Saldo: {amt} {sym}",
    balanceLoading: "Saldo: …",
    noCoins: "Tidak ada koin yang dikonfigurasi",
    legDown: "Salah satu node koin ini mati — jalankan (atau cek Pengaturan → Koin) sebelum memasang.",
    // Swap-protocol pin (only offered when a pair+network supports more than
    // one). v2 label reuses coins.protoPrivate ("Private (Taproot)").
    protocol: "Jenis swap",
    protoStandard: "Standar (HTLC)",
    // Titles for the review/confirm dialog shown before posting (see OfferForm).
    reviewOfferTitle: "Tinjau penawaran Anda",
    reviewSlipTitle: "Tinjau slip Anda",
    term: "Timelock pengaman",
    termShort: "Pendek",
    termMedium: "Sedang",
    termLong: "Panjang",
    termHint: {
      short: "Pendek — dana auto-refund paling cepat jika perdagangan macet (~12j / 6j), dengan margin pengaman terkecil.",
      medium: "Sedang — jendela refund seimbang (~24j / 12j).",
      long: "Panjang (paling aman) — margin pengaman paling lebar; auto-refund setelah ~36j / 18j jika perdagangan macet.",
    },
    validFor: "Berlaku selama (menit)",
    validForMins: "{mins} mnt",
    validForHint:
      "Berapa lama penawaran tetap terpasang. Selama Anda online, penawaran disegarkan otomatis; setelah itu kedaluwarsa. Menutup aplikasi akan menariknya.",
    note: "Penawaran berukuran tetap — tidak ada yang dikunci sampai seseorang mengambilnya. Jumlah berada on-chain; Anda membayar biaya jaringan di atasnya dan Corkboard tidak memungut apa pun. Timelock adalah jendela auto-refund jika swap macet.",
    post: "Pasang penawaran",
    makeSlip: "Buat slip",
    slipTitle: "Slip penawaran privat Anda",
    slipExplainer:
      "Kirim ini ke teman Anda. Mereka menempelkannya ke Satchel untuk mengambilnya. Tidak ada yang dikunci; kedaluwarsa dalam {ttl}.",
    copy: "Salin",
    copied: "Tersalin",
    makeAnother: "Buat lagi",
    myPrivateTitle: "Penawaran privat saya",
    myPrivateEmpty: "Tidak ada penawaran privat yang aktif.",
    privateExpires: "kedaluwarsa {when}",
    privateExpired: "kedaluwarsa",
    cancel: "Batal",
    cancelTip: "Berhenti menghormati slip ini — teman yang masih memegangnya tidak bisa lagi mengambilnya.",
  },
  takeSlip: {
    intro:
      "Seorang teman mengirimi Anda slip penawaran privat (diawali dengan pactoffer1:). Tempel di sini untuk meninjau dan mengambilnya — persis seperti penawaran dari board.",
    placeholder: "pactoffer1:…",
    take: "Tinjau & ambil",
    invalid: "Itu tidak tampak seperti slip — seharusnya diawali dengan pactoffer1:.",
    previewLabel: "Slip ini menawarkan",
  },
  // PRIVATE nav group (off-market slips) — screen titles + intros. The form,
  // slip output, my-slips list and take flow reuse makeOffer.*/takeSlip.* copy.
  private: {
    createTitle: "Buat penawaran privat",
    createIntro:
      "Susun penawaran bertanda tangan dan serahkan ke teman sebagai slip lewat obrolan Anda sendiri. Tidak ada yang terdaftar di mana pun — dan tidak ada yang dikunci sampai kalian berdua mendanai.",
    slipsIntro:
      "Slip yang sudah Anda buat. Siapa pun yang memegang slip bisa mengambilnya sampai kedaluwarsa; batalkan salah satu untuk berhenti menghormatinya sebelum itu.",
    slipsEmptyBody: "Buat penawaran privat untuk mendapatkan slip yang bisa Anda kirim ke teman.",
    receiveTitle: "Ambil penawaran privat",
    received: "Diambil — ikuti di Swap.",
  },
  // Shared take-confirmation dialog (board take + slip take).
  takeConfirm: {
    title: "Ambil penawaran ini?",
    confirm: "Ambil penawaran",
    counterparty: "Lawan transaksi",
    youGive: "Anda berikan",
    youReceive: "Anda terima",
    safetyRefund: "Refund pengaman",
    offerAge: "Usia penawaran",
    makerFundsFirst:
      "Maker mengunci {sym} mereka lebih dulu — Anda tidak pernah mengirim duluan. Anda masih bisa membatalkan sebelum mendanai sisi Anda, dan engine akan auto-refund setelah timelock pengaman jika swap macet.",
  },
  header: {
    activeMerchant: "Merchant aktif — klik untuk beralih atau mengelola",
    manageMerchants: "Kelola Merchant…",
    noMerchant: "tidak ada merchant",
    openMenu: "Buka menu",
    collapseMenu: "ciutkan menu",
    settings: "Pengaturan",
    language: "Bahasa",
    pactConnected: "Engine terhubung",
    pactUnreachable: "Engine tidak terjangkau",
    liveSwapsOne: "1 swap sedang berjalan — klik untuk melihat",
    liveSwapsMany: "{count} swap sedang berjalan — klik untuk melihat",
    liveSwapsNone: "Tidak ada swap yang berjalan",
    coinOk: "{name} — terhubung · tip {tip}",
    coinUnconfigured: "{name} — belum diatur",
    coinError: "{name} — {status}",
    relaysOk: "Relay Nostr — {up}/{total} terhubung",
    relaysDown: "Relay Nostr — tidak ada dari {total} yang terhubung",
  },
  network: {
    mainnet: "MainNet",
    testnet: "TestNet",
    regtest: "RegTest",
    signet: "Signet",
    notRealFunds: "Bukan dana sungguhan — ini jaringan {network}",
  },
  // Watch-only mode: a viewer session with no coins. Browse the board and
  // withdraw your own offers, but no posting / taking / funding.
  watchOnly: {
    badge: "Hanya pantau",
    badgeTip:
      "Mode hanya-pantau — jelajahi board dan tarik penawaran Anda sendiri, tapi Anda tidak bisa memasang, mengambil, atau mendanai. Atur koin di Pengaturan untuk berdagang.",
    coinWizardButton: "Jelajahi dalam mode hanya-pantau",
    coinWizardHint:
      "Lewati pengaturan koin dan cukup jelajahi board (hanya-baca). Anda tetap bisa menarik penawaran Anda sendiri — berguna untuk menarik penawaran yang ditinggalkan sesi lain. Matikan kapan saja di Pengaturan.",
    postBlockedTitle: "Mode hanya-pantau",
    postBlockedBody:
      "Ini sesi hanya-pantau, jadi tidak bisa memasang penawaran. Atur setidaknya dua koin di Pengaturan → Koin untuk berdagang.",
    takeBlockedBody: "Mode hanya-pantau — Anda bisa meninjau penawaran ini, tapi mengambilnya membutuhkan koin yang sudah diatur.",
    takeBlockedTip: "Mode hanya-pantau — atur koin di Pengaturan untuk mengambil penawaran.",
  },
  merchants: {
    title: "Merchant Anda",
    intro:
      "Sebuah merchant adalah satu identitas dagang — dengan seed dan riwayat swap-nya sendiri. Berdagang di bawah merchant berbeda menjaga konteks tetap tidak bisa dikaitkan (identitas sekali pakai). Koin utama Anda ada di dompet Anda sendiri, bukan di sini.",
    // First-run welcome (empty merchant manager).
    welcomeTitle: "Selamat datang di Satchel",
    welcomeIntro:
      "Satchel berdagang di bawah sebuah “merchant” — satu identitas dagang dengan seed-nya sendiri. Anda belum punya: buat yang baru, atau impor frasa pemulihan yang sudah ada untuk memulai.",
    importMerchant: "Impor merchant",
    none: "Belum ada merchant.",
    switch: "beralih",
    newMerchant: "Merchant baru",
    thisMerchant: "merchant ini",
    nameLabel: "Nama merchant",
    namePlaceholder: "mis. Utama",
    introFirst:
      "Atur identitas dagang pertama Anda (sebuah “merchant”). Identitas ini hanya memegang kunci transit panas untuk swap yang berjalan — koin utama Anda tetap di dompet Anda sendiri.",
    introNew: "Merchant baru adalah identitas segar dan terpisah dengan seed dan riwayat swap-nya sendiri.",
    // UI-5 merchant selector (phoenix wallet-selector parity).
    createNew: "Buat baru",
    import: "Impor",
    load: "Muat Merchant",
    loaded: "termuat",
    locked: "terkunci",
    lockedTip: "Seed terenkripsi — buka dengan frasa sandi Anda saat memuatnya.",
    close: "Tutup",
    idLabel: "folder",
    switching: "Beralih merchant…",
    switchingBody: "Menjalankan ulang engine terhadap folder itu.",
  },
  // Seed create/import (SeedForm) + counterparty + status.
  seed: {
    intro: "Buat seed yang benar-benar baru, atau impor yang sudah Anda miliki.",
    createNew: "Buat baru",
    createDesc: "Hasilkan seed baru. Anda mencadangkan frasa pemulihannya.",
    import: "Impor",
    importDesc: "Pulihkan dari frasa 12/24 kata yang sudah ada.",
    recoveryLabel: "Frasa pemulihan",
    encrypt: "Enkripsi",
    encryptDesc:
      "Frasa sandi melindungi seed saat disimpan. Anda memasukkannya sekali per sesi — Satchel tidak pernah menyimpannya. Catatan: auto-refund tanpa pengawasan akan terhenti setelah restart sampai Anda memasukkannya kembali.",
    noPassphrase: "Tanpa frasa sandi (disarankan)",
    noPassphraseDesc:
      "Auto-refund tetap bekerja menembus reboot tanpa perlu memasukkan apa pun — ini hanya seed transit panas. Risikonya: akses file/host akan membuka kunci transit + identitas merchant ini.",
    passphraseLabel: "Frasa sandi",
    passphrasePlaceholder: "pilih frasa sandi",
    revealTitle: "Catat frasa pemulihan Anda",
    revealBody:
      "Siapa pun yang memiliki kata-kata ini mengendalikan kunci panas merchant ini. Satchel tidak menyimpan salinan — simpan secara offline. Berikutnya Anda akan mengonfirmasi beberapa kata.",
    ackLabel: "Saya telah mencatat frasa pemulihan saya.",
    // Multi-step onboarding (create/import -> secret -> confirm -> passphrase).
    chooseTitle: "Atur {label}",
    enterTitle: "Impor frasa pemulihan Anda",
    enterBody:
      "Ketik tiap kata — terisi otomatis sambil Anda mengetik — atau tempel seluruh frasa. Kami memeriksanya sebelum Anda lanjut.",
    wordCount: "{n} kata",
    wordAria: "Kata {n}",
    checkIncomplete: "Masukkan semua {n} kata.",
    checkUnknown: "Beberapa kata tidak ada di daftar kata BIP39 — periksa yang disorot.",
    checkBadChecksum: "Checksum tidak cocok — periksa ulang kata-kata Anda dan urutannya.",
    checkOk: "Frasa pemulihan tampak valid.",
    verifyTitle: "Konfirmasi cadangan Anda",
    verifyBody: "Ketik kata pada posisi-posisi ini untuk mengonfirmasi Anda sudah mencatat frasanya.",
    verifyWord: "Kata #{n}",
    verifyMismatch: "Itu tidak cocok dengan frasa Anda — periksa cadangan Anda.",
    passphraseTitle: "Lindungi seed",
    passphraseBody:
      "Secara opsional enkripsi seed yang tersimpan dengan frasa sandi. Anda bisa melewatinya — lihat trade-off di bawah.",
  },
  counterparty: {
    you: "Ini Anda",
    youShort: "Anda",
    unknown: "identitas tidak dikenal",
    // Short fingerprint fallback (identity.ts shortId) when no pubkey is known.
    unknownShort: "tak dikenal",
  },
  status: {
    notConnectedTitle: "Tidak terhubung ke engine",
    disconnectedBody:
      "Satchel tidak bisa menjangkau engine. Mungkin masih memulai, atau koneksi node merchant aktif sedang mati. Coba lagi, atau beralih merchant dari pemilih di bagian atas.",
    openInSatchel: "Buka ini di Satchel",
    noTauriBody:
      "Ini antarmuka Satchel — ia membutuhkan jembatan Tauri untuk menjangkau engine. Jalankan aplikasi desktop (cargo tauri dev) alih-alih browser.",
  },
  settings: {
    title: "Pengaturan",
    subtitle: "Preferensi seluruh aplikasi untuk instalasi ini.",
    // UI-3 Settings tabs.
    tabGeneral: "Umum",
    tabCoins: "Koin",
    tabNetwork: "Jaringan",
    tabAbout: "Tentang",
    appearance: "Tampilan",
    theme: "Tema",
    themeDark: "Gelap",
    themeLight: "Terang",
    themeSystem: "Sistem",
    themeHint: "Pilih tampilan Satchel. Sistem mengikuti pengaturan OS Anda.",
    language: "Bahasa",
    languageHint: "Bahasa lain akan hadir seiring kontribusi terjemahan.",
    mode: "Mode",
    watchOnly: "Mode hanya-pantau",
    watchOnlyHint:
      "Jelajahi board tanpa mengatur koin. Anda tetap bisa menarik penawaran Anda sendiri, tapi tidak bisa memasang, mengambil, atau mendanai. Matikan untuk berdagang (Anda perlu setidaknya dua koin terhubung).",
    network: "Jaringan",
    boards: "Corkboard",
    boardsDesc:
      "Board HTTP swakelola opsional. Tambahkan yang Anda percaya; biarkan kosong untuk mengandalkan Nostr.",
    boardsNone: "Tidak ada yang dikonfigurasi",
    nostrRelays: "Relay Nostr",
    nostrRelaysDesc:
      "Relay membawa papan pengumuman melalui jaringan terdesentralisasi — tidak ada operator yang bisa membaca atau mencocokkan penawaran Anda. Sudah terisi set bawaan; sunting sesuka Anda.",
    nostrRelaysOff: "Mati — transport Nostr dinonaktifkan",
    addUrl: "Tambah",
    removeUrl: "Hapus",
    relayInvalid: "Masukkan URL relay ws:// atau wss://",
    boardInvalid: "Masukkan URL board http:// atau https://",
    netSave: "Simpan & sambung ulang",
    netSaving: "Menyimpan & menyambung ulang…",
    netSaved: "Tersimpan",
    // Fees tab — fee-bump policy (per active merchant).
    tabFees: "Biaya",
    fees: "Penaikan biaya",
    feesScope: "Pengaturan ini berlaku untuk merchant aktif.",
    feesIntro:
      "Trade-off keamanan/biaya untuk penaikan biaya, bukan pengaturan wajib. Nilai baru berlaku untuk penaikan mendatang; swap yang sudah didanai mempertahankan kebijakan saat didanai.",
    feeMax: "Feerate maksimum (sat/vB)",
    feeMaxHint:
      "Batas atas untuk setiap penaikan biaya. Bawaan 500, juga maksimum sistem mutlak. Turunkan untuk membatasi biaya.",
    feeReservation: "Cadangan penaikan pendanaan (×)",
    feeReservationHint:
      "Saldo yang disisihkan pemeriksaan dana sebagai ruang penaikan. Lebih tinggi menyelamatkan lonjakan biaya yang lebih besar tapi mengikat lebih banyak saldo dan menolak lebih banyak swap. Bawaan 3.",
    feeCommitted: "Kelebihan provisi redeem (×)",
    feeCommittedHint:
      "Seberapa banyak ekstra biaya redeem v2 dibayar di muka agar terkonfirmasi bahkan saat Satchel ditutup. Hanya berlaku untuk swap baru. Bawaan 2.",
    feeSave: "Simpan",
    feeSaving: "Menyimpan…",
    feeSaved: "Tersimpan",
    feeReset: "Setel ulang ke bawaan",
    coins: "Koin & node",
    coinsHint: "Hubungkan tiap koin ke node Anda sendiri. Genesis diperiksa sebelum apa pun disimpan.",
    about: "Tentang",
    version: "Versi {version}",
    updateUpToDate: "Versi terbaru",
    updateCheckPlaceholder: "Pemeriksaan pembaruan hadir di rilis berikutnya.",
    trustModel: "Di mana kunci Anda tersimpan",
    trustModelBody:
      "Rahasia berada di engine, tidak pernah di Satchel. Seed merchant duduk di folder data engine (terenkripsi atau teks biasa — pilihan Anda); Satchel tidak menyimpan seed atau frasa sandi. Seed bersifat panas secara desain (hanya kunci transit) — sapu hasil yang besar ke dompet dingin Anda sendiri.",
  },
  coins: {
    intro:
      "Hubungkan tiap koin ke node Anda sendiri. URL pertama adalah dompet node Anda sendiri — ia mendanai leg swap Anda dan menerima hasilnya. Sebelum apa pun disimpan, Satchel memeriksa blok genesis node sehingga dana tidak akan pernah dikirim ke chain yang salah. Koneksi dibagi di antara semua merchant Anda.",
    networkBadge: "Mengonfigurasi untuk jaringan {network}",
    needMerchant:
      "Hubungkan merchant dulu — pengaturan koin membutuhkan engine yang berjalan. Gunakan pemilih merchant di kanan atas.",
    pairsTitle: "Pasangan dagang",
    pairsHint:
      "Pasangan diturunkan dari kemampuan tiap koin — tidak ada daftar tetap. Sebuah pasangan terbuka begitu kedua koinnya terhubung.",
    noPairs: "Tidak ada pasangan yang tersedia.",
    notSetUp: "Belum diatur",
    connectedTip: "Terhubung · tip {tip}",
    connError: "Kesalahan koneksi",
    setUp: "Atur",
    editConnection: "Sunting koneksi",
    remove: "hapus",
    disconnectTip: "Putuskan koneksi koin ini",
    disconnectTitle: "Putuskan {coin}?",
    disconnectBody: "Swap yang membutuhkannya tidak akan tersedia sampai Anda menyambung kembali.",
    ready: "Siap berdagang",
    connectMissing: "Hubungkan {coins}",
    notBuildable: "Belum bisa dibangun",
    // Swap-protocol chips on a pair (pact-htlc-v1 HTLC vs pact-htlc-v2 adaptor).
    protoPrivate: "Privat (Taproot)",
    protoPrivateTip: "Swap privat (adaptor Taproot/MuSig2) — tampak seperti pembayaran biasa on-chain",
    protoHtlcTip: "Swap HTLC klasik",
    // Coin-setup backend choices (CoinSetup).
    // CoinSetup dialog.
    setupTitle: "Hubungkan {coin}",
    setupIntro:
      "Arahkan Satchel ke node {sym} Anda sendiri. Tidak ada yang disimpan sampai node lolos pemeriksaan blok genesis — dana Anda hanya pernah menyentuh chain {sym} yang asli.",
    confirmationsLabel: "Konfirmasi sebelum final",
    confirmationsHint:
      "Seberapa dalam sebuah pendanaan atau redeem di chain ini harus ada sebelum swap bertindak atasnya — margin keamanan reorg. Lebih tinggi lebih aman tapi lebih lambat; biarkan kosong untuk bawaan yang disarankan ({default}).",
    validateNode: "Validasi node",
    checking: "Memeriksa node…",
    genesisOk: "Genesis cocok — ini chain yang benar",
    genesisDetail: "tinggi tip {tip} · genesis {hash}…",
    genesisBad: "Ditolak — tidak menyimpan",
    errorShort: "kesalahan",
    // Structured connection form (CoinSetup v2).
    rpcHostLabel: "Host RPC",
    rpcPortLabel: "Port RPC",
    authMethodLabel: "Autentikasi",
    authCookie: "File cookie",
    authCookieDesc: "Baca otomatis .cookie node dari direktori datanya (bawaan, tanpa menyimpan kata sandi).",
    authUserPass: "Pengguna / kata sandi",
    authUserPassDesc: "rpcuser / rpcpassword dari konfigurasi node Anda — diperlukan untuk node jarak jauh.",
    rpcUserLabel: "Nama pengguna RPC",
    rpcPasswordLabel: "Kata sandi RPC",
    datadirLabel: "Direktori data node",
    cookiePathNote: "Cookie dibaca dari {path} di bawah direktori ini.",
    walletLabel: "Nama dompet (opsional)",
    walletPlaceholder: "dompet node Anda",
    needPort: "Masukkan port RPC dulu.",
    validateFirst: "Validasi node sebelum menyimpan.",
    savingReconnecting: "Menyimpan & menyambung ulang…",
    connected: "{coin} terhubung",
    // Template picker (a coins.toml coin the engine version doesn't support).
    unsupportedByEngine: "Tidak didukung",
    unsupportedByEngineTip:
      "Koin ini didefinisikan di coins.toml tapi tidak dibangun ke dalam versi engine ini, jadi tidak bisa diperdagangkan.",
  },
  coinWizard: {
    title: "Hubungkan koin Anda",
    intro:
      "Pilih setidaknya dua koin dan arahkan masing-masing ke node Anda sendiri. Swap membutuhkan dua chain, jadi perdagangan terbuka begitu dua node terhubung dan aktif. Anda bisa menambah atau mengubah koin nanti di Pengaturan.",
    progress: "{count} dari {min} koin terhubung",
    continue: "Lanjutkan",
    live: "Aktif",
    nodeDown: "Node mati",
  },
  wallets: {
    intro:
      "Ini adalah dompet dari node Anda sendiri (yang dipakai engine untuk mendanai swap dan menerima hasil) — kunci Anda, mesin Anda. Satchel tidak pernah menyimpan koin Anda.",
    hotSeedNudge:
      "Ini dompet belanja pada seed panas, bukan brankas — sapu saldo yang besar ke dompet dingin/inti Anda sendiri.",
    notConnected: "Tidak terhubung",
    notConnectedBody: "Hubungkan merchant dulu — tampilan dompet membutuhkan engine yang berjalan.",
    noCoins: "Belum ada koin yang diatur",
    noCoinsBody: "Hubungkan koin di Pengaturan → Koin dan dompetnya akan muncul di sini.",
    goToCoins: "Ke Koin",
    watchOnlyTitle: "Tidak ada dompet dalam mode hanya-pantau",
    watchOnlyBody:
      "Ini sesi hanya-pantau tanpa koin terhubung, jadi tidak ada dompet untuk ditampilkan. Matikan hanya-pantau di Pengaturan dan hubungkan koin untuk mendanai swap.",
    walletName: "dompet · {wallet}",
    walletScopedHint: "Setiap RPC untuk koin ini dicakupkan ke dompet node ini.",
    walletDefault: "dompet bawaan (tidak dicakup)",
    walletDefaultHint:
      "Tidak ada dompet yang diatur untuk koin ini, jadi RPC memakai dompet bawaan node. Atur satu di Pengaturan → Koin untuk mencakupkan setiap panggilan ke dompet tertentu.",
    balanceLabel: "Saldo {symbol}",
  },
  corkboard: {
    noBoardTitle: "Tidak ada Corkboard terhubung",
    noBoardBody:
      "Corkboard adalah papan buletin bersama tempat maker menyematkan penawaran. Ia tidak pernah mencocokkan perdagangan atau menyimpan koin — arahkan Satchel ke salah satu yang Anda percaya untuk menjelajah dan memasang.",
    noPairs: "Tidak ada pasangan yang tersedia",
    board: "Corkboard",
    boardSettings: "Konfigurasikan di Pengaturan",
    filterAll: "Semua",
    filterMine: "Milik saya",
    noOffers: "Tidak ada penawaran yang bisa Anda ambil sekarang",
    noOffersBody:
      "Penawaran muncul di sini begitu seorang maker memasang satu untuk pasangan yang sudah Anda atur. Anda juga bisa memasang milik Anda sendiri.",
    hiddenOffers:
      "{count} penawaran lagi untuk pasangan yang belum Anda hubungkan. Atur kedua koin untuk memperdagangkannya:",
    yourOffer: "penawaran Anda",
    offerStaged: "memasang…",
    offerStagedTip:
      "Dipasang dari perangkat ini dan menunggu dikonfirmasi balik dari relay. Sedang beriklan; menjadi aktif begitu sebuah relay menggemakannya.",
    take: "Ambil penawaran",
    legDown: "Salah satu node pasangan ini mati — jalankan (atau cek Pengaturan → Koin) sebelum mengambil.",
    withdraw: "Tarik",
    withdrawTip: "Tarik seketika — penawaran tidak pernah mengunci dana",
    safetyRefund: "refund pengaman",
    safetyRefundTip:
      "Jika swap macet, kedua sisi auto-refund — leg taker terbuka kuncinya lebih dulu, milik Anda sedikit kemudian. Tidak ada yang berakhir terjebak.",
    activeTitle: "Swap aktif Anda",
    states: {
      takenByUs: "diambil oleh Anda",
      revoked: "ditarik",
      expired: "kedaluwarsa",
    },
    // Two-sided order-book view of the Corkboard. Bids = makers giving the
    // quote coin to get the base; asks = the reverse. The ladder is a way to
    // READ the board — it never matches or prioritises (load-bearing).
    book: {
      bids: "Permintaan",
      asks: "Penawaran",
      bidsHint: "ingin {base} · membayar {quote}",
      asksHint: "menjual {base} · seharga {quote}",
      price: "Harga",
      size: "Ukuran",
      noBids: "Tidak ada permintaan",
      noAsks: "Tidak ada penawaran",
      spread: "Spread {pct}",
      spreadOneSided: "Satu sisi",
      crossed: "bersilangan",
      crossedTip: "Permintaan teratas ≥ penawaran teratas. Board tidak pernah mencocokkan otomatis, jadi penawaran yang tumpang tindih ini hanya diam — ambil sisi mana pun.",
      mid: "tengah {price}",
      levelOffers: "{count} penawaran pada harga ini — pilih satu untuk diambil",
      depthTip: "Total {sym} yang ditawarkan pada harga ini di {count} pengumuman.",
      selectLevel: "Pilih level harga di atas untuk melihat penawaran di sana.",
      paneHeader: "{size} {base} @ {price} {unit}",
      denomTip: "Unit tampilan untuk jumlah {coin}",
      showMore: "Tampilkan {count} lagi",
      showLess: "Tampilkan {count} teratas",
    },
  },
  relays: {
    title: "Relay",
    subtitle: "Konektivitas langsung ke relay Nostr Anda — jaringan yang dilalui penawaran dan pengambilan Anda. Tambah atau hapus relay di Pengaturan → Jaringan.",
    connectedCount: "{up} / {total} terhubung",
    refresh: "Segarkan",
    ms: "{ms} ms",
    up: "naik",
    down: "turun",
    statsTip: "{success}/{attempts} koneksi berhasil · ↓{down} ↑{up}",
    none: "Tidak ada relay yang dikonfigurasi",
    noneBody: "Tambahkan relay Nostr di Pengaturan → Jaringan untuk menerbitkan dan menerima penawaran melalui jaringan.",
    goToNetwork: "Ke Pengaturan",
    notConnected: "Tidak terhubung",
    notConnectedBody: "Tampilan relay membutuhkan engine yang berjalan — hubungkan merchant dulu.",
  },
  swaps: {
    title: "Swap",
    hint: "Buku besar lengkap Anda — swap yang berjalan di atas, perdagangan selesai di bawah. Anda juga bisa bertindak pada swap aktif dari Corkboard.",
    activeTitle: "Sedang berjalan",
    historyTitle: "Riwayat",
    none: "Belum ada swap — ambil penawaran di Corkboard.",
    // Active-swaps dock action buttons + the refund-time label.
    cancel: "batalkan",
    refund: "refund",
    dump: "dump log",
    dumpHint: "Salin bundel diagnostik bebas-rahasia (state + baris log) untuk swap ini, untuk ditempel ke pengembang.",
    dumpCopied: "Diagnostik tersalin — tempel ke pengembang.",
    dumpFailed: "Tidak bisa menyalin bundel diagnostik.",
    refundAt: "refund {when}",
    // Confirm dialogs for acting on a live swap (ActiveSwaps).
    cancelTitle: "Batalkan swap ini?",
    cancelConfirm: "Batalkan swap",
    cancelKeep: "Tetap pertahankan",
    // Abort reason recorded on the swap when cancelled from Satchel.
    cancelReason: "dibatalkan di Satchel",
    cancelBody:
      "Ini meninggalkan swap sebelum Anda mendanai. Tidak ada milik Anda yang dikunci, jadi Anda tidak kehilangan apa pun — penawaran hanya tidak akan selesai.",
    refundTitle: "Tarik kembali dana Anda?",
    refundConfirm: "Refund",
    refundBody:
      "Timelock pengaman sudah lewat, jadi Anda bisa mengambil kembali dana yang Anda kunci. Ini menyiarkan refund Anda sekarang; engine juga melakukannya otomatis setelah batas waktu.",
    col: {
      swap: "swap",
      role: "peran",
      state: "state",
      amounts: "beri → terima",
      when: "kapan",
      finalTx: "tx akhir",
    },
    // Expandable per-leg on-chain detail (the audit trail). We show both
    // funding txs + OUR settlement; never the counterparty's settlement or the
    // swap secret.
    audit: {
      toggle: "Tampilkan detail on-chain",
      title: "Detail on-chain",
      youLocked: "Anda mengunci",
      theyLocked: "mereka mengunci",
      funding: "Pendanaan",
      received: "Diterima",
      refunded: "Direfund",
      pending: "belum on-chain",
      copy: "Salin id transaksi",
      copied: "Id transaksi tersalin",
    },
  },
  fees: {
    title: "Pratinjau biaya jaringan",
    estimated: "perkiraan",
    provisionalNote: "Build pactd ini belum menampilkan estimasi biaya.",
    summary: "Sebuah swap adalah 2 transaksi on-chain yang Anda bayar: pendanaan di chain-pemberi, redeem di chain-penerima.",
    fallbackTip: "Sebuah node tidak terjangkau, jadi feerate bawaan konservatif dipakai — anggap ini sebagai perkiraan.",
    ifItStalls: "(jika macet)",
  },
  funds: {
    insufficient:
      "{sym} tidak cukup untuk mendanai swap ini — butuh ~{need} {sym} (jumlah + biaya pendanaan), dompet punya {have} {sym}.",
  },
  wizard: {
    back: "Kembali",
    continue: "Lanjutkan",
  },
  // UI-4 docked activity log.
  log: {
    title: "Aktivitas",
    empty: "— log aktivitas —",
    count: "{count} baris",
    collapse: "Ciutkan log",
    expand: "Bentangkan log",
    // Activity-log lines emitted by the frontend. `{err}` carries a raw engine
    // message (itself not translated — it comes from pactd over the wire).
    noTauri: "tidak berjalan di dalam Satchel — antarmuka ini membutuhkan jembatan Tauri",
    startupError: "startup: {err}",
    notConnected: "tidak terhubung: {err}",
    connected: "terhubung ke pactd {version} ({protocol})",
    listcoinsError: "listcoins: {err}",
    watchOnlyError: "hanya-pantau: {err}",
    switchedMerchant: "beralih ke merchant {id}",
    switchMerchantError: "beralih merchant: {err}",
    loadMerchantError: "muat merchant: {err}",
    merchantCreated: "merchant {id} dibuat",
    merchantReady: "merchant siap",
    actionOk: "{action} {id}: ok",
    actionError: "{action} {id}: {err}",
    diagCopied: "diagnostik untuk {id} tersalin ({count} baris log) — tempel ke pengembang",
    dumpError: "dump {id}: {err}",
    coinDisconnected: "{coin} terputus",
    removeCoinError: "hapus koin: {err}",
    tookOffer: "mengambil penawaran {id} — sekarang muncul di swap aktif Anda di bawah",
    takeError: "ambil: {err}",
    offerWithdrawn: "penawaran {id} ditarik",
    withdrawError: "tarik: {err}",
    postedOffer: "memasang penawaran {id} — tarik kapan saja; tidak ada yang dikunci",
    createdSlip: "membuat slip penawaran privat — kirim ke teman Anda",
    tookPrivateOffer: "mengambil penawaran privat {id} — sekarang muncul di swap aktif Anda",
    cancelledPrivateOffer: "membatalkan penawaran privat {id}",
    cancelError: "batal: {err}",
    noticeboardUpdated: "papan pengumuman diperbarui",
    feePolicyUpdated: "kebijakan biaya diperbarui",
  },
  // Relative-time + freshness prose from format.ts (rendered via the tr() mirror,
  // since those are pure non-component helpers). Unit letters stay inside the
  // template so a translation owns the whole phrase.
  format: {
    ageUnknown: "usia tidak diketahui",
    justNow: "baru saja",
    minutesAgo: "{n}mnt lalu",
    hoursAgo: "{n}j lalu",
    daysAgo: "{n}h lalu",
    expiryNow: "sekarang",
    expirySoon: "segera",
    inMinutes: "dalam ~{n}mnt",
    inHours: "dalam ~{n}j",
    inDays: "dalam ~{n}h",
    posted: "dipasang {age}",
    expires: "kedaluwarsa {time}",
  },
  // Plain-language swap story per (role, state) — the honest "who is exposed
  // when" framing shown on every active swap. {a}/{b} are coin tickers; {t1}/{t2}
  // are local refund times. Rendered via tr() (narrate() is a pure helper).
  narrate: {
    finalizing: "Anda mengklaim {got} Anda — konfirmasi akhir. Biarkan aplikasi terbuka sampai terkubur; {gave} Anda tetap terlindungi sampai saat itu.",
    initiating:
      "Pengambilan terkirim — menunggu maker memulai swap. Belum ada yang dikunci; otomatis dibatalkan jika mereka tidak menanggapi.",
    created: "Penawaran terkirim — menunggu sisi lain menyetujui. Tidak ada yang berkomitmen.",
    acceptedMaker: "Syarat disepakati. Berikutnya: kunci {a} Anda. Sampai Anda mendanai, Anda masih bisa membatalkan dengan bebas.",
    acceptedTaker: "Syarat disepakati. Sisi lain mengunci {a} mereka lebih dulu — Anda tidak pernah mengirim duluan.",
    noncesExchanged:
      "Menyiapkan swap privat — bertukar materi penandatanganan. Belum ada yang dikunci.",
    signedMaker:
      "Kedua sisi menandatangani. Daemon Anda mengunci {a}, lalu mengklaim {b} secara otomatis. Jika ada yang macet, {a} Anda kembali pada {t1}.",
    signedTaker:
      "Kedua sisi menandatangani. Daemon Anda mengunci {b} dan mengklaim {a} begitu sisi lain bergerak. Jaring pengaman: refund pada {t2}.",
    fundedAMaker:
      "{a} Anda terkunci. Menunggu sisi lain mengunci {b} mereka. Jika mereka tidak pernah melakukannya, {a} Anda kembali otomatis pada {t1}.",
    fundedATaker:
      "{a} mereka terkunci dan terverifikasi. Berikutnya: kunci {b} Anda. Jaring pengaman: refund otomatis pada {t2} jika ada yang macet.",
    fundedBMaker: "Keduanya terkunci. Daemon Anda mengklaim {b} begitu terkonfirmasi dengan aman.",
    fundedBTaker: "Keduanya terkunci. Daemon Anda akan mengklaim {a} begitu sisi lain mengambil {b} mereka.",
    completed: "Swap selesai — {coin} ada di dompet Anda.",
    refunded: "Swap tidak selesai, jadi {coin} Anda kembali otomatis. Tidak ada yang hilang selain biaya.",
    aborted: "Dibatalkan sebelum ada uang yang bergerak.",
  },
  progress: {
    awaitingLock: "Menunggu penguncian mereka",
    awaitingClaim: "Menunggu klaim mereka",
    theirLock: "Mengonfirmasi penguncian mereka",
    securing: "Mengamankan {coin} Anda",
    blocks: "+{n} blok",
    feeBumped: "Biaya dinaikkan",
    reorg: "Reorg terdeteksi — memeriksa ulang",
  },
  exit: {
    // Exit-gate dialog (fund safety, C6). The engine manages alone, so "keep
    // running" detaches it (it keeps watching timelocks + servicing offers).
    liveTitle: "Sebuah swap sedang berjalan",
    liveBodyOne:
      "1 swap sedang berjalan. Ia diatur oleh timelock on-chain — engine harus tetap berjalan untuk redeem atau refund sebelum batas waktu.",
    liveBodyMany:
      "{count} swap sedang berjalan. Mereka diatur oleh timelock on-chain — engine harus tetap berjalan untuk redeem atau refund sebelum batas waktu.",
    keepRunningExplain:
      "Menutup jendela membuat engine tetap berjalan di latar belakang, jadi ia menyelesaikan swap tanpa antarmuka. Anda bisa membuka kembali Satchel kapan saja untuk memeriksanya.",
    forceQuitWarn: "Memaksa keluar sekarang menghentikan engine dan bisa kehilangan dana.",
    // {word} is the confirm word below; a translation may localize both together.
    typeToConfirm: "Untuk tetap memaksa keluar, ketik {word} di bawah.",
    confirmWord: "QUIT",
    keepRunning: "Tetap jalan, tutup jendela",
    keepWithdraw: "Tetap jalan + tarik penawaran",
    keepLeaveOffers: "Tetap jalan, biarkan penawaran terpasang",
    forceQuit: "Paksa keluar",
    offersTitle: "Anda memiliki penawaran terpasang",
    offersBodyOne:
      "1 penawaran milik Anda masih ada di Corkboard. Penawaran tidak mengunci apa pun, tapi membiarkannya terpasang berarti lawan transaksi masih bisa mengambilnya saat Satchel ditutup — engine akan melayani pengambilan itu.",
    offersBodyMany:
      "{count} penawaran milik Anda masih ada di Corkboard. Penawaran tidak mengunci apa pun, tapi membiarkannya terpasang berarti lawan transaksi masih bisa mengambilnya saat Satchel ditutup — engine akan melayani pengambilan itu.",
    withdrawExit: "Tarik semua & keluar",
  },
  unlock: {
    title: "Buka kunci merchant",
    body:
      "Seed merchant ini terenkripsi. Masukkan frasa sandinya untuk membukanya pada sesi ini — Satchel menyimpannya hanya di memori dan melupakannya saat keluar.",
    switchMerchant: "Beralih merchant",
    unlock: "Buka kunci",
  },
  common: {
    cancel: "Batal",
    confirm: "Konfirmasi",
    save: "Simpan",
    done: "Selesai",
    later: "Nanti",
    retry: "Coba sambung lagi",
  },
};
