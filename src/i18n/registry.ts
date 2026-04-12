import type { Locale } from "./types";
import type { MessageTree } from "./types";
import { zhCN } from "./bundles/zh-CN";
import { en } from "./bundles/en";

/** Register new locales here; keep bundle shapes aligned with zh-CN. */
const bundles: Record<Locale, MessageTree> = {
  "zh-CN": zhCN,
  en
};

export function getBundle(locale: Locale): MessageTree {
  return bundles[locale] ?? bundles["zh-CN"];
}
