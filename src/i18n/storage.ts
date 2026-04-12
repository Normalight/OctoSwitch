import { DEFAULT_LOCALE, LOCALE_STORAGE_KEY, LOCALES, type Locale } from "./types";

function isLocale(x: string): x is Locale {
  return (LOCALES as readonly string[]).includes(x);
}

export function readStoredLocale(): Locale {
  try {
    const raw = localStorage.getItem(LOCALE_STORAGE_KEY);
    if (raw && isLocale(raw)) return raw;
  } catch {
    /* ignore */
  }
  return DEFAULT_LOCALE;
}

export function writeStoredLocale(locale: Locale): void {
  try {
    localStorage.setItem(LOCALE_STORAGE_KEY, locale);
  } catch {
    /* ignore */
  }
}

export function htmlLangForLocale(locale: Locale): string {
  return locale === "en" ? "en" : "zh-CN";
}
