import { ModelStack } from "./ModelStack";
import { Modal } from "./Modal";
import type { CopilotStatus, ModelBinding, Provider } from "../types";

type ProviderEditModalProps = {
  open: boolean;
  mode: "create" | "edit";
  provider?: Provider;
  form: {
    name: string;
    baseUrl: string;
    apiKeyRef: string;
    apiFormat: string;
    authMode: string;
    enabled: boolean;
    showApiKey: boolean;
    advancedOpen: boolean;
  };
  setForm: {
    name: (v: string) => void;
    baseUrl: (v: string) => void;
    apiKeyRef: (v: string) => void;
    apiFormat: (v: string) => void;
    authMode: (v: string) => void;
    enabled: (v: boolean) => void;
    showApiKey: (v: boolean) => void;
    advancedOpen: (v: boolean) => void;
  };
  busy: boolean;
  healthMsg: string | null;
  healthDetail: string | null;
  onSave: () => void;
  onDelete: () => void;
  onHealth: () => void;
  onClose: () => void;
  copilotStatus: CopilotStatus | null;
  modelsForProvider: ModelBinding[];
  onAddBinding: () => void;
  onEditBinding: (binding: ModelBinding) => void;
  onRemoveBinding: (binding: ModelBinding) => void;
  groupAlias: (id: string) => string;
  t: (key: string, vars?: Record<string, string | number>) => string;
};

