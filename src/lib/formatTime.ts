/** 将后端 RFC3339 等时间串格式化为本地「年-月-日 时:分:秒」。 */
export function formatCompactDateTime(raw: string): string {
  const trimmed = raw.trim();
  const d = new Date(trimmed);
  if (Number.isNaN(d.getTime())) {
    return trimmed.length > 19 ? `${trimmed.slice(0, 19)}…` : trimmed;
  }

  const pad = (n: number) => String(n).padStart(2, "0");
  const y = d.getFullYear();
  const mo = pad(d.getMonth() + 1);
  const day = pad(d.getDate());
  const h = pad(d.getHours());
  const min = pad(d.getMinutes());
  const sec = pad(d.getSeconds());

  return `${y}-${mo}-${day} ${h}:${min}:${sec}`;
}

/** 趋势图横轴：月-日 时:分，避免 RFC3339 过长。 */
export function formatChartBucketLabel(raw: string): string {
  const trimmed = raw.trim();
  const d = new Date(trimmed);
  if (Number.isNaN(d.getTime())) {
    return trimmed.length > 16 ? `${trimmed.slice(0, 16)}…` : trimmed;
  }
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}
