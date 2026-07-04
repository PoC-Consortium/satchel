import { useEffect, useRef, useState } from "react";
import {
  Autocomplete,
  Box,
  Button,
  Checkbox,
  createFilterOptions,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
  FormControlLabel,
  Stack,
  TextField,
  ToggleButton,
  ToggleButtonGroup,
  Typography,
} from "@mui/material";
import ChoiceCard from "../components/ChoiceCard";
import { errMsg, rpc } from "../api/tauri";
import { BIP39_WORDS, isBip39Word, isValidMnemonic } from "../bip39";
import { useApp } from "../AppContext";
import { useT } from "../i18n";
import { C } from "../theme";

// Prefix match, capped — the BIP39 list is 2048 words; show a short suggestion
// list as the user types (Phoenix-style autocomplete).
const filterWords = createFilterOptions<string>({ matchFrom: "start", limit: 8 });

// Provision the (already active) merchant's seed, Phoenix-style and stepwise:
//   choose (create | import)
//   create:  reveal mnemonic (+ "written down") -> verify 3 random words -> passphrase
//   import:  enter phrase (numbered word grid, like the reveal grid but editable;
//            per-word autocomplete + typo flagging + checksum gate) -> passphrase
// The passphrase step is the OPTIONAL at-rest encryption (not a BIP39 word).
// For create we generate the mnemonic WITHOUT persisting (generateseed) so it
// can be confirmed first; both paths commit via importseed once the passphrase
// step is done. Shared by the first-run wizard and the phase-"seed" resume gate.

type SeedMode = "create" | "import";
type Step = "choose" | "reveal" | "verify" | "enter" | "passphrase";

// Pick 3 distinct word positions to quiz, deterministically sorted for display.
function pickVerifyIndices(n: number): number[] {
  const idx = new Set<number>();
  while (idx.size < Math.min(3, n)) idx.add(Math.floor(Math.random() * n));
  return [...idx].sort((a, b) => a - b);
}

