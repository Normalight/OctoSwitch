import type { ReactNode } from "react";
import { createContext, useCallback, useContext, useMemo, useState } from "react";
import { DEFAULT_LOCALE, type Locale } from "./types";
import { getBundle } from "./registry";
import { getMessage, interpolate } from "./translate";
import { htmlLangForLocale, readStoredLocale, writeStoredLocale } from "./storage";

type I18nContextValue = {
  locale: Locale;
  setLocale: (locale: Locale) => void;
  t: (path: string, vars?: Record<string, string | number>) => string;
};

const I18nContext = createContext<I18nContextValue | null>(null);

const DOC_TITLE_KEYS: Record<Locale, string> = {
  "zh-CN": "OctoSwitch",
  en: "OctoSwitch"
};

function applyDocumentLang(locale: Locale): void {
  const lang = htmlLangForLocale(locale);
  if (typeof document !== "undefined") {
    document.documentElement.lang = lang;
    document.title = DOC_TITLE_KEYS[locale] ?? DOC_TITLE_KEYS[DEFAULT_LOCALE];
  }
}

export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>(() => {
    const initial = readStoredLocale();
    applyDocumentLang(initial);
    return initial;
  });

  const setLocale = useCallback((next: Locale) => {
    setLocaleState(next);
    writeStoredLocale(next);
    applyDocumentLang(next);
  }, []);

  const t = useCallback(
    (path: string, vars?: Record<string, string | number>) => {
      const primary = getBundle(locale);
      const fallback = getBundle(DEFAULT_LOCALE);
      const raw = getMessage(primary, path) ?? getMessage(fallback, path) ?? path;
      return interpolate(raw, vars);
    },
    [locale]
  );

  const value = useMemo(() => ({ locale, setLocale, t }), [locale, setLocale, t]);

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n(): I18nContextValue {
  const ctx = useContext(I18nContext);
  if (!ctx) throw new Error("useI18n must be used within I18nProvider");
  return ctx;
}
