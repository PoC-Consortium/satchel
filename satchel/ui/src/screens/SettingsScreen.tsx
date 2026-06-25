import { useEffect, useState } from "react";
import {
  Box,
  Button,
  Card,
  CardContent,
  IconButton,
  MenuItem,
  Select,
  Switch,
  Tab,
  Tabs,
  TextField,
  ToggleButton,
  ToggleButtonGroup,
  Typography,
} from "@mui/material";
import { useColorScheme } from "@mui/material/styles";
import DarkModeOutlinedIcon from "@mui/icons-material/DarkModeOutlined";
import LightModeOutlinedIcon from "@mui/icons-material/LightModeOutlined";
import SettingsBrightnessOutlinedIcon from "@mui/icons-material/SettingsBrightnessOutlined";
import CloseIcon from "@mui/icons-material/Close";
import type { ReactNode } from "react";
import { useApp } from "../AppContext";
import { usePrefs } from "../prefs";
import { useI18n, useT, LANGUAGES } from "../i18n";
import type { UiPrefs } from "../api/types";
import { APP_VERSION, UPDATE_AVAILABLE } from "../version";
import { errMsg, listCoinConfig, rpc, saveBoard, saveNostrRelays } from "../api/tauri";
import CoinsScreen from "./CoinsScreen";

// UI-3: Settings is split into MUI Tabs — General/Appearance (theme, language),
// Coins (node config), Network (relays + boards), About (version + update
// placeholder + the trust-model note). All prior functionality is preserved;
// it is only reorganised behind tabs.
type SettingsTab = "general" | "coins" | "network" | "fees" | "about";

export default function SettingsScreen() {
  const t = useT();
  const [tab, setTab] = useState<SettingsTab>("general");

  return (
    <Box sx={{ display: "flex", flexDirection: "column", gap: 2.5 }}>
      <Box>
        <Typography variant="h1" sx={{ fontSize: 18, mb: 0.5 }}>
          {t("settings.title")}
        </Typography>
        <Typography sx={{ color: "text.secondary", fontSize: 13 }}>
          {t("settings.subtitle")}
        </Typography>
      </Box>

      <Tabs
        value={tab}
        onChange={(_, v: SettingsTab) => setTab(v)}
        variant="scrollable"
        scrollButtons="auto"
        sx={{ borderBottom: 1, borderColor: "divider", minHeight: 40 }}
      >
        <Tab value="general" label={t("settings.tabGeneral")} sx={{ minHeight: 40 }} />
        <Tab value="coins" label={t("settings.tabCoins")} sx={{ minHeight: 40 }} />
        <Tab value="network" label={t("settings.tabNetwork")} sx={{ minHeight: 40 }} />
        <Tab value="fees" label={t("settings.tabFees")} sx={{ minHeight: 40 }} />
        <Tab value="about" label={t("settings.tabAbout")} sx={{ minHeight: 40 }} />
      </Tabs>

      {tab === "general" && <GeneralTab />}
      {tab === "coins" && <CoinsTab />}
      {tab === "network" && <NetworkTab />}
      {tab === "fees" && <FeesTab />}
      {tab === "about" && <AboutTab />}
    </Box>
  );
}

function GeneralTab() {
  const t = useT();
  const { watchOnly, setWatchOnly } = useApp();
  return (
    <>
      <Section title={t("settings.appearance")}>
        <Row label={t("settings.theme")} hint={t("settings.themeHint")}>
          <ThemeToggle />
        </Row>
        <Row label={t("settings.language")} hint={t("settings.languageHint")}>
          <LanguageSelect />
        </Row>
      </Section>
      <Box sx={{ mt: 2.5 }}>
        <Section title={t("settings.mode")}>
          <Row label={t("settings.watchOnly")} hint={t("settings.watchOnlyHint")}>
            <Switch
              checked={watchOnly}
              onChange={(_, on) => void setWatchOnly(on)}
              inputProps={{ "aria-label": t("settings.watchOnly") }}
            />
          </Row>
        </Section>
      </Box>
    </>
  );
}

function CoinsTab() {
  const t = useT();
  return (
    <Section title={t("settings.coins")}>
      <Typography sx={{ color: "text.secondary", fontSize: 13, mb: 2 }}>
        {t("settings.coinsHint")}
      </Typography>
      {/* Coins moved out of the top-level nav: node setup is configuration. */}
      <CoinsScreen />
    </Section>
  );
}

