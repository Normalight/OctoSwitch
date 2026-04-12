/** 将常见 SQLite UNIQUE 等错误转为可读文案（绑定 / 分组 / 供应商） */
export function mapCommonDbError(raw: unknown, t: (key: string) => string): string {
  const s = raw instanceof Error ? raw.message : String(raw);
  if (s.includes("不能包含字符 /")) {
    if (s.includes("分组别名")) return t("groups.errAliasNoSlash");
    return t("models.errModelNameNoSlash");
  }
  if (!s.includes("UNIQUE constraint failed")) return s;
  if (s.includes("model_bindings.model_name")) {
    return t("models.errDuplicateModelName");
  }
  if (s.includes("model_groups") && s.includes("alias")) {
    return t("models.errDuplicateGroupAlias");
  }
  if (s.includes("providers")) {
    return t("providers.errDuplicateName");
  }
  return s;
}

/** @deprecated 使用 mapCommonDbError；保留别名以免大范围改名 */
export const mapModelBindingSaveError = mapCommonDbError;