export default function SeedForm({
  label,
  mode: presetMode,
  onDone,
  onBack,
  onLater,
}: {
  label: string;
  /** When set (wizard flow), the create/import choice was already made upstream
   *  — skip the "choose" step. Unset (resume gate) → start at "choose". */
  mode?: SeedMode;
  onDone: () => void | Promise<void>;
  /** Back from the first seed step (preset flow) — returns to the name step. */
  onBack?: () => void;
  onLater?: () => void;
}) {
  const { log } = useApp();
  const t = useT();
  const [mode, setMode] = useState<SeedMode>(presetMode ?? "create");
  const [step, setStep] = useState<Step>(
    presetMode === "import" ? "enter" : presetMode === "create" ? "reveal" : "choose",
  );

  // Preset create: generate the phrase on mount (the choose step is skipped).
  useEffect(() => {
    if (presetMode === "create") void startCreate();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  const [mnemonic, setMnemonic] = useState(""); // generated (create) or typed (import)
  // Import path: a fixed-length grid of word slots (12 or 24), the source of
  // truth for the imported phrase. `mnemonic` is kept in sync from these.
  const [wordCount, setWordCount] = useState(12);
  const [entryWords, setEntryWords] = useState<string[]>(() => Array(12).fill(""));
  const [ack, setAck] = useState(false);
  const [verifyIdx, setVerifyIdx] = useState<number[]>([]);
  const [verifyIn, setVerifyIn] = useState<string[]>(["", "", ""]);
  const [encrypt, setEncrypt] = useState(false);
  const [passphrase, setPassphrase] = useState("");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState("");
  // Regtest is a throwaway network — skip the "confirm you wrote it down"
  // word-verification step there (pure friction when testing). getinfo carries
  // the launch network even during first-run (AppContext.network isn't set yet
  // at the wizard); default to NOT skipping if it can't be read.
  const [skipVerify, setSkipVerify] = useState(false);
  useEffect(() => {
    rpc<{ network?: string }>("getinfo")
      .then((gi) => setSkipVerify(gi.network === "regtest"))
      .catch(() => {});
  }, []);

  const words = mnemonic.trim() ? mnemonic.trim().split(/\s+/) : [];

  // Import: commit a new set of word slots and mirror them into `mnemonic`
  // (the value `commit()` persists). Empty trailing slots are dropped from the
  // phrase; the checksum gate below still requires every slot filled.
  function setEntry(next: string[]) {
    setEntryWords(next);
    setMnemonic(next.map((w) => w.trim().toLowerCase()).filter(Boolean).join(" "));
  }

  // Switch between a 12- and 24-word phrase, preserving any words already typed.
  function changeCount(n: number) {
    setWordCount(n);
    const next = entryWords.slice(0, n);
    while (next.length < n) next.push("");
    setEntry(next);
  }

  // create → generate a fresh phrase (NOT persisted yet) and reveal it.
  async function startCreate(count = wordCount) {
    setErr("");
    setBusy(true);
    try {
      const r = await rpc<{ mnemonic: string }>("generateseed", [count]);
      setMnemonic(r.mnemonic);
      setVerifyIdx(pickVerifyIndices(r.mnemonic.trim().split(/\s+/).length));
      setVerifyIn(["", "", ""]);
      setAck(false);
      setStep("reveal");
    } catch (e) {
      setErr(errMsg(e));
    } finally {
      setBusy(false);
    }
  }

  function onChoose() {
    if (mode === "create") void startCreate();
    else setStep("enter");
  }

  const verifyOk = verifyIdx.length > 0 && verifyIdx.every((idx, k) => verifyIn[k].trim().toLowerCase() === words[idx]);

  // The terminal action: persist the (created or imported) seed via importseed.
  async function commit() {
    const pass = encrypt ? passphrase : "";
    if (encrypt && pass.length < 1) {
      setErr(t("seed.passphrasePlaceholder"));
      return;
    }
    const phrase = mnemonic.trim();
    if (!phrase) {
      setErr(t("seed.recoveryLabel"));
      return;
    }
    setErr("");
    setBusy(true);
    try {
      await rpc("importseed", [phrase, pass]);
      log(t("log.merchantReady"));
      await onDone();
    } catch (e) {
      setErr(errMsg(e));
      setBusy(false);
    }
  }

  // ---- step: choose create vs import ----
  if (step === "choose") {
    return (
      <>
        <DialogTitle>{t("seed.chooseTitle", { label })}</DialogTitle>
        <DialogContent>
          <DialogContentText sx={{ mb: 2 }}>{t("seed.intro")}</DialogContentText>
          <Stack direction="row" spacing={1.5}>
            <ChoiceCard
              title={t("seed.createNew")}
              desc={t("seed.createDesc")}
              selected={mode === "create"}
              onClick={() => setMode("create")}
            />
            <ChoiceCard
              title={t("seed.import")}
              desc={t("seed.importDesc")}
              selected={mode === "import"}
              onClick={() => setMode("import")}
            />
          </Stack>
          {err && <ErrLine msg={err} />}
        </DialogContent>
        <DialogActions sx={{ px: 3, pb: 2 }}>
          {onLater && (
            <Button color="inherit" onClick={onLater} sx={{ mr: "auto" }}>
              {t("common.later")}
            </Button>
          )}
          <Button variant="contained" disabled={busy} onClick={onChoose}>
            {t("wizard.continue")}
          </Button>
        </DialogActions>
      </>
    );
  }

  // ---- step: reveal the generated phrase (create) ----
  if (step === "reveal") {
    return (
      <>
        <DialogTitle>{t("seed.revealTitle")}</DialogTitle>
        <DialogContent>
          <DialogContentText sx={{ mb: 1.5 }}>{t("seed.revealBody")}</DialogContentText>
          <Box sx={{ display: "flex", alignItems: "center", gap: 1.5, mb: 1.5 }}>
            <ToggleButtonGroup
              exclusive
              size="small"
              disabled={busy}
              value={words.length === 24 ? 24 : 12}
              onChange={(_, v: number | null) => {
                if (v && v !== words.length) {
                  setWordCount(v);
                  void startCreate(v);
                }
              }}
            >
              <ToggleButton value={12}>{t("seed.wordCount", { n: 12 })}</ToggleButton>
              <ToggleButton value={24}>{t("seed.wordCount", { n: 24 })}</ToggleButton>
            </ToggleButtonGroup>
            <Typography sx={{ color: "text.secondary", fontSize: 12 }}>
              {t("seed.wordCountHint")}
            </Typography>
          </Box>
          <WordGrid words={words} />
          <FormControlLabel
            control={<Checkbox checked={ack} onChange={(e) => setAck(e.target.checked)} />}
            label={t("seed.ackLabel")}
          />
        </DialogContent>
        <DialogActions sx={{ px: 3, pb: 2 }}>
          <Button color="inherit" onClick={() => (presetMode ? onBack?.() : setStep("choose"))} sx={{ mr: "auto" }}>
            {t("wizard.back")}
          </Button>
          <Button variant="contained" disabled={!ack || words.length === 0} onClick={() => setStep(skipVerify ? "passphrase" : "verify")}>
            {t("wizard.continue")}
          </Button>
        </DialogActions>
      </>
    );
  }

  // ---- step: verify a few words (create) ----
  if (step === "verify") {
    return (
      <>
        <DialogTitle>{t("seed.verifyTitle")}</DialogTitle>
        <DialogContent>
          <DialogContentText sx={{ mb: 2 }}>{t("seed.verifyBody")}</DialogContentText>
          <Stack spacing={1.5}>
            {verifyIdx.map((idx, k) => {
              const typed = verifyIn[k].trim().toLowerCase();
              const wrong = !!typed && typed !== words[idx];
              return (
                <Autocomplete
                  key={idx}
                  freeSolo
                  autoHighlight
                  options={BIP39_WORDS as string[]}
                  filterOptions={filterWords}
                  inputValue={verifyIn[k]}
                  onInputChange={(_, val) =>
                    setVerifyIn((v) => v.map((x, j) => (j === k ? val : x)))
                  }
                  renderInput={(params) => (
                    <TextField
                      {...params}
                      label={t("seed.verifyWord", { n: idx + 1 })}
                      error={wrong}
                      helperText={wrong ? t("seed.verifyMismatch") : " "}
                      slotProps={{ htmlInput: { ...params.inputProps, style: { fontFamily: C.mono } } }}
                    />
                  )}
                />
              );
            })}
          </Stack>
        </DialogContent>
        <DialogActions sx={{ px: 3, pb: 2 }}>
          <Button color="inherit" onClick={() => setStep("reveal")} sx={{ mr: "auto" }}>
            {t("wizard.back")}
          </Button>
          <Button variant="contained" disabled={!verifyOk} onClick={() => setStep("passphrase")}>
            {t("wizard.continue")}
          </Button>
        </DialogActions>
      </>
    );
  }

  // ---- step: enter an existing phrase (import) ----
  if (step === "enter") {
    const slots = entryWords.map((w) => w.trim().toLowerCase());
    const allFilled = slots.every(Boolean) && slots.length === wordCount;
    const hasUnknown = slots.some((w) => w && !isBip39Word(w));
    // The same gate pactd's importseed applies — a complete, checksum-valid
    // phrase — so "Continue" never advances a phrase the backend would reject.
    const checksumOk = allFilled && !hasUnknown && isValidMnemonic(slots.join(" "));
    // Status line: unknown words first (the actionable typo), then incomplete,
    // then a checksum miss, then the all-clear.
    const status = hasUnknown
      ? { msg: t("seed.checkUnknown"), color: "error.main" }
      : !allFilled
        ? { msg: t("seed.checkIncomplete", { n: wordCount }), color: "text.secondary" }
        : !checksumOk
          ? { msg: t("seed.checkBadChecksum"), color: "error.main" }
          : { msg: t("seed.checkOk"), color: "success.main" };
    return (
      <>
        <DialogTitle>{t("seed.enterTitle")}</DialogTitle>
        <DialogContent>
          <DialogContentText sx={{ mb: 1.5 }}>{t("seed.enterBody")}</DialogContentText>
          <ToggleButtonGroup
            size="small"
            exclusive
            value={wordCount}
            onChange={(_, v: number | null) => v && changeCount(v)}
            sx={{ mb: 0.5 }}
          >
            <ToggleButton value={12}>{t("seed.wordCount", { n: 12 })}</ToggleButton>
            <ToggleButton value={24}>{t("seed.wordCount", { n: 24 })}</ToggleButton>
          </ToggleButtonGroup>
          <WordEntryGrid words={entryWords} onChange={setEntry} />
          <Typography sx={{ color: status.color, fontSize: 12.5, mt: 1, minHeight: 18 }}>
            {status.msg}
          </Typography>
        </DialogContent>
        <DialogActions sx={{ px: 3, pb: 2 }}>
          <Button color="inherit" onClick={() => (presetMode ? onBack?.() : setStep("choose"))} sx={{ mr: "auto" }}>
            {t("wizard.back")}
          </Button>
          <Button variant="contained" disabled={!checksumOk} onClick={() => setStep("passphrase")}>
            {t("wizard.continue")}
          </Button>
        </DialogActions>
      </>
    );
  }

  // ---- step: optional at-rest passphrase, then commit ----
  return (
    <>
      <DialogTitle>{t("seed.passphraseTitle")}</DialogTitle>
      <DialogContent>
        <DialogContentText sx={{ mb: 2 }}>{t("seed.passphraseBody")}</DialogContentText>
        <Stack direction="row" spacing={1.5}>
          <ChoiceCard
            title={t("seed.noPassphrase")}
            desc={t("seed.noPassphraseDesc")}
            selected={!encrypt}
            onClick={() => setEncrypt(false)}
          />
          <ChoiceCard
            title={t("seed.encrypt")}
            desc={t("seed.encryptDesc")}
            selected={encrypt}
            onClick={() => setEncrypt(true)}
          />
        </Stack>
        {encrypt && (
          <TextField
            label={t("seed.passphraseLabel")}
            type="password"
            placeholder={t("seed.passphrasePlaceholder")}
            value={passphrase}
            onChange={(e) => setPassphrase(e.target.value)}
            fullWidth
            margin="normal"
          />
        )}
        {err && <ErrLine msg={err} />}
      </DialogContent>
      <DialogActions sx={{ px: 3, pb: 2 }}>
        <Button color="inherit" onClick={() => setStep(mode === "create" ? (skipVerify ? "reveal" : "verify") : "enter")} sx={{ mr: "auto" }}>
          {t("wizard.back")}
        </Button>
        <Button variant="contained" disabled={busy} onClick={() => void commit()}>
          {t("common.done")}
        </Button>
      </DialogActions>
    </>
  );
}

// The mnemonic as a numbered 3-column grid, selectable for copy.
function WordGrid({ words }: { words: string[] }) {
  return (
    <Box
      sx={{
        fontFamily: C.mono,
        fontSize: 15,
        lineHeight: 1.9,
        bgcolor: "background.default",
        border: `1px dashed ${C.accent}`,
        borderRadius: 2,
        p: 2,
        my: 1.5,
        userSelect: "all",
        display: "grid",
        gridTemplateColumns: "repeat(3, 1fr)",
        columnGap: 2,
      }}
    >
      {words.map((w, i) => (
        <span key={i}>
          {i + 1}.&nbsp;{w}
        </span>
      ))}
    </Box>
  );
}

// The editable twin of WordGrid: the same numbered, dashed, 3-column mono grid,
// but each cell is a per-word BIP39 autocomplete (Phoenix-style). Words are
// flagged red when they're not in the wordlist; typing a space (or picking a
// suggestion) commits the word and jumps to the next slot; pasting a whole
// phrase fills from the focused slot onward.
function WordEntryGrid({
  words,
  onChange,
}: {
  words: string[];
  onChange: (next: string[]) => void;
}) {
  const t = useT();
  const refs = useRef<(HTMLInputElement | null)[]>([]);

  const focusCell = (i: number) => refs.current[i]?.focus();

  const setWord = (i: number, val: string) => {
    const next = words.slice();
    next[i] = val;
    onChange(next);
  };

  // Spread `parts` across slots starting at `start` (paste / multi-word input).
  const fillFrom = (start: number, parts: string[]) => {
    const next = words.slice();
    let last = start;
    for (let k = 0; k < parts.length && start + k < next.length; k++) {
      next[start + k] = parts[k].trim().toLowerCase();
      last = start + k;
    }
    onChange(next);
    focusCell(Math.min(last + 1, next.length - 1));
  };

  return (
    <Box
      sx={{
        bgcolor: "background.default",
        border: `1px dashed ${C.accent}`,
        borderRadius: 2,
        p: 2,
        my: 1.5,
        display: "grid",
        gridTemplateColumns: "repeat(3, 1fr)",
        columnGap: 2,
        rowGap: 1.25,
      }}
    >
      {words.map((w, i) => {
        const typed = w.trim().toLowerCase();
        const wrong = !!typed && !isBip39Word(typed);
        return (
          <Autocomplete
            key={i}
            freeSolo
            autoHighlight
            selectOnFocus
            handleHomeEndKeys
            options={BIP39_WORDS as string[]}
            filterOptions={filterWords}
            inputValue={w}
            onInputChange={(_, val, reason) => {
              if (reason === "input" && /\s/.test(val)) {
                // Space (or an inline multi-word burst) commits + advances.
                const parts = val.trim().split(/\s+/).filter(Boolean);
                if (parts.length > 1) {
                  fillFrom(i, parts);
                } else {
                  setWord(i, parts[0] ?? "");
                  focusCell(i + 1);
                }
              } else {
                setWord(i, val);
              }
            }}
            onChange={(_, val) => {
              // Picked a suggestion → normalise and jump to the next slot.
              if (typeof val === "string") {
                setWord(i, val.trim().toLowerCase());
                focusCell(i + 1);
              }
            }}
            // Hide the per-cell clear/popup icons — too busy in a 12/24 grid.
            sx={{ "& .MuiAutocomplete-endAdornment": { display: "none" } }}
            renderInput={(params) => (
              <TextField
                {...params}
                variant="standard"
                error={wrong}
                inputRef={(el: HTMLInputElement | null) => {
                  refs.current[i] = el;
                }}
                autoFocus={i === 0}
                onPaste={(e) => {
                  const text = e.clipboardData.getData("text");
                  if (/\s/.test(text.trim())) {
                    e.preventDefault();
                    fillFrom(i, text.trim().split(/\s+/).filter(Boolean));
                  }
                }}
                slotProps={{
                  input: {
                    ...params.InputProps,
                    startAdornment: (
                      <Box
                        component="span"
                        sx={{
                          color: "text.secondary",
                          fontFamily: C.mono,
                          fontSize: 12,
                          mr: 0.75,
                          minWidth: 18,
                          textAlign: "right",
                          userSelect: "none",
                        }}
                      >
                        {i + 1}.
                      </Box>
                    ),
                  },
                  htmlInput: {
                    ...params.inputProps,
                    "aria-label": t("seed.wordAria", { n: i + 1 }),
                    autoCapitalize: "none",
                    autoCorrect: "off",
                    spellCheck: false,
                    style: { fontFamily: C.mono, fontSize: 14 },
                  },
                }}
              />
            )}
          />
        );
      })}
    </Box>
  );
}

function ErrLine({ msg }: { msg: string }) {
  return (
    <Typography sx={{ color: "error.main", fontSize: 13, mt: 1.25, whiteSpace: "pre-wrap" }}>{msg}</Typography>
  );
}
