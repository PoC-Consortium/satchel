import { useEffect, useState } from "react";
import {
  Autocomplete,
  Box,
  Button,
  Checkbox,
  Chip,
  createFilterOptions,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
  FormControlLabel,
  Stack,
  TextField,
  Typography,
} from "@mui/material";
import ChoiceCard from "../components/ChoiceCard";
import { errMsg, rpc } from "../api/tauri";
import { BIP39_WORDS, isBip39Word } from "../bip39";
import { useApp } from "../AppContext";
import { useT } from "../i18n";
import { C } from "../theme";

// Prefix match, capped — the BIP39 list is 2048 words; show a short suggestion
// list as the user types (Phoenix-style autocomplete).
const filterWords = createFilterOptions<string>({ matchFrom: "start", limit: 8 });

// Provision the (already active) merchant's seed, Phoenix-style and stepwise:
//   choose (create | import)
//   create:  reveal mnemonic (+ "written down") -> verify 3 random words -> passphrase
//   import:  enter phrase -> passphrase
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
  const [mnemonic, setMnemonic] = useState(""); // generated (create) or pasted (import)
  const [ack, setAck] = useState(false);
  const [verifyIdx, setVerifyIdx] = useState<number[]>([]);
  const [verifyIn, setVerifyIn] = useState<string[]>(["", "", ""]);
  const [encrypt, setEncrypt] = useState(false);
  const [passphrase, setPassphrase] = useState("");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState("");

  const words = mnemonic.trim() ? mnemonic.trim().split(/\s+/) : [];
  // Import path: words as lowercase chips for the Autocomplete value.
  const importWords = mnemonic.trim() ? mnemonic.trim().toLowerCase().split(/\s+/) : [];

  // create → generate a fresh phrase (NOT persisted yet) and reveal it.
  async function startCreate() {
    setErr("");
    setBusy(true);
    try {
      const r = await rpc<{ mnemonic: string }>("generateseed", []);
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
      log("merchant ready");
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
          <Button variant="contained" disabled={!ack || words.length === 0} onClick={() => setStep("verify")}>
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
    return (
      <>
        <DialogTitle>{t("seed.enterTitle")}</DialogTitle>
        <DialogContent>
          <DialogContentText sx={{ mb: 1.5 }}>{t("seed.enterBody")}</DialogContentText>
          <Autocomplete
            multiple
            freeSolo
            autoHighlight
            options={BIP39_WORDS as string[]}
            filterOptions={filterWords}
            value={importWords}
            onChange={(_, v) =>
              setMnemonic((v as string[]).map((w) => w.trim().toLowerCase()).filter(Boolean).join(" "))
            }
            renderTags={(value, getTagProps) =>
              value.map((opt, i) => {
                const { key, ...tagProps } = getTagProps({ index: i });
                // Colour words not in the BIP39 list red so typos stand out.
                return (
                  <Chip
                    key={key}
                    {...tagProps}
                    label={opt}
                    size="small"
                    variant="outlined"
                    color={isBip39Word(opt) ? "default" : "error"}
                    sx={{ fontFamily: C.mono }}
                  />
                );
              })
            }
            renderInput={(params) => (
              <TextField
                {...params}
                label={t("seed.recoveryLabel")}
                placeholder={importWords.length ? "" : t("seed.importPlaceholder")}
                autoFocus
                onPaste={(e) => {
                  const text = e.clipboardData.getData("text");
                  // Paste of a whole space-separated phrase → split into chips.
                  if (/\s/.test(text.trim())) {
                    e.preventDefault();
                    const add = text.trim().toLowerCase().split(/\s+/).filter(Boolean);
                    setMnemonic([...importWords, ...add].join(" "));
                  }
                }}
              />
            )}
          />
        </DialogContent>
        <DialogActions sx={{ px: 3, pb: 2 }}>
          <Button color="inherit" onClick={() => (presetMode ? onBack?.() : setStep("choose"))} sx={{ mr: "auto" }}>
            {t("wizard.back")}
          </Button>
          <Button variant="contained" disabled={!mnemonic.trim()} onClick={() => setStep("passphrase")}>
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
        <Button color="inherit" onClick={() => setStep(mode === "create" ? "verify" : "enter")} sx={{ mr: "auto" }}>
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

function ErrLine({ msg }: { msg: string }) {
  return (
    <Typography sx={{ color: "error.main", fontSize: 13, mt: 1.25, whiteSpace: "pre-wrap" }}>{msg}</Typography>
  );
}
