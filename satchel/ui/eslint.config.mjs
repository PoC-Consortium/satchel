// Focused lint config: its ONE job is to guarantee no user-visible string is
// hardcoded outside the i18n bundle (src/i18n). It is intentionally NOT a full
// lint regime — only `i18next/no-literal-string` runs, in jsx-only mode (JSX
// text + a curated set of human-readable attributes), so styling/variant props
// and code identifiers don't generate noise.
//
//   npm run lint   → must be clean. A new literal a user could read fails it.
//
// react-hooks is registered ONLY so the codebase's inline
// `// eslint-disable-next-line react-hooks/exhaustive-deps` directives resolve
// (the rules are off — this config does not lint hooks).
//
// Allowed exceptions (`words.exclude` + en.ts itself): technical, non-prose
// tokens that are not translatable copy — glyphs/separators (→ ↓ ↔ · ⏱ — …),
// unit fragments (h, sat/vB), the version "v" prefix, URLs, the "QUIT" confirm
// word, capability/protocol names, coin tickers, the slip prefix.

import tseslint from "typescript-eslint";
import i18next from "eslint-plugin-i18next";
import reactHooks from "eslint-plugin-react-hooks";

export default [
  { ignores: ["dist/**", "node_modules/**", "*.config.*"] },
  // react-hooks is off, so its inline disable directives are "unused" — don't
  // report that (they're vestigial, not worth touching source for).
  { linterOptions: { reportUnusedDisableDirectives: "off" } },
  // The bundle is the home for copy; literals there are the point.
  { ignores: ["src/i18n/**"] },
  {
    files: ["src/**/*.{ts,tsx}"],
    languageOptions: {
      parser: tseslint.parser,
      parserOptions: { ecmaFeatures: { jsx: true }, sourceType: "module" },
    },
    plugins: { i18next, "react-hooks": reactHooks },
    rules: {
      // Off — present only so inline disable directives for these rules resolve.
      "react-hooks/exhaustive-deps": "off",
      "react-hooks/rules-of-hooks": "off",
      "i18next/no-literal-string": [
        "error",
        {
          mode: "jsx-only",
          "jsx-attributes": {
            include: ["title", "label", "placeholder", "aria-label", "alt"],
          },
          words: {
            exclude: [
              // Default punctuation/format noise (ASCII).
              "[0-9!-/:-@[-`{-~]+",
              "[\\s]+",
              // Glyphs / separators used as visual punctuation in JSX.
              "↓", "→", "↔", "·", "⏱", "—", "…", "✓", "✗",
              // Unit / format fragments that aren't translatable prose.
              "h", "h /", "sat/vB", "v",
              // MUI theme color tokens passed as args (not user copy).
              "primary.main", "text.primary", "text.secondary",
              // Technical, non-translatable tokens.
              "QUIT", "CLTV", "SegWit", "Taproot", "pactoffer1:…", "0.0",
              // URLs / example placeholders, not prose.
              "^https?://.*", "^tcp://.*", "^wss://.*",
              "word1 word2 word3 …",
              "http://host:port",
              "http://user:pass@127.0.0.1:port/wallet/yourwallet",
            ],
          },
        },
      ],
    },
  },
];
