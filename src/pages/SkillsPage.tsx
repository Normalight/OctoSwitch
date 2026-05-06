import { useEffect, useState } from "react";
import { LoadErrorBanner } from "../components/LoadErrorBanner";
import { ConfirmDialog } from "../components/Dialogs";
import { Modal } from "../components/Modal";
import { useDragToReorder } from "../hooks/useDragToReorder";
import { useI18n } from "../i18n";
import { tauriApi } from "../lib/api/tauri";
import { formatError } from "../lib/formatError";
import { useModelGroups } from "../hooks/useModelGroups";
import type {
  TaskRoutePreference
} from "../types";

type ModalState =
  | { open: false }
  | {
      open: true;
      mode: "create" | "edit";
      current?: TaskRoutePreference;
    };

type PluginModalState = { open: boolean };

type PreferenceForm = {
  task_kind: string;
  target_group: string;
  prompt_template: string;
  is_enabled: boolean;
};

const EMPTY_FORM: PreferenceForm = {
  task_kind: "",
  target_group: "",
  prompt_template: "",
  is_enabled: true,
};

export function SkillsPage() {
  const { t } = useI18n();
  const {
    groups,
    loadError: groupsLoadError,
    clearLoadError: clearGroupsLoadError,
  } = useModelGroups();
  const [preferences, setPreferences] = useState<TaskRoutePreference[]>([]);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [modal, setModal] = useState<ModalState>({ open: false });
  const [pluginModal, setPluginModal] = useState<PluginModalState>({ open: false });
  const [form, setForm] = useState(EMPTY_FORM);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [confirmMsg, setConfirmMsg] = useState({ title: "", message: "" });
  const [confirmAction, setConfirmAction] = useState<(() => void) | null>(null);

  const {
    orderedItems: orderedPreferences,
    draggingId: pointerDraggingPreferenceId,
    dragHoverId: preferenceDragHoverId,
    startDrag: startPreferencePointerDrag,
  } = useDragToReorder(preferences, {
    persistOrder: async (orderedIds) => {
      setBusy(true);
      try {
        for (const [idx, id] of orderedIds.entries()) {
          await tauriApi.updateTaskRoutePreference(id, { sort_order: idx });
        }
        await loadPreferences();
      } catch (e) {
        setError(formatError(e));
      } finally {
        setBusy(false);
      }
    },
    getId: (item) => item.id,
    busy,
  });

  const loadPreferences = async () => {
    setLoading(true);
    try {
      setPreferences(await tauriApi.listTaskRoutePreferences());
      setError("");
    } catch (e) {
      setError(formatError(e));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void loadPreferences();
  }, []);

  const openCreate = () => {
    setForm({
      ...EMPTY_FORM,
      target_group: groups[0]?.alias ?? "",
    });
    setModal({ open: true, mode: "create" });
  };

  const openEdit = (preference: TaskRoutePreference) => {
    setForm({
      task_kind: preference.task_kind,
      target_group: preference.target_group,
      prompt_template: preference.prompt_template ?? "",
      is_enabled: preference.is_enabled,
    });
    setModal({ open: true, mode: "edit", current: preference });
  };

  const savePreference = async () => {
    if (!form.task_kind.trim() || !form.target_group.trim()) {
      setError(t("skills.requiredError"));
      return;
    }
    setBusy(true);
    try {
      const payload = {
        task_kind: form.task_kind.trim(),
        target_group: form.target_group.trim(),
        prompt_template: form.prompt_template.trim() || null,
        is_enabled: form.is_enabled,
      };

      if (modal.open && modal.mode === "edit" && modal.current) {
        await tauriApi.updateTaskRoutePreference(modal.current.id, payload);
      } else {
        await tauriApi.createTaskRoutePreference(payload);
      }

      setModal({ open: false });
      setForm(EMPTY_FORM);
      await loadPreferences();
    } catch (e) {
      setError(formatError(e));
    } finally {
      setBusy(false);
    }
  };

  const removePreference = (id: string, taskKind: string) => {
    setConfirmMsg({
      title: t("skills.deletePreferenceConfirmTitle"),
      message: t("skills.deletePreferenceConfirmBody", { taskKind }),
    });
    setConfirmAction(() => async () => {
      setBusy(true);
      try {
        await tauriApi.deleteTaskRoutePreference(id);
        await loadPreferences();
      } catch (e) {
        setError(formatError(e));
      } finally {
        setBusy(false);
      }
    });
    setConfirmOpen(true);
  };

  const toggleEnabled = async (preference: TaskRoutePreference) => {
    setBusy(true);
    try {
      await tauriApi.updateTaskRoutePreference(preference.id, {
        is_enabled: !preference.is_enabled,
      });
      await loadPreferences();
    } catch (e) {
      setError(formatError(e));
    } finally {
      setBusy(false);
    }
  };

  const enabledPreferences = preferences.filter((item) => item.is_enabled);

  return (
    <section className="page-resource models-page page-groups skills-page">
      <LoadErrorBanner message={groupsLoadError} onDismiss={clearGroupsLoadError} />
      <div className="providers-page-head">
        <div className="providers-page-head__intro">
          <h2 className="page-title providers-page__title">{t("app.skills")}</h2>
          <p className="page-lead muted providers-page-head__lead">
            {t("skills.lead")}
          </p>
        </div>
        <div className="settings-section-actions">
          <button
            type="button"
            className="btn btn--ghost btn--sm"
            onClick={() => setPluginModal({ open: true })}
            disabled={busy}
          >
            {t("skills.pluginManageButton")}
          </button>
          <button
            type="button"
            className="btn btn--primary btn--sm providers-page-head__add"
            onClick={openCreate}
            disabled={busy}
          >
            {t("skills.add")}
          </button>
        </div>
      </div>

      {loading ? <p className="muted">{t("common.loading")}</p> : null}
      {error ? <p className="form-error">{error}</p> : null}

      <div
        className={`provider-list skills-list sortable-list${pointerDraggingPreferenceId ? " sortable-list--dragging" : ""}`}
      >
        {orderedPreferences.map((preference) => (
          <article
            key={preference.id}
            data-sortable-id={preference.id}
            className={[
              "model-group-card skills-pref-card card card--compact sortable-item",
              !preference.is_enabled ? "disabled" : "",
              pointerDraggingPreferenceId === preference.id ? "sortable-item--dragging" : "",
              pointerDraggingPreferenceId && preferenceDragHoverId === preference.id && pointerDraggingPreferenceId !== preference.id
                ? "sortable-item--drop-hover"
                : "",
            ]
              .filter(Boolean)
              .join(" ")}
          >
            <div className="model-group-card__head">
              <div className="model-group-card__display skills-pref-card__display">
                <div
                  className="drag-handle"
                  title={t("common.dragToSort")}
                  onPointerDown={(e) => startPreferencePointerDrag(preference.id, e)}
                >
                  <svg aria-hidden="true" viewBox="0 0 24 24" width="16" height="16" stroke="currentColor" strokeWidth="2" fill="none" strokeLinecap="round" strokeLinejoin="round">
                    <circle cx="9" cy="12" r="1"/><circle cx="9" cy="5" r="1"/><circle cx="9" cy="19" r="1"/><circle cx="15" cy="12" r="1"/><circle cx="15" cy="5" r="1"/><circle cx="15" cy="19" r="1"/>
                  </svg>
                </div>
                <h3 className="model-group-card__title">
                  {preference.task_kind}
                  {!preference.is_enabled ? (
                    <span className="model-group-card__state muted">{t("skills.disabled")}</span>
                  ) : null}
                </h3>
                <span
                  className={`routing-debug-badge ${
                    preference.is_enabled
                      ? "routing-debug-badge--active"
                      : "routing-debug-badge--disabled"
                  }`}
                >
                  {preference.is_enabled ? t("skills.enabled") : t("skills.disabled")}
                </span>
              </div>

              <div className="skills-pref-card__meta-row">
                <span className="skills-pref-card__meta-pill">
                  {t("skills.routePrefix")}: {preference.target_group}
                </span>
                <p className="skills-pref-card__meta muted">
                  {preference.prompt_template?.trim() ? preference.prompt_template : t("skills.noTemplate")}
                </p>
              </div>

              <div className="model-group-card__actions">
                <button
                  type="button"
                  className="btn btn--ghost btn--sm btn--icon"
                  title={t("common.edit")}
                  onClick={() => openEdit(preference)}
                >
                  <svg viewBox="0 0 24 24" width="16" height="16" stroke="currentColor" strokeWidth="2" fill="none" strokeLinecap="round" strokeLinejoin="round">
                    <path d="M17 3a2.828 2.828 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5L17 3z"/>
                  </svg>
                </button>
                <button
                  type="button"
                  className={`btn btn--ghost btn--sm btn--icon ${preference.is_enabled ? "btn-danger" : ""}`}
                  title={preference.is_enabled ? t("skills.disableAction") : t("skills.enableAction")}
                  onClick={() => void toggleEnabled(preference)}
                >
                  {preference.is_enabled ? (
                    <svg viewBox="0 0 24 24" width="16" height="16" stroke="currentColor" strokeWidth="2" fill="none" strokeLinecap="round" strokeLinejoin="round">
                      <circle cx="12" cy="12" r="10"/><line x1="15" y1="9" x2="9" y2="15"/><line x1="9" y1="9" x2="15" y2="15"/>
                    </svg>
                  ) : (
                    <svg viewBox="0 0 24 24" width="16" height="16" stroke="currentColor" strokeWidth="2" fill="none" strokeLinecap="round" strokeLinejoin="round">
                      <polygon points="5 3 19 12 5 21 5 3"/>
                    </svg>
                  )}
                </button>
                <button
                  type="button"
                  className="btn btn--ghost btn--sm btn--icon btn-danger"
                  title={t("common.delete")}
                  onClick={() => removePreference(preference.id, preference.task_kind)}
                >
                  <svg viewBox="0 0 24 24" width="16" height="16" stroke="currentColor" strokeWidth="2" fill="none" strokeLinecap="round" strokeLinejoin="round">
                    <polyline points="3 6 5 6 21 6"/>
                    <path d="M19 6l-1 14H6L5 6"/>
                    <path d="M10 11v6"/>
                    <path d="M14 11v6"/>
                    <path d="M9 6V4h6v2"/>
                  </svg>
                </button>
              </div>
            </div>
          </article>
        ))}

        {!loading && preferences.length === 0 ? (
          <div className="model-group-card skills-pref-card card card--compact">
            <div className="model-group-card__head">
              <div className="model-group-card__display">
                <h3 className="model-group-card__title">{t("skills.empty")}</h3>
                <span className="model-group-active-pill">
                  {t("skills.emptyHint")}
                </span>
              </div>
              <div className="model-group-card__actions">
                <span className="routing-debug-badge routing-debug-badge--active">
                {t("skills.routesBadge", { count: enabledPreferences.length })}
                </span>
              </div>
            </div>
          </div>
        ) : null}
      </div>

      <Modal
        title={modal.open && modal.mode === "edit" ? t("skills.modalEdit") : t("skills.modalCreate")}
        open={modal.open}
        onClose={() => {
          setModal({ open: false });
          setForm(EMPTY_FORM);
        }}
        footer={
          <div className="panel-actions flat">
            <button type="button" className="btn btn--primary" onClick={() => void savePreference()} disabled={busy}>
              {t("common.save")}
            </button>
            <button type="button" className="btn btn--ghost" onClick={() => setModal({ open: false })}>
              {t("common.cancel")}
            </button>
          </div>
        }
      >
        <div className="settings-tab-stack">
          <label className="routing-debug-select">
            <span>{t("skills.taskKind")}</span>
            <input
              value={form.task_kind}
              onChange={(e) => setForm((prev) => ({ ...prev, task_kind: e.target.value }))}
              placeholder={t("skills.taskKindPlaceholder")}
            />
          </label>
          <label className="routing-debug-select">
            <span>{t("skills.targetGroup")}</span>
            <select
              value={form.target_group}
              onChange={(e) => setForm((prev) => ({ ...prev, target_group: e.target.value }))}
            >
              <option value="">{t("skills.targetGroupPlaceholder")}</option>
              {groups.map((group) => (
                <option key={group.id} value={group.alias}>
                  {group.alias}
                </option>
              ))}
            </select>
          </label>
          {groups.length === 0 ? <p className="form-hint muted">{t("skills.groupsEmpty")}</p> : null}
          <label className="routing-debug-select">
            <span>{t("skills.promptTemplate")}</span>
            <textarea
              className="skills-textarea"
              rows={8}
              value={form.prompt_template}
              onChange={(e) => setForm((prev) => ({ ...prev, prompt_template: e.target.value }))}
              placeholder={t("skills.promptTemplatePlaceholder")}
            />
          </label>
          <label className="settings-behavior-item">
            <span className="settings-behavior-label">{t("skills.fieldEnabled")}</span>
            <input
              type="checkbox"
              checked={form.is_enabled}
              onChange={(e) => setForm((prev) => ({ ...prev, is_enabled: e.target.checked }))}
            />
          </label>
        </div>
      </Modal>

      <Modal
        title={t("skills.pluginModalTitle")}
        open={pluginModal.open}
        onClose={() => setPluginModal({ open: false })}
        footer={
          <div className="panel-actions flat">
            <button type="button" className="btn btn--ghost" onClick={() => setPluginModal({ open: false })}>
              {t("common.cancel")}
            </button>
          </div>
        }
      >
        <div className="settings-tab-stack">
          <p className="form-hint muted">{t("skills.pluginModalLead")}</p>

          <div className="card card--compact">
            <div className="skills-kv">
              <span>{t("skills.pluginMarketplaceUrlLabel")}</span>
              <strong>https://github.com/Normalight/OctoSwitch</strong>
            </div>
          </div>

          <div className="skills-callout">
            <strong>{t("skills.pluginUsageTitle")}</strong>
            <pre className="skills-textarea skills-textarea--compact">
{`/plugin marketplace add https://github.com/Normalight/OctoSwitch
/plugin install octoswitch@octoswitch
/plugin update octoswitch
/agents`}
            </pre>
          </div>

          <p className="form-hint muted">{t("skills.reloadTitle")}</p>
          <p className="form-hint muted">{t("skills.reloadBody")}</p>
        </div>
      </Modal>

      <ConfirmDialog
        title={confirmMsg.title}
        message={confirmMsg.message}
        open={confirmOpen}
        onClose={() => { setConfirmOpen(false); setConfirmAction(null); }}
        onConfirm={() => { if (confirmAction) void confirmAction(); }}
        confirmText={t("common.delete")}
        confirmVariant="danger"
      />
    </section>
  );
}
