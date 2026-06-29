// The Greek (Ελληνικά) string bundle. Mirrors en.ts key-for-key; only values are translated.
import type { Bundle } from "./en";

export const el: Bundle = {
  app: {
    name: "Satchel",
  },
  // In-app update notifications (sidebar version badge + dialog).
  update: {
    title: "Διαθέσιμη ενημέρωση",
    upToDate: "Είστε ενημερωμένοι",
    current: "Εγκατεστημένη",
    latest: "Τελευταία",
    notesTitle: "Σημειώσεις έκδοσης",
    get: "Λήψη της ενημέρωσης",
    dismiss: "Απόρριψη",
    close: "Κλείσιμο",
    badgeTooltip: "Διαθέσιμη ενημέρωση — κάντε κλικ για λεπτομέρειες",
    versionTooltip: "Κάντε κλικ για έλεγχο ενημερώσεων",
  },
  // Risk disclaimer (first-run welcome + Settings → About).
  disclaimer: {
    title: "Αυτο-φύλαξη — τα κλειδιά σας, η ευθύνη σας",
    body: "Το Satchel εκτελεί ατομικές ανταλλαγές χωρίς θεματοφύλακα: μόνο εσείς κατέχετε τα κλειδιά σας, ενώ ο σπόρος ενός merchant κρατά θερμά κλειδιά διέλευσης όσο μια ανταλλαγή είναι σε εξέλιξη. Τα πρωτόκολλα ανταλλαγής (v1 HTLC και v2 Taproot/MuSig2) είναι ελεγμένα και ενεργά στο mainnet. Με άδεια MIT και παρέχεται ως έχει, χωρίς καμία εγγύηση — δημιουργήστε αντίγραφο ασφαλείας της φράσης ανάκτησής σας και χρησιμοποιήστε το με δική σας ευθύνη.",
  },
  nav: {
    public: "Δημόσιες",
    corkboard: "Corkboard",
    postOffer: "Δημοσίευση προσφοράς",
    private: "Ιδιωτικές",
    privateCreate: "Δημιουργία απόκομματος",
    privateReceive: "Λήψη απόκομματος",
    privateSlips: "Τα απόκομματά μου",
    swaps: "Ανταλλαγές",
    relays: "Relays",
    wallets: "Πορτοφόλια",
    contacts: "Contacts",
    settings: "Ρυθμίσεις",
    coins: "Νομίσματα",
  },
  makeOffer: {
    title: "Δημοσίευση προσφοράς",
    intro:
      "Δημοσιεύστε μια υπογεγραμμένη προσφορά στο Corkboard. Τίποτα δεν κλειδώνεται — είναι απλώς μια αγγελία· αποσύρετέ την οποτεδήποτε, και μια ανταλλαγή ξεκινά μόνο όταν κάποιος την αποδεχτεί και χρηματοδοτήσουν και οι δύο πλευρές.",
    give: "Δίνετε",
    want: "Λαμβάνετε",
    // Canonical pair + direction: pick the pair, choose Sell/Buy the base, enter
    // the base amount and a quote-per-base price (invariant to direction).
    pair: "Ζεύγος",
    noPairs: "Καμία διαπραγματεύσιμη ζεύγος — συνδέστε τουλάχιστον δύο νομίσματα στις Ρυθμίσεις → Νομίσματα.",
    sell: "Πώληση {sym}",
    buy: "Αγορά {sym}",
    amount: "Ποσό",
    youGive: "Δίνετε",
    youGet: "Λαμβάνετε",
    price: "Τιμή",
    priceUnit: "{unit} ανά {base}",
    pricePlaceholder: "τιμή μονάδας",
    balance: "Υπόλοιπο: {amt} {sym}",
    balanceLoading: "Υπόλοιπο: …",
    noCoins: "Δεν έχουν ρυθμιστεί νομίσματα",
    legDown: "Ο κόμβος ενός από αυτά τα νομίσματα είναι εκτός λειτουργίας — εκκινήστε τον (ή ελέγξτε Ρυθμίσεις → Νομίσματα) πριν τη δημοσίευση.",
    // Swap-protocol pin (only offered when a pair+network supports more than
    // one). v2 label reuses coins.protoPrivate ("Private (Taproot)").
    protocol: "Τύπος ανταλλαγής",
    protoStandard: "Τυπική (HTLC)",
    // Titles for the review/confirm dialog shown before posting (see OfferForm).
    reviewOfferTitle: "Ελέγξτε την προσφορά σας",
    reviewSlipTitle: "Ελέγξτε το απόκομμά σας",
    term: "Χρονοκλείδωμα ασφαλείας",
    termShort: "Σύντομο",
    termMedium: "Μεσαίο",
    termLong: "Μακρύ",
    termHint: {
      short: "Σύντομο — τα κεφάλαια επιστρέφονται αυτόματα ταχύτερα αν η συναλλαγή κολλήσει (~12ώ / 6ώ), με το μικρότερο περιθώριο ασφαλείας.",
      medium: "Μεσαίο — ισορροπημένο παράθυρο επιστροφής (~24ώ / 12ώ).",
      long: "Μακρύ (ασφαλέστερο) — το ευρύτερο περιθώριο ασφαλείας· αυτόματη επιστροφή μετά από ~36ώ / 18ώ αν η συναλλαγή κολλήσει.",
    },
    validFor: "Ισχύει για (λεπτά)",
    validForMins: "{mins} λεπτά",
    validForHint:
      "Πόσο χρόνο παραμένει καταχωρημένη η προσφορά. Όσο είστε συνδεδεμένοι, διατηρείται αυτόματα φρέσκια· μετά από αυτό λήγει. Το κλείσιμο της εφαρμογής την αποσύρει.",
    note: "Προσφορά σταθερού μεγέθους — τίποτα δεν κλειδώνεται μέχρι κάποιος να την αποδεχτεί. Τα ποσά είναι on-chain· πληρώνετε τέλη δικτύου επιπλέον και το Corkboard δεν χρεώνει τίποτα. Το χρονοκλείδωμα είναι το παράθυρο αυτόματης επιστροφής αν μια ανταλλαγή κολλήσει.",
    post: "Δημοσίευση προσφοράς",
    makeSlip: "Δημιουργία απόκομματος",
    slipTitle: "Το ιδιωτικό σας απόκομμα προσφοράς",
    slipExplainer:
      "Στείλτε το στον φίλο σας. Το επικολλά στο Satchel για να το αποδεχτεί. Τίποτα δεν κλειδώνεται· λήγει σε {ttl}.",
    copy: "Αντιγραφή",
    copied: "Αντιγράφηκε",
    makeAnother: "Δημιουργία άλλου",
    myPrivateTitle: "Οι ιδιωτικές μου προσφορές",
    myPrivateEmpty: "Καμία εκκρεμής ιδιωτική προσφορά.",
    privateExpires: "λήγει {when}",
    privateExpired: "έληξε",
    cancel: "Ακύρωση",
    cancelTip: "Σταματήστε να τηρείτε αυτό το απόκομμα — ένας φίλος που το κρατά ακόμα δεν θα μπορεί πλέον να το αποδεχτεί.",
  },
  takeSlip: {
    intro:
      "Ένας φίλος σάς έστειλε ένα ιδιωτικό απόκομμα προσφοράς (ξεκινά με pactoffer1:). Επικολλήστε το εδώ για να το ελέγξετε και να το αποδεχτείτε — ακριβώς όπως μια προσφορά από τον πίνακα.",
    placeholder: "pactoffer1:…",
    take: "Έλεγχος & αποδοχή",
    invalid: "Αυτό δεν μοιάζει με απόκομμα — θα έπρεπε να ξεκινά με pactoffer1:.",
    previewLabel: "Αυτό το απόκομμα προσφέρει",
  },
  // PRIVATE nav group (off-market slips) — screen titles + intros. The form,
  // slip output, my-slips list and take flow reuse makeOffer.*/takeSlip.* copy.
  private: {
    createTitle: "Δημιουργία ιδιωτικής προσφοράς",
    createIntro:
      "Φτιάξτε μια υπογεγραμμένη προσφορά και δώστε την σε έναν φίλο ως απόκομμα μέσω της δικής σας συνομιλίας. Τίποτα δεν καταχωρείται πουθενά — και τίποτα δεν κλειδώνεται μέχρι να χρηματοδοτήσετε και οι δύο.",
    slipsIntro:
      "Απόκομματα που έχετε δημιουργήσει. Όποιος κρατά ένα απόκομμα μπορεί να το αποδεχτεί μέχρι να λήξει· ακυρώστε ένα για να σταματήσετε να το τηρείτε πριν από τότε.",
    slipsEmptyBody: "Δημιουργήστε μια ιδιωτική προσφορά για να αποκτήσετε ένα απόκομμα που μπορείτε να στείλετε σε έναν φίλο.",
    receiveTitle: "Λήψη ιδιωτικής προσφοράς",
    received: "Αποδεκτή — παρακολουθήστε την στις Ανταλλαγές.",
  },
  // Shared take-confirmation dialog (board take + slip take).
  takeConfirm: {
    title: "Αποδοχή αυτής της προσφοράς;",
    confirm: "Αποδοχή προσφοράς",
    counterparty: "Αντισυμβαλλόμενος",
    youGive: "Δίνετε",
    youReceive: "Λαμβάνετε",
    safetyRefund: "Επιστροφή ασφαλείας",
    offerAge: "Ηλικία προσφοράς",
    makerFundsFirst:
      "Ο maker κλειδώνει τα {sym} του πρώτος — εσείς δεν στέλνετε ποτέ πρώτοι. Μπορείτε ακόμα να ακυρώσετε πριν χρηματοδοτήσετε την πλευρά σας, και η μηχανή επιστρέφει αυτόματα τα κεφάλαια μετά το χρονοκλείδωμα ασφαλείας αν η ανταλλαγή κολλήσει.",
  },
  header: {
    activeMerchant: "Ενεργός merchant — κάντε κλικ για εναλλαγή ή διαχείριση",
    manageMerchants: "Διαχείριση Merchants…",
    noMerchant: "κανένας merchant",
    openMenu: "Άνοιγμα μενού",
    collapseMenu: "σύμπτυξη μενού",
    settings: "Ρυθμίσεις",
    language: "Γλώσσα",
    pactConnected: "Η μηχανή συνδέθηκε",
    pactUnreachable: "Η μηχανή δεν είναι προσβάσιμη",
    liveSwapsOne: "1 ανταλλαγή σε εξέλιξη — κάντε κλικ για προβολή",
    liveSwapsMany: "{count} ανταλλαγές σε εξέλιξη — κάντε κλικ για προβολή",
    liveSwapsNone: "Καμία ανταλλαγή σε εξέλιξη",
    coinOk: "{name} — συνδεδεμένο · κορυφή {tip}",
    coinUnconfigured: "{name} — δεν έχει ρυθμιστεί",
    coinError: "{name} — {status}",
    relaysOk: "Relays Nostr — {up}/{total} συνδεδεμένα",
    relaysDown: "Relays Nostr — κανένα από {total} συνδεδεμένο",
  },
  network: {
    mainnet: "MainNet",
    testnet: "TestNet",
    regtest: "RegTest",
    signet: "Signet",
    notRealFunds: "Όχι πραγματικά κεφάλαια — αυτό είναι το δίκτυο {network}",
  },
  // Watch-only mode: a viewer session with no coins. Browse the board and
  // withdraw your own offers, but no posting / taking / funding.
  watchOnly: {
    badge: "Μόνο παρακολούθηση",
    badgeTip:
      "Λειτουργία μόνο παρακολούθησης — περιηγηθείτε στον πίνακα και αποσύρετε τις δικές σας προσφορές, αλλά δεν μπορείτε να δημοσιεύσετε, να αποδεχτείτε ή να χρηματοδοτήσετε. Ρυθμίστε νομίσματα στις Ρυθμίσεις για να διαπραγματευτείτε.",
    coinWizardButton: "Περιήγηση σε λειτουργία μόνο παρακολούθησης",
    coinWizardHint:
      "Παραλείψτε τη ρύθμιση νομισμάτων και απλώς περιηγηθείτε στον πίνακα (μόνο για ανάγνωση). Μπορείτε ακόμα να αποσύρετε τις δικές σας προσφορές — βολικό για την απόσυρση προσφορών που άφησε μια άλλη συνεδρία. Απενεργοποιήστε το οποτεδήποτε στις Ρυθμίσεις.",
    postBlockedTitle: "Λειτουργία μόνο παρακολούθησης",
    postBlockedBody:
      "Αυτή είναι μια συνεδρία μόνο παρακολούθησης, οπότε δεν μπορεί να δημοσιεύσει προσφορές. Ρυθμίστε τουλάχιστον δύο νομίσματα στις Ρυθμίσεις → Νομίσματα για να διαπραγματευτείτε.",
    takeBlockedBody: "Λειτουργία μόνο παρακολούθησης — μπορείτε να ελέγξετε αυτή την προσφορά, αλλά η αποδοχή της απαιτεί ρυθμισμένα νομίσματα.",
    takeBlockedTip: "Λειτουργία μόνο παρακολούθησης — ρυθμίστε νομίσματα στις Ρυθμίσεις για να αποδέχεστε προσφορές.",
  },
  merchants: {
    title: "Οι merchants σας",
    intro:
      "Ένας merchant είναι μία ταυτότητα διαπραγμάτευσης — με τον δικό του σπόρο και ιστορικό ανταλλαγών. Η διαπραγμάτευση κάτω από διαφορετικό merchant διατηρεί τα πλαίσια μη συνδέσιμα (μια ταυτότητα μιας χρήσης). Τα κύρια νομίσματά σας βρίσκονται στο δικό σας πορτοφόλι, όχι εδώ.",
    // First-run welcome (empty merchant manager).
    welcomeTitle: "Καλώς ήρθατε στο Satchel",
    welcomeIntro:
      "Το Satchel διαπραγματεύεται κάτω από έναν «merchant» — μία ταυτότητα διαπραγμάτευσης με τον δικό της σπόρο. Δεν έχετε καμία ακόμα: δημιουργήστε μία νέα, ή εισαγάγετε μια υπάρχουσα φράση ανάκτησης για να ξεκινήσετε.",
    importMerchant: "Εισαγωγή merchant",
    none: "Κανένας merchant ακόμα.",
    switch: "εναλλαγή",
    newMerchant: "Νέος merchant",
    thisMerchant: "αυτός ο merchant",
    nameLabel: "Όνομα merchant",
    namePlaceholder: "π.χ. Κύριος",
    rename: "Μετονομασία",
    introFirst:
      "Ρυθμίστε την πρώτη σας ταυτότητα διαπραγμάτευσης (έναν «merchant»). Κρατά μόνο θερμά κλειδιά διέλευσης για ανταλλαγές σε εξέλιξη — τα κύρια νομίσματά σας παραμένουν στο δικό σας πορτοφόλι.",
    introNew: "Ένας νέος merchant είναι μια φρέσκια, ξεχωριστή ταυτότητα με τον δικό της σπόρο και ιστορικό ανταλλαγών.",
    // UI-5 merchant selector (phoenix wallet-selector parity).
    createNew: "Δημιουργία νέου",
    import: "Εισαγωγή",
    load: "Φόρτωση Merchant",
    loaded: "φορτώθηκε",
    locked: "κλειδωμένος",
    lockedTip: "Κρυπτογραφημένος σπόρος — ξεκλειδώστε με τη συνθηματική φράση σας όταν τον φορτώνετε.",
    close: "Κλείσιμο",
    idLabel: "φάκελος",
    switching: "Εναλλαγή merchant…",
    switchingBody: "Επανεκκίνηση της μηχανής για αυτόν τον φάκελο.",
  },
  // Seed create/import (SeedForm) + counterparty + status.
  seed: {
    intro: "Δημιουργήστε έναν ολοκαίνουργιο σπόρο, ή εισαγάγετε έναν που ήδη έχετε.",
    createNew: "Δημιουργία νέου",
    createDesc: "Δημιουργήστε έναν φρέσκο σπόρο. Δημιουργείτε αντίγραφο ασφαλείας της φράσης ανάκτησης.",
    import: "Εισαγωγή",
    importDesc: "Επαναφορά από μια υπάρχουσα φράση 12/24 λέξεων.",
    recoveryLabel: "Φράση ανάκτησης",
    encrypt: "Κρυπτογράφηση",
    encryptDesc:
      "Μια συνθηματική φράση προστατεύει τον σπόρο σε ηρεμία. Την εισάγετε μία φορά ανά συνεδρία — το Satchel δεν την αποθηκεύει ποτέ. Σημείωση: η αυτόματη επιστροφή χωρίς επίβλεψη παύει μετά από επανεκκίνηση μέχρι να την εισαγάγετε ξανά.",
    noPassphrase: "Χωρίς συνθηματική φράση (συνιστάται)",
    noPassphraseDesc:
      "Η αυτόματη επιστροφή συνεχίζει να λειτουργεί μετά από επανεκκινήσεις χωρίς να χρειάζεται να εισαγάγετε τίποτα — αυτός είναι μόνο ένας θερμός σπόρος διέλευσης. Κόστος: η πρόσβαση στο αρχείο/υπολογιστή εκθέτει τα κλειδιά διέλευσης + την ταυτότητα αυτού του merchant.",
    passphraseLabel: "Συνθηματική φράση",
    passphrasePlaceholder: "επιλέξτε μια συνθηματική φράση",
    revealTitle: "Σημειώστε τη φράση ανάκτησής σας",
    revealBody:
      "Όποιος έχει αυτές τις λέξεις ελέγχει τα θερμά κλειδιά αυτού του merchant. Το Satchel δεν κρατά αντίγραφο — αποθηκεύστε το εκτός σύνδεσης. Στη συνέχεια θα επιβεβαιώσετε μερικές λέξεις.",
    ackLabel: "Έχω σημειώσει τη φράση ανάκτησής μου.",
    // Multi-step onboarding (create/import -> secret -> confirm -> passphrase).
    chooseTitle: "Ρύθμιση {label}",
    enterTitle: "Εισαγάγετε τη φράση ανάκτησής σας",
    enterBody:
      "Πληκτρολογήστε κάθε λέξη — συμπληρώνονται αυτόματα καθώς προχωράτε — ή επικολλήστε ολόκληρη τη φράση. Την ελέγχουμε πριν συνεχίσετε.",
    wordCount: "{n} λέξεις",
    wordAria: "Λέξη {n}",
    checkIncomplete: "Εισαγάγετε και τις {n} λέξεις.",
    checkUnknown: "Ορισμένες λέξεις δεν είναι στη λίστα λέξεων BIP39 — ελέγξτε τις επισημασμένες.",
    checkBadChecksum: "Το άθροισμα ελέγχου δεν ταιριάζει — ελέγξτε ξανά τις λέξεις σας και τη σειρά τους.",
    checkOk: "Η φράση ανάκτησης φαίνεται έγκυρη.",
    verifyTitle: "Επιβεβαιώστε το αντίγραφο ασφαλείας σας",
    verifyBody: "Πληκτρολογήστε τις λέξεις σε αυτές τις θέσεις για να επιβεβαιώσετε ότι σημειώσατε τη φράση.",
    verifyWord: "Λέξη #{n}",
    verifyMismatch: "Αυτές δεν ταιριάζουν με τη φράση σας — ελέγξτε το αντίγραφο ασφαλείας σας.",
    passphraseTitle: "Προστατέψτε τον σπόρο",
    passphraseBody:
      "Προαιρετικά κρυπτογραφήστε τον αποθηκευμένο σπόρο με μια συνθηματική φράση. Μπορείτε να το παραλείψετε — δείτε την αντιστάθμιση παρακάτω.",
  },
  counterparty: {
    you: "Αυτός είστε εσείς",
    youShort: "εσείς",
    unknown: "άγνωστη ταυτότητα",
    // Short fingerprint fallback (identity.ts shortId) when no pubkey is known.
    unknownShort: "άγνωστο",
  },
  contacts: {
    // TODO(i18n): translate — English fallback for now.
    title: "Contacts",
    subtitle: "Your private nicknames for the people you trade with.",
    privacyNote:
      "Contacts are stored only on this device and are never shared, published, or sent to a relay. A nickname is your label — the identicon and fingerprint remain the real identity.",
    searchPlaceholder: "Search nick, note, or key",
    empty: "No contacts yet. Click a counterparty's identicon anywhere to add one.",
    emptyFiltered: "No contacts match this filter.",
    count: "{n} contacts",
    colWho: "Identity",
    colNick: "Nickname",
    colNote: "Notes",
    colStatus: "Standing",
    colAdded: "Added",
    colActions: "",
    filterAll: "All",
    filterTrusted: "Trusted",
    filterBlocked: "Blocked",
    // Corkboard toggle: drop blocked makers' offers from the ladder.
    hideBlocked: "Hide blocked offers",
    statusTrusted: "Trusted",
    statusNeutral: "Neutral",
    statusBlocked: "Blocked",
    menuAdd: "Add to contacts…",
    menuEdit: "Edit contact…",
    menuMarkTrusted: "Mark as trusted",
    menuMarkNeutral: "Mark as neutral",
    menuMarkBlocked: "Block",
    menuCopyKey: "Copy public key",
    menuOpen: "Open in Contacts",
    keyCopied: "Public key copied",
    editTitle: "Edit contact",
    addTitle: "Add contact",
    nickLabel: "Nickname",
    nickPlaceholder: "e.g. Alice from the meetup",
    noteLabel: "Notes",
    notePlaceholder: "Anything you want to remember — how to reach them, past trades…",
    save: "Save",
    cancel: "Cancel",
    remove: "Remove contact",
    removeConfirmTitle: "Remove contact?",
    removeConfirmBody: "This deletes your local nickname and notes for {who}. It can't be undone.",
    blockedWarning: "You blocked this counterparty",
    blockedWarningBody:
      "You marked this person as blocked. Blocking is only a personal reminder — it does not stop the trade. Continue only if you mean to.",
  },
  status: {
    notConnectedTitle: "Δεν υπάρχει σύνδεση με τη μηχανή",
    disconnectedBody:
      "Το Satchel δεν μπορεί να φτάσει τη μηχανή. Μπορεί να εκκινεί ακόμα, ή οι συνδέσεις κόμβων του ενεργού merchant να είναι εκτός λειτουργίας. Δοκιμάστε ξανά, ή αλλάξτε merchant από τον επιλογέα στην κορυφή.",
    openInSatchel: "Ανοίξτε το στο Satchel",
    noTauriBody:
      "Αυτό είναι το περιβάλλον του Satchel — χρειάζεται τη γέφυρα Tauri για να φτάσει τη μηχανή. Εκκινήστε την εφαρμογή για επιτραπέζιο (cargo tauri dev) αντί για πρόγραμμα περιήγησης.",
  },
  settings: {
    title: "Ρυθμίσεις",
    subtitle: "Προτιμήσεις σε επίπεδο εφαρμογής για αυτή την εγκατάσταση.",
    // UI-3 Settings tabs.
    tabGeneral: "Γενικά",
    tabCoins: "Νομίσματα",
    tabNetwork: "Δίκτυο",
    tabAbout: "Σχετικά",
    appearance: "Εμφάνιση",
    theme: "Θέμα",
    themeDark: "Σκούρο",
    themeLight: "Φωτεινό",
    themeSystem: "Σύστημα",
    themeHint: "Επιλέξτε πώς φαίνεται το Satchel. Το «Σύστημα» ακολουθεί τη ρύθμιση του λειτουργικού σας.",
    language: "Γλώσσα",
    languageHint: "Περισσότερες γλώσσες προστίθενται καθώς συνεισφέρονται μεταφράσεις.",
    mode: "Λειτουργία",
    watchOnly: "Λειτουργία μόνο παρακολούθησης",
    watchOnlyHint:
      "Περιηγηθείτε στον πίνακα χωρίς να ρυθμίσετε νομίσματα. Μπορείτε ακόμα να αποσύρετε τις δικές σας προσφορές, αλλά δεν μπορείτε να δημοσιεύσετε, να αποδεχτείτε ή να χρηματοδοτήσετε. Απενεργοποιήστε για διαπραγμάτευση (θα χρειαστείτε τουλάχιστον δύο συνδεδεμένα νομίσματα).",
    network: "Δίκτυο",
    boards: "Corkboards",
    boardsDesc:
      "Προαιρετικοί HTTP πίνακες που φιλοξενείτε μόνοι σας. Προσθέστε όσους εμπιστεύεστε· αφήστε κενό για να βασιστείτε στο Nostr.",
    boardsNone: "Κανένας ρυθμισμένος",
    nostrRelays: "Relays Nostr",
    nostrRelaysDesc:
      "Τα relays μεταφέρουν τον πίνακα ανακοινώσεων μέσω ενός αποκεντρωμένου δικτύου — κανένας διαχειριστής δεν μπορεί να διαβάσει ή να ταιριάξει τις προσφορές σας. Προρυθμισμένο με ένα προεπιλεγμένο σύνολο· επεξεργαστείτε ελεύθερα.",
    nostrRelaysOff: "Ανενεργό — η μεταφορά Nostr είναι απενεργοποιημένη",
    addUrl: "Προσθήκη",
    removeUrl: "Αφαίρεση",
    relayInvalid: "Εισαγάγετε μια διεύθυνση relay ws:// ή wss://",
    boardInvalid: "Εισαγάγετε μια διεύθυνση πίνακα http:// ή https://",
    netSave: "Αποθήκευση & επανασύνδεση",
    netSaving: "Αποθήκευση & επανασύνδεση…",
    netSaved: "Αποθηκεύτηκε",
    // Fees tab — fee-bump policy (per active merchant).
    tabFees: "Τέλη",
    fees: "Αύξηση τελών",
    feesScope: "Αυτές οι ρυθμίσεις ισχύουν για τον ενεργό merchant.",
    feesIntro:
      "Αντισταθμίσεις ασφάλειας/κόστους για τις αυξήσεις τελών, όχι απαραίτητη ρύθμιση. Οι νέες τιμές ισχύουν για μελλοντικές αυξήσεις· οι ανταλλαγές που έχουν ήδη χρηματοδοτηθεί διατηρούν την πολιτική υπό την οποία χρηματοδοτήθηκαν.",
    feeMax: "Μέγιστο feerate (sat/vB)",
    feeMaxHint:
      "Ανώτατο όριο για κάθε αύξηση τέλους. Προεπιλογή 500, που είναι και το σκληρό μέγιστο του συστήματος. Χαμηλώστε το για να περιορίσετε το κόστος.",
    feeReservation: "Δέσμευση αύξησης χρηματοδότησης (×)",
    feeReservationHint:
      "Το υπόλοιπο που ο έλεγχος κεφαλαίων δεσμεύει ως περιθώριο αύξησης. Υψηλότερο διασώζει μεγαλύτερες αυξήσεις τελών αλλά δεσμεύει περισσότερο υπόλοιπο και απορρίπτει περισσότερες ανταλλαγές. Προεπιλογή 3.",
    feeCommitted: "Υπερπρόβλεψη εξαργύρωσης (×)",
    feeCommittedHint:
      "Πόσο επιπλέον προπληρώνεται το τέλος εξαργύρωσης v2 ώστε να επιβεβαιωθεί ακόμα και όταν το Satchel είναι κλειστό. Ισχύει μόνο για νέες ανταλλαγές. Προεπιλογή 2.",
    feeSave: "Αποθήκευση",
    feeSaving: "Αποθήκευση…",
    feeSaved: "Αποθηκεύτηκε",
    feeReset: "Επαναφορά στις προεπιλογές",
    coins: "Νομίσματα & κόμβοι",
    coinsHint: "Συνδέστε κάθε νόμισμα με τον δικό σας κόμβο. Το genesis ελέγχεται πριν αποθηκευτεί οτιδήποτε.",
    about: "Σχετικά",
    version: "Έκδοση {version}",
    updateUpToDate: "Ενημερωμένο",
    updateCheckPlaceholder: "Ο έλεγχος ενημερώσεων φτάνει σε μεταγενέστερη έκδοση.",
    trustModel: "Πού βρίσκονται τα κλειδιά σας",
    trustModelBody:
      "Τα μυστικά βρίσκονται στη μηχανή, ποτέ στο Satchel. Ο σπόρος του merchant βρίσκεται στον φάκελο δεδομένων της μηχανής (κρυπτογραφημένος ή σε απλό κείμενο — δική σας επιλογή)· το Satchel δεν αποθηκεύει σπόρο ή συνθηματική φράση. Ο σπόρος είναι θερμός εκ σχεδιασμού (μόνο κλειδιά διέλευσης) — μεταφέρετε σημαντικά έσοδα στο δικό σας ψυχρό πορτοφόλι.",
  },
  coins: {
    intro:
      "Συνδέστε κάθε νόμισμα με τον δικό σας κόμβο. Η πρώτη διεύθυνση URL είναι το ίδιο το πορτοφόλι του κόμβου σας — χρηματοδοτεί τα σκέλη ανταλλαγής σας και λαμβάνει τα έσοδα. Πριν αποθηκευτεί οτιδήποτε, το Satchel ελέγχει το genesis block του κόμβου ώστε τα κεφάλαια να μην μπορούν ποτέ να σταλούν σε λάθος αλυσίδα. Οι συνδέσεις είναι κοινές μεταξύ όλων των merchants σας.",
    networkBadge: "Ρύθμιση για το δίκτυο {network}",
    needMerchant:
      "Συνδέστε πρώτα έναν merchant — η ρύθμιση νομισμάτων χρειάζεται τη μηχανή σε λειτουργία. Χρησιμοποιήστε τον επιλογέα merchant πάνω δεξιά.",
    pairsTitle: "Ζεύγη διαπραγμάτευσης",
    pairsHint:
      "Τα ζεύγη προκύπτουν από το τι μπορεί να κάνει κάθε νόμισμα — δεν υπάρχει σταθερή λίστα. Ένα ζεύγος ανοίγει μόλις συνδεθούν και τα δύο νομίσματά του.",
    noPairs: "Δεν υπάρχουν διαθέσιμα ζεύγη.",
    notSetUp: "Δεν έχει ρυθμιστεί",
    connectedTip: "Συνδεδεμένο · κορυφή {tip}",
    connError: "Σφάλμα σύνδεσης",
    setUp: "Ρύθμιση",
    editConnection: "Επεξεργασία σύνδεσης",
    remove: "αφαίρεση",
    disconnectTip: "Αποσυνδέστε αυτό το νόμισμα",
    disconnectTitle: "Αποσύνδεση {coin};",
    disconnectBody: "Οι ανταλλαγές που το χρειάζονται δεν θα είναι διαθέσιμες μέχρι να επανασυνδεθείτε.",
    ready: "Έτοιμο για διαπραγμάτευση",
    connectMissing: "Συνδέστε {coins}",
    notBuildable: "Δεν είναι ακόμα εφικτό",
    // Swap-protocol chips on a pair (pact-htlc-v1 HTLC vs pact-htlc-v2 adaptor).
    protoPrivate: "Ιδιωτική (Taproot)",
    protoPrivateTip: "Ιδιωτική ανταλλαγή (προσαρμογέας Taproot/MuSig2) — μοιάζει με μια συνηθισμένη πληρωμή on-chain",
    protoHtlcTip: "Κλασική ανταλλαγή HTLC",
    // Coin-setup backend choices (CoinSetup).
    // CoinSetup dialog.
    setupTitle: "Σύνδεση {coin}",
    setupIntro:
      "Κατευθύνετε το Satchel στον δικό σας κόμβο {sym}. Τίποτα δεν αποθηκεύεται μέχρι ο κόμβος να περάσει τον έλεγχο genesis block — τα κεφάλαιά σας αγγίζουν πάντα μόνο την πραγματική αλυσίδα {sym}.",
    confirmationsLabel: "Επιβεβαιώσεις πριν την οριστικοποίηση",
    confirmationsHint:
      "Πόσο βαθιά πρέπει να είναι μια χρηματοδότηση ή εξαργύρωση σε αυτή την αλυσίδα πριν δράσει πάνω της μια ανταλλαγή — το περιθώριο ασφαλείας έναντι reorg. Υψηλότερο είναι ασφαλέστερο αλλά πιο αργό· αφήστε κενό για τη συνιστώμενη προεπιλογή ({default}).",
    validateNode: "Επικύρωση κόμβου",
    checking: "Έλεγχος του κόμβου…",
    genesisOk: "Το genesis ταίριαξε — αυτή είναι η σωστή αλυσίδα",
    genesisDetail: "ύψος κορυφής {tip} · genesis {hash}…",
    genesisBad: "Απορρίφθηκε — δεν αποθηκεύεται",
    errorShort: "σφάλμα",
    // Structured connection form (CoinSetup v2).
    rpcHostLabel: "Διεύθυνση RPC",
    rpcPortLabel: "Θύρα RPC",
    authMethodLabel: "Έλεγχος ταυτότητας",
    authCookie: "Αρχείο cookie",
    authCookieDesc: "Αυτόματη ανάγνωση του .cookie του κόμβου από τον κατάλογο δεδομένων του (η προεπιλογή, δεν αποθηκεύεται κωδικός).",
    authUserPass: "Χρήστης / κωδικός",
    authUserPassDesc: "Το rpcuser / rpcpassword από τη ρύθμιση του κόμβου σας — απαιτείται για απομακρυσμένο κόμβο.",
    rpcUserLabel: "Όνομα χρήστη RPC",
    rpcPasswordLabel: "Κωδικός RPC",
    datadirLabel: "Κατάλογος δεδομένων κόμβου",
    cookiePathNote: "Το cookie διαβάζεται από {path} κάτω από αυτόν τον κατάλογο.",
    walletLabel: "Όνομα πορτοφολιού (προαιρετικό)",
    walletPlaceholder: "το πορτοφόλι του κόμβου σας",
    needPort: "Εισαγάγετε πρώτα τη θύρα RPC.",
    validateFirst: "Επικυρώστε τον κόμβο πριν την αποθήκευση.",
    savingReconnecting: "Αποθήκευση & επανασύνδεση…",
    connected: "{coin} συνδεδεμένο",
    // Template picker (a coins.toml coin the engine version doesn't support).
    unsupportedByEngine: "Μη υποστηριζόμενο",
    unsupportedByEngineTip:
      "Αυτό το νόμισμα ορίζεται στο coins.toml αλλά δεν είναι ενσωματωμένο σε αυτή την έκδοση της μηχανής, οπότε δεν μπορεί να διαπραγματευτεί.",
  },
  coinWizard: {
    title: "Συνδέστε τα νομίσματά σας",
    intro:
      "Επιλέξτε τουλάχιστον δύο νομίσματα και κατευθύνετε καθένα στον δικό σας κόμβο. Μια ανταλλαγή χρειάζεται δύο αλυσίδες, οπότε η διαπραγμάτευση ξεκλειδώνει μόλις συνδεθούν και είναι ενεργοί δύο κόμβοι. Μπορείτε να προσθέσετε ή να αλλάξετε νομίσματα αργότερα στις Ρυθμίσεις.",
    progress: "{count} από {min} νομίσματα συνδεδεμένα",
    continue: "Συνέχεια",
    live: "Ενεργό",
    nodeDown: "Κόμβος εκτός",
  },
  wallets: {
    intro:
      "Αυτά είναι τα πορτοφόλια των δικών σας κόμβων (αυτά που χρησιμοποιεί η μηχανή για να χρηματοδοτεί ανταλλαγές και να λαμβάνει έσοδα) — τα κλειδιά σας, η μηχανή σας. Το Satchel δεν κρατά ποτέ τα νομίσματά σας.",
    hotSeedNudge:
      "Αυτό είναι ένα πορτοφόλι δαπανών σε θερμό σπόρο, όχι θησαυροφυλάκιο — μεταφέρετε σημαντικά υπόλοιπα στο δικό σας ψυχρό/core πορτοφόλι.",
    notConnected: "Δεν υπάρχει σύνδεση",
    notConnectedBody: "Συνδέστε πρώτα έναν merchant — η προβολή πορτοφολιού χρειάζεται τη μηχανή σε λειτουργία.",
    noCoins: "Δεν έχουν ρυθμιστεί νομίσματα ακόμα",
    noCoinsBody: "Συνδέστε ένα νόμισμα στις Ρυθμίσεις → Νομίσματα και το πορτοφόλι του εμφανίζεται εδώ.",
    goToCoins: "Μετάβαση στα Νομίσματα",
    watchOnlyTitle: "Κανένα πορτοφόλι σε λειτουργία μόνο παρακολούθησης",
    watchOnlyBody:
      "Αυτή είναι μια συνεδρία μόνο παρακολούθησης χωρίς συνδεδεμένα νομίσματα, οπότε δεν υπάρχουν πορτοφόλια να εμφανιστούν. Απενεργοποιήστε τη λειτουργία μόνο παρακολούθησης στις Ρυθμίσεις και συνδέστε ένα νόμισμα για να χρηματοδοτείτε ανταλλαγές.",
    walletName: "πορτοφόλι · {wallet}",
    walletScopedHint: "Κάθε RPC για αυτό το νόμισμα περιορίζεται σε αυτό το πορτοφόλι κόμβου.",
    walletDefault: "προεπιλεγμένο πορτοφόλι (χωρίς περιορισμό)",
    walletDefaultHint:
      "Δεν έχει οριστεί πορτοφόλι για αυτό το νόμισμα, οπότε τα RPC χρησιμοποιούν το προεπιλεγμένο πορτοφόλι του κόμβου. Ορίστε ένα στις Ρυθμίσεις → Νομίσματα για να περιορίσετε κάθε κλήση σε ένα συγκεκριμένο πορτοφόλι.",
    balanceLabel: "υπόλοιπο {symbol}",
  },
  corkboard: {
    noBoardTitle: "Δεν υπάρχει συνδεδεμένο Corkboard",
    noBoardBody:
      "Ένα Corkboard είναι ένας κοινός πίνακας ανακοινώσεων όπου οι makers καρφιτσώνουν προσφορές. Δεν ταιριάζει ποτέ συναλλαγές ούτε κρατά νομίσματα — κατευθύνετε το Satchel σε ένα που εμπιστεύεστε για να περιηγηθείτε και να δημοσιεύσετε.",
    noPairs: "Δεν υπάρχουν διαθέσιμα ζεύγη",
    board: "Corkboard",
    boardSettings: "Ρύθμιση στις Ρυθμίσεις",
    filterAll: "Όλες",
    filterMine: "Δικές μου",
    noOffers: "Καμία προσφορά που μπορείτε να αποδεχτείτε αυτή τη στιγμή",
    noOffersBody:
      "Οι προσφορές εμφανίζονται εδώ μόλις ένας maker δημοσιεύσει μία για ένα ζεύγος που έχετε ρυθμίσει. Μπορείτε επίσης να δημοσιεύσετε τη δική σας.",
    yourOffer: "η προσφορά σας",
    offerStaged: "δημοσίευση…",
    offerStagedTip:
      "Δημοσιεύτηκε από αυτή τη συσκευή και αναμένει επιβεβαίωση από ένα relay. Διαφημίζεται· γίνεται ενεργή μόλις ένα relay την αναμεταδώσει.",
    take: "Αποδοχή προσφοράς",
    legDown: "Ο κόμβος ενός από αυτό το ζεύγος είναι εκτός λειτουργίας — εκκινήστε τον (ή ελέγξτε Ρυθμίσεις → Νομίσματα) πριν την αποδοχή.",
    withdraw: "Απόσυρση",
    withdrawTip: "Απόσυρση άμεσα — μια προσφορά δεν κλειδώνει ποτέ κεφάλαια",
    safetyRefund: "επιστροφή ασφαλείας",
    safetyRefundTip:
      "Αν η ανταλλαγή κολλήσει, και οι δύο πλευρές επιστρέφουν αυτόματα — το σκέλος του taker ξεκλειδώνει πρώτο, το δικό σας λίγο αργότερα. Κανείς δεν μένει εγκλωβισμένος.",
    activeTitle: "Οι ενεργές σας ανταλλαγές",
    states: {
      takenByUs: "αποδεκτή από εσάς",
      revoked: "αποσυρμένη",
      expired: "ληγμένη",
    },
    // Two-sided order-book view of the Corkboard. Bids = makers giving the
    // quote coin to get the base; asks = the reverse. The ladder is a way to
    // READ the board — it never matches or prioritises (load-bearing).
    book: {
      bids: "Προσφορές αγοράς",
      asks: "Προσφορές πώλησης",
      bidsHint: "θέλουν {base} · πληρώνοντας {quote}",
      asksHint: "πωλούν {base} · έναντι {quote}",
      price: "Τιμή",
      size: "Μέγεθος",
      noBids: "Καμία προσφορά αγοράς",
      noAsks: "Καμία προσφορά πώλησης",
      spread: "Διαφορά {pct}",
      spreadOneSided: "Μονόπλευρη",
      crossed: "διασταυρωμένη",
      crossedTip: "Η κορυφαία προσφορά αγοράς ≥ η κορυφαία προσφορά πώλησης. Ο πίνακας δεν ταιριάζει ποτέ αυτόματα, οπότε αυτές οι επικαλυπτόμενες προσφορές απλώς παραμένουν — αποδεχτείτε οποιαδήποτε πλευρά.",
      mid: "μέση {price}",
      levelOffers: "{count} προσφορά(ές) σε αυτή την τιμή — επιλέξτε μία για αποδοχή",
      depthTip: "Συνολικά {sym} προς διάθεση σε αυτή την τιμή σε {count} αγγελία(ες).",
      selectLevel: "Επιλέξτε ένα επίπεδο τιμής παραπάνω για να δείτε τις προσφορές εκεί.",
      paneHeader: "{size} {base} @ {price} {unit}",
      denomTip: "Μονάδα εμφάνισης για ποσά {coin}",
      showMore: "Εμφάνιση {count} ακόμα",
      showLess: "Εμφάνιση των κορυφαίων {count}",
    },
  },
  relays: {
    title: "Relays",
    subtitle: "Ζωντανή συνδεσιμότητα με τα relays Nostr σας — το δίκτυο μέσω του οποίου ταξιδεύουν οι προσφορές και οι αποδοχές σας. Προσθέστε ή αφαιρέστε relays στις Ρυθμίσεις → Δίκτυο.",
    connectedCount: "{up} / {total} συνδεδεμένα",
    refresh: "Ανανέωση",
    ms: "{ms} ms",
    up: "ενεργό",
    down: "ανενεργό",
    statsTip: "{success}/{attempts} επιτυχείς συνδέσεις · ↓{down} ↑{up}",
    none: "Δεν έχουν ρυθμιστεί relays",
    noneBody: "Προσθέστε ένα relay Nostr στις Ρυθμίσεις → Δίκτυο για να δημοσιεύετε και να λαμβάνετε προσφορές μέσω του δικτύου.",
    goToNetwork: "Μετάβαση στις Ρυθμίσεις",
    notConnected: "Δεν υπάρχει σύνδεση",
    notConnectedBody: "Η προβολή relay χρειάζεται τη μηχανή σε λειτουργία — συνδέστε πρώτα έναν merchant.",
  },
  swaps: {
    maker: "Maker",
    taker: "Taker",
    title: "Ανταλλαγές",
    hint: "Το πλήρες καθολικό σας — οι ανταλλαγές σε εξέλιξη στην κορυφή, οι ολοκληρωμένες συναλλαγές παρακάτω. Μπορείτε επίσης να ενεργείτε σε ενεργές ανταλλαγές από το Corkboard.",
    activeTitle: "Σε εξέλιξη",
    historyTitle: "Ιστορικό",
    none: "Καμία ανταλλαγή ακόμα — αποδεχτείτε μια προσφορά στο Corkboard.",
    // Active-swaps dock action buttons + the refund-time label.
    cancel: "ακύρωση",
    refund: "επιστροφή",
    dump: "εξαγωγή αρχείων καταγραφής",
    dumpHint: "Αντιγράψτε ένα πακέτο διαγνωστικών χωρίς μυστικά (κατάσταση + γραμμές καταγραφής) για αυτή την ανταλλαγή, για επικόλληση στους προγραμματιστές.",
    dumpCopied: "Τα διαγνωστικά αντιγράφηκαν — επικολλήστε τα στους προγραμματιστές.",
    dumpFailed: "Δεν ήταν δυνατή η αντιγραφή του πακέτου διαγνωστικών.",
    refundAt: "επιστροφή {when}",
    // Confirm dialogs for acting on a live swap (ActiveSwaps).
    cancelTitle: "Ακύρωση αυτής της ανταλλαγής;",
    cancelConfirm: "Ακύρωση ανταλλαγής",
    cancelKeep: "Διατήρηση",
    // Abort reason recorded on the swap when cancelled from Satchel.
    cancelReason: "ακυρώθηκε στο Satchel",
    cancelBody:
      "Αυτό εγκαταλείπει την ανταλλαγή πριν χρηματοδοτήσετε. Τίποτα δικό σας δεν είναι ακόμα κλειδωμένο, οπότε δεν χάνετε τίποτα — απλώς η προσφορά δεν θα ολοκληρωθεί.",
    refundTitle: "Ανάκτηση των κεφαλαίων σας;",
    refundConfirm: "Επιστροφή",
    refundBody:
      "Το χρονοκλείδωμα ασφαλείας έχει παρέλθει, οπότε μπορείτε να ανακτήσετε τα κεφάλαια που κλειδώσατε. Αυτό μεταδίδει την επιστροφή σας τώρα· η μηχανή το κάνει επίσης αυτόματα μετά την προθεσμία.",
    col: {
      swap: "ανταλλαγή",
      role: "ρόλος",
      state: "κατάσταση",
      amounts: "δίνει → λαμβάνει",
      when: "πότε",
      finalTx: "τελική συναλλαγή",
    },
    // Expandable per-leg on-chain detail (the audit trail). We show both
    // funding txs + OUR settlement; never the counterparty's settlement or the
    // swap secret.
    audit: {
      toggle: "Εμφάνιση λεπτομέρειας on-chain",
      title: "Λεπτομέρεια on-chain",
      youLocked: "κλειδώσατε",
      theyLocked: "κλείδωσαν",
      funding: "Χρηματοδότηση",
      received: "Ελήφθη",
      refunded: "Επιστράφηκε",
      pending: "όχι ακόμα on-chain",
      copy: "Αντιγραφή αναγνωριστικού συναλλαγής",
      copied: "Το αναγνωριστικό συναλλαγής αντιγράφηκε",
    },
  },
  fees: {
    title: "Προεπισκόπηση κόστους δικτύου",
    estimated: "εκτιμώμενο",
    provisionalNote: "Αυτή η έκδοση pactd δεν εκθέτει ακόμα εκτίμηση τελών.",
    summary: "Μια ανταλλαγή είναι 2 on-chain συναλλαγές που πληρώνετε: χρηματοδότηση στην αλυσίδα που δίνετε, εξαργύρωση στην αλυσίδα που λαμβάνετε.",
    fallbackTip: "Ένας κόμβος ήταν απρόσιτος, οπότε χρησιμοποιήθηκε ένα συντηρητικό προεπιλεγμένο feerate — αντιμετωπίστε τα ως εκτίμηση.",
    ifItStalls: "(αν κολλήσει)",
  },
  funds: {
    insufficient:
      "Δεν υπάρχουν αρκετά {sym} για τη χρηματοδότηση αυτής της ανταλλαγής — χρειάζονται ~{need} {sym} (ποσό + τέλος χρηματοδότησης), το πορτοφόλι έχει {have} {sym}.",
  },
  wizard: {
    back: "Πίσω",
    continue: "Συνέχεια",
  },
  // UI-4 docked activity log.
  log: {
    title: "Δραστηριότητα",
    empty: "— αρχείο καταγραφής δραστηριότητας —",
    count: "{count} γραμμές",
    collapse: "Σύμπτυξη αρχείου καταγραφής",
    expand: "Ανάπτυξη αρχείου καταγραφής",
    // Activity-log lines emitted by the frontend. `{err}` carries a raw engine
    // message (itself not translated — it comes from pactd over the wire).
    noTauri: "δεν εκτελείται μέσα στο Satchel — αυτό το περιβάλλον χρειάζεται τη γέφυρα Tauri",
    startupError: "εκκίνηση: {err}",
    notConnected: "χωρίς σύνδεση: {err}",
    connected: "συνδέθηκε με το pactd {version} ({protocol})",
    listcoinsError: "listcoins: {err}",
    watchOnlyError: "μόνο παρακολούθηση: {err}",
    switchedMerchant: "έγινε εναλλαγή στον merchant {id}",
    renamedMerchant: "ο merchant μετονομάστηκε σε {name}",
    renameMerchantError: "μετονομασία merchant: {err}",
    switchMerchantError: "εναλλαγή merchant: {err}",
    loadMerchantError: "φόρτωση merchant: {err}",
    merchantCreated: "δημιουργήθηκε ο merchant {id}",
    merchantReady: "ο merchant είναι έτοιμος",
    actionOk: "{action} {id}: εντάξει",
    actionError: "{action} {id}: {err}",
    diagCopied: "τα διαγνωστικά για το {id} αντιγράφηκαν ({count} γραμμές καταγραφής) — επικολλήστε τα στους προγραμματιστές",
    dumpError: "εξαγωγή {id}: {err}",
    coinDisconnected: "{coin} αποσυνδέθηκε",
    removeCoinError: "αφαίρεση νομίσματος: {err}",
    tookOffer: "αποδεχτήκατε την προσφορά {id} — εμφανίζεται τώρα στις ενεργές ανταλλαγές σας παρακάτω",
    takeError: "αποδοχή: {err}",
    offerWithdrawn: "η προσφορά {id} αποσύρθηκε",
    withdrawError: "απόσυρση: {err}",
    postedOffer: "δημοσιεύτηκε η προσφορά {id} — αποσύρετέ την οποτεδήποτε· τίποτα δεν είναι κλειδωμένο",
    createdSlip: "δημιουργήθηκε ένα ιδιωτικό απόκομμα προσφοράς — στείλτε το στον φίλο σας",
    tookPrivateOffer: "αποδεχτήκατε την ιδιωτική προσφορά {id} — εμφανίζεται τώρα στις ενεργές ανταλλαγές σας",
    cancelledPrivateOffer: "ακυρώθηκε η ιδιωτική προσφορά {id}",
    cancelError: "ακύρωση: {err}",
    noticeboardUpdated: "ο πίνακας ανακοινώσεων ενημερώθηκε",
    feePolicyUpdated: "η πολιτική τελών ενημερώθηκε",
  },
  // Relative-time + freshness prose from format.ts (rendered via the tr() mirror,
  // since those are pure non-component helpers). Unit letters stay inside the
  // template so a translation owns the whole phrase.
  format: {
    ageUnknown: "άγνωστη ηλικία",
    justNow: "μόλις τώρα",
    minutesAgo: "πριν {n}λ",
    hoursAgo: "πριν {n}ώ",
    daysAgo: "πριν {n}η",
    expiryNow: "τώρα",
    expirySoon: "σύντομα",
    inMinutes: "σε ~{n}λ",
    inHours: "σε ~{n}ώ",
    inDays: "σε ~{n}η",
    posted: "δημοσιεύτηκε {age}",
    expires: "λήγει {time}",
  },
  // Plain-language swap story per (role, state) — the honest "who is exposed
  // when" framing shown on every active swap. {a}/{b} are coin tickers; {t1}/{t2}
  // are local refund times. Rendered via tr() (narrate() is a pure helper).
  narrate: {
    finalizing: "Διεκδικήσατε τα {got} σας — τελικές επιβεβαιώσεις. Κρατήστε την εφαρμογή ανοιχτή μέχρι να θαφτεί· τα {gave} σας παραμένουν προστατευμένα μέχρι τότε.",
    initiating:
      "Η αποδοχή στάλθηκε — αναμονή να ξεκινήσει την ανταλλαγή ο maker. Τίποτα δεν είναι ακόμα κλειδωμένο· ακυρώνεται από μόνη της αν δεν απαντήσουν.",
    created: "Η προσφορά στάλθηκε — αναμονή να συμφωνήσει η άλλη πλευρά. Τίποτα δεν έχει δεσμευτεί.",
    acceptedMaker: "Οι όροι συμφωνήθηκαν. Επόμενο: κλειδώστε τα {a} σας. Μέχρι να χρηματοδοτήσετε, μπορείτε ακόμα να ακυρώσετε ελεύθερα.",
    acceptedTaker: "Οι όροι συμφωνήθηκαν. Η άλλη πλευρά κλειδώνει τα {a} της πρώτη — εσείς δεν στέλνετε ποτέ πρώτοι.",
    noncesExchanged:
      "Ρύθμιση της ιδιωτικής ανταλλαγής — ανταλλαγή υλικού υπογραφής. Τίποτα δεν είναι ακόμα κλειδωμένο.",
    signedMaker:
      "Και οι δύο πλευρές υπέγραψαν. Ο daemon σας κλειδώνει τα {a}, μετά διεκδικεί τα {b} αυτόματα. Αν κάτι κολλήσει, τα {a} σας επιστρέφουν στις {t1}.",
    signedTaker:
      "Και οι δύο πλευρές υπέγραψαν. Ο daemon σας κλειδώνει τα {b} και διεκδικεί τα {a} τη στιγμή που κινείται η άλλη πλευρά. Δίχτυ ασφαλείας: επιστροφή στις {t2}.",
    fundedAMaker:
      "Τα {a} σας είναι κλειδωμένα. Αναμονή να κλειδώσει η άλλη πλευρά τα {b} της. Αν δεν το κάνουν ποτέ, τα {a} σας επιστρέφουν αυτόματα στις {t1}.",
    fundedATaker:
      "Τα {a} τους είναι κλειδωμένα και επαληθευμένα. Επόμενο: κλειδώστε τα {b} σας. Δίχτυ ασφαλείας: αυτόματη επιστροφή στις {t2} αν κάτι κολλήσει.",
    fundedBMaker: "Και τα δύο κλειδώθηκαν. Ο daemon σας διεκδικεί τα {b} μόλις επιβεβαιωθούν με ασφάλεια.",
    fundedBTaker: "Και τα δύο κλειδώθηκαν. Ο daemon σας θα διεκδικήσει τα {a} τη στιγμή που η άλλη πλευρά πάρει τα {b} της.",
    completed: "Η ανταλλαγή ολοκληρώθηκε — τα {coin} είναι στο πορτοφόλι σας.",
    refunded: "Η ανταλλαγή δεν ολοκληρώθηκε, οπότε τα {coin} σας επέστρεψαν αυτόματα. Δεν χάθηκε τίποτα εκτός των τελών.",
    aborted: "Ακυρώθηκε πριν κινηθούν χρήματα.",
  },
  progress: {
    awaitingLock: "Αναμονή για το κλείδωμά τους",
    awaitingClaim: "Αναμονή για την εξαργύρωσή τους",
    theirLock: "Επιβεβαίωση του κλειδώματός τους",
    ourLock: "Επιβεβαίωση του κλειδώματός σας",
    securing: "Διασφάλιση των {coin} σας",
    blocks: "+{n} μπλοκ",
    feeBumped: "Αύξηση τέλους",
    reorg: "Εντοπίστηκε reorg — επανέλεγχος",
  },
  exit: {
    // Exit-gate dialog (fund safety, C6). The engine manages alone, so "keep
    // running" detaches it (it keeps watching timelocks + servicing offers).
    liveTitle: "Μια ανταλλαγή είναι σε εξέλιξη",
    liveBodyOne:
      "1 ανταλλαγή είναι σε εξέλιξη. Διέπεται από on-chain χρονοκλειδώματα — η μηχανή πρέπει να συνεχίσει να εκτελείται για να εξαργυρώσει ή να επιστρέψει πριν την προθεσμία.",
    liveBodyMany:
      "{count} ανταλλαγές είναι σε εξέλιξη. Διέπονται από on-chain χρονοκλειδώματα — η μηχανή πρέπει να συνεχίσει να εκτελείται για να εξαργυρώσει ή να επιστρέψει πριν την προθεσμία.",
    keepRunningExplain:
      "Το κλείσιμο του παραθύρου διατηρεί τη μηχανή σε λειτουργία στο παρασκήνιο, ώστε να ολοκληρώσει την ανταλλαγή χωρίς διεπαφή. Μπορείτε να ανοίξετε ξανά το Satchel οποτεδήποτε για να την ελέγξετε.",
    forceQuitWarn: "Η βίαιη έξοδος τώρα σταματά τη μηχανή και μπορεί να οδηγήσει σε απώλεια κεφαλαίων.",
    // {word} is the confirm word below; a translation may localize both together.
    typeToConfirm: "Για βίαιη έξοδο ούτως ή άλλως, πληκτρολογήστε {word} παρακάτω.",
    confirmWord: "QUIT",
    keepRunning: "Διατήρηση λειτουργίας, κλείσιμο παραθύρου",
    keepWithdraw: "Διατήρηση λειτουργίας + απόσυρση προσφορών",
    keepLeaveOffers: "Διατήρηση λειτουργίας, οι προσφορές παραμένουν",
    forceQuit: "Βίαιη έξοδος",
    offersTitle: "Έχετε δημοσιευμένες προσφορές",
    offersBodyOne:
      "1 προσφορά σας βρίσκεται ακόμα στο Corkboard. Οι προσφορές δεν κλειδώνουν τίποτα, αλλά αν την αφήσετε σημαίνει ότι αντισυμβαλλόμενοι μπορούν ακόμα να την αποδεχτούν όσο το Satchel είναι κλειστό — η μηχανή θα εξυπηρετήσει την αποδοχή.",
    offersBodyMany:
      "{count} προσφορές σας βρίσκονται ακόμα στο Corkboard. Οι προσφορές δεν κλειδώνουν τίποτα, αλλά αν τις αφήσετε σημαίνει ότι αντισυμβαλλόμενοι μπορούν ακόμα να τις αποδεχτούν όσο το Satchel είναι κλειστό — η μηχανή θα εξυπηρετήσει τις αποδοχές.",
    withdrawExit: "Απόσυρση όλων & έξοδος",
  },
  unlock: {
    title: "Ξεκλείδωμα merchant",
    body:
      "Ο σπόρος αυτού του merchant είναι κρυπτογραφημένος. Εισαγάγετε τη συνθηματική του φράση για να τον ξεκλειδώσετε για αυτή τη συνεδρία — το Satchel τον κρατά μόνο στη μνήμη και τον ξεχνά κατά την έξοδο.",
    switchMerchant: "Εναλλαγή merchant",
    unlock: "Ξεκλείδωμα",
  },
  common: {
    cancel: "Ακύρωση",
    confirm: "Επιβεβαίωση",
    save: "Αποθήκευση",
    done: "Έγινε",
    later: "Αργότερα",
    retry: "Επανάληψη σύνδεσης",
  },
};
