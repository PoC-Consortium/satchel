// The Vietnamese (Tiếng Việt) string bundle. Mirrors en.ts key-for-key; only values are translated.
import type { Bundle } from "./en";

export const vi: Bundle = {
  app: {
    name: "Satchel",
    tagline: "swap phi tín nhiệm",
  },
  // In-app update notifications (sidebar version badge + dialog).
  update: {
    title: "Có bản cập nhật",
    upToDate: "Bạn đang dùng phiên bản mới nhất",
    current: "Đã cài",
    latest: "Mới nhất",
    notesTitle: "Ghi chú phát hành",
    get: "Tải bản cập nhật",
    dismiss: "Bỏ qua",
    close: "Đóng",
    badgeTooltip: "Có bản cập nhật — nhấn để xem chi tiết",
    versionTooltip: "Nhấn để kiểm tra cập nhật",
  },
  // Risk disclaimer (first-run welcome + Settings → About).
  disclaimer: {
    title: "Tự quản lý — khóa của bạn, trách nhiệm của bạn",
    body: "Satchel thực hiện atomic swap phi giám hộ: chỉ mình bạn giữ khóa của mình, còn seed của một merchant giữ các khóa chuyển tiếp nóng trong khi một swap đang diễn ra. Các giao thức swap (v1 HTLC và v2 Taproot/MuSig2) đã được rà soát và đang chạy trên mainnet. Phần mềm theo giấy phép MIT và cung cấp nguyên trạng, không có bảo hành — hãy sao lưu cụm từ khôi phục của bạn và tự chịu rủi ro khi sử dụng.",
  },
  nav: {
    public: "Công khai",
    corkboard: "Corkboard",
    postOffer: "Đăng đề nghị",
    private: "Riêng tư",
    privateCreate: "Tạo phiếu",
    privateReceive: "Nhận phiếu",
    privateSlips: "Phiếu của tôi",
    swaps: "Swap",
    relays: "Relay",
    wallets: "Ví",
    settings: "Cài đặt",
    coins: "Đồng coin",
  },
  makeOffer: {
    title: "Đăng đề nghị",
    intro:
      "Đăng một đề nghị đã ký lên Corkboard. Không có gì bị khóa cả — đây chỉ là một mẩu quảng cáo; rút lại bất cứ lúc nào, và swap chỉ bắt đầu khi có người nhận và cả hai bên cùng nạp tiền.",
    give: "Bạn đưa",
    want: "Bạn nhận",
    // Canonical pair + direction: pick the pair, choose Sell/Buy the base, enter
    // the base amount and a quote-per-base price (invariant to direction).
    pair: "Cặp",
    noPairs: "Không có cặp giao dịch nào — hãy kết nối ít nhất hai đồng coin trong Cài đặt → Đồng coin.",
    sell: "Bán {sym}",
    buy: "Mua {sym}",
    amount: "Số lượng",
    youGive: "Bạn đưa",
    youGet: "Bạn nhận",
    price: "Giá",
    priceUnit: "{unit} cho mỗi {base}",
    pricePlaceholder: "đơn giá",
    balance: "Số dư: {amt} {sym}",
    balanceLoading: "Số dư: …",
    noCoins: "Chưa cấu hình đồng coin nào",
    sameCoin: "Coin đưa và coin nhận phải khác nhau.",
    legDown: "Node của một trong các đồng coin này đang ngừng hoạt động — hãy khởi động nó (hoặc kiểm tra Cài đặt → Đồng coin) trước khi đăng.",
    // Swap-protocol pin (only offered when a pair+network supports more than
    // one). v2 label reuses coins.protoPrivate ("Private (Taproot)").
    protocol: "Loại swap",
    protoStandard: "Tiêu chuẩn (HTLC)",
    // Titles for the review/confirm dialog shown before posting (see OfferForm).
    reviewOfferTitle: "Xem lại đề nghị của bạn",
    reviewSlipTitle: "Xem lại phiếu của bạn",
    term: "Timelock an toàn",
    termShort: "Ngắn",
    termMedium: "Trung bình",
    termLong: "Dài",
    termHint: {
      short: "Ngắn — tiền tự hoàn lại nhanh nhất nếu giao dịch bị đình trệ (~12h / 6h), với biên an toàn nhỏ nhất.",
      medium: "Trung bình — khoảng thời gian hoàn tiền cân bằng (~24h / 12h).",
      long: "Dài (an toàn nhất) — biên an toàn rộng nhất; tự hoàn tiền sau ~36h / 18h nếu giao dịch bị đình trệ.",
    },
    validFor: "Hiệu lực trong (phút)",
    validForMins: "{mins} phút",
    validForHint:
      "Đề nghị được niêm yết trong bao lâu. Khi bạn còn trực tuyến, nó sẽ tự được làm mới; sau khoảng thời gian này nó hết hạn. Đóng ứng dụng sẽ rút nó xuống.",
    note: "Đề nghị kích thước cố định — không có gì bị khóa cho đến khi ai đó nhận nó. Số tiền nằm trên on-chain; bạn trả thêm phí mạng và Corkboard không thu phí gì. Timelock là khoảng thời gian tự hoàn tiền nếu một swap bị đình trệ.",
    post: "Đăng đề nghị",
    makeSlip: "Tạo phiếu",
    slipTitle: "Phiếu đề nghị riêng tư của bạn",
    slipExplainer:
      "Gửi phiếu này cho bạn bè của bạn. Họ dán nó vào Satchel để nhận. Không có gì bị khóa; phiếu hết hạn sau {ttl}.",
    copy: "Sao chép",
    copied: "Đã sao chép",
    makeAnother: "Tạo phiếu khác",
    myPrivateTitle: "Đề nghị riêng tư của tôi",
    myPrivateEmpty: "Không có đề nghị riêng tư nào đang chờ.",
    privateExpires: "hết hạn {when}",
    privateExpired: "đã hết hạn",
    cancel: "Hủy",
    cancelTip: "Ngừng tôn trọng phiếu này — bạn bè còn giữ nó sẽ không thể nhận được nữa.",
  },
  takeSlip: {
    open: "Dán một phiếu",
    title: "Nhận đề nghị riêng tư",
    intro:
      "Một người bạn đã gửi cho bạn một phiếu đề nghị riêng tư (nó bắt đầu bằng pactoffer1:). Dán nó vào đây để xem lại và nhận — y như một đề nghị từ bảng tin.",
    placeholder: "pactoffer1:…",
    take: "Xem lại & nhận",
    invalid: "Đó không giống một phiếu — nó phải bắt đầu bằng pactoffer1:.",
    previewLabel: "Phiếu này đề nghị",
  },
  // PRIVATE nav group (off-market slips) — screen titles + intros. The form,
  // slip output, my-slips list and take flow reuse makeOffer.*/takeSlip.* copy.
  private: {
    createTitle: "Tạo đề nghị riêng tư",
    createIntro:
      "Tạo một đề nghị đã ký và trao cho bạn bè dưới dạng một phiếu qua kênh chat riêng của bạn. Không có gì được niêm yết ở đâu cả — và không có gì bị khóa cho đến khi cả hai cùng nạp tiền.",
    slipsIntro:
      "Các phiếu bạn đã tạo. Bất kỳ ai giữ một phiếu đều có thể nhận nó cho đến khi hết hạn; hủy một phiếu để ngừng tôn trọng nó trước thời điểm đó.",
    slipsEmptyBody: "Tạo một đề nghị riêng tư để có một phiếu mà bạn có thể gửi cho bạn bè.",
    receiveTitle: "Nhận đề nghị riêng tư",
    received: "Đã nhận — theo dõi trong mục Swap.",
  },
  // Shared take-confirmation dialog (board take + slip take).
  takeConfirm: {
    title: "Nhận đề nghị này?",
    confirm: "Nhận đề nghị",
    counterparty: "Đối tác",
    youGive: "Bạn đưa",
    youReceive: "Bạn nhận",
    safetyRefund: "Hoàn tiền an toàn",
    offerAge: "Tuổi của đề nghị",
    makerFundsFirst:
      "Maker khóa {sym} của họ trước — bạn không bao giờ gửi trước. Bạn vẫn có thể hủy trước khi nạp tiền vào phần của mình, và engine tự hoàn tiền sau timelock an toàn nếu swap bị đình trệ.",
  },
  header: {
    activeMerchant: "Merchant đang hoạt động — nhấn để chuyển hoặc quản lý",
    manageMerchants: "Quản lý Merchant…",
    noMerchant: "không có merchant",
    openMenu: "Mở menu",
    collapseMenu: "thu gọn menu",
    settings: "Cài đặt",
    language: "Ngôn ngữ",
    pactConnected: "Engine đã kết nối",
    pactUnreachable: "Không liên lạc được engine",
    liveSwapsOne: "1 swap đang diễn ra — nhấn để xem",
    liveSwapsMany: "{count} swap đang diễn ra — nhấn để xem",
    liveSwapsNone: "Không có swap nào đang diễn ra",
    coinOk: "{name} — đã kết nối · đỉnh {tip}",
    coinUnconfigured: "{name} — chưa thiết lập",
    coinError: "{name} — {status}",
    relaysOk: "Relay Nostr — {up}/{total} đã kết nối",
    relaysDown: "Relay Nostr — không kết nối được cái nào trong {total}",
  },
  network: {
    mainnet: "MainNet",
    testnet: "TestNet",
    regtest: "RegTest",
    signet: "Signet",
    notRealFunds: "Không phải tiền thật — đây là mạng {network}",
  },
  // Watch-only mode: a viewer session with no coins. Browse the board and
  // withdraw your own offers, but no posting / taking / funding.
  watchOnly: {
    badge: "Chỉ xem",
    badgeTip:
      "Chế độ chỉ xem — duyệt bảng tin và rút các đề nghị của riêng bạn, nhưng bạn không thể đăng, nhận hoặc nạp tiền. Hãy thiết lập đồng coin trong Cài đặt để giao dịch.",
    coinWizardButton: "Duyệt ở chế độ chỉ xem",
    coinWizardHint:
      "Bỏ qua thiết lập đồng coin và chỉ duyệt bảng tin (chỉ đọc). Bạn vẫn có thể rút các đề nghị của riêng mình — tiện để gỡ các đề nghị do một phiên khác để lại. Tắt nó bất cứ lúc nào trong Cài đặt.",
    postBlockedTitle: "Chế độ chỉ xem",
    postBlockedBody:
      "Đây là một phiên chỉ xem, nên không thể đăng đề nghị. Hãy thiết lập ít nhất hai đồng coin trong Cài đặt → Đồng coin để giao dịch.",
    takeBlockedBody: "Chế độ chỉ xem — bạn có thể xem lại đề nghị này, nhưng để nhận nó cần phải thiết lập đồng coin.",
    takeBlockedTip: "Chế độ chỉ xem — hãy thiết lập đồng coin trong Cài đặt để nhận đề nghị.",
  },
  merchants: {
    title: "Các merchant của bạn",
    intro:
      "Một merchant là một danh tính giao dịch — với seed và lịch sử swap riêng của nó. Giao dịch dưới một merchant khác giúp các bối cảnh không thể liên kết với nhau (một danh tính dùng một lần). Số coin chính của bạn nằm trong ví của riêng bạn, không phải ở đây.",
    // First-run welcome (empty merchant manager).
    welcomeTitle: "Chào mừng đến với Satchel",
    welcomeIntro:
      "Satchel giao dịch dưới một “merchant” — một danh tính giao dịch với seed riêng của nó. Bạn chưa có cái nào: hãy tạo một cái mới, hoặc nhập một cụm từ khôi phục có sẵn để bắt đầu.",
    importMerchant: "Nhập một merchant",
    none: "Chưa có merchant nào.",
    active: "đang hoạt động",
    switch: "chuyển",
    newMerchant: "Merchant mới",
    thisMerchant: "merchant này",
    nameLabel: "Tên merchant",
    namePlaceholder: "vd. Chính",
    introFirst:
      "Hãy thiết lập danh tính giao dịch đầu tiên của bạn (một “merchant”). Nó chỉ giữ các khóa chuyển tiếp nóng cho các swap đang diễn ra — số coin chính của bạn vẫn ở trong ví của riêng bạn.",
    introNew: "Một merchant mới là một danh tính mới, riêng biệt với seed và lịch sử swap riêng của nó.",
    // UI-5 merchant selector (phoenix wallet-selector parity).
    createNew: "Tạo mới",
    import: "Nhập",
    load: "Tải Merchant",
    loaded: "đã tải",
    locked: "đã khóa",
    lockedTip: "Seed được mã hóa — mở khóa bằng cụm mật khẩu của bạn khi tải nó.",
    close: "Đóng",
    idLabel: "thư mục",
    switching: "Đang chuyển merchant…",
    switchingBody: "Đang khởi động lại engine với thư mục đó.",
  },
  // Seed create/import (SeedForm) + counterparty + status.
  seed: {
    intro: "Tạo một seed hoàn toàn mới, hoặc nhập một seed bạn đã có.",
    createNew: "Tạo mới",
    createDesc: "Tạo một seed mới. Bạn sao lưu cụm từ khôi phục.",
    import: "Nhập",
    importDesc: "Khôi phục từ một cụm từ 12/24 từ có sẵn.",
    recoveryLabel: "Cụm từ khôi phục",
    importPlaceholder: "từ1 từ2 từ3 …",
    encrypt: "Mã hóa",
    encryptDesc:
      "Một cụm mật khẩu bảo vệ seed khi không sử dụng. Bạn nhập nó một lần mỗi phiên — Satchel không bao giờ lưu nó. Lưu ý: việc tự hoàn tiền không cần giám sát sẽ tạm dừng sau khi khởi động lại cho đến khi bạn nhập lại nó.",
    noPassphrase: "Không có cụm mật khẩu (khuyến nghị)",
    noPassphraseDesc:
      "Việc tự hoàn tiền vẫn hoạt động qua các lần khởi động lại mà không cần nhập gì — đây chỉ là một seed chuyển tiếp nóng. Cái giá: ai truy cập được tệp/máy chủ sẽ thấy được khóa chuyển tiếp + danh tính của merchant này.",
    passphraseLabel: "Cụm mật khẩu",
    passphrasePlaceholder: "chọn một cụm mật khẩu",
    createTitle: "Tạo seed",
    importTitle: "Nhập seed",
    secureTitle: "Bảo mật {label}",
    revealTitle: "Ghi lại cụm từ khôi phục của bạn",
    revealBody:
      "Bất kỳ ai có các từ này đều kiểm soát được khóa nóng của merchant này. Satchel không giữ bản sao nào — hãy lưu trữ nó ngoại tuyến. Tiếp theo bạn sẽ xác nhận một vài từ.",
    ackLabel: "Tôi đã ghi lại cụm từ khôi phục của mình.",
    // Multi-step onboarding (create/import -> secret -> confirm -> passphrase).
    chooseTitle: "Thiết lập {label}",
    enterTitle: "Nhập cụm từ khôi phục của bạn",
    enterBody:
      "Gõ từng từ — chúng tự hoàn thành khi bạn gõ — hoặc dán toàn bộ cụm từ. Chúng tôi kiểm tra nó trước khi bạn tiếp tục.",
    wordCount: "{n} từ",
    wordAria: "Từ {n}",
    checkIncomplete: "Hãy nhập đủ {n} từ.",
    checkUnknown: "Một số từ không có trong danh sách từ BIP39 — hãy kiểm tra những từ được tô sáng.",
    checkBadChecksum: "Checksum không khớp — hãy kiểm tra lại các từ và thứ tự của chúng.",
    checkOk: "Cụm từ khôi phục có vẻ hợp lệ.",
    verifyTitle: "Xác nhận bản sao lưu của bạn",
    verifyBody: "Gõ các từ ở những vị trí này để xác nhận rằng bạn đã ghi lại cụm từ.",
    verifyWord: "Từ #{n}",
    verifyMismatch: "Những từ đó không khớp với cụm từ của bạn — hãy kiểm tra bản sao lưu.",
    passphraseTitle: "Bảo vệ seed",
    passphraseBody:
      "Tùy chọn mã hóa seed đã lưu bằng một cụm mật khẩu. Bạn có thể bỏ qua bước này — xem đánh đổi bên dưới.",
  },
  counterparty: {
    you: "Đây là bạn",
    youShort: "bạn",
    unknown: "danh tính không xác định",
    // Short fingerprint fallback (identity.ts shortId) when no pubkey is known.
    unknownShort: "không rõ",
  },
  status: {
    notConnectedTitle: "Chưa kết nối với engine",
    disconnectedBody:
      "Satchel không liên lạc được với engine. Có thể nó vẫn đang khởi động, hoặc các kết nối node của merchant đang hoạt động có thể bị ngừng. Hãy thử lại, hoặc chuyển merchant từ bộ chọn ở trên cùng.",
    openInSatchel: "Mở cái này trong Satchel",
    noTauriBody:
      "Đây là giao diện của Satchel — nó cần cầu nối Tauri để liên lạc với engine. Hãy khởi chạy ứng dụng desktop (cargo tauri dev) thay vì trình duyệt.",
  },
  settings: {
    title: "Cài đặt",
    subtitle: "Tùy chọn toàn ứng dụng cho bản cài đặt này.",
    // UI-3 Settings tabs.
    tabGeneral: "Chung",
    tabCoins: "Đồng coin",
    tabNetwork: "Mạng",
    tabAbout: "Giới thiệu",
    appearance: "Giao diện",
    theme: "Chủ đề",
    themeDark: "Tối",
    themeLight: "Sáng",
    themeSystem: "Hệ thống",
    themeHint: "Chọn cách Satchel hiển thị. Hệ thống sẽ theo cài đặt của hệ điều hành của bạn.",
    language: "Ngôn ngữ",
    languageHint: "Sẽ có thêm ngôn ngữ khi các bản dịch được đóng góp.",
    mode: "Chế độ",
    watchOnly: "Chế độ chỉ xem",
    watchOnlyHint:
      "Duyệt bảng tin mà không cần thiết lập đồng coin. Bạn vẫn có thể rút các đề nghị của riêng mình, nhưng không thể đăng, nhận hoặc nạp tiền. Tắt để giao dịch (bạn sẽ cần kết nối ít nhất hai đồng coin).",
    network: "Mạng",
    boards: "Các Corkboard",
    boardsDesc:
      "Các bảng HTTP tự lưu trữ tùy chọn. Thêm bất kỳ cái nào bạn tin tưởng; để trống để dựa vào Nostr.",
    boardsNone: "Chưa cấu hình cái nào",
    nostrRelays: "Relay Nostr",
    nostrRelaysDesc:
      "Các relay truyền bảng tin qua một mạng phi tập trung — không nhà điều hành nào đọc hay khớp được các đề nghị của bạn. Đã cài sẵn một bộ mặc định; bạn có thể chỉnh sửa thoải mái.",
    nostrRelaysOff: "Tắt — đã vô hiệu hóa truyền tải Nostr",
    addUrl: "Thêm",
    removeUrl: "Xóa",
    relayInvalid: "Hãy nhập một URL relay ws:// hoặc wss://",
    boardInvalid: "Hãy nhập một URL bảng http:// hoặc https://",
    netSave: "Lưu & kết nối lại",
    netSaving: "Đang lưu & kết nối lại…",
    netSaved: "Đã lưu",
    // Fees tab — fee-bump policy (per active merchant).
    tabFees: "Phí",
    fees: "Tăng phí",
    feesScope: "Các cài đặt này áp dụng cho merchant đang hoạt động.",
    feesIntro:
      "Các đánh đổi an toàn/chi phí cho việc tăng phí, không phải thiết lập bắt buộc. Giá trị mới áp dụng cho các lần tăng phí trong tương lai; các swap đã nạp tiền vẫn giữ chính sách lúc chúng được nạp.",
    feeMax: "Feerate tối đa (sat/vB)",
    feeMaxHint:
      "Trần cho mỗi lần tăng phí. Mặc định 500, cũng là mức tối đa cứng của hệ thống. Hạ xuống để giới hạn chi phí.",
    feeReservation: "Dự trữ tăng phí nạp tiền (×)",
    feeReservationHint:
      "Phần số dư mà việc kiểm tra quỹ để dành làm khoảng trống cho việc tăng phí. Cao hơn thì cứu được các cú tăng phí lớn hơn nhưng giữ nhiều số dư hơn và từ chối nhiều swap hơn. Mặc định 3.",
    feeCommitted: "Trả dư cho redeem (×)",
    feeCommittedHint:
      "Phí redeem của v2 được trả trước thêm bao nhiêu để nó được xác nhận ngay cả khi Satchel đã đóng. Chỉ áp dụng cho các swap mới. Mặc định 2.",
    feeStep: "Bước leo thang RBF (%)",
    feeStepHint: "Phí của một giao dịch bị kẹt leo lên quyết liệt đến mức nào mỗi lượt bộ lập lịch. Mặc định 50.",
    feeSave: "Lưu",
    feeSaving: "Đang lưu…",
    feeSaved: "Đã lưu",
    feeReset: "Đặt lại về mặc định",
    coins: "Đồng coin & node",
    coinsHint: "Kết nối mỗi đồng coin với node của riêng bạn. Genesis được kiểm tra trước khi bất kỳ thứ gì được lưu.",
    about: "Giới thiệu",
    version: "Phiên bản {version}",
    updateUpToDate: "Đã cập nhật",
    updateCheckPlaceholder: "Tính năng kiểm tra cập nhật sẽ có ở một bản phát hành sau.",
    trustModel: "Khóa của bạn nằm ở đâu",
    trustModelBody:
      "Các bí mật nằm trong engine, không bao giờ trong Satchel. Seed của merchant nằm trong thư mục dữ liệu của engine (mã hóa hoặc văn bản thuần — tùy bạn); Satchel không lưu seed hay cụm mật khẩu nào. Seed nóng theo thiết kế (chỉ là khóa chuyển tiếp) — hãy quét số lợi nhuận đáng kể về ví lạnh của riêng bạn.",
  },
  coins: {
    intro:
      "Kết nối mỗi đồng coin với node của riêng bạn. URL đầu tiên là ví của chính node bạn — nó nạp tiền cho các nhánh swap và nhận số lợi nhuận. Trước khi bất kỳ thứ gì được lưu, Satchel kiểm tra khối genesis của node để tiền không bao giờ có thể gửi nhầm chain. Các kết nối được dùng chung cho tất cả merchant của bạn.",
    networkBadge: "Đang cấu hình cho mạng {network}",
    needMerchant:
      "Hãy kết nối một merchant trước — thiết lập đồng coin cần engine đang chạy. Dùng bộ chọn merchant ở góc trên bên phải.",
    pairsTitle: "Các cặp giao dịch",
    pairsHint:
      "Các cặp được suy ra từ những gì mỗi đồng coin có thể làm — không có danh sách cố định. Một cặp mở ra khi cả hai đồng coin của nó được kết nối.",
    noPairs: "Không có cặp nào khả dụng.",
    notSetUp: "Chưa thiết lập",
    connectedTip: "Đã kết nối · đỉnh {tip}",
    connError: "Lỗi kết nối",
    setUp: "Thiết lập",
    editConnection: "Sửa kết nối",
    remove: "xóa",
    disconnectTip: "Ngắt kết nối đồng coin này",
    disconnectTitle: "Ngắt kết nối {coin}?",
    disconnectBody: "Các swap cần nó sẽ không khả dụng cho đến khi bạn kết nối lại.",
    ready: "Sẵn sàng giao dịch",
    connectMissing: "Kết nối {coins}",
    notBuildable: "Chưa thể tạo được",
    // Swap-protocol chips on a pair (pact-htlc-v1 HTLC vs pact-htlc-v2 adaptor).
    protoPrivate: "Riêng tư (Taproot)",
    protoPrivateTip: "Swap riêng tư (adaptor Taproot/MuSig2) — trông giống một thanh toán thông thường trên on-chain",
    protoHtlcTip: "Swap HTLC cổ điển",
    // Coin-setup backend choices (CoinSetup).
    backendCoreTitle: "Ví Core RPC",
    backendCoreDesc: "Ví của chính node bạn nạp tiền cho swap và nhận số lợi nhuận.",
    backendHardwareTitle: "Phần cứng",
    backendHardwareDesc: "Ký Ledger / PSBT cho nhánh nạp tiền.",
    backendLater: "để sau",
    // CoinSetup dialog.
    setupTitle: "Kết nối {coin}",
    setupIntro:
      "Trỏ Satchel đến node {sym} của riêng bạn. Không có gì được lưu cho đến khi node vượt qua bài kiểm tra khối genesis — tiền của bạn chỉ luôn chạm vào chain {sym} thật.",
    backendUrlLabel: "URL backend của node",
    backendUrlHint:
      "URL đầu tiên = ví của chính node bạn (nạp tiền cho swap, nhận lợi nhuận). Thêm các máy chủ Electrum (tcp://host:port) sau dấu phẩy để có thêm các góc nhìn chain độc lập.",
    fundingWallet: "Ví nạp tiền",
    confirmationsLabel: "Số xác nhận trước khi chốt",
    confirmationsHint:
      "Một lần nạp tiền hoặc redeem trên chain này phải sâu đến mức nào trước khi một swap hành động dựa trên nó — biên an toàn chống reorg. Cao hơn thì an toàn hơn nhưng chậm hơn; để trống cho giá trị mặc định khuyến nghị ({default}).",
    validateNode: "Xác thực node",
    checking: "Đang kiểm tra node…",
    genesisOk: "Genesis khớp — đây đúng là chain cần dùng",
    genesisDetail: "độ cao đỉnh {tip} · genesis {hash}…",
    genesisBad: "Bị từ chối — không lưu",
    errorShort: "lỗi",
    // Structured connection form (CoinSetup v2).
    rpcHostLabel: "Host RPC",
    rpcPortLabel: "Cổng RPC",
    authMethodLabel: "Xác thực",
    authCookie: "Tệp cookie",
    authCookieDesc: "Tự đọc tệp .cookie của node từ thư mục dữ liệu của nó (mặc định, không lưu mật khẩu).",
    authUserPass: "Người dùng / mật khẩu",
    authUserPassDesc: "rpcuser / rpcpassword từ cấu hình node của bạn — cần cho một node ở xa.",
    rpcUserLabel: "Tên người dùng RPC",
    rpcPasswordLabel: "Mật khẩu RPC",
    datadirLabel: "Thư mục dữ liệu của node",
    cookiePathNote: "Cookie được đọc từ {path} trong thư mục này.",
    walletLabel: "Tên ví (tùy chọn)",
    walletPlaceholder: "ví của node bạn",
    needPort: "Hãy nhập cổng RPC trước.",
    validateFirst: "Hãy xác thực node trước khi lưu.",
    savingReconnecting: "Đang lưu & kết nối lại…",
    connected: "{coin} đã kết nối",
    // Template picker (a coins.toml coin the engine version doesn't support).
    unsupportedByEngine: "Không hỗ trợ",
    unsupportedByEngineTip:
      "Đồng coin này được định nghĩa trong coins.toml nhưng không được tích hợp vào phiên bản engine này, nên không thể giao dịch.",
  },
  coinWizard: {
    title: "Kết nối các đồng coin của bạn",
    intro:
      "Chọn ít nhất hai đồng coin và trỏ mỗi cái đến node của riêng bạn. Một swap cần hai chain, nên việc giao dịch sẽ mở khóa khi hai node được kết nối và hoạt động. Bạn có thể thêm hoặc thay đổi đồng coin sau trong Cài đặt.",
    progress: "{count} trên {min} đồng coin đã kết nối",
    continue: "Tiếp tục",
    live: "Hoạt động",
    nodeDown: "Node ngừng",
  },
  wallets: {
    intro:
      "Đây là ví của chính các node của bạn (những node mà engine dùng để nạp tiền cho swap và nhận lợi nhuận) — khóa của bạn, máy của bạn. Satchel không bao giờ giữ coin của bạn.",
    hotSeedNudge:
      "Đây là một ví chi tiêu trên một seed nóng, không phải két sắt — hãy quét số dư đáng kể về ví lạnh/core của riêng bạn.",
    notConnected: "Chưa kết nối",
    notConnectedBody: "Hãy kết nối một merchant trước — giao diện ví cần engine đang chạy.",
    noCoins: "Chưa thiết lập đồng coin nào",
    noCoinsBody: "Kết nối một đồng coin trong Cài đặt → Đồng coin và ví của nó sẽ xuất hiện ở đây.",
    goToCoins: "Đến mục Đồng coin",
    watchOnlyTitle: "Không có ví nào ở chế độ chỉ xem",
    watchOnlyBody:
      "Đây là một phiên chỉ xem không có đồng coin nào được kết nối, nên không có ví nào để hiển thị. Hãy tắt chế độ chỉ xem trong Cài đặt và kết nối một đồng coin để nạp tiền cho swap.",
    walletName: "ví · {wallet}",
    walletScopedHint: "Mọi RPC cho đồng coin này đều được giới hạn vào ví node này.",
    walletDefault: "ví mặc định (không giới hạn)",
    walletDefaultHint:
      "Chưa đặt ví cho đồng coin này, nên các RPC dùng ví mặc định của node. Hãy đặt một ví trong Cài đặt → Đồng coin để giới hạn mọi lệnh gọi vào một ví cụ thể.",
    balanceLabel: "Số dư {symbol}",
    receive: "Nhận",
    send: "Gửi",
    sendTo: "Gửi đến địa chỉ",
    amount: "Số lượng",
    sendTitle: "Gửi {amount} {sym}?",
    sendConfirmBody: "Đến {to}\n\nLệnh này chi từ ví của chính node bạn và không thể hoàn tác.",
  },
  corkboard: {
    noBoardTitle: "Chưa kết nối Corkboard nào",
    noBoardBody:
      "Một Corkboard là một bảng tin dùng chung nơi các maker ghim đề nghị. Nó không bao giờ khớp giao dịch hay giữ coin — hãy trỏ Satchel đến một cái bạn tin tưởng để duyệt và đăng.",
    noPairs: "Không có cặp nào khả dụng",
    board: "Corkboard",
    boardSettings: "Cấu hình trong Cài đặt",
    filterAll: "Tất cả",
    filterMine: "Của tôi",
    offered: "{symbol} được đề nghị",
    noOffers: "Hiện không có đề nghị nào bạn có thể nhận",
    noOffersBody:
      "Các đề nghị xuất hiện ở đây ngay khi một maker đăng một đề nghị cho cặp mà bạn đã thiết lập. Bạn cũng có thể đăng đề nghị của riêng mình.",
    hiddenOffers:
      "Thêm {count} đề nghị cho các cặp bạn chưa kết nối. Hãy thiết lập cả hai đồng coin để giao dịch chúng:",
    yourOffer: "đề nghị của bạn",
    offerStaged: "đang đăng…",
    offerStagedTip:
      "Đã đăng từ thiết bị này và đang chờ được xác nhận lại từ một relay. Nó đang quảng cáo; nó sẽ hoạt động khi một relay phản hồi lại nó.",
    take: "Nhận đề nghị",
    legDown: "Node của một trong các đồng coin của cặp này đang ngừng — hãy khởi động nó (hoặc kiểm tra Cài đặt → Đồng coin) trước khi nhận.",
    withdraw: "Rút",
    withdrawTip: "Rút ngay lập tức — một đề nghị không bao giờ khóa tiền",
    safetyRefund: "hoàn tiền an toàn",
    safetyRefundTip:
      "Nếu swap bị đình trệ, cả hai bên đều tự hoàn tiền — nhánh của taker mở khóa trước, của bạn muộn hơn một chút. Không ai bị kẹt cả.",
    activeTitle: "Các swap đang hoạt động của bạn",
    states: {
      open: "mở",
      takenByUs: "bạn đã nhận",
      revoked: "đã rút",
      expired: "đã hết hạn",
    },
    // Two-sided order-book view of the Corkboard. Bids = makers giving the
    // quote coin to get the base; asks = the reverse. The ladder is a way to
    // READ the board — it never matches or prioritises (load-bearing).
    book: {
      bids: "Lệnh mua",
      asks: "Lệnh bán",
      bidsHint: "muốn {base} · trả bằng {quote}",
      asksHint: "bán {base} · lấy {quote}",
      price: "Giá",
      size: "Khối lượng",
      noBids: "Không có lệnh mua",
      noAsks: "Không có lệnh bán",
      spread: "Chênh lệch {pct}",
      spreadOneSided: "Một chiều",
      crossed: "giao nhau",
      crossedTip: "Lệnh mua cao nhất ≥ lệnh bán thấp nhất. Bảng không bao giờ tự khớp, nên các đề nghị chồng lấn này chỉ nằm đó — hãy nhận một trong hai bên.",
      mid: "trung bình {price}",
      levelOffers: "{count} đề nghị ở mức giá này — chọn một để nhận",
      depthTip: "Tổng {sym} đang được đề nghị ở mức giá này trên {count} mẩu tin.",
      takerNote: "Nhận nó, bạn đưa {give} và nhận {get}.",
      selectLevel: "Chọn một mức giá ở trên để xem các đề nghị ở đó.",
      paneHeader: "{size} {base} @ {price} {unit}",
      denomTip: "Đơn vị hiển thị cho số lượng {coin}",
      showMore: "Hiển thị thêm {count}",
      showLess: "Hiển thị {count} hàng đầu",
    },
  },
  relays: {
    title: "Relay",
    subtitle: "Kết nối trực tiếp đến các relay Nostr của bạn — mạng mà các đề nghị và lệnh nhận của bạn di chuyển qua. Thêm hoặc xóa relay trong Cài đặt → Mạng.",
    connectedCount: "{up} / {total} đã kết nối",
    refresh: "Làm mới",
    ms: "{ms} ms",
    up: "lên",
    down: "xuống",
    statsTip: "{success}/{attempts} lần kết nối thành công · ↓{down} ↑{up}",
    none: "Chưa cấu hình relay nào",
    noneBody: "Thêm một relay Nostr trong Cài đặt → Mạng để đăng và nhận các đề nghị qua mạng.",
    goToNetwork: "Đến Cài đặt",
    notConnected: "Chưa kết nối",
    notConnectedBody: "Giao diện relay cần engine đang chạy — hãy kết nối một merchant trước.",
  },
  swaps: {
    title: "Swap",
    hint: "Sổ cái đầy đủ của bạn — các swap đang diễn ra ở trên cùng, các giao dịch đã hoàn tất ở dưới. Bạn cũng có thể hành động trên các swap đang chạy từ Corkboard.",
    activeTitle: "Đang diễn ra",
    historyTitle: "Lịch sử",
    none: "Chưa có swap nào — hãy nhận một đề nghị trên Corkboard.",
    // Active-swaps dock action buttons + the refund-time label.
    cancel: "hủy",
    refund: "hoàn tiền",
    dump: "xuất log",
    dumpHint: "Sao chép một gói chẩn đoán không chứa bí mật (trạng thái + các dòng log) cho swap này, để dán cho các nhà phát triển.",
    dumpCopied: "Đã sao chép chẩn đoán — dán cho các nhà phát triển.",
    dumpFailed: "Không sao chép được gói chẩn đoán.",
    refundAt: "hoàn tiền {when}",
    // Confirm dialogs for acting on a live swap (ActiveSwaps).
    cancelTitle: "Hủy swap này?",
    cancelConfirm: "Hủy swap",
    cancelKeep: "Giữ lại",
    // Abort reason recorded on the swap when cancelled from Satchel.
    cancelReason: "đã hủy trong Satchel",
    cancelBody:
      "Việc này từ bỏ swap trước khi bạn nạp tiền. Chưa có gì của bạn bị khóa, nên bạn không mất gì — chỉ là đề nghị sẽ không hoàn tất.",
    refundTitle: "Rút tiền của bạn về?",
    refundConfirm: "Hoàn tiền",
    refundBody:
      "Timelock an toàn đã qua, nên bạn có thể đòi lại số tiền bạn đã khóa. Việc này phát đi lệnh hoàn tiền của bạn ngay bây giờ; engine cũng tự làm việc đó sau hạn chót.",
    col: {
      swap: "swap",
      role: "vai trò",
      state: "trạng thái",
      amounts: "đưa → nhận",
      when: "khi nào",
      finalTx: "tx cuối",
    },
    // Expandable per-leg on-chain detail (the audit trail). We show both
    // funding txs + OUR settlement; never the counterparty's settlement or the
    // swap secret.
    audit: {
      toggle: "Hiện chi tiết on-chain",
      title: "Chi tiết on-chain",
      youLocked: "bạn đã khóa",
      theyLocked: "họ đã khóa",
      funding: "Nạp tiền",
      received: "Đã nhận",
      refunded: "Đã hoàn tiền",
      pending: "chưa có trên on-chain",
      copy: "Sao chép ID giao dịch",
      copied: "Đã sao chép ID giao dịch",
    },
  },
  fees: {
    title: "Xem trước chi phí mạng",
    estimated: "ước tính",
    provisionalNote: "Bản dựng pactd này chưa cung cấp ước tính phí.",
    summary: "Một swap là 2 giao dịch on-chain mà bạn trả phí: nạp tiền trên chain bạn đưa, redeem trên chain bạn nhận.",
    fallbackTip: "Một node không liên lạc được, nên đã dùng một feerate mặc định thận trọng — hãy coi đây chỉ là ước đoán.",
    ifItStalls: "(nếu nó bị đình trệ)",
  },
  funds: {
    insufficient:
      "Không đủ {sym} để nạp cho swap này — cần ~{need} {sym} (số tiền + phí nạp), ví có {have} {sym}.",
  },
  wizard: {
    welcome: "Chào mừng đến với Satchel",
    connectTitle: "Kết nối engine Pact",
    connectIntro:
      "Satchel là một client gọn nhẹ của engine Pact — phần lõi giữ khóa của bạn và chạy các swap. Hãy chọn cách liên lạc với nó.",
    managed: "Chạy engine Pact tích hợp sẵn",
    managedDesc: "Satchel khởi chạy và giám sát engine Pact của riêng nó. Khuyến nghị.",
    external: "Kết nối tới một engine Pact bên ngoài",
    externalDesc: "Trỏ đến một engine Pact bạn đã chạy sẵn (đặt SATCHEL_PACTD_URL + cookie trước khi khởi chạy).",
    externalNote:
      "Chế độ bên ngoài được chọn qua các biến môi trường trước khi khởi chạy Satchel. Hãy khởi chạy lại với SATCHEL_PACTD_URL đã đặt để dùng nó.",
    coinsTitle: "Thêm các đồng coin của bạn",
    coinsIntro:
      "Sau khi merchant của bạn được tạo, hãy kết nối mỗi đồng coin với node của riêng bạn trong Cài đặt → Đồng coin. Chọn một đồng coin và một backend (Electrum công cộng để khỏi thiết lập, hoặc node của riêng bạn); genesis được kiểm tra với mạng này trước khi bất kỳ thứ gì được lưu.",
    coinsTemplatesSoon: "Các mẫu đồng coin một-nhấp sẽ có ở đây trong một bản phát hành sau.",
    back: "Quay lại",
    continue: "Tiếp tục",
    finish: "Hoàn tất thiết lập",
  },
  // UI-4 docked activity log.
  log: {
    title: "Hoạt động",
    empty: "— nhật ký hoạt động —",
    count: "{count} dòng",
    collapse: "Thu gọn log",
    expand: "Mở rộng log",
    // Activity-log lines emitted by the frontend. `{err}` carries a raw engine
    // message (itself not translated — it comes from pactd over the wire).
    noTauri: "không chạy bên trong Satchel — giao diện này cần cầu nối Tauri",
    startupError: "khởi động: {err}",
    notConnected: "chưa kết nối: {err}",
    connected: "đã kết nối tới pactd {version} ({protocol})",
    listcoinsError: "listcoins: {err}",
    watchOnlyError: "chỉ xem: {err}",
    switchedMerchant: "đã chuyển sang merchant {id}",
    switchMerchantError: "chuyển merchant: {err}",
    loadMerchantError: "tải merchant: {err}",
    merchantCreated: "đã tạo merchant {id}",
    merchantReady: "merchant đã sẵn sàng",
    actionOk: "{action} {id}: ok",
    actionError: "{action} {id}: {err}",
    diagCopied: "đã sao chép chẩn đoán cho {id} ({count} dòng log) — dán cho các nhà phát triển",
    dumpError: "dump {id}: {err}",
    coinDisconnected: "{coin} đã ngắt kết nối",
    removeCoinError: "xóa coin: {err}",
    tookOffer: "đã nhận đề nghị {id} — giờ nó xuất hiện trong các swap đang hoạt động của bạn bên dưới",
    takeError: "nhận: {err}",
    offerWithdrawn: "đã rút đề nghị {id}",
    withdrawError: "rút: {err}",
    postedOffer: "đã đăng đề nghị {id} — rút lại bất cứ lúc nào; không có gì bị khóa",
    createdSlip: "đã tạo một phiếu đề nghị riêng tư — gửi nó cho bạn bè của bạn",
    tookPrivateOffer: "đã nhận đề nghị riêng tư {id} — giờ nó xuất hiện trong các swap đang hoạt động của bạn",
    cancelledPrivateOffer: "đã hủy đề nghị riêng tư {id}",
    cancelError: "hủy: {err}",
    noticeboardUpdated: "đã cập nhật bảng tin",
    feePolicyUpdated: "đã cập nhật chính sách phí",
  },
  // Relative-time + freshness prose from format.ts (rendered via the tr() mirror,
  // since those are pure non-component helpers). Unit letters stay inside the
  // template so a translation owns the whole phrase.
  format: {
    ageUnknown: "không rõ thời gian",
    justNow: "vừa xong",
    minutesAgo: "{n} phút trước",
    hoursAgo: "{n} giờ trước",
    daysAgo: "{n} ngày trước",
    expiryNow: "ngay bây giờ",
    expirySoon: "sắp tới",
    inMinutes: "trong ~{n} phút",
    inHours: "trong ~{n} giờ",
    inDays: "trong ~{n} ngày",
    posted: "đăng {age}",
    expires: "hết hạn {time}",
  },
  // Plain-language swap story per (role, state) — the honest "who is exposed
  // when" framing shown on every active swap. {a}/{b} are coin tickers; {t1}/{t2}
  // are local refund times. Rendered via tr() (narrate() is a pure helper).
  narrate: {
    initiating:
      "Đã gửi lệnh nhận — đang chờ maker bắt đầu swap. Chưa có gì bị khóa; nó tự hủy nếu họ không phản hồi.",
    created: "Đã gửi đề nghị — đang chờ phía bên kia đồng ý. Chưa cam kết gì.",
    acceptedMaker: "Đã thống nhất các điều khoản. Tiếp theo: khóa {a} của bạn. Cho đến khi bạn nạp tiền, bạn vẫn có thể hủy thoải mái.",
    acceptedTaker: "Đã thống nhất các điều khoản. Phía bên kia khóa {a} của họ trước — bạn không bao giờ gửi trước.",
    noncesExchanged:
      "Đang thiết lập swap riêng tư — trao đổi vật liệu ký. Chưa có gì bị khóa.",
    signedMaker:
      "Cả hai bên đã ký. Daemon của bạn khóa {a}, rồi tự động đòi {b}. Nếu có gì đình trệ, {a} của bạn sẽ trở về lúc {t1}.",
    signedTaker:
      "Cả hai bên đã ký. Daemon của bạn khóa {b} và đòi {a} ngay khi phía bên kia ra tay. Lưới an toàn: hoàn tiền lúc {t2}.",
    fundedAMaker:
      "{a} của bạn đã bị khóa. Đang chờ phía bên kia khóa {b} của họ. Nếu họ không bao giờ làm, {a} của bạn sẽ tự trở về lúc {t1}.",
    fundedATaker:
      "{a} của họ đã bị khóa và được xác minh. Tiếp theo: khóa {b} của bạn. Lưới an toàn: tự hoàn tiền lúc {t2} nếu có gì đình trệ.",
    fundedBMaker: "Cả hai đã khóa. Daemon của bạn đòi {b} ngay khi nó được xác nhận an toàn.",
    fundedBTaker: "Cả hai đã khóa. Daemon của bạn sẽ đòi {a} ngay khi phía bên kia lấy {b} của họ.",
    redeemedB:
      "Bạn đã đòi {b} — đang chờ nó được xác nhận. {a} đã khóa của bạn vẫn được bảo vệ cho đến khi việc này hoàn tất.",
    completed: "Swap hoàn tất — {coin} đã nằm trong ví của bạn.",
    refunded: "Swap không hoàn tất, nên {coin} của bạn đã tự trở về. Không mất gì ngoài phí.",
    aborted: "Đã hủy trước khi có bất kỳ khoản tiền nào di chuyển.",
  },
  exit: {
    // Exit-gate dialog (fund safety, C6). The engine manages alone, so "keep
    // running" detaches it (it keeps watching timelocks + servicing offers).
    liveTitle: "Có một swap đang diễn ra",
    liveBodyOne:
      "1 swap đang diễn ra giữa chừng. Nó được điều phối bởi các timelock on-chain — engine phải tiếp tục chạy để redeem hoặc hoàn tiền trước hạn chót.",
    liveBodyMany:
      "{count} swap đang diễn ra giữa chừng. Chúng được điều phối bởi các timelock on-chain — engine phải tiếp tục chạy để redeem hoặc hoàn tiền trước hạn chót.",
    keepRunningExplain:
      "Đóng cửa sổ vẫn giữ engine chạy ở chế độ nền, nên nó hoàn tất swap mà không cần giao diện. Bạn có thể mở lại Satchel bất cứ lúc nào để kiểm tra.",
    forceQuitWarn: "Buộc thoát ngay bây giờ sẽ dừng engine và có thể làm mất tiền.",
    // {word} is the confirm word below; a translation may localize both together.
    typeToConfirm: "Để vẫn buộc thoát, hãy gõ {word} bên dưới.",
    confirmWord: "QUIT",
    keepRunning: "Tiếp tục chạy, đóng cửa sổ",
    keepWithdraw: "Tiếp tục chạy + rút các đề nghị",
    keepLeaveOffers: "Tiếp tục chạy, để các đề nghị nguyên đó",
    forceQuit: "Buộc thoát",
    offersTitle: "Bạn có các đề nghị đã đăng",
    offersBodyOne:
      "1 đề nghị của bạn vẫn còn trên Corkboard. Đề nghị không khóa gì cả, nhưng để nó nguyên đó nghĩa là các đối tác vẫn có thể nhận nó khi Satchel đã đóng — engine sẽ phục vụ lệnh nhận.",
    offersBodyMany:
      "{count} đề nghị của bạn vẫn còn trên Corkboard. Đề nghị không khóa gì cả, nhưng để chúng nguyên đó nghĩa là các đối tác vẫn có thể nhận chúng khi Satchel đã đóng — engine sẽ phục vụ các lệnh nhận.",
    withdrawExit: "Rút tất cả & thoát",
  },
  unlock: {
    title: "Mở khóa merchant",
    body:
      "Seed của merchant này được mã hóa. Hãy nhập cụm mật khẩu của nó để mở khóa cho phiên này — Satchel chỉ giữ nó trong bộ nhớ và quên nó khi thoát.",
    switchMerchant: "Chuyển merchant",
    unlock: "Mở khóa",
  },
  common: {
    cancel: "Hủy",
    confirm: "Xác nhận",
    save: "Lưu",
    done: "Xong",
    later: "Để sau",
    retry: "Thử kết nối lại",
  },
};
