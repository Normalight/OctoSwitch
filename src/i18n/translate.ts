import type { MessageTree } from "./types";

export function getMessage(tree: MessageTree | undefined, path: string): string | undefined {
  const parts = path.split(".").filter(Boolean);
  let cur: string | MessageTree | undefined = tree;
  for (const p of parts) {
    if (cur === undefined || typeof cur === "string") return undefined;
    cur = cur[p];
  }
  return typeof cur === "string" ? cur : undefined;
}

export function interpolate(template: string, vars?: Record<string, string | number>): string {
  if (!vars) return template;
  return template.replace(/\{(\w+)\}/g, (_, key: string) =>
    key in vars ? String(vars[key]) : `{${key}}`
  );
}