// Network transports — two independent, equal noticeboards (the engine fans a
// post out across both): the decentralized Nostr relay set (prewired) and any
// optional self-hosted Corkboards. Each is a plain add/remove URL list; saving
// relaunches the active merchant's pactd so it picks the transports up.
function NetworkTab() {
  const { log } = useApp();
  const t = useT();
  const [boards, setBoards] = useState<string[]>([]);
  const [relays, setRelays] = useState<string[]>([]);
  const [loaded, setLoaded] = useState(false);
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState("");

  async function load() {
    try {
      const cfg = await listCoinConfig();
      setBoards(cfg.board_urls || []);
      setRelays(cfg.nostr_relays || []);
    } catch {
      /* ok */
    } finally {
      setLoaded(true);
    }
  }
  useEffect(() => {
    void load();
  }, []);

  async function save() {
    setBusy(true);
    setStatus(t("settings.netSaving"));
    try {
      // Relays first so the default transport is live even if a board URL trips.
      await saveNostrRelays(relays.join(","));
      await saveBoard(boards.join(","));
      log(t("log.noticeboardUpdated"));
      setStatus(t("settings.netSaved"));
      await load();
    } catch (e) {
      setStatus(errMsg(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Section title={t("settings.network")}>
      {/* The network mode (mainnet/testnet/regtest) is fixed at launch (a CLI
          arg), not a setting — it's shown in the top-bar badge, so no row here. */}
      <UrlList
        title={t("settings.nostrRelays")}
        desc={t("settings.nostrRelaysDesc")}
        urls={relays}
        onChange={setRelays}
        placeholder="wss://relay.example.com"
        emptyLabel={t("settings.nostrRelaysOff")}
        validate={(u) => /^wss?:\/\/.+/i.test(u)}
        invalidLabel={t("settings.relayInvalid")}
        disabled={busy}
      />
      <UrlList
        title={t("settings.boards")}
        desc={t("settings.boardsDesc")}
        urls={boards}
        onChange={setBoards}
        placeholder="http://host:port"
        emptyLabel={t("settings.boardsNone")}
        validate={(u) => /^https?:\/\/.+/i.test(u)}
        invalidLabel={t("settings.boardInvalid")}
        disabled={busy}
      />

      <Box sx={{ display: "flex", alignItems: "center", gap: 1.5, mt: 1.5 }}>
        <Button variant="contained" onClick={() => void save()} disabled={busy || !loaded}>
          {t("settings.netSave")}
        </Button>
        {status && (
          <Typography sx={{ fontSize: 13, color: "text.secondary" }}>{status}</Typography>
        )}
      </Box>
    </Section>
  );
}

// A plain editable list of URLs (one transport kind). Rows have a remove button;
// the input validates the scheme before adding and de-dupes silently.
function UrlList({
  title,
  desc,
  urls,
  onChange,
  placeholder,
  emptyLabel,
  validate,
  invalidLabel,
  disabled,
}: {
  title: string;
  desc: string;
  urls: string[];
  onChange: (next: string[]) => void;
  placeholder: string;
  emptyLabel: string;
  validate: (u: string) => boolean;
  invalidLabel: string;
  disabled?: boolean;
}) {
  const t = useT();
  const [draft, setDraft] = useState("");
  const [err, setErr] = useState("");

  function add() {
    const u = draft.trim();
    if (!u) return;
    if (!validate(u)) {
      setErr(invalidLabel);
      return;
    }
    if (!urls.includes(u)) onChange([...urls, u]);
    setDraft("");
    setErr("");
  }

  return (
    <Box sx={{ py: 1.5, borderTop: 1, borderColor: "divider" }}>
      <Typography sx={{ fontWeight: 600, fontSize: 14 }}>{title}</Typography>
      <Typography sx={{ color: "text.secondary", fontSize: 12, mb: 1.25 }}>{desc}</Typography>

      {urls.length === 0 ? (
        <Typography sx={{ fontSize: 13, color: "text.secondary", fontStyle: "italic", mb: 1.25 }}>
          {emptyLabel}
        </Typography>
      ) : (
        <Box sx={{ display: "flex", flexDirection: "column", gap: 0.5, mb: 1.25 }}>
          {urls.map((u) => (
            <Box
              key={u}
              sx={{
                display: "flex",
                alignItems: "center",
                gap: 1,
                bgcolor: "action.hover",
                borderRadius: 1,
                pl: 1.25,
                pr: 0.5,
                py: 0.25,
              }}
            >
              <Typography
                sx={{
                  flex: 1,
                  minWidth: 0,
                  fontSize: 13,
                  fontFamily: "monospace",
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                  whiteSpace: "nowrap",
                }}
              >
                {u}
              </Typography>
              <IconButton
                size="small"
                disabled={disabled}
                aria-label={t("settings.removeUrl")}
                onClick={() => onChange(urls.filter((x) => x !== u))}
              >
                <CloseIcon fontSize="small" />
              </IconButton>
            </Box>
          ))}
        </Box>
      )}

      <Box sx={{ display: "flex", gap: 1, alignItems: "flex-start" }}>
        <TextField
          size="small"
          fullWidth
          placeholder={placeholder}
          value={draft}
          disabled={disabled}
          error={!!err}
          helperText={err || undefined}
          onChange={(e) => {
            setDraft(e.target.value);
            setErr("");
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              add();
            }
          }}
        />
        <Button
          size="small"
          variant="outlined"
          color="inherit"
          disabled={disabled}
          onClick={add}
          sx={{ flex: "none", mt: 0.25 }}
        >
          {t("settings.addUrl")}
        </Button>
      </Box>
    </Box>
  );
}

// Fee-bump policy — per the active merchant (pactd's store owns it; the engine
// reloads it on launch). Read via getfeepolicy, written via setfeepolicy (typed,
// applied live, no relaunch). Four knobs; the low-level min_fee_sat floor is not
// exposed but is preserved across saves.
type FeePolicy = {
  max_feerate_sat_vb: number;
  min_fee_sat: number;
  reservation_mult: number;
  committed_mult: number;
};

const FEE_DEFAULTS: FeePolicy = {
  max_feerate_sat_vb: 500,
  min_fee_sat: 1000,
  reservation_mult: 3,
  committed_mult: 2,
};

function FeesTab() {
  const t = useT();
  const { log } = useApp();
  const [pol, setPol] = useState<FeePolicy>(FEE_DEFAULTS);
  const [loaded, setLoaded] = useState(false);
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState("");

  async function load() {
    try {
      const p = await rpc<FeePolicy>("getfeepolicy", []);
      setPol({ ...FEE_DEFAULTS, ...p });
    } catch {
      /* keep defaults if the engine isn't ready */
    } finally {
      setLoaded(true);
    }
  }
  useEffect(() => {
    void load();
  }, []);

  async function save() {
    setBusy(true);
    setStatus(t("settings.feeSaving"));
    try {
      // Positional params: max, min, reservation, committed. min_fee_sat is not
      // editable here but is round-tripped so it isn't reset.
      await rpc("setfeepolicy", [
        pol.max_feerate_sat_vb,
        pol.min_fee_sat,
        pol.reservation_mult,
        pol.committed_mult,
      ]);
      log(t("log.feePolicyUpdated"));
      setStatus(t("settings.feeSaved"));
      await load();
    } catch (e) {
      setStatus(errMsg(e));
    } finally {
      setBusy(false);
    }
  }

  const num = (key: keyof FeePolicy, min: number, max: number) => (
    <TextField
      size="small"
      type="number"
      value={pol[key]}
      disabled={busy}
      inputProps={{ min, max, step: 1 }}
      onChange={(e) => {
        const v = Math.floor(Number(e.target.value));
        if (Number.isFinite(v)) setPol({ ...pol, [key]: v });
      }}
      sx={{ width: 120 }}
    />
  );

  // Built outside the JSX so the field-key string literals aren't flagged by the
  // i18n no-literal-string guard (they are property keys, not display copy).
  const maxField = num("max_feerate_sat_vb", 1, 500);
  const reservationField = num("reservation_mult", 1, 100);
  const committedField = num("committed_mult", 1, 100);

  return (
    <Section title={t("settings.fees")}>
      <Typography sx={{ color: "text.secondary", fontSize: 13, mb: 0.5 }}>
        {t("settings.feesScope")}
      </Typography>
      <Typography sx={{ color: "text.secondary", fontSize: 13, mb: 1.5 }}>
        {t("settings.feesIntro")}
      </Typography>
      <Row label={t("settings.feeMax")} hint={t("settings.feeMaxHint")}>
        {maxField}
      </Row>
      <Row label={t("settings.feeReservation")} hint={t("settings.feeReservationHint")}>
        {reservationField}
      </Row>
      <Row label={t("settings.feeCommitted")} hint={t("settings.feeCommittedHint")}>
        {committedField}
      </Row>
      <Box sx={{ display: "flex", alignItems: "center", gap: 1.5, mt: 1.5 }}>
        <Button variant="contained" onClick={() => void save()} disabled={busy || !loaded}>
          {t("settings.feeSave")}
        </Button>
        <Button
          variant="outlined"
          color="inherit"
          disabled={busy}
          onClick={() => setPol((prev) => ({ ...FEE_DEFAULTS, min_fee_sat: prev.min_fee_sat }))}
        >
          {t("settings.feeReset")}
        </Button>
        {status && <Typography sx={{ fontSize: 13, color: "text.secondary" }}>{status}</Typography>}
      </Box>
    </Section>
  );
}

function AboutTab() {
  const t = useT();
  return (
    <Section title={t("settings.about")}>
      <Row label={t("settings.version", { version: APP_VERSION })} hint={t("settings.updateCheckPlaceholder")}>
        <Typography sx={{ fontSize: 13, color: UPDATE_AVAILABLE ? "primary.main" : "text.secondary" }}>
          {t("settings.updateUpToDate")}
        </Typography>
      </Row>
      <Box sx={{ mt: 1 }}>
        <Typography sx={{ fontWeight: 600, fontSize: 13, mb: 0.5 }}>
          {t("settings.trustModel")}
        </Typography>
        <Typography sx={{ color: "text.secondary", fontSize: 13 }}>
          {t("settings.trustModelBody")}
        </Typography>
      </Box>
      <Box sx={{ mt: 1.5 }}>
        <Typography sx={{ fontWeight: 600, fontSize: 13, mb: 0.5, color: "warning.main" }}>
          {t("disclaimer.title")}
        </Typography>
        <Typography sx={{ color: "text.secondary", fontSize: 13 }}>
          {t("disclaimer.body")}
        </Typography>
      </Box>
    </Section>
  );
}

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <Card variant="outlined">
      <CardContent>
        <Typography
          sx={{ fontSize: 12, textTransform: "uppercase", letterSpacing: "0.08em", color: "text.secondary", mb: 1.5 }}
        >
          {title}
        </Typography>
        {children}
      </CardContent>
    </Card>
  );
}

