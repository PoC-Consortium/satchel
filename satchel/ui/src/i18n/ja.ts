// The Japanese (日本語) string bundle. Mirrors en.ts key-for-key; only values are translated.
import type { Bundle } from "./en";

export const ja: Bundle = {
  app: {
    name: "Satchel",
  },
  // In-app update notifications (sidebar version badge + dialog).
  update: {
    title: "アップデートがあります",
    upToDate: "最新の状態です",
    current: "インストール済み",
    latest: "最新版",
    notesTitle: "リリースノート",
    get: "アップデートを取得",
    dismiss: "閉じる",
    close: "閉じる",
    badgeTooltip: "アップデートがあります — クリックで詳細を表示",
    versionTooltip: "クリックしてアップデートを確認",
  },
  // Risk disclaimer (first-run welcome + Settings → About).
  disclaimer: {
    title: "セルフカストディ — 鍵はあなたのもの、責任もあなたに",
    body: "Satchel はノンカストディアルなアトミックスワップを実行します。鍵を保持するのはあなただけで、スワップの実行中はマーチャントのシードがホットな中継鍵を保持します。スワップのプロトコル（v1 HTLC と v2 Taproot/MuSig2）はレビュー済みで、メインネットで稼働しています。MIT ライセンスのもと現状有姿で提供され、いかなる保証もありません — リカバリーフレーズをバックアップし、自己責任でご利用ください。",
  },
  nav: {
    public: "公開",
    corkboard: "Corkboard",
    postOffer: "オファーを投稿",
    private: "プライベート",
    privateCreate: "スリップを作成",
    privateReceive: "スリップを受け取る",
    privateSlips: "マイスリップ",
    swaps: "スワップ",
    relays: "リレー",
    wallets: "ウォレット",
    contacts: "Contacts",
    settings: "設定",
    coins: "コイン",
  },
  makeOffer: {
    title: "オファーを投稿",
    intro:
      "署名済みのオファーを Corkboard に投稿します。何もロックされません — これは単なる広告です。いつでも取り下げられ、誰かが応じて双方が資金を入れたときに初めてスワップが始まります。",
    give: "渡す",
    want: "受け取る",
    // Canonical pair + direction: pick the pair, choose Sell/Buy the base, enter
    // the base amount and a quote-per-base price (invariant to direction).
    pair: "ペア",
    noPairs: "取引可能なペアがありません — 設定 → コインで少なくとも 2 つのコインを接続してください。",
    sell: "{sym} を売る",
    buy: "{sym} を買う",
    amount: "数量",
    youGive: "渡す",
    youGet: "受け取る",
    price: "価格",
    priceUnit: "{base} あたり {unit}",
    pricePlaceholder: "単価",
    balance: "残高: {amt} {sym}",
    balanceLoading: "残高: …",
    noCoins: "コインが未設定です",
    legDown: "これらのコインのいずれかのノードが停止しています — 投稿前に起動してください（または設定 → コインを確認）。",
    // Swap-protocol pin (only offered when a pair+network supports more than
    // one). v2 label reuses coins.protoPrivate ("Private (Taproot)").
    protocol: "スワップの種類",
    protoStandard: "標準 (HTLC)",
    // Titles for the review/confirm dialog shown before posting (see OfferForm).
    reviewOfferTitle: "オファーを確認",
    reviewSlipTitle: "スリップを確認",
    term: "安全タイムロック",
    termShort: "短い",
    termMedium: "中程度",
    termLong: "長い",
    termHint: {
      short: "短い — 取引が滞った場合に最も速く自動返金されますが（約 12 時間 / 6 時間）、安全マージンは最も小さくなります。",
      medium: "中程度 — バランスの取れた返金期間（約 24 時間 / 12 時間）。",
      long: "長い（最も安全）— 安全マージンが最も広く、取引が滞った場合は約 36 時間 / 18 時間後に自動返金されます。",
    },
    validFor: "有効期間（分）",
    validForMins: "{mins} 分",
    validForHint:
      "オファーが掲載され続ける時間です。オンラインの間は自動的に更新され、これを過ぎると期限切れになります。アプリを閉じると取り下げられます。",
    note: "固定サイズのオファー — 誰かが応じるまで何もロックされません。数量はオンチェーンで、ネットワーク手数料は別途自己負担、Corkboard は一切課金しません。タイムロックはスワップが滞った場合の自動返金期間です。",
    post: "オファーを投稿",
    makeSlip: "スリップを作成",
    slipTitle: "あなたのプライベートオファースリップ",
    slipExplainer:
      "これを友人に送ってください。友人が Satchel に貼り付けて応じます。何もロックされず、{ttl} で期限切れになります。",
    copy: "コピー",
    copied: "コピーしました",
    makeAnother: "もう一つ作成",
    myPrivateTitle: "マイプライベートオファー",
    myPrivateEmpty: "未処理のプライベートオファーはありません。",
    privateExpires: "{when} に期限切れ",
    privateExpired: "期限切れ",
    cancel: "キャンセル",
    cancelTip: "このスリップの履行を停止します — まだ保持している友人もこれを取れなくなります。",
  },
  takeSlip: {
    intro:
      "友人がプライベートオファースリップ（pactoffer1: で始まります）を送ってきました。ここに貼り付けて、ボードのオファーとまったく同じように確認して応じてください。",
    placeholder: "pactoffer1:…",
    take: "確認して取る",
    invalid: "これはスリップではないようです — pactoffer1: で始まる必要があります。",
    previewLabel: "このスリップの内容",
  },
  // PRIVATE nav group (off-market slips) — screen titles + intros. The form,
  // slip output, my-slips list and take flow reuse makeOffer.*/takeSlip.* copy.
  private: {
    createTitle: "プライベートオファーを作成",
    createIntro:
      "署名済みのオファーを作成し、ご自身のチャットでスリップとして友人に渡します。どこにも掲載されず、双方が資金を入れるまで何もロックされません。",
    slipsIntro:
      "作成したスリップです。スリップを保持している人は期限切れまで誰でも取れます。それ以前に履行を止めるにはキャンセルしてください。",
    slipsEmptyBody: "プライベートオファーを作成すると、友人に送れるスリップが手に入ります。",
    receiveTitle: "プライベートオファーを取る",
    received: "取得しました — スワップで追跡できます。",
  },
  // Shared take-confirmation dialog (board take + slip take).
  takeConfirm: {
    title: "このオファーを取りますか？",
    confirm: "オファーを取る",
    counterparty: "取引相手",
    youGive: "渡す",
    youReceive: "受け取る",
    safetyRefund: "安全返金",
    offerAge: "オファーの経過時間",
    makerFundsFirst:
      "メーカーが先に {sym} をロックします — あなたが先に送ることは決してありません。自分の側に資金を入れる前ならまだキャンセルでき、スワップが滞った場合は安全タイムロック後にエンジンが自動返金します。",
  },
  header: {
    activeMerchant: "アクティブなマーチャント — クリックで切り替えまたは管理",
    manageMerchants: "マーチャントを管理…",
    noMerchant: "マーチャントなし",
    openMenu: "メニューを開く",
    collapseMenu: "メニューを折りたたむ",
    settings: "設定",
    language: "言語",
    pactConnected: "エンジン接続済み",
    pactUnreachable: "エンジンに到達できません",
    liveSwapsOne: "1 件のスワップが進行中 — クリックで表示",
    liveSwapsMany: "{count} 件のスワップが進行中 — クリックで表示",
    liveSwapsNone: "進行中のスワップはありません",
    coinOk: "{name} — 接続済み · ティップ {tip}",
    coinUnconfigured: "{name} — 未設定",
    coinError: "{name} — {status}",
    relaysOk: "Nostr リレー — {up}/{total} 接続済み",
    relaysDown: "Nostr リレー — {total} 件中いずれも未接続",
  },
  network: {
    mainnet: "MainNet",
    testnet: "TestNet",
    regtest: "RegTest",
    signet: "Signet",
    notRealFunds: "実際の資金ではありません — これは {network} ネットワークです",
  },
  // Watch-only mode: a viewer session with no coins. Browse the board and
  // withdraw your own offers, but no posting / taking / funding.
  watchOnly: {
    badge: "閲覧のみ",
    badgeTip:
      "閲覧のみモード — ボードを閲覧して自分のオファーを取り下げられますが、投稿・取得・資金の投入はできません。取引するには設定でコインを設定してください。",
    coinWizardButton: "閲覧のみモードで閲覧する",
    coinWizardHint:
      "コインの設定をスキップして、ボードを閲覧（読み取り専用）するだけです。自分のオファーは取り下げられるので、別のセッションが残したオファーを引き上げるのに便利です。設定でいつでもオフにできます。",
    postBlockedTitle: "閲覧のみモード",
    postBlockedBody:
      "これは閲覧のみのセッションのため、オファーを投稿できません。取引するには設定 → コインで少なくとも 2 つのコインを設定してください。",
    takeBlockedBody: "閲覧のみモード — このオファーは確認できますが、取得にはコインの設定が必要です。",
    takeBlockedTip: "閲覧のみモード — オファーを取得するには設定でコインを設定してください。",
  },
  merchants: {
    title: "マーチャント一覧",
    intro:
      "マーチャントは一つの取引アイデンティティです — 独自のシードとスワップ履歴を持ちます。別のマーチャントで取引すれば、コンテキストを紐付けられなくできます（使い捨てのアイデンティティ）。メインのコインはここではなく、ご自身のウォレットに保管されます。",
    // First-run welcome (empty merchant manager).
    welcomeTitle: "Satchel へようこそ",
    welcomeIntro:
      "Satchel は「マーチャント」のもとで取引します — 独自のシードを持つ一つの取引アイデンティティです。まだ一つもありません。新しく作成するか、既存のリカバリーフレーズをインポートして始めましょう。",
    importMerchant: "マーチャントをインポート",
    none: "マーチャントがまだありません。",
    switch: "切り替え",
    newMerchant: "新しいマーチャント",
    thisMerchant: "このマーチャント",
    nameLabel: "マーチャント名",
    namePlaceholder: "例: メイン",
    rename: "名前を変更",
    introFirst:
      "最初の取引アイデンティティ（「マーチャント」）を設定します。進行中のスワップ用のホットな中継鍵のみを保持します — メインのコインはご自身のウォレットに残ります。",
    introNew: "新しいマーチャントは、独自のシードとスワップ履歴を持つ、まったく別個のアイデンティティです。",
    // UI-5 merchant selector (phoenix wallet-selector parity).
    createNew: "新規作成",
    import: "インポート",
    load: "マーチャントを読み込む",
    loaded: "読み込み済み",
    locked: "ロック中",
    lockedTip: "暗号化されたシード — 読み込むときにパスフレーズで解除します。",
    close: "閉じる",
    idLabel: "フォルダ",
    switching: "マーチャントを切り替え中…",
    switchingBody: "そのフォルダに対してエンジンを再起動しています。",
  },
  // Seed create/import (SeedForm) + counterparty + status.
  seed: {
    intro: "まったく新しいシードを作成するか、すでにお持ちのものをインポートします。",
    createNew: "新規作成",
    createDesc: "新しいシードを生成します。リカバリーフレーズはご自身でバックアップしてください。",
    import: "インポート",
    importDesc: "既存の 12/24 語のフレーズから復元します。",
    recoveryLabel: "リカバリーフレーズ",
    encrypt: "暗号化",
    encryptDesc:
      "パスフレーズで保存中のシードを保護します。セッションごとに一度入力します — Satchel は保存しません。注意: 再起動後は、再入力するまで無人の自動返金が一時停止します。",
    noPassphrase: "パスフレーズなし（推奨）",
    noPassphraseDesc:
      "何も入力せずとも自動返金が再起動をまたいで機能し続けます — これはあくまでホットな中継シードです。代償: ファイルやホストへのアクセスにより、このマーチャントの中継鍵とアイデンティティが露呈します。",
    passphraseLabel: "パスフレーズ",
    passphrasePlaceholder: "パスフレーズを選択",
    revealTitle: "リカバリーフレーズを書き留めてください",
    revealBody:
      "この単語を持つ者は誰でもこのマーチャントのホットキーを制御できます。Satchel はコピーを保持しません — オフラインで保管してください。次にいくつかの単語を確認します。",
    ackLabel: "リカバリーフレーズを書き留めました。",
    // Multi-step onboarding (create/import -> secret -> confirm -> passphrase).
    chooseTitle: "{label} を設定",
    enterTitle: "リカバリーフレーズを入力",
    enterBody:
      "各単語を入力してください — 入力に応じてオートコンプリートされます — またはフレーズ全体を貼り付けてください。続行する前に検証します。",
    wordCount: "{n} 語",
    wordCountHint:
      "12 語で十分です — これはコールドストレージではなく、一時的な中継用のホットウォレットです。長いフレーズがよければ 24 語を選んでください。",
    wordAria: "単語 {n}",
    checkIncomplete: "{n} 語すべてを入力してください。",
    checkUnknown: "一部の単語が BIP39 のワードリストにありません — ハイライトされたものを確認してください。",
    checkBadChecksum: "チェックサムが一致しません — 単語とその順序を再確認してください。",
    checkOk: "リカバリーフレーズは有効なようです。",
    verifyTitle: "バックアップを確認",
    verifyBody: "フレーズを書き留めたことを確認するため、これらの位置の単語を入力してください。",
    verifyWord: "単語 #{n}",
    verifyMismatch: "フレーズと一致しません — バックアップを確認してください。",
    passphraseTitle: "シードを保護",
    passphraseBody:
      "任意で、保存するシードをパスフレーズで暗号化できます。スキップも可能です — 下のトレードオフをご覧ください。",
  },
  counterparty: {
    you: "これはあなたです",
    youShort: "あなた",
    unknown: "不明なアイデンティティ",
    // Short fingerprint fallback (identity.ts shortId) when no pubkey is known.
    unknownShort: "不明",
  },
  contacts: {
    title: "連絡先",
    subtitle: "取引相手につける、あなただけのニックネーム。",
    privacyNote:
      "連絡先はこのデバイスにのみ保存され、共有・公開・リレーへの送信は一切されません。ニックネームはあなた用のラベルであり、本当のアイデンティティは識別アイコンとフィンガープリントです。",
    searchPlaceholder: "ニックネーム・メモ・鍵で検索",
    empty: "まだ連絡先がありません。どこかで取引相手の識別アイコンをクリックして追加してください。",
    emptyFiltered: "このフィルターに一致する連絡先はありません。",
    count: "{n} 件の連絡先",
    colWho: "アイデンティティ",
    colNick: "ニックネーム",
    colNote: "メモ",
    colStatus: "ステータス",
    colAdded: "追加日",
    colActions: "",
    filterAll: "すべて",
    filterTrusted: "信頼済み",
    filterBlocked: "ブロック",
    // Corkboard toggle: drop blocked makers' offers from the ladder.
    hideBlocked: "ブロックしたオファーを非表示",
    statusTrusted: "信頼済み",
    statusNeutral: "中立",
    statusBlocked: "ブロック",
    menuAdd: "連絡先に追加…",
    menuEdit: "連絡先を編集…",
    menuMarkTrusted: "信頼済みにする",
    menuMarkNeutral: "中立にする",
    menuMarkBlocked: "ブロック",
    menuCopyKey: "公開鍵をコピー",
    menuOpen: "連絡先で開く",
    keyCopied: "公開鍵をコピーしました",
    editTitle: "連絡先を編集",
    addTitle: "連絡先を追加",
    nickLabel: "ニックネーム",
    nickPlaceholder: "例: ミートアップで会ったアリス",
    noteLabel: "メモ",
    notePlaceholder: "覚えておきたいことは何でも — 連絡方法、過去の取引など…",
    save: "保存",
    cancel: "キャンセル",
    remove: "連絡先を削除",
    removeConfirmTitle: "連絡先を削除しますか？",
    removeConfirmBody: "{who} のローカルのニックネームとメモが削除されます。元に戻せません。",
    blockedWarning: "この取引相手をブロックしています",
    blockedWarningBody:
      "この相手をブロック済みとしてマークしています。ブロックは個人的な注意喚起にすぎず、取引を止めるものではありません。意図して行う場合のみ続行してください。",
  },
  status: {
    notConnectedTitle: "エンジンに接続されていません",
    disconnectedBody:
      "Satchel はエンジンに到達できません。まだ起動中か、アクティブなマーチャントのノード接続が停止している可能性があります。再試行するか、上部のセレクターからマーチャントを切り替えてください。",
    openInSatchel: "これを Satchel で開く",
    noTauriBody:
      "これは Satchel の UI です — エンジンに到達するには Tauri ブリッジが必要です。ブラウザではなくデスクトップアプリ（cargo tauri dev）で起動してください。",
  },
  settings: {
    title: "設定",
    subtitle: "このインストール全体のアプリ設定です。",
    // UI-3 Settings tabs.
    tabGeneral: "一般",
    tabCoins: "コイン",
    tabNetwork: "ネットワーク",
    tabAbout: "概要",
    appearance: "外観",
    theme: "テーマ",
    themeDark: "ダーク",
    themeLight: "ライト",
    themeSystem: "システム",
    themeHint: "Satchel の見た目を選択します。システムは OS の設定に従います。",
    language: "言語",
    languageHint: "翻訳が寄せられるにつれて、対応言語が増えていきます。",
    mode: "モード",
    watchOnly: "閲覧のみモード",
    watchOnlyHint:
      "コインを設定せずにボードを閲覧します。自分のオファーは取り下げられますが、投稿・取得・資金の投入はできません。取引するにはオフにしてください（少なくとも 2 つのコインの接続が必要です）。",
    network: "ネットワーク",
    boards: "Corkboard",
    boardsDesc:
      "任意でセルフホストの HTTP ボードを使えます。信頼できるものを追加してください。空のままにすると Nostr に依存します。",
    boardsNone: "未設定",
    nostrRelays: "Nostr リレー",
    nostrRelaysDesc:
      "リレーは分散ネットワーク上で掲示板を運びます — 運営者があなたのオファーを読んだりマッチングしたりすることはできません。デフォルトのセットが事前設定されています。自由に編集してください。",
    nostrRelaysOff: "オフ — Nostr トランスポート無効",
    addUrl: "追加",
    removeUrl: "削除",
    relayInvalid: "ws:// または wss:// のリレー URL を入力してください",
    boardInvalid: "http:// または https:// のボード URL を入力してください",
    netSave: "保存して再接続",
    netSaving: "保存して再接続中…",
    netSaved: "保存しました",
    // Fees tab — fee-bump policy (per active merchant).
    tabFees: "手数料",
    fees: "手数料引き上げ",
    feesScope: "これらの設定はアクティブなマーチャントに適用されます。",
    feesIntro:
      "手数料引き上げの安全性とコストのトレードオフです。必須の設定ではありません。新しい値は今後の引き上げに適用されます。すでに資金を入れたスワップは、その時の方針を維持します。",
    feeMax: "最大フィーレート (sat/vB)",
    feeMaxHint:
      "すべての手数料引き上げの上限です。デフォルトは 500 で、システム上の絶対的な最大値でもあります。下げるとコストを抑えられます。",
    feeReservation: "資金引き上げ予備 (×)",
    feeReservationHint:
      "資金チェックが引き上げの余裕として確保する金額です。高くすると大きな手数料スパイクに対応できますが、より多くの残高が拘束され、より多くのスワップが拒否されます。デフォルトは 3 です。",
    feeCommitted: "リデーム過剰引き当て (×)",
    feeCommittedHint:
      "Satchel を閉じていても確定するよう、v2 のリデーム手数料を前払いする割増分です。新しいスワップにのみ適用されます。デフォルトは 2 です。",
    feeSave: "保存",
    feeSaving: "保存中…",
    feeSaved: "保存しました",
    feeReset: "デフォルトにリセット",
    coins: "コインとノード",
    coinsHint: "各コインをご自身のノードに接続します。保存前にジェネシスを確認します。",
    about: "概要",
    version: "バージョン {version}",
    updateUpToDate: "最新の状態です",
    updateCheckPlaceholder: "アップデートのチェックは今後のリリースで追加されます。",
    trustModel: "鍵の保管場所",
    trustModelBody:
      "秘密情報はエンジン内にあり、Satchel には決して入りません。マーチャントのシードはエンジンのデータフォルダにあります（暗号化するか平文かはあなたの選択次第）。Satchel はシードもパスフレーズも保存しません。シードは設計上ホット（中継鍵のみ）です — まとまった収益はご自身のコールドウォレットへスイープしてください。",
  },
  coins: {
    intro:
      "各コインをご自身のノードに接続します。最初の URL はあなたのノード自身のウォレットです — スワップの脚に資金を入れ、収益を受け取ります。保存前に、Satchel はノードのジェネシスブロックを確認するので、資金が誤ったチェーンに送られることは決してありません。接続はすべてのマーチャント間で共有されます。",
    networkBadge: "{network} ネットワーク向けに設定中",
    needMerchant:
      "まずマーチャントを接続してください — コインの設定にはエンジンの稼働が必要です。右上のマーチャントセレクターを使ってください。",
    pairsTitle: "取引ペア",
    pairsHint:
      "ペアは各コインの対応機能から導かれます — 固定のリストはありません。両方のコインが接続されるとペアが開きます。",
    noPairs: "利用可能なペアがありません。",
    notSetUp: "未設定",
    connectedTip: "接続済み · ティップ {tip}",
    connError: "接続エラー",
    setUp: "設定",
    editConnection: "接続を編集",
    remove: "削除",
    disconnectTip: "このコインを切断",
    disconnectTitle: "{coin} を切断しますか？",
    disconnectBody: "再接続するまで、これを必要とするスワップは利用できません。",
    ready: "取引可能",
    connectMissing: "{coins} を接続",
    notBuildable: "まだ構築できません",
    // Swap-protocol chips on a pair (pact-htlc-v1 HTLC vs pact-htlc-v2 adaptor).
    protoPrivate: "プライベート (Taproot)",
    protoPrivateTip: "プライベートスワップ（Taproot/MuSig2 アダプター）— オンチェーンでは通常の支払いに見えます",
    protoHtlcTip: "クラシックな HTLC スワップ",
    // Coin-setup backend choices (CoinSetup).
    // CoinSetup dialog.
    setupTitle: "{coin} を接続",
    setupIntro:
      "Satchel をご自身の {sym} ノードに向けます。ノードがジェネシスブロックのチェックに合格するまで何も保存されません — あなたの資金が触れるのは本物の {sym} チェーンだけです。",
    confirmationsLabel: "確定前の承認数",
    confirmationsHint:
      "このチェーン上の資金投入やリデームが、スワップが動作する前にどれだけ深くなる必要があるか — リオーグ安全マージンです。高いほど安全ですが遅くなります。推奨デフォルト（{default}）を使うには空欄のままにしてください。",
    validateNode: "ノードを検証",
    checking: "ノードを確認中…",
    genesisOk: "ジェネシスが一致 — これは正しいチェーンです",
    genesisDetail: "ティップ高 {tip} · ジェネシス {hash}…",
    genesisBad: "拒否 — 保存しません",
    errorShort: "エラー",
    // Structured connection form (CoinSetup v2).
    rpcHostLabel: "RPC ホスト",
    rpcPortLabel: "RPC ポート",
    authMethodLabel: "認証",
    authCookie: "Cookie ファイル",
    authCookieDesc: "ノードのデータディレクトリから .cookie を自動で読み取ります（デフォルト、パスワードは保存されません）。",
    authUserPass: "ユーザー / パスワード",
    authUserPassDesc: "ノードの設定にある rpcuser / rpcpassword — リモートノードに必要です。",
    rpcUserLabel: "RPC ユーザー名",
    rpcPasswordLabel: "RPC パスワード",
    datadirLabel: "ノードのデータディレクトリ",
    cookiePathNote: "Cookie はこのディレクトリ下の {path} から読み取られます。",
    walletLabel: "ウォレット名（任意）",
    walletPlaceholder: "あなたのノードのウォレット",
    needPort: "まず RPC ポートを入力してください。",
    validateFirst: "保存する前にノードを検証してください。",
    savingReconnecting: "保存して再接続中…",
    connected: "{coin} 接続済み",
    // Nodeless (Electrum) connection mode (epic #58).
    modeLabel: "接続タイプ",
    modeNode: "自分のノード",
    modeNodeDesc: "Core RPC — ノードのウォレットがスワップに資金を入れます。最大限の主権。",
    modeNodeless: "Electrum",
    modeNodelessDesc:
      "ノードは不要です。チェーンデータは Electrum サーバーから取得され、ウォレットはあなたの Pact シード上にあります。",
    electrumUrlsLabel: "Electrum サーバー",
    electrumUrlsHelp:
      "1 行に 1 つ: tcp://host:port または ssl://host:port。メインネットでは、チェーンの見え方を相互チェックするため、少なくとも 2 つの独立したサーバーが必要です。",
    electrumNeedUrl: "Electrum サーバーの URL を少なくとも 1 つ入力してください（tcp:// または ssl://）。",
    electrumBadUrl: "Electrum の URL は tcp:// または ssl:// で始まる必要があります — 入力値: {url}",
    validateServers: "サーバーを検証",
    connRpcLocal: "RPC（ローカル）",
    connRpcRemote: "RPC（リモート）",
    connElectrumLocal: "Electrum（ローカル）",
    connElectrumRemote: "Electrum（リモート）",
    connRpcTip:
      "このコインは Bitcoin Core 系のノードと RPC で通信します。スワップにはノードのウォレットが資金を入れます。",
    connElectrumTip:
      "このコインは Electrum サーバーに接続します — ノードは不要です。ウォレットはあなたの Pact シード上にあります。",
    // Template picker (a coins.toml coin the engine version doesn't support).
    unsupportedByEngine: "非対応",
    unsupportedByEngineTip:
      "このコインは coins.toml で定義されていますが、このバージョンのエンジンには組み込まれていないため、取引できません。",
  },
  coinWizard: {
    title: "コインを接続",
    intro:
      "少なくとも 2 つのコインを選び、それぞれをご自身のノードに向けます。スワップには 2 つのチェーンが必要なので、2 つのノードが接続されて稼働すると取引が解放されます。コインは後から設定で追加・変更できます。",
    progress: "{count} / {min} 個のコインを接続済み",
    continue: "続ける",
    live: "稼働中",
    nodeDown: "ノード停止",
  },
  wallets: {
    intro:
      "これらはあなた自身のノードのウォレットです（エンジンがスワップに資金を入れ、収益を受け取るために使うもの）— あなたの鍵、あなたのマシンです。Satchel があなたのコインを保持することは決してありません。",
    hotSeedNudge:
      "これはホットなシード上の使用ウォレットであり、金庫ではありません — まとまった残高はご自身のコールド/Core ウォレットへスイープしてください。",
    notConnected: "未接続",
    notConnectedBody: "まずマーチャントを接続してください — ウォレット表示にはエンジンの稼働が必要です。",
    noCoins: "コインがまだ設定されていません",
    noCoinsBody: "設定 → コインでコインを接続すると、そのウォレットがここに表示されます。",
    goToCoins: "コインへ移動",
    watchOnlyTitle: "閲覧のみモードではウォレットはありません",
    watchOnlyBody:
      "これはコインが接続されていない閲覧のみのセッションのため、表示するウォレットがありません。設定で閲覧のみをオフにし、コインを接続してスワップに資金を入れてください。",
    walletName: "ウォレット · {wallet}",
    walletScopedHint: "このコインのすべての RPC は、このノードウォレットにスコープされます。",
    walletDefault: "デフォルトウォレット（未スコープ）",
    walletDefaultHint:
      "このコインにウォレットが設定されていないため、RPC はノードのデフォルトウォレットを使用します。設定 → コインで設定すると、すべての呼び出しを特定のウォレットにスコープできます。",
    balanceLabel: "{symbol} 残高",
    // ---- nodeless (pact-seed bdk) wallet: send / receive / activity --------
    pactSeed: "pact シードウォレット",
    pactSeedHint:
      "このコインはノードレスで動作します。ウォレットはあなたの Pact シード上にあり、Electrum サーバーから同期されます — ノードは不要です。送金・受取・履歴はすべてここで行えます。",
    receive: "受け取る",
    send: "送る",
    activity: "アクティビティ",
    copy: "コピー",
    copied: "コピーしました",
    close: "閉じる",
    refresh: "更新",
    receiveTitle: "{sym} を受け取る",
    receiveIntro:
      "pact シードウォレットからの新しいアドレスです。ここに送られたコインは、確認後に残高へ反映されます。",
    receiveIntroRpc:
      "ノードのウォレットからの新しいアドレスです。ここに送られたコインは、確認後に残高へ反映されます。",
    receiveFreshNote:
      "このダイアログを開くたびに新しいアドレスが発行されます。古いアドレスも引き続き使えます — 新しいアドレスの方がプライバシーに優れているだけです。",
    sendTitle: "{sym} を送る",
    sendIntro: "利用可能: {balance} {sym}。",
    sendAddressLabel: "受取先の {sym} アドレス",
    sendAmountLabel: "金額",
    sendNeedAddress: "受取先のアドレスを入力してください。",
    sendNeedAmount: "金額を入力してください。",
    sendOverBalance: "利用可能な残高を超えています。",
    sendFeeNote: "ネットワーク手数料は別途加算され、現在の手数料相場から自動的に選ばれます。",
    sendBroadcast: "送信しました — {txid}… が処理中です（{sym}）。",
    sendConfirm: "送る",
    activityTitle: "{sym} のアクティビティ",
    activityEmpty: "まだ何もありません — コインを受け取るかスワップを完了すると、ここに表示されます。",
    activityWhen: "日時",
    activityDirection: "方向",
    activityAmount: "金額（{sym}）",
    activityFee: "手数料",
    activityConfs: "承認数",
    activityTxid: "トランザクション",
    activityPending: "保留中",
    activitySent: "送金",
    activityReceived: "受取",
  },
  corkboard: {
    noBoardTitle: "Corkboard が接続されていません",
    noBoardBody:
      "Corkboard は、メーカーがオファーを掲示する共有の掲示板です。取引のマッチングもコインの保持も決して行いません — 信頼できるものに Satchel を向けて、閲覧と投稿を行ってください。",
    noPairs: "利用可能なペアがありません",
    board: "Corkboard",
    boardSettings: "設定で構成",
    filterAll: "すべて",
    filterMine: "自分",
    noOffers: "今すぐ取れるオファーはありません",
    noOffersBody:
      "設定したペアにメーカーがオファーを投稿すると、すぐにここに表示されます。自分でオファーを投稿することもできます。",
    yourOffer: "あなたのオファー",
    offerStaged: "投稿中…",
    offerStagedTip:
      "このデバイスから投稿され、リレーからの確認待ちです。広告中で、リレーがエコーすると稼働状態になります。",
    take: "オファーを取る",
    legDown: "このペアのいずれかのノードが停止しています — 取得前に起動してください（または設定 → コインを確認）。",
    withdraw: "取り下げ",
    withdrawTip: "即座に取り下げ — オファーが資金をロックすることは決してありません",
    safetyRefund: "安全返金",
    safetyRefundTip:
      "スワップが滞った場合、双方が自動返金されます — テイカーの脚が先に解除され、あなたの脚は少し後に解除されます。誰も立ち往生しません。",
    activeTitle: "進行中のスワップ",
    states: {
      takenByUs: "あなたが取得",
      revoked: "取り下げ済み",
      expired: "期限切れ",
    },
    // Two-sided order-book view of the Corkboard. Bids = makers giving the
    // quote coin to get the base; asks = the reverse. The ladder is a way to
    // READ the board — it never matches or prioritises (load-bearing).
    book: {
      bids: "買い",
      asks: "売り",
      bidsHint: "{base} が欲しい · {quote} で支払い",
      asksHint: "{base} を売る · {quote} で",
      price: "価格",
      size: "サイズ",
      noBids: "買いなし",
      noAsks: "売りなし",
      spread: "スプレッド {pct}",
      spreadOneSided: "片側のみ",
      crossed: "クロス",
      crossedTip: "最高買い ≥ 最安売り。ボードは自動マッチングを行わないので、これらの重なるオファーはそのまま残ります — どちらの側でも取れます。",
      mid: "中値 {price}",
      levelOffers: "この価格に {count} 件のオファー — 一つ選んで取ってください",
      depthTip: "この価格で {count} 件の掲示にわたり提供中の {sym} の合計。",
      selectLevel: "上の価格レベルを選ぶと、そこのオファーが表示されます。",
      paneHeader: "{size} {base} @ {price} {unit}",
      denomTip: "{coin} 数量の表示単位",
      showMore: "さらに {count} 件を表示",
      showLess: "上位 {count} 件を表示",
    },
  },
  relays: {
    title: "リレー",
    subtitle: "Nostr リレーへのライブ接続性 — あなたのオファーと取得が通るネットワークです。リレーの追加・削除は設定 → ネットワークで行えます。",
    connectedCount: "{up} / {total} 接続済み",
    refresh: "更新",
    ms: "{ms} ms",
    up: "接続中",
    down: "切断",
    statsTip: "{success}/{attempts} 回接続成功 · ↓{down} ↑{up}",
    none: "リレーが未設定です",
    noneBody: "ネットワーク経由でオファーを公開・受信するには、設定 → ネットワークで Nostr リレーを追加してください。",
    goToNetwork: "設定へ移動",
    notConnected: "未接続",
    notConnectedBody: "リレー表示にはエンジンの稼働が必要です — まずマーチャントを接続してください。",
  },
  swaps: {
    maker: "Maker",
    taker: "Taker",
    title: "スワップ",
    hint: "あなたの全台帳です — 進行中のスワップが上、完了した取引が下に表示されます。Corkboard からもライブのスワップを操作できます。",
    activeTitle: "進行中",
    historyTitle: "履歴",
    none: "スワップはまだありません — Corkboard でオファーを取ってください。",
    // Active-swaps dock action buttons + the refund-time label.
    cancel: "キャンセル",
    dump: "ログを出力",
    dumpHint: "このスワップの秘密情報を含まない診断バンドル（状態 + ログ行）をコピーし、開発者に貼り付けて渡します。",
    dumpCopied: "診断情報をコピーしました — 開発者に貼り付けてください。",
    dumpFailed: "診断バンドルをコピーできませんでした。",
    refundAt: "{when} に返金",
    // Confirm dialogs for acting on a live swap (ActiveSwaps).
    cancelTitle: "このスワップをキャンセルしますか？",
    cancelConfirm: "スワップをキャンセル",
    cancelKeep: "そのままにする",
    // Abort reason recorded on the swap when cancelled from Satchel.
    cancelReason: "Satchel でキャンセル",
    cancelBody:
      "これは資金を入れる前にスワップを放棄します。あなたのものはまだ何もロックされていないので、損失はありません — オファーが完了しないだけです。",
    col: {
      swap: "スワップ",
      role: "役割",
      state: "状態",
      amounts: "渡す → 受け取る",
      when: "日時",
      finalTx: "最終 tx",
    },
    // Expandable per-leg on-chain detail (the audit trail). We show both
    // funding txs + OUR settlement; never the counterparty's settlement or the
    // swap secret.
    audit: {
      toggle: "オンチェーン詳細を表示",
      title: "オンチェーン詳細",
      youLocked: "あなたがロック",
      theyLocked: "相手がロック",
      funding: "資金投入",
      received: "受領済み",
      refunded: "返金済み",
      pending: "まだオンチェーンにありません",
      copy: "トランザクション ID をコピー",
      copied: "トランザクション ID をコピーしました",
    },
  },
  fees: {
    title: "ネットワークコストのプレビュー",
    estimated: "見積もり",
    provisionalNote: "この pactd ビルドはまだ手数料の見積もりを公開していません。",
    summary: "スワップは、あなたが支払う 2 つのオンチェーントランザクションです: 渡すチェーンでの資金投入と、受け取るチェーンでのリデームです。",
    fallbackTip: "ノードに到達できなかったため、保守的なデフォルトのフィーレートが使われました — これらは目安として扱ってください。",
    ifItStalls: "（滞った場合）",
  },
  funds: {
    insufficient:
      "このスワップに資金を入れるには {sym} が足りません — 約 {need} {sym}（数量 + 資金手数料）が必要ですが、ウォレットには {have} {sym} しかありません。",
  },
  wizard: {
    back: "戻る",
    continue: "続ける",
  },
  // UI-4 docked activity log.
  log: {
    title: "アクティビティ",
    empty: "— アクティビティログ —",
    count: "{count} 行",
    collapse: "ログを折りたたむ",
    expand: "ログを展開",
    // Activity-log lines emitted by the frontend. `{err}` carries a raw engine
    // message (itself not translated — it comes from pactd over the wire).
    noTauri: "Satchel 内で実行されていません — この UI には Tauri ブリッジが必要です",
    startupError: "起動: {err}",
    notConnected: "未接続: {err}",
    connected: "pactd {version} に接続しました（{protocol}）",
    listcoinsError: "listcoins: {err}",
    watchOnlyError: "閲覧のみ: {err}",
    switchedMerchant: "マーチャント {id} に切り替えました",
    renamedMerchant: "マーチャント名を {name} に変更しました",
    renameMerchantError: "マーチャント名の変更: {err}",
    switchMerchantError: "マーチャント切り替え: {err}",
    loadMerchantError: "マーチャント読み込み: {err}",
    merchantCreated: "マーチャント {id} を作成しました",
    merchantReady: "マーチャント準備完了",
    actionOk: "{action} {id}: ok",
    actionError: "{action} {id}: {err}",
    diagCopied: "{id} の診断情報をコピーしました（{count} ログ行）— 開発者に貼り付けてください",
    dumpError: "ダンプ {id}: {err}",
    coinDisconnected: "{coin} を切断しました",
    removeCoinError: "コイン削除: {err}",
    tookOffer: "オファー {id} を取得しました — 下の進行中スワップに表示されます",
    takeError: "取得: {err}",
    offerWithdrawn: "オファー {id} を取り下げました",
    withdrawError: "取り下げ: {err}",
    postedOffer: "オファー {id} を投稿しました — いつでも取り下げ可能、何もロックされません",
    createdSlip: "プライベートオファースリップを作成しました — 友人に送ってください",
    tookPrivateOffer: "プライベートオファー {id} を取得しました — 進行中スワップに表示されます",
    cancelledPrivateOffer: "プライベートオファー {id} をキャンセルしました",
    cancelError: "キャンセル: {err}",
    noticeboardUpdated: "掲示板を更新しました",
    feePolicyUpdated: "手数料ポリシーを更新しました",
  },
  // Relative-time + freshness prose from format.ts (rendered via the tr() mirror,
  // since those are pure non-component helpers). Unit letters stay inside the
  // template so a translation owns the whole phrase.
  format: {
    ageUnknown: "経過時間不明",
    justNow: "たった今",
    minutesAgo: "{n} 分前",
    hoursAgo: "{n} 時間前",
    daysAgo: "{n} 日前",
    expiryNow: "今",
    expirySoon: "まもなく",
    inMinutes: "約 {n} 分後",
    inHours: "約 {n} 時間後",
    inDays: "約 {n} 日後",
    posted: "{age} に投稿",
    expires: "{time} に期限切れ",
  },
  // Plain-language swap story per (role, state) — the honest "who is exposed
  // when" framing shown on every active swap. {a}/{b} are coin tickers; {t1}/{t2}
  // are local refund times. Rendered via tr() (narrate() is a pure helper).
  narrate: {
    finalizing: "{got} を請求しました — 最終確認中。埋まるまでアプリを開いたままにしてください。それまで {gave} は保護されます。",
    initiating:
      "テイクを送信 — メーカーがスワップを開始するのを待っています。まだ何もロックされていません。相手が応答しなければ自動でキャンセルされます。",
    created: "オファーを送信 — 相手の同意を待っています。何もコミットされていません。",
    acceptedMaker: "条件に合意。次: あなたの {a} をロックします。資金を入れるまで、自由にキャンセルできます。",
    acceptedTaker: "条件に合意。相手が先に {a} をロックします — あなたが先に送ることはありません。",
    noncesExchanged:
      "プライベートスワップを設定中 — 署名素材を交換しています。まだ何もロックされていません。",
    signedMaker:
      "双方が署名し、あなたの {a} はロックされました。相手がロックして確認すると、あなたのデーモンが自動的に {b} を請求します。何かが滞った場合、あなたの {a} は {t1} に返ってきます。",
    signedTaker:
      "双方が署名しました。相手の {a} が確認されると、あなたのデーモンがあなたの {b} をロックし、その後自動的に {a} を請求します。あなたの {b} がロックされた後は、何かが滞った場合 {t2} に返ってきます。",
    fundedAMaker:
      "あなたの {a} はロックされました。相手が {b} をロックするのを待っています。相手がしない場合、あなたの {a} は {t1} に自動的に返ってきます。",
    fundedATaker:
      "相手の {a} はロックされ、検証されました。次: あなたの {b} をロックします。セーフティネット: 何かが滞った場合 {t2} に自動返金。",
    fundedBMaker: "双方ロック済み。あなたのデーモンが、安全に確定し次第 {b} を請求します。",
    fundedBTaker: "双方ロック済み。相手が {b} を取った瞬間に、あなたのデーモンが {a} を請求します。",
    completed: "スワップ完了 — {coin} はあなたのウォレットにあります。",
    refunded: "スワップは完了しなかったため、あなたの {coin} は自動的に返ってきました。失ったのは手数料だけです。",
    aborted: "資金が動く前にキャンセルされました。",
  },
  progress: {
    awaitingLock: "相手のロックを待機中",
    awaitingClaim: "相手の請求を待機中",
    theirLock: "相手のロックを確認中",
    ourLock: "自分のロックを確認中",
    securing: "{coin} を保全中",
    funding: "{coin} をロック中 — 進まない場合はウォレットを解除してください",
    blocks: "+{n} ブロック",
    feeBumped: "手数料を引き上げ",
    reorg: "再編成を検出 — 再確認中",
  },
  exit: {
    // Exit-gate dialog (fund safety, C6). The engine manages alone, so "keep
    // running" detaches it (it keeps watching timelocks + servicing offers).
    liveTitle: "スワップが進行中です",
    liveBodyOne:
      "1 件のスワップが進行中です。オンチェーンのタイムロックに支配されており — 期限前にリデームまたは返金するため、エンジンを稼働させ続ける必要があります。",
    liveBodyMany:
      "{count} 件のスワップが進行中です。オンチェーンのタイムロックに支配されており — 期限前にリデームまたは返金するため、エンジンを稼働させ続ける必要があります。",
    keepRunningExplain:
      "ウィンドウを閉じてもエンジンはバックグラウンドで稼働し続けるので、ヘッドレスでスワップを完了します。いつでも Satchel を再度開いて確認できます。",
    forceQuitWarn: "今すぐ強制終了するとエンジンが停止し、資金を失う可能性があります。",
    // {word} is the confirm word below; a translation may localize both together.
    typeToConfirm: "それでも強制終了するには、下に {word} と入力してください。",
    confirmWord: "QUIT",
    keepRunning: "稼働を続けてウィンドウを閉じる",
    keepWithdraw: "稼働を続けてオファーを取り下げる",
    keepLeaveOffers: "稼働を続けてオファーを残す",
    forceQuit: "強制終了",
    offersTitle: "オファーを投稿しています",
    offersBodyOne:
      "あなたのオファー 1 件がまだ Corkboard にあります。オファーは何もロックしませんが、残しておくと Satchel を閉じている間も取引相手が取れます — エンジンがその取得を処理します。",
    offersBodyMany:
      "あなたのオファー {count} 件がまだ Corkboard にあります。オファーは何もロックしませんが、残しておくと Satchel を閉じている間も取引相手が取れます — エンジンがそれらの取得を処理します。",
    withdrawExit: "すべて取り下げて終了",
  },
  unlock: {
    title: "マーチャントを解除",
    body:
      "このマーチャントのシードは暗号化されています。このセッションで解除するにはパスフレーズを入力してください — Satchel はメモリ内にのみ保持し、終了時に忘れます。",
    switchMerchant: "マーチャントを切り替え",
    unlock: "解除",
  },
  common: {
    cancel: "キャンセル",
    confirm: "確認",
    save: "保存",
    done: "完了",
    later: "後で",
    retry: "接続を再試行",
  },
};
