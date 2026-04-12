import { useI18n } from "../i18n";
import type { ModelBinding } from "../types";

type ModelStackProps = {
  bindings: ModelBinding[];
  /** Prefix for list item keys (e.g. provider id) */
  keyPrefix: string;
  /** Max visible items before truncation. Default: show all. */
  maxVisible?: number;
  /** "summary" = chips only; "detail" = chip + edit/delete buttons */
  variant?: "summary" | "detail";
  /** Called when the delete button is clicked (detail mode only) */
  onRemove?: (binding: ModelBinding) => void;
  /** Called when the edit button is clicked (detail mode only) */
  onEdit?: (binding: ModelBinding) => void;
  /** Resolve a group ID to its alias label */
  groupAlias?: (groupId: string) => string;
  /** 在 chip 上展示分组标签（供应商编辑弹窗内列表改为仅在子窗口展示分组） */
  showGroupOnChip?: boolean;
  /** 是否在 chip 上展示目标模型名（供应商页默认不展示） */
  showUpstreamOnChip?: boolean;
};

const DEFAULT_MAX = 6;

export function ModelStack({
  bindings,
  keyPrefix,
  maxVisible = DEFAULT_MAX,
  variant = "detail",
  onRemove,
  onEdit,
  groupAlias = (id) => id,
  showGroupOnChip = true,
  showUpstreamOnChip = true,
}: ModelStackProps) {
  const { t } = useI18n();
  const cap = Number.isFinite(maxVisible) ? maxVisible : bindings.length;
  const shown = bindings.slice(0, cap);
  const hidden = Math.max(0, bindings.length - cap);
  const stackClass =
    variant === "summary"
      ? "provider-model-stack provider-model-stack--inline"
      : "provider-model-stack";

  const titleFor = (m: ModelBinding) => {
    const groups = m.group_ids.map(groupAlias).join(" · ");
    const base = showUpstreamOnChip ? `${m.model_name} · ${m.upstream_model_name}` : m.model_name;
    return groups ? `${base}\n${groups}` : base;
  };

  return (
    <div className={stackClass} role="list">
      {shown.map((m) =>
        variant === "detail" ? (
          <div key={`${keyPrefix}-${m.id}`} className="provider-model-row" role="listitem">
            <button
              type="button"
              className={`provider-model-chip${showUpstreamOnChip ? " provider-model-chip--with-target" : ""}`}
              disabled
              title={titleFor(m)}
            >
              {showUpstreamOnChip ? (
                <span className="provider-model-chip__row">
                  <span className="provider-model-chip__name">{m.model_name}</span>
                  <span className="provider-model-chip__sep muted" aria-hidden>
                    ·
                  </span>
                  <span className="provider-model-chip__target muted">{m.upstream_model_name}</span>
                </span>
              ) : (
                <span className="provider-model-chip__name">{m.model_name}</span>
              )}
              {showGroupOnChip && m.group_ids.length > 0 ? (
                <span className="provider-model-chip__groups">{m.group_ids.map(groupAlias).join(" · ")}</span>
              ) : null}
            </button>
            <button
              type="button"
              className="btn btn--ghost btn--sm btn--icon"
              title={t("common.edit")}
              aria-label={t("common.edit")}
              onClick={() => onEdit?.(m)}
            >
              <svg viewBox="0 0 24 24" width="16" height="16" stroke="currentColor" strokeWidth="2" fill="none" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                <path d="M17 3a2.828 2.828 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5L17 3z"/>
              </svg>
            </button>
            <button
              type="button"
              className="btn btn--ghost btn--sm btn--icon"
              title={t("common.delete")}
              aria-label={t("common.delete")}
              onClick={() => onRemove?.(m)}
            >
              <svg viewBox="0 0 24 24" width="16" height="16" stroke="currentColor" strokeWidth="2" fill="none" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                <path d="M3 6h18" />
                <path d="M8 6V4h8v2" />
                <rect x="6" y="6" width="12" height="14" rx="2" ry="2" />
                <path d="M10 11v6" />
                <path d="M14 11v6" />
              </svg>
            </button>
          </div>
        ) : (
          <button
            key={`${keyPrefix}-${m.id}`}
            type="button"
            className={`provider-model-chip${showUpstreamOnChip ? " provider-model-chip--with-target" : ""}`}
            disabled
            title={titleFor(m)}
          >
            {showUpstreamOnChip ? (
              <span className="provider-model-chip__row">
                <span className="provider-model-chip__name">{m.model_name}</span>
                <span className="provider-model-chip__sep muted" aria-hidden>
                  ·
                </span>
                <span className="provider-model-chip__target muted">{m.upstream_model_name}</span>
              </span>
            ) : (
              <span className="provider-model-chip__name">{m.model_name}</span>
            )}
          </button>
        )
      )}
      {hidden > 0 ? (
        <span className="provider-model-chip provider-model-chip--more" role="status">
          {t("providers.hiddenModels", { n: hidden })}
        </span>
      ) : null}
    </div>
  );
}
