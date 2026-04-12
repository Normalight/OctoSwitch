export const LOCALES = ["zh-CN", "en"] as const;
export type Locale = (typeof LOCALES)[number];

export const DEFAULT_LOCALE: Locale = "zh-CN";

export const LOCALE_STORAGE_KEY = "os-locale";

export type MessageTree = { [key: string]: string | MessageTree };
