import { Modal } from "./Modal";
import { UpstreamModelPicker } from "./UpstreamModelPicker";
import type { ModelBinding } from "../types";

type Props = {
  open: boolean;
  mode: "create" | "edit";
  providerId: string;
  /** 编辑时传入，用于展示所属分组 */
  binding: ModelBinding | null;
  routingName: string;
  upstream: string;
  setRoutingName: (v: string) => void;
  setUpstream: (v: string) => void;
  routingError: string | null;
  upstreamError: string | null;
  submitError: string | null;
  busy: boolean;
  onSubmit: () => void;
  onClose: () => void;
  onRequestDelete?: () => void;
  groupAlias: (groupId: string) => string;
  t: (key: string, vars?: Record<string, string | number>) => string;
};

export function ProviderBindingModal({
  open,
  mode,
  providerId,
  binding,
  routingName,
  upstream,
  setRoutingName,
  setUpstream,
  routingError,
  upstreamError,
  submitError,
  busy,
  onSubmit,
  onClose,
  onRequestDelete,
  groupAlias,
  t,
}: Props) {
  const title = mode === "create" ? t("models.modalBindingCreate") : t("models.modalBindingEdit");
  const isEdit = mode === "edit";

  return (
    <Modal
      variant="nested"
      title={title}
      open={open}
      onClose={() => {
        if (!busy) onClose();
      }}
      footer={
        <div className="provider-binding-submodal__footer">
          <div className="provider-binding-submodal__footer-start">
            <button type="button" className="btn btn--primary" disabled={busy} onClick={onSubmit}>
              {t("common.save")}
            </button>
            <button type="button" className="btn btn--ghost" disabled={busy} onClick={onClose}>
              {t("common.cancel")}
            </button>
            {isEdit && onRequestDelete ? (
              <button type="button" className="btn btn--danger" disabled={busy} onClick={onRequestDelete}>
                {t("common.delete")}
              </button>
            ) : null}
          </div>
        </div>
      }
    >
      <div className="provider-binding-submodal">
        {isEdit && binding ? (
          <section className="provider-binding-submodal__meta" aria-labelledby="provider-binding-groups-title">
            <h4 id="provider-binding-groups-title" className="provider-binding-submodal__meta-title">
              {t("models.bindingGroupsSection")}
            </h4>
            {binding.group_ids.length > 0 ? (
              <ul className="provider-binding-submodal__group-list">
                {binding.group_ids.map((gid) => (
                  <li key={gid} className="provider-binding-submodal__group-pill">
                    {groupAlias(gid)}
                  </li>
                ))}
              </ul>
            ) : (
              <p className="provider-binding-submodal__meta-empty muted">{t("models.bindingNoGroups")}</p>
            )}
          </section>
        ) : (
          <p className="provider-binding-submodal__hint muted">{t("providers.bindingCreateHint")}</p>
        )}

        <div className="provider-binding-submodal__form form-stack">
          <label>
            <span className="provider-binding-submodal__label">{t("models.labelLogicalName")}</span>
            <input
              value={routingName}
              onChange={(e) => setRoutingName(e.target.value)}
              disabled={busy || isEdit}
              placeholder={t("models.phRoutingName")}
              autoComplete="off"
            />
            {routingError ? <p className="form-error">{routingError}</p> : null}
          </label>
          <label>
            <span className="provider-binding-submodal__label">{t("models.labelUpstream")}</span>
            <UpstreamModelPicker
              value={upstream}
              onChange={setUpstream}
              providerId={providerId}
              disabled={busy}
              validationError={upstreamError}
              t={t}
            />
          </label>
        </div>
        {submitError ? <p className="form-error provider-binding-submodal__server-err">{submitError}</p> : null}
      </div>
    </Modal>
  );
}
