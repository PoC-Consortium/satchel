import { createContext, useCallback, useContext, useMemo, type ReactNode } from "react";
import { en } from "./en";
import { usePrefs } from "../prefs";

// A deliberately small i18n layer (mirrors phoenix's I18nService + LANGUAGES +
// pipe) — no runtime dependency. Strings are dot-addressed against the bundle;
// `{name}` placeholders are filled from `vars`. English ships now; another
// language is just another bundle registered in BUNDLES.
//
// UI-1: the chosen language is persisted in satchel.json (via PrefsProvider),
// not the webview's localStorage — Satchel owns all persisted state.

export interface Language {
  code: string;
  name: string;
  nativeName: string;
}

export const LANGUAGES: Language[] = [{ code: "en", name: "English", nativeName: "English" }];

const BUNDLES: Record<string, unknown> = { en };

function lookup(bundle: unknown, key: string): string | undefined {
  let cur: unknown = bundle;
  for (const part of key.split(".")) {
    if (cur && typeof cur === "object" && part in (cur as Record<string, unknown>)) {
      cur = (cur as Record<string, unknown>)[part];
    } else {
      return undefined;
    }
  }
  return typeof cur === "string" ? cur : undefined;
}

function fill(s: string, vars?: Record<string, string | number>): string {
  if (!vars) return s;
  return s.replace(/\{(\w+)\}/g, (m, k) => (k in vars ? String(vars[k]) : m));
}

export type Translate = (key: string, vars?: Record<string, string | number>) => string;

interface I18nCtx {
  lang: string;
  setLang: (code: string) => void;
  t: Translate;
}

const Ctx = createContext<I18nCtx | null>(null);

export function I18nProvider({ children }: { children: ReactNode }) {
  const { prefs, update } = usePrefs();
  // The active language is the persisted pref, validated against shipped
  // bundles (an unknown/dropped language falls back to English).
  const lang = prefs.language in BUNDLES ? prefs.language : "en";

  const setLang = useCallback(
    (code: string) => {
      if (!(code in BUNDLES)) return;
      update({ language: code });
    },
    [update],
  );

  const t = useCallback<Translate>(
    (key, vars) => {
      const hit = lookup(BUNDLES[lang], key) ?? lookup(en, key);
      return hit === undefined ? key : fill(hit, vars);
    },
    [lang],
  );

  // Keep the module-level mirror (`tr`) in lockstep with the active language so
  // non-component code — pure helpers in format.ts / narrate.ts / identity.ts —
  // translates against the same bundle without each call site threading `t`.
  // Runs on every render (cheap); the provider sits above all consumers, so by
  // the time any helper runs during a child render `_active` is already current.
  _active = t;

  const value = useMemo(() => ({ lang, setLang, t }), [lang, setLang, t]);
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

// Module-level mirror of the active translate function, for code that can't call
// the `useT()` hook (non-component pure helpers). Defaults to the English bundle
// so it's safe before the provider mounts and in tests. I18nProvider overwrites
// it with the language-aware `t` on every render. Components should still prefer
// `useT()` (it subscribes them to re-render on a language switch); reach for
// `tr()` only from plain functions outside the React tree.
let _active: Translate = (key, vars) => {
  const hit = lookup(en, key);
  return hit === undefined ? key : fill(hit, vars);
};

/** Translate from non-component code (format.ts, narrate.ts, …). Mirrors `t`. */
export function tr(key: string, vars?: Record<string, string | number>): string {
  return _active(key, vars);
}

export function useI18n(): I18nCtx {
  const c = useContext(Ctx);
  if (!c) throw new Error("useI18n outside I18nProvider");
  return c;
}

/** Sugar for the common case — just the translate function. */
export function useT(): Translate {
  return useI18n().t;
}
