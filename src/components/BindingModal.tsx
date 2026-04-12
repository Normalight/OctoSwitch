import { Modal } from "./Modal";
import { UpstreamModelPicker } from "./UpstreamModelPicker";
import type { ModelBinding, Provider } from "../types";

type BindingModalProps = {
  open: boolean;
  editingBinding: ModelBinding | null;
  form: {
    modelName: string;
    upstream: string;
    providerId: string;
    enabled: boolean;
  };
  setForm: {
    modelName: (v: string) => void;
    upstream: (v: string) => void;
    providerId: (v: string) => void;
    enabled: (v: boolean) => void;
  };
  providers: Provider[];
  routingError: string | null;
  upstreamError: string | null;
  busy: boolean;
  error: string | null;
  onSave: () => void;
  onClose: () => void;
  /** Shown in edit mode */
  onDelete?: () => void;
  t: (key: string, vars?: Record<string, string | number>) => string;
};

export function BindingModal({
  open,
  editingBinding,
  form,
  setForm,
  providers,
  routingError,
  upstreamError,
  busy,
  error,
  onSave,
  onClose,
  onDelete,
  t,
}: BindingModalProps) {
  const providerIdForFetch = form.providerId || null;

  return (
    <Modal
      title={editingBinding ? t("models.modalBindingEdit") : t("models.modalBindingCreate")}
      open={open}
      onClose={() => {
        if (!busy) onClose();
      }}
      footer={
        <div className="panel-actions flat">
          <button type="button" className="btn btn--primary" disabled={busy} onClick={onSave}>
            {editingBinding ? t("common.save") : (
              <svg viewBox="0 0 24 24" width="16" height="16" stroke="currentColor" strokeWidth="2.5" fill="none" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                <line x1="12" y1="5" x2="12" y2="19" /><line x1="5" y1="12" x2="19" y2="12" />
              </svg>
            )}
          </button>
          {editingBinding && onDelete ? (
            <button type="button" className="btn btn--danger" disabled={busy} onClick={onDelete}>
              {t("common.delete")}
            </button>
          ) : null}
        </div>
      }
    >
      <div className="form-stack">
        <label>
          {t("models.labelLogicalName")}
          <input
            value={form.modelName}
            onChange={(e) => setForm.modelName(e.target.value)}
            disabled={!!editingBinding}
            placeholder={t("models.phRoutingName")}
          />
          {routingError ? <p className="form-error">{routingError}</p> : null}
        </label>
        <label>
          {t("models.labelProviderSelect")}
          <select
            value={form.providerId}
            onChange={(e) => setForm.providerId(e.target.value)}
            disabled={!!editingBinding}
          >
            <option value="">{t("models.selectProvider")}</option>
            {providers.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name}
              </option>
            ))}
          </select>
        </label>
        <label>
          {t("models.labelUpstream")}
          <UpstreamModelPicker
            value={form.upstream}
            onChange={(v) => setForm.upstream(v)}
            providerId={providerIdForFetch}
            disabled={busy}
            validationError={upstreamError}
            t={t}
          />
        </label>
        <label className="field-checkbox">
          <span className="field-checkbox-text">{t("models.enableBinding")}</span>
          <input type="checkbox" checked={form.enabled} onChange={(e) => setForm.enabled(e.target.checked)} />
        </label>
        {error ? <p className="form-error">{error}</p> : null}
      </div>
    </Modal>
  );
}