function Row({ label, hint, children }: { label: string; hint?: string; children: ReactNode }) {
  return (
    <Box
      sx={{
        display: "flex",
        alignItems: "center",
        gap: 2,
        py: 1.25,
        "&:not(:last-of-type)": { borderBottom: 1, borderColor: "divider" },
      }}
    >
      <Box sx={{ flex: 1, minWidth: 0 }}>
        <Typography sx={{ fontWeight: 600, fontSize: 14 }}>{label}</Typography>
        {hint && <Typography sx={{ color: "text.secondary", fontSize: 12 }}>{hint}</Typography>}
      </Box>
      <Box sx={{ flex: "none" }}>{children}</Box>
    </Box>
  );
}

function ThemeToggle() {
  const { mode, setMode } = useColorScheme();
  const { prefs, update } = usePrefs();
  const t = useT();
  // SSR/first paint guard — `mode` is undefined until mounted on the client;
  // fall back to the persisted pref so the toggle shows the saved choice.
  const value = mode ?? prefs.theme;
  return (
    <ToggleButtonGroup
      exclusive
      size="small"
      value={value}
      onChange={(_, v) => {
        if (!v) return;
        setMode(v); // repaint now
        update({ theme: v as UiPrefs["theme"] }); // persist to satchel.json
      }}
      aria-label={t("settings.theme")}
    >
      <ToggleButton value="light" aria-label={t("settings.themeLight")}>
        <LightModeOutlinedIcon fontSize="small" sx={{ mr: 0.75 }} />
        {t("settings.themeLight")}
      </ToggleButton>
      <ToggleButton value="dark" aria-label={t("settings.themeDark")}>
        <DarkModeOutlinedIcon fontSize="small" sx={{ mr: 0.75 }} />
        {t("settings.themeDark")}
      </ToggleButton>
      <ToggleButton value="system" aria-label={t("settings.themeSystem")}>
        <SettingsBrightnessOutlinedIcon fontSize="small" sx={{ mr: 0.75 }} />
        {t("settings.themeSystem")}
      </ToggleButton>
    </ToggleButtonGroup>
  );
}

function LanguageSelect() {
  const { lang, setLang } = useI18n();
  return (
    <Select size="small" value={lang} onChange={(e) => setLang(e.target.value)} sx={{ minWidth: 160 }}>
      {LANGUAGES.map((l) => (
        <MenuItem key={l.code} value={l.code}>
          {l.nativeName}
        </MenuItem>
      ))}
    </Select>
  );
}
