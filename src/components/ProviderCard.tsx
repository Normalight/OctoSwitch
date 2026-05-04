import type { CopilotAccountStatus, ModelBinding, ProviderSummary } from "../types";
import type { PointerEvent } from "react";

type ProviderCardProps = {
  provider: ProviderSummary;
  models: ModelBinding[];
  /** Called when the edit button is clicked */
  onEdit: (provider: ProviderSummary) => void;
  /** Drag-to-reorder: start dragging this card */
  onDragStart: (id: string, e: PointerEvent<HTMLDivElement>) => void;
  /** Visual state for active / drop-hover while reordering */
  reorderUi?: { activeId: string | null; hoverId: string | null };
  /** Whether the page is currently busy (disables edit button) */
  busy: boolean;
  /** Resolve a group ID to its alias label */
  groupAlias: (groupId: string) => string;
  /** Map of provider_id → copilot account info */
  copilotAccountMap: Map<string, CopilotAccountStatus>;
  /** i18n translation helper — only needed for copilot tag text */
  t: (key: string) => string;
  /** Pre-computed tag label for this provider's API format */
  apiFormatTagLabel: string;
  /** Render function for the model stack (passed through for flexibility) */
  renderModels: () => React.ReactNode;
};

export function ProviderCard({
  provider: p,
  busy,
  onEdit,
  onDragStart,
  reorderUi,
  groupAlias,
  copilotAccountMap,
  t,
  apiFormatTagLabel,
  renderModels,
}: ProviderCardProps) {
  const isDragging = reorderUi?.activeId === p.id;
  const isDropHover =
    Boolean(reorderUi?.activeId) &&
    reorderUi?.hoverId === p.id &&
    reorderUi?.activeId !== p.id;

  const copilotAcc = copilotAccountMap.get(p.id);
  const tagContent = copilotAcc ? (
    <span className={`provider-card__tag ${copilotAcc.authenticated ? "is-copilot-active" : "is-copilot-inactive"}`}>
      {t("copilot.tagCopilot")}
    </span>
  ) : (
    <span className={`provider-card__tag ${p.is_enabled ? "" : "is-off"}`}>
      {apiFormatTagLabel}
    </span>
  );

  return (
    <div
      key={p.id}
      data-sortable-id={p.id}
      className={[
        "provider-card card card--compact sortable-item",
        isDragging ? "sortable-item--dragging" : "",
        isDropHover ? "sortable-item--drop-hover" : "",
      ]
        .filter(Boolean)
        .join(" ")}
    >
      <div className="provider-card__head">
        <div
          className="drag-handle"
          title={t("common.dragToSort")}
          onPointerDown={(e) => onDragStart(p.id, e)}
        >
          <svg aria-hidden="true" viewBox="0 0 24 24" width="16" height="16" stroke="currentColor" strokeWidth="2" fill="none" strokeLinecap="round" strokeLinejoin="round">
            <circle cx="9" cy="12" r="1"/><circle cx="9" cy="5" r="1"/><circle cx="9" cy="19" r="1"/><circle cx="15" cy="12" r="1"/><circle cx="15" cy="5" r="1"/><circle cx="15" cy="19" r="1"/>
          </svg>
        </div>
        <span className="provider-card__name">{p.name}</span>
        {tagContent}
      </div>
      <div className="provider-card__body">
        {renderModels()}
      </div>
      <div className="provider-card__actions">
        <button
          type="button"
          className="btn btn--ghost btn--sm btn--icon"
          title={t("common.edit")}
          disabled={busy}
          onClick={() => onEdit(p)}
        >
          <svg viewBox="0 0 24 24" width="16" height="16" stroke="currentColor" strokeWidth="2" fill="none" strokeLinecap="round" strokeLinejoin="round">
            <path d="M17 3a2.828 2.828 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5L17 3z"/>
          </svg>
        </button>
      </div>
    </div>
  );
}
