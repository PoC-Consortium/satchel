import { createTheme } from "@mui/material/styles";

// Custom palette tokens (the phoenix surface tints + state-chip fills) so that
// switching color scheme repaints them. MUI's cssVariables feature emits a
// `--mui-palette-<path>` CSS var for every string leaf under `palette`, which is
// what makes `C.*` below live-update on a dark↔light switch.
declare module "@mui/material/styles" {
  interface Palette {
    surface: { raised: string; glyph: string; tooltip: string; mine: string };
    goodTint: { bg: string; border: string };
    warnTint: { bg: string; border: string };
    badTint: { bg: string; border: string };
  }
  interface PaletteOptions {
    surface?: { raised: string; glyph: string; tooltip: string; mine: string };
    goodTint?: { bg: string; border: string };
    warnTint?: { bg: string; border: string };
    badTint?: { bg: string; border: string };
  }
}

const MONO = '"Consolas", "SFMono-Regular", "Menlo", monospace';

// Phoenix-like dark palette (ported verbatim from the old index.html :root so
// the dark scheme stays visually continuous) plus a calm matching light scheme.
const dark = {
  background: { default: "#14171a", paper: "#1c2126" },
  primary: { main: "#d9a743", contrastText: "#14171a" },
  success: { main: "#5fb878" },
  error: { main: "#d9534f" },
  warning: { main: "#d9a743" },
  divider: "#2c343b",
  text: { primary: "#d6dde3", secondary: "#7d8a94" },
  surface: { raised: "#20262c", glyph: "#22282e", tooltip: "#0c0f12", mine: "#1a2026" },
  goodTint: { bg: "#16211b", border: "#2c4733" },
  warnTint: { bg: "#211d16", border: "#433923" },
  badTint: { bg: "#241a1a", border: "#48302f" },
} as const;

const light = {
  background: { default: "#f4f6f8", paper: "#ffffff" },
  primary: { main: "#b9842a", contrastText: "#ffffff" },
  success: { main: "#3f9c5b" },
  error: { main: "#c4423d" },
  warning: { main: "#b9842a" },
  divider: "#dde2e7",
  text: { primary: "#1d2429", secondary: "#5b6770" },
  surface: { raised: "#eef1f4", glyph: "#eef1f4", tooltip: "#1d2429", mine: "#eef3f8" },
  goodTint: { bg: "#e8f5ec", border: "#bfe0c9" },
  warnTint: { bg: "#fdf3e1", border: "#e8d4a6" },
  badTint: { bg: "#fdeceb", border: "#f0c4c1" },
} as const;

// Semantic token handles used across the UI. Each resolves to a live CSS var so
// the dark↔light↔system toggle repaints without re-rendering. `mono` is the one
// non-color constant (a font stack, not theme-dependent).
export const C = {
  bg: "var(--mui-palette-background-default)",
  panel: "var(--mui-palette-background-paper)",
  line: "var(--mui-palette-divider)",
  fg: "var(--mui-palette-text-primary)",
  dim: "var(--mui-palette-text-secondary)",
  accent: "var(--mui-palette-primary-main)",
  good: "var(--mui-palette-success-main)",
  bad: "var(--mui-palette-error-main)",
  raised: "var(--mui-palette-surface-raised)",
  glyphBg: "var(--mui-palette-surface-glyph)",
  tooltipBg: "var(--mui-palette-surface-tooltip)",
  mineBg: "var(--mui-palette-surface-mine)",
  goodTintBg: "var(--mui-palette-goodTint-bg)",
  goodTintBorder: "var(--mui-palette-goodTint-border)",
  warnTintBg: "var(--mui-palette-warnTint-bg)",
  warnTintBorder: "var(--mui-palette-warnTint-border)",
  badTintBg: "var(--mui-palette-badTint-bg)",
  badTintBorder: "var(--mui-palette-badTint-border)",
  mono: MONO,
};

export const theme = createTheme({
  cssVariables: { colorSchemeSelector: "class" },
  defaultColorScheme: "dark",
  colorSchemes: {
    dark: { palette: { mode: "dark", ...dark } },
    light: { palette: { mode: "light", ...light } },
  },
  shape: { borderRadius: 8 },
  typography: {
    fontFamily: 'system-ui, "Segoe UI", sans-serif',
    fontSize: 14,
    h1: { fontSize: 16, letterSpacing: "0.08em", fontWeight: 600 },
    button: { textTransform: "none", fontWeight: 600 },
  },
  components: {
    MuiCssBaseline: {
      styleOverrides: {
        body: { background: C.bg },
        "::selection": { background: "rgba(217,167,67,0.28)" },
      },
    },
    MuiAppBar: {
      styleOverrides: {
        root: {
          backgroundColor: C.panel,
          backgroundImage: "none",
          borderBottom: `1px solid ${C.line}`,
          boxShadow: "none",
        },
      },
    },
    MuiDrawer: {
      styleOverrides: {
        paper: { backgroundColor: C.panel, borderRight: `1px solid ${C.line}` },
      },
    },
    MuiPaper: {
      styleOverrides: { root: { backgroundImage: "none" } },
    },
    MuiCard: {
      styleOverrides: {
        root: { backgroundColor: C.panel, border: `1px solid ${C.line}` },
      },
    },
    MuiDialog: {
      styleOverrides: {
        paper: { backgroundColor: C.panel, border: `1px solid ${C.line}`, backgroundImage: "none" },
      },
    },
    MuiTooltip: {
      styleOverrides: {
        tooltip: { backgroundColor: C.tooltipBg, border: `1px solid ${C.line}`, fontSize: 12 },
      },
    },
    MuiOutlinedInput: {
      styleOverrides: {
        root: { backgroundColor: C.bg },
        input: { "&::placeholder": { color: C.dim, opacity: 1 } },
      },
    },
  },
});
