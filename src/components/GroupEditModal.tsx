import { Modal } from "./Modal";
import type { ModelBinding, ModelGroup, Provider } from "../types";

type GroupEditModalProps = {
  open: boolean;
  mode: "create" | "edit";
  group?: ModelGroup;
  aliasDraft: string;
  onAliasChange: (v: string) => void;
  busy: boolean;
  error: string;
  onSave: () => void;
  onDelete: () => void;
  onClose: () => void;
  /** Members of the group (only used in edit mode) */
  members: ModelBinding[];
  /** Resolve provider ID to name */
  providerLabel: (id: string) => string;
  /** All providers for the dropdown */
  providers: Provider[];
  /** Bindings available for a provider (excluding current group) */
  bindingsForProvider: (providerId: string, excludeGroupId?: string) => ModelBinding[];
  /** Member picker state */
  memPick: { provider: string; binding: string };
  onMemPickChange: (provider: string, binding: string) => void;
  memberError: string;
  onAddMember: () => void;
  onRemoveMember: (binding: ModelBinding) => void;
  onSetActive: (binding: ModelBinding) => void;
  t: (key: string) => string;
};

export function GroupEditModal({
  open,
  mode,
  group,
  aliasDraft,
  onAliasChange,
  busy,
  error,
  onSave,
  onDelete,
  onClose,
  members,
  providerLabel,
  providers,
  bindingsForProvider,
  memPick,
  onMemPickChange,
  memberError,
  onAddMember,
  onRemoveMember,
  onSetActive,
  t,
}: GroupEditModalProps) {
  return (
    <Modal
      title={mode === "create" ? t("models.modalGroupCreate") : t("models.modalGroupEdit")}
      open={open}
      onClose={onClose}
      footer={
        <div className="panel-actions flat" style={{ width: "100%", justifyContent: "space-between" }}>
          {mode === "edit" ? (
            <button type="button" className="btn btn--danger" disabled={busy} onClick={onDelete}>
              {t("models.deleteGroup")}
            </button>
          ) : <span />}
          <button type="button" className="btn btn--primary" disabled={busy} onClick={onSave}>
            {t("common.save")}
          </button>
        </div>
      }
    >
      <div className="form-stack">
        <label>
          {t("models.groupTitle")}
          <input value={aliasDraft} onChange={(e) => onAliasChange(e.target.value)} placeholder={t("models.phAlias")} />
        </label>
        <p className="form-hint muted">{t("models.groupHint")}</p>

        {mode === "edit" && group && (
          <>
            <h4 className="edit-panel-section-title">{t("groups.members")}</h4>
            {members.length === 0 ? (
              <p className="muted" style={{ fontSize: "0.85rem" }}>{t("groups.noMembers")}</p>
            ) : (
              <div className="edit-member-list">
                {members.map((m) => (
                  <div key={m.id} className="edit-member-row">
                    <div className="edit-member-info">
                      <span className="edit-member-name">{m.model_name}</span>
                      <span className="muted" style={{ fontSize: "0.8rem" }}>
                        {providerLabel(m.provider_id)} &middot; {m.upstream_model_name}
                      </span>
                      {!m.is_enabled ? (
                        <span className="muted" style={{ fontSize: "0.75rem" }}> ({t("groups.memberDisabled")})</span>
                      ) : null}
                    </div>
                    <div className="edit-member-actions">
                      {m.id === group.active_binding_id && (
                        <span className="member-active-badge">{t("groups.activeIndicator")}</span>
                      )}
                      <button
                        type="button"
                        className="btn btn--ghost btn--sm"
                        disabled={busy}
                        onClick={() => onSetActive(m)}
                      >
                        {t("groups.setActive")}
                      </button>
                      <button
                        type="button"
                        className="btn btn--danger-ghost btn--sm"
                        disabled={busy}
                        onClick={() => onRemoveMember(m)}
                      >
                        {t("groups.removeMember")}
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            )}

            <h4 className="edit-panel-section-title">{t("groups.addMemberInline")}</h4>
            <label>
              {t("models.labelProviderSelect")}
              <select
                value={memPick.provider}
                onChange={(e) => onMemPickChange(e.target.value, "")}
              >
                <option value="">{t("models.selectProvider")}</option>
                {providers.map((p) => (
                  <option key={p.id} value={p.id}>{p.name}</option>
                ))}
              </select>
            </label>
            <label>
              {t("groups.pickModel")}
              <select
                value={memPick.binding}
                onChange={(e) => onMemPickChange(memPick.provider, e.target.value)}
                disabled={!memPick.provider}
              >
                <option value="">{t("groups.chooseBinding")}</option>
                {bindingsForProvider(memPick.provider, group.id).map((m) => (
                  <option key={m.id} value={m.id}>{m.model_name}</option>
                ))}
              </select>
            </label>
            <button
              type="button"
              className="btn btn--primary btn--sm"
              disabled={busy || !memPick.binding}
              onClick={onAddMember}
            >
              {t("common.add")}
            </button>
            {memberError ? <p className="form-error">{memberError}</p> : null}
          </>
        )}

        {error ? <p className="form-error">{error}</p> : null}
      </div>
    </Modal>
  );
}