export function ProviderEditModal({
  open,
  mode,
  provider,
  form,
  setForm,
  busy,
  healthMsg,
  healthDetail,
  onSave,
  onDelete,
  onHealth,
  onClose,
  copilotStatus,
  modelsForProvider,
  onAddBinding,
  onEditBinding,
  onRemoveBinding,
  groupAlias,
  t,
}: ProviderEditModalProps) {
  const getCopilotStatusText = () => {
    if (!copilotStatus?.authenticated) return t("copilot.statusUnauthorized");
    const expiresTs = Number(copilotStatus.token_expires_at ?? 0) * 1000;
    const now = Date.now();
    const isExpired = expiresTs > 0 && now >= expiresTs;
    const isExpiringSoon = expiresTs > 0 && now < expiresTs && expiresTs - now <= 5 * 60 * 1000;
    if (isExpired) return t("copilot.statusExpired");
    if (isExpiringSoon) return t("copilot.statusExpiring");
    return t("copilot.statusAuthorized");
  };

  return (
    <Modal
      title={mode === "create" ? t("providers.modalCreate") : t("providers.modalEdit")}
      open={open}
      onClose={onClose}
      footer={
        <div className="provider-edit-modal__footer">
          <div className="provider-edit-modal__footer-primary">
            <button type="button" className="btn btn--primary" disabled={busy || !form.name || !form.baseUrl} onClick={onSave}>
              {t("common.save")}
            </button>
            {mode === "edit" ? (
              <button type="button" className="btn btn--danger" disabled={busy} onClick={onDelete}>
                {t("common.delete")}
              </button>
            ) : null}
          </div>
          {mode === "edit" ? (
            <div className="provider-edit-modal__footer-health">
              <button type="button" className="btn btn--ghost" disabled={busy} onClick={onHealth}>
                {t("providers.healthCheck")}
              </button>
            </div>
          ) : null}
        </div>
      }
    >
      {mode === "edit" && healthMsg ? (
        <div className="provider-edit-modal__health-panel">
          <p className="provider-edit-modal__health-result muted" title={healthDetail || healthMsg || undefined}>
            {healthMsg}
          </p>
        </div>
      ) : null}
      <div className="form-stack">
        <label>
          {t("providers.labelName")}
          <input value={form.name} onChange={(e) => setForm.name(e.target.value)} placeholder={t("providers.phName")} />
        </label>
        <label>
          {t("providers.labelBaseUrl")}
          <input value={form.baseUrl} onChange={(e) => setForm.baseUrl(e.target.value)} placeholder="https://api.example.com" />
        </label>
        <label>
          {t("providers.labelApiKey")}
          <span className="api-key-input-row">
            <input
              type={form.showApiKey ? "text" : "password"}
              value={form.apiKeyRef}
              onChange={(e) => setForm.apiKeyRef(e.target.value)}
              placeholder={t("providers.phApiKey")}
            />
            <button
              type="button"
              className="api-key-toggle-btn"
              title={form.showApiKey ? t("copilot.close") : t("providers.showApiKey")}
              aria-label={form.showApiKey ? t("providers.hideApiKey") : t("providers.showApiKey")}
              onClick={() => setForm.showApiKey(!form.showApiKey)}
            >
              {form.showApiKey ? (
                <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94" />
                  <path d="M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19" />
                  <path d="M14.12 14.12a3 3 0 1 1-4.24-4.24" />
                  <line x1="1" y1="1" x2="23" y2="23" />
                </svg>
              ) : (
                <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
                  <circle cx="12" cy="12" r="3" />
                </svg>
              )}
            </button>
          </span>
        </label>
        <label className="field-checkbox">
          <span className="field-checkbox-text">{t("providers.enableProvider")}</span>
          <input type="checkbox" checked={form.enabled} onChange={(e) => setForm.enabled(e.target.checked)} />
        </label>

        <details className="advanced-options" open={form.advancedOpen} onToggle={(e) => setForm.advancedOpen(e.currentTarget.open)}>
          <summary className="advanced-options__summary">{t("providers.advancedOptions")}</summary>
          <label>
            {t("providers.labelApiFormat")}
            <select value={form.apiFormat} onChange={(e) => setForm.apiFormat(e.target.value)}>
              <option value="anthropic">{t("providers.apiFormatAnthropic")}</option>
              <option value="openai_chat">{t("providers.apiFormatOpenaiChat")}</option>
              <option value="openai_responses">{t("providers.apiFormatOpenaiResponses")}</option>
            </select>
          </label>
          <label>
            {t("providers.labelAuthMode")}
            <select value={form.authMode} onChange={(e) => setForm.authMode(e.target.value)}>
              <option value="bearer">{t("providers.authModeBearer")}</option>
              <option value="anthropic_api_key">{t("providers.authModeAnthropicApiKey")}</option>
            </select>
          </label>
        </details>
        {mode === "create" ? (
          <div className="provider-linked-models">
            <p className="provider-linked-models__empty muted">{t("providers.hintBindingsAfterSave")}</p>
          </div>
        ) : null}
      </div>
      {mode === "edit" && provider ? (
        <div className="provider-linked-models provider-linked-models--standalone">
            {provider.id === "copilot" ? (
              <div className="provider-linked-models__empty muted">
                <p>
                  {t("copilot.providerStatus")}: {getCopilotStatusText()}
                  {copilotStatus?.authenticated && copilotStatus.account_type
                    ? ` (${t("copilot.accountType")}: ${copilotStatus.account_type})`
                    : ""}
                </p>
                {copilotStatus?.authenticated && copilotStatus.account_login ? (
                  <p>
                    {t("copilot.accountLogin")}: {copilotStatus.account_login}
                  </p>
                ) : null}
                {copilotStatus?.authenticated && copilotStatus.token_expires_at ? (
                  <p>
                    {t("copilot.expiresAt")}: {new Date(Number(copilotStatus.token_expires_at) * 1000).toLocaleString()}
                  </p>
                ) : null}
              </div>
            ) : null}

            {provider.id !== "copilot" ? (
              <>
                <div className="provider-linked-models__toolbar">
                  <h4 className="provider-linked-models__title provider-linked-models__title--inline">{t("providers.linkedModels")}</h4>
                  <button type="button" className="btn btn--primary btn--sm" disabled={busy} onClick={onAddBinding}>
                    {t("models.addBinding")}
                  </button>
                </div>

                {modelsForProvider.length === 0 ? (
                  <p className="provider-linked-models__empty muted">{t("providers.noModelsYet")}</p>
                ) : (
                  <ModelStack
                    bindings={modelsForProvider}
                    keyPrefix={`modal-${provider.id}`}
                    maxVisible={999}
                    variant="detail"
                    onRemove={onRemoveBinding}
                    onEdit={onEditBinding}
                    groupAlias={groupAlias}
                    showGroupOnChip={false}
                    showUpstreamOnChip={false}
                  />
                )}
              </>
            ) : null}
          </div>
        ) : null}
    </Modal>
  );
}
