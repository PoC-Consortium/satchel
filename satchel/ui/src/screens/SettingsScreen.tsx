import { useEffect, useState } from "react";
import {
  Box,
  Button,
  Card,
  CardContent,
  MenuItem,
  Select,
  Tab,
  Tabs,
  ToggleButton,
  ToggleButtonGroup,
  Typography,
} from "@mui/material";
import { useColorScheme } from "@mui/material/styles";
import DarkModeOutlinedIcon from "@mui/icons-material/DarkModeOutlined";
import LightModeOutlinedIcon from "@mui/icons-material/LightModeOutlined";
import SettingsBrightnessOutlinedIcon from "@mui/icons-material/SettingsBrightnessOutlined";
import type { ReactNode } from "react";
import { useApp } from "../AppContext";
import { usePrefs } from "../prefs";
import { useI18n, useT, LANGUAGES } from "../i18n";
import type { UiPrefs } from "../api/types";
import { APP_VERSION, UPDATE_AVAILABLE } from "../version";
import { isMainnet } from "../format";
import { listCoinConfig } from "../api/tauri";
import NetworkStamp from "../components/NetworkStamp";
import BoardConfig from "../dialogs/BoardConfig";
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
    <Section title={t("settings.appearance")}>
      <Row label={t("settings.theme")} hint={t("settings.themeHint")}>
        <ThemeToggle />
      </Row>
      <Row label={t("settings.language")} hint={t("settings.languageHint")}>
        <LanguageSelect />
      </Row>
    </Section>
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

function NetworkTab() {
  const { network } = useApp();
  const t = useT();
  const [boards, setBoards] = useState("");
  const [relays, setRelays] = useState("");
  const [recommended, setRecommended] = useState<string[]>([]);
  const [cfgOpen, setCfgOpen] = useState(false);

  async function load() {
    try {
      const cfg = await listCoinConfig();
      setBoards((cfg.board_urls || []).join(","));
      setRelays((cfg.nostr_relays || []).join(","));
      setRecommended(cfg.recommended_nostr_relays || []);
    } catch {
      /* ok */
    }
  }
  useEffect(() => {
    void load();
  }, []);

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
      <Row
        label={t("settings.boards")}
        hint={boards ? boards.replace(/,/g, ", ") : t("settings.boardsNone")}
      >
        <Button size="small" variant="outlined" color="inherit" onClick={() => setCfgOpen(true)}>
          {t("settings.boardsConfigure")}
        </Button>
      </Row>
      <Row
        label={t("settings.nostrRelays")}
        hint={relays ? relays.replace(/,/g, ", ") : t("settings.nostrRelaysOff")}
      >
        <Button size="small" variant="outlined" color="inherit" onClick={() => setCfgOpen(true)}>
          {t("settings.boardsConfigure")}
        </Button>
      </Row>
      {cfgOpen && (
        <BoardConfig
          initialUrls={boards}
          initialRelays={relays}
          recommendedRelays={recommended}
          onClose={() => setCfgOpen(false)}
          onSaved={load}
        />
      )}
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
