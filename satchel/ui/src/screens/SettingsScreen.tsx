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
import { isMainnet } from "../format";
import { errMsg, listCoinConfig, saveBoard, saveNostrRelays, setAutoFund } from "../api/tauri";
import NetworkStamp from "../components/NetworkStamp";
import CoinsScreen from "./CoinsScreen";

// UI-3: Settings is split into MUI Tabs — General/Appearance (theme, language),
// Coins (node config), Network (network display), About (version + update
// placeholder + the trust-model note). All prior functionality is preserved;
// it is only reorganised behind tabs.
type SettingsTab = "general" | "coins" | "network" | "about";

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
        <Tab value="about" label={t("settings.tabAbout")} sx={{ minHeight: 40 }} />
      </Tabs>

      {tab === "general" && <GeneralTab />}
      {tab === "coins" && <CoinsTab />}
      {tab === "network" && <NetworkTab />}
      {tab === "about" && <AboutTab />}
    </Box>
  );
}

function GeneralTab() {
  const t = useT();
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
      <Section title={t("settings.swaps")}>
        <Row label={t("settings.autoFund")} hint={t("settings.autoFundHint")}>
          <AutoFundToggle />
        </Row>
      </Section>
    </>
  );
}

// RC2 #1: auto-fund toggle. Reads the live state from getinfo; setAutoFund
// applies it immediately (no relaunch) and persists it. Turning it OFF switches
// to manual funding, where the funding-required alert kicks in.
function AutoFundToggle() {
  const { info } = useApp();
  const [on, setOn] = useState<boolean>(info?.auto_fund ?? true);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    if (typeof info?.auto_fund === "boolean") setOn(info.auto_fund);
  }, [info?.auto_fund]);
  const toggle = async (next: boolean) => {
    setBusy(true);
    setOn(next);
    try {
      await setAutoFund(next);
    } catch {
      setOn(!next); // revert on failure; the next getinfo poll reconciles
    } finally {
      setBusy(false);
    }
  };
  return <Switch checked={on} disabled={busy} onChange={(_, v) => void toggle(v)} />;
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
  const { network, log } = useApp();
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
      log("noticeboard updated");
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
      <Row label={t("settings.network")} hint={t("settings.networkHint")}>
        <Box sx={{ display: "flex", alignItems: "center", gap: 1.25 }}>
          {network && isMainnet(network) ? (
            <Typography sx={{ fontWeight: 600 }}>{t("network.mainnet")}</Typography>
          ) : (
            <NetworkStamp network={network} />
          )}
        </Box>
      </Row>

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
