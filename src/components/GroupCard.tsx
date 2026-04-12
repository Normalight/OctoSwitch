import type { ModelBinding, ModelGroup, Provider } from "../types";
import type { PointerEvent } from "react";

type GroupCardProps = {
  group: ModelGroup;
  members: ModelBinding[];
  activeProvider: string;
  activeModel: string;
  busy: boolean;
  onDragStart: (id: string, e: PointerEvent<HTMLDivElement>) => void;
  reorderUi?: { activeId: string | null; hoverId: string | null };
  onEdit: (group: ModelGroup) => void;
  onToggle: (group: ModelGroup) => void;
  t: (key: string) => string;
};

export function GroupCard({
  group: g,
  activeProvider,
  activeModel,
  busy,
  onDragStart,
  reorderUi,
  onEdit,
  onToggle,
  t,
}: GroupCardProps) {
  const isDragging = reorderUi?.activeId === g.id;
  const isDropHover =
    Boolean(reorderUi?.activeId) &&
    reorderUi?.hoverId === g.id &&
    reorderUi?.activeId !== g.id;

  return (
    <div
      key={g.id}
      data-sortable-id={g.id}
      className={[
        "model-group-card card card--compact sortable-item",
        !g.is_enabled ? "disabled" : "",
        isDragging ? "sortable-item--dragging" : "",
        isDropHover ? "sortable-item--drop-hover" : "",
      ]
        .filter(Boolean)
        .join(" ")}
    >
      <div className="model-group-card__head">
        <div className="model-group-card__display">
          <div
            className="drag-handle"
            title={t("common.dragToSort")}
            onPointerDown={(e) => onDragStart(g.id, e)}
          >
            <svg aria-hidden="true" viewBox="0 0 24 24" width="16" height="16" stroke="currentColor" strokeWidth="2" fill="none" strokeLinecap="round" strokeLinejoin="round">
              <circle cx="9" cy="12" r="1"/><circle cx="9" cy="5" r="1"/><circle cx="9" cy="19" r="1"/><circle cx="15" cy="12" r="1"/><circle cx="15" cy="5" r="1"/><circle cx="15" cy="19" r="1"/>
            </svg>
          </div>
          <h3 className="model-group-card__title">
            {g.alias}
            {!g.is_enabled ? (
              <span className="model-group-card__state muted">{t("models.groupDisabled")}</span>
            ) : null}
          </h3>
          <span className="model-group-active-pill">
            {activeProvider ? (
              <>
                <span className="model-group-active-pill__part">{activeProvider}</span>
                <span className="model-group-active-pill__sep" aria-hidden="true">&middot;</span>
                <span className="model-group-active-pill__part">{activeModel}</span>
              </>
            ) : (
              t("models.notChosen")
            )}
          </span>
        </div>
        <div className="model-group-card__actions">
          <button
            type="button"
            className="btn btn--ghost btn--sm btn--icon"
            title={t("models.editGroup")}
            disabled={busy}
            onClick={() => onEdit(g)}
          >
            <svg viewBox="0 0 24 24" width="16" height="16" stroke="currentColor" strokeWidth="2" fill="none" strokeLinecap="round" strokeLinejoin="round">
              <path d="M17 3a2.828 2.828 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5L17 3z"/>
            </svg>
          </button>
          <button
            type="button"
            className={`btn btn--ghost btn--sm btn--icon ${g.is_enabled ? "btn-danger" : ""}`}
            title={g.is_enabled ? t("models.disableGroup") : t("models.enableGroup")}
            disabled={busy}
            onClick={() => onToggle(g)}
          >
            {g.is_enabled ? (
              <svg viewBox="0 0 24 24" width="16" height="16" stroke="currentColor" strokeWidth="2" fill="none" strokeLinecap="round" strokeLinejoin="round">
                <circle cx="12" cy="12" r="10"/><line x1="15" y1="9" x2="9" y2="15"/><line x1="9" y1="9" x2="15" y2="15"/>
              </svg>
            ) : (
              <svg viewBox="0 0 24 24" width="16" height="16" stroke="currentColor" strokeWidth="2" fill="none" strokeLinecap="round" strokeLinejoin="round">
                <polygon points="5 3 19 12 5 21 5 3"/>
              </svg>
            )}
          </button>
        </div>
      </div>
    </div>
  );
}
