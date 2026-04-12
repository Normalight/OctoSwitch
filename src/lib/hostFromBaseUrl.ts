/** 从基础 URL 解析主机名；无前缀时按 HTTPS 解析。 */
export function hostFromBaseUrl(raw: string): string | null {
  const t = raw.trim();
  if (!t) return null;
  try {
    const href = t.includes("://") ? t : `https://${t}`;
    const u = new URL(href);
    return u.hostname || null;
  } catch {
    return null;
  }
}
