export function getTotalInputWithCache(
  inputTokens?: number | null,
  cacheReadTokens?: number | null
): number {
  return (inputTokens ?? 0) + (cacheReadTokens ?? 0);
}
