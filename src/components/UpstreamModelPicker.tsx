import { useEffect, useMemo, useState } from "react";
import { fetchUpstreamModels, mapFetchModelsError } from "../lib/api/model-fetch";
import type { FetchedModel } from "../types/fetched_model";

type TFn = (path: string, vars?: Record<string, string | number>) => string;

type UpstreamModelPickerProps = {
  value: string;
  onChange: (v: string) => void;
  providerId: string | null;
  disabled?: boolean;
  /** Form-level validation (e.g. whitespace-only upstream), separate from fetch errors */
  validationError?: string | null;
  t: TFn;
};

export function UpstreamModelPicker({
  value,
  onChange,
  providerId,
  disabled,
  validationError = null,
  t,
}: UpstreamModelPickerProps) {
  const [loading, setLoading] = useState(false);
  const [models, setModels] = useState<FetchedModel[] | null>(null);
  const [fetchErr, setFetchErr] = useState<string | null>(null);
  const [fetchOk, setFetchOk] = useState<string | null>(null);

  useEffect(() => {
    if (!providerId) {
      setModels(null);
      setFetchErr(null);
      setFetchOk(null);
    }
  }, [providerId]);

  /** 列表倒序（id 降序），与接口返回顺序相反 */
  const sortedModels = useMemo(() => {
    if (!models?.length) return [];
    return [...models].sort((a, b) => b.id.localeCompare(a.id));
  }, [models]);

  const fetchedIdSet = useMemo(() => new Set((models ?? []).map((m) => m.id)), [models]);

  const onFetch = async () => {
    setFetchErr(null);
    setFetchOk(null);
    if (!providerId) {
      setFetchErr(t("models.fetchModelsNeedProvider"));
      return;
    }
    setLoading(true);
    try {
      const list = await fetchUpstreamModels(providerId);
      setModels(list);
      if (list.length === 0) {
        setFetchOk(t("models.fetchModelsEmpty"));
      } else {
        setFetchOk(t("models.fetchModelsSuccess", { count: list.length }));
        const sortedDesc = [...list].sort((a, b) => b.id.localeCompare(a.id));
        onChange(sortedDesc[0].id);
      }
    } catch (e) {
      setModels(null);
      setFetchErr(mapFetchModelsError(e, t, { hasProvider: !!providerId }));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="upstream-model-field">
      <div className="upstream-model-field__row">
        <div className="upstream-model-field__combo">
          <input
            className="upstream-model-field__input"
            value={value}
            onChange={(e) => onChange(e.target.value)}
            disabled={disabled}
            autoComplete="off"
            placeholder={t("models.labelUpstream")}
          />
          {models && models.length > 0 ? (
            <div className="upstream-model-field__combo-trigger" title={t("models.pickFromList")}>
              <select
                className="upstream-model-field__combo-select"
                value={fetchedIdSet.has(value) ? value : ""}
                disabled={disabled}
                aria-label={t("models.pickFromList")}
                onChange={(e) => {
                  const v = e.target.value;
                  if (v) onChange(v);
                }}
              >
                {/* 无“清空”项：当前值不在列表中时绑定到隐藏空项，避免下拉顶格出现占位符 */}
                <option value="" hidden />
                {sortedModels.map((m) => (
                  <option key={m.id} value={m.id}>
                    {m.id}
                  </option>
                ))}
              </select>
              <span className="upstream-model-field__combo-chevron" aria-hidden>
                <svg viewBox="0 0 24 24" width="16" height="16" stroke="currentColor" strokeWidth="2" fill="none">
                  <polyline points="6 9 12 15 18 9" />
                </svg>
              </span>
            </div>
          ) : null}
        </div>
        <button
          type="button"
          className="btn btn--ghost btn--sm btn--icon"
          disabled={disabled || loading || !providerId}
          title={t("models.fetchUpstreamTitle")}
          onClick={() => void onFetch()}
          aria-label={t("models.fetchUpstreamModels")}
        >
          {loading ? (
            <span className="upstream-model-field__spin" aria-hidden />
          ) : (
            <svg viewBox="0 0 24 24" width="16" height="16" stroke="currentColor" strokeWidth="2" fill="none" aria-hidden>
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
              <polyline points="7 10 12 15 17 10" />
              <line x1="12" y1="15" x2="12" y2="3" />
            </svg>
          )}
        </button>
      </div>
      {fetchErr ? <p className="form-error">{fetchErr}</p> : null}
      {validationError ? <p className="form-error">{validationError}</p> : null}
      {fetchOk && !fetchErr ? <p className="form-hint muted">{fetchOk}</p> : null}
    </div>
  );
}
