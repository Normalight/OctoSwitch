import { useEffect, useMemo, useState } from "react";
import { Modal } from "../components/Modal";
import { useI18n } from "../i18n";
import { tauriApi } from "../lib/api/tauri";
import { useModelGroups } from "../hooks/useModelGroups";
import { useModels } from "../hooks/useModels";
import type {
  LocalPluginStatus,
  LocalPluginSyncResult,
  ModelBinding,
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

const EMPTY_FORM = {
  task_kind: "",
  target_group: "",
  target_member: "",
  prompt_template: "",
  is_enabled: true,
};

export function SkillsPage() {
  const { t } = useI18n();
  const { groups } = useModelGroups();
  const { models } = useModels();
  const [preferences, setPreferences] = useState<TaskRoutePreference[]>([]);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [pluginStatus, setPluginStatus] = useState<LocalPluginStatus | null>(null);
  const [pluginSyncResult, setPluginSyncResult] = useState<LocalPluginSyncResult | null>(null);
  const [modal, setModal] = useState<ModalState>({ open: false });
  const [pluginModal, setPluginModal] = useState<PluginModalState>({ open: false });
  const [form, setForm] = useState(EMPTY_FORM);

  const loadPreferences = async () => {
    setLoading(true);
    try {
      setPreferences(await tauriApi.listTaskRoutePreferences());
      setError("");
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const loadPluginStatus = async () => {
    setBusy(true);
    try {
      const status = await tauriApi.inspectCcSwitchOctoswitchPlugin();
      setPluginStatus(status);
      setError("");
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const syncPlugin = async () => {
    setBusy(true);
    try {
      const result = await tauriApi.syncCcSwitchOctoswitchPlugin();
      setPluginSyncResult(result);
      setPluginStatus(result.status);
      setError("");
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    void loadPreferences();
    void loadPluginStatus();
  }, []);

  const membersByGroup = useMemo(() => {
    const map = new Map<string, ModelBinding[]>();
    for (const group of groups) {
      map.set(
        group.alias,
        models.filter((model) => model.group_ids.includes(group.id))
      );
    }
    return map;
  }, [groups, models]);

  const currentMembers = membersByGroup.get(form.target_group) ?? [];

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
      target_member: preference.target_member ?? "",
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
      if (modal.open && modal.mode === "edit" && modal.current) {
        await tauriApi.updateTaskRoutePreference(modal.current.id, {
          task_kind: form.task_kind.trim(),
          target_group: form.target_group.trim(),
          target_member: form.target_member.trim() || null,
          prompt_template: form.prompt_template.trim() || null,
          is_enabled: form.is_enabled,
        });
      } else {
        await tauriApi.createTaskRoutePreference({
          task_kind: form.task_kind.trim(),
          target_group: form.target_group.trim(),
          target_member: form.target_member.trim() || null,
          prompt_template: form.prompt_template.trim() || null,
          is_enabled: form.is_enabled,
        });
      }
      setModal({ open: false });
      setForm(EMPTY_FORM);
      await loadPreferences();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const removePreference = async (id: string) => {
    setBusy(true);
    try {
      await tauriApi.deleteTaskRoutePreference(id);
      await loadPreferences();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const toggleEnabled = async (preference: TaskRoutePreference) => {
    setBusy(true);
    try {
      await tauriApi.updateTaskRoutePreference(preference.id, {
        is_enabled: !preference.is_enabled,
      });
      await loadPreferences();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="page-resource models-page page-groups">
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

      <div className="settings-tab-stack" style={{ marginBottom: 16 }}>
        <div className="card card--compact">
          <h3 style={{ marginTop: 0 }}>{t("skills.routesSectionTitle")}</h3>
          <p className="form-hint muted">{t("skills.routesSectionLead")}</p>
          <ul className="form-hint muted" style={{ margin: "8px 0 0", paddingLeft: 18 }}>
            <li>{t("skills.routesHint1")}</li>
            <li>{t("skills.routesHint2")}</li>
            <li>{t("skills.routesHint3")}</li>
          </ul>
          <p className="form-hint muted" style={{ marginTop: 10 }}>
            {t("skills.routesLimitation")}
          </p>
        </div>

        <div className="card card--compact">
          <div
            style={{
              display: "flex",
              justifyContent: "space-between",
              gap: 12,
              alignItems: "flex-start",
              flexWrap: "wrap",
            }}
          >
            <div>
              <h3 style={{ marginTop: 0 }}>{t("skills.pluginSectionTitle")}</h3>
              <p className="form-hint muted">{t("skills.pluginSectionLead")}</p>
            </div>
            {pluginStatus ? (
              <span
                className={`routing-debug-badge ${
                  pluginStatus.up_to_date
                    ? "routing-debug-badge--active"
                    : "routing-debug-badge--disabled"
                }`}
              >
                {pluginStatus.up_to_date
                  ? t("skills.pluginUpToDate")
                  : t("skills.pluginNeedsUpdate")}
              </span>
            ) : null}
          </div>

          <div className="form-hint muted" style={{ display: "grid", gap: 6 }}>
            <span>{t("skills.pluginMarketplaceUrlLabel")}: https://github.com/Normalight/OctoSwitch</span>
            <span>{t("skills.pluginRepoModeLabel")}: {t("skills.pluginRepoModeValue")}</span>
            {pluginStatus ? (
              <>
                <span>{t("skills.pluginTrackedRepo")}: {pluginStatus.tracked_path}</span>
                <span>{t("skills.pluginInstalledPath")}: {pluginStatus.installed_path}</span>
                <span>{t("skills.pluginStatusLabel")}: {pluginStatus.up_to_date ? t("skills.pluginUpToDate") : t("skills.pluginNeedsUpdate")}</span>
              </>
            ) : null}
          </div>

          <details className="form-hint muted" style={{ marginTop: 10 }}>
            <summary style={{ cursor: "pointer" }}>{t("skills.pluginUsageTitle")}</summary>
            <pre className="skills-textarea" style={{ whiteSpace: "pre-wrap", marginTop: 8 }}>
{`/plugin marketplace add https://github.com/Normalight/OctoSwitch
/plugin install octoswitch@octoswitch
/plugin marketplace update octoswitch
/plugin update octoswitch`}
            </pre>
          </details>
        </div>
      </div>

      <div className="skills-grid">
        {preferences.map((preference) => (
          <article key={preference.id} className="skills-card card card--compact">
            <div className="skills-card__head">
              <div>
                <h3>{preference.task_kind}</h3>
                <p className="form-hint muted">
                  {t("skills.routePrefix")}: {preference.target_group}
                  {preference.target_member ? `/${preference.target_member}` : ""}
                </p>
              </div>
              <span className={`routing-debug-badge ${preference.is_enabled ? "routing-debug-badge--active" : "routing-debug-badge--disabled"}`}>
                {preference.is_enabled ? t("skills.enabled") : t("skills.disabled")}
              </span>
            </div>
            <p className="form-hint muted">
              {preference.prompt_template?.trim()
                ? preference.prompt_template
                : t("skills.noTemplate")}
            </p>
            <div className="settings-section-actions">
              <button type="button" className="btn btn--ghost btn--sm" onClick={() => openEdit(preference)}>
                {t("common.edit")}
              </button>
              <button type="button" className="btn btn--ghost btn--sm" onClick={() => void toggleEnabled(preference)}>
                {preference.is_enabled ? t("skills.disableAction") : t("skills.enableAction")}
              </button>
              <button type="button" className="btn btn--danger btn--sm" onClick={() => void removePreference(preference.id)}>
                {t("common.delete")}
              </button>
            </div>
          </article>
        ))}
        {!loading && preferences.length === 0 ? (
          <div className="card card--compact">
            <p className="muted">{t("skills.empty")}</p>
            <pre className="skills-textarea" style={{ whiteSpace: "pre-wrap", marginTop: 8 }}>
{`/task-route implementation --target Sonnet/gpt-5.4
/task-route review --target Opus/gpt-5.4
/task-route search --target Haiku/MiniMax-M2.7`}
            </pre>
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
              onChange={(e) =>
                setForm((prev) => ({
                  ...prev,
                  target_group: e.target.value,
                  target_member: "",
                }))
              }
            >
              {groups.map((group) => (
                <option key={group.id} value={group.alias}>
                  {group.alias}
                </option>
              ))}
            </select>
          </label>
          <label className="routing-debug-select">
            <span>{t("skills.targetMember")}</span>
            <select
              value={form.target_member}
              onChange={(e) => setForm((prev) => ({ ...prev, target_member: e.target.value }))}
            >
              <option value="">{t("skills.useActiveMember")}</option>
              {currentMembers.map((member) => (
                <option key={member.id} value={member.model_name}>
                  {member.model_name}
                </option>
              ))}
            </select>
          </label>
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
            <button type="button" className="btn btn--primary" onClick={() => void syncPlugin()} disabled={busy || !pluginStatus}>
              {t("skills.pluginSyncButton")}
            </button>
            <button type="button" className="btn btn--ghost" onClick={() => void loadPluginStatus()} disabled={busy}>
              {t("skills.pluginRefreshButton")}
            </button>
            <button type="button" className="btn btn--ghost" onClick={() => setPluginModal({ open: false })}>
              {t("common.cancel")}
            </button>
          </div>
        }
      >
        <div className="settings-tab-stack">
          <p className="form-hint muted">{t("skills.pluginModalLead")}</p>
          {pluginStatus ? (
            <>
              <div className="card card--compact">
                <div className="form-hint muted" style={{ display: "grid", gap: 6 }}>
                  <span>{t("skills.pluginMarketplaceUrlLabel")}: https://github.com/Normalight/OctoSwitch</span>
                  <span>{t("skills.pluginMarketplace")}: {pluginStatus.marketplace_path}</span>
                  <span>{t("skills.pluginRepoRef")}: {pluginStatus.marketplace_repo}</span>
                  <span>{t("skills.pluginTrackedRepo")}: {pluginStatus.tracked_path}</span>
                  <span>{t("skills.pluginInstalledPath")}: {pluginStatus.installed_path}</span>
                  <span>{t("skills.pluginTrackedState")}: {pluginStatus.tracked_exists ? t("common.yes") : t("common.no")}</span>
                  <span>{t("skills.pluginInstalledState")}: {pluginStatus.installed_exists ? t("common.yes") : t("common.no")}</span>
                  <span>{t("skills.pluginStatusLabel")}: {pluginStatus.up_to_date ? t("skills.pluginUpToDate") : t("skills.pluginNeedsUpdate")}</span>
                  <span>{t("skills.pluginTrackedFiles")}: {pluginStatus.tracked_file_count}</span>
                  <span>{t("skills.pluginInstalledFiles")}: {pluginStatus.installed_file_count}</span>
                  <span>{t("skills.pluginMissingFiles")}: {pluginStatus.missing_files.length}</span>
                  <span>{t("skills.pluginChangedFiles")}: {pluginStatus.changed_files.length}</span>
                </div>
              </div>
              <div className="card card--compact">
                <h3 style={{ marginTop: 0 }}>{t("skills.pluginConfigTitle")}</h3>
                <p className="form-hint muted">{t("skills.pluginConfigLead")}</p>
                <ul className="form-hint muted" style={{ margin: "8px 0 0", paddingLeft: 18 }}>
                  <li>{t("skills.pluginConfigHint1")}</li>
                  <li>{t("skills.pluginConfigHint2")}</li>
                  <li>{t("skills.pluginConfigHint3")}</li>
                </ul>
              </div>
              <details className="form-hint muted">
                <summary style={{ cursor: "pointer" }}>{t("skills.pluginDiffDetails")}</summary>
                <pre className="skills-textarea" style={{ whiteSpace: "pre-wrap", marginTop: 8 }}>
                  {JSON.stringify(
                    {
                      missing_files: pluginStatus.missing_files,
                      changed_files: pluginStatus.changed_files,
                    },
                    null,
                    2
                  )}
                </pre>
              </details>
            </>
          ) : (
            <p className="muted">{t("common.loading")}</p>
          )}
          <p className="form-hint muted">{t("skills.pluginSyncHint")}</p>
          {pluginSyncResult ? (
            <details className="form-hint muted" open>
              <summary style={{ cursor: "pointer" }}>{t("skills.pluginSyncResultTitle")}</summary>
              <pre className="skills-textarea" style={{ whiteSpace: "pre-wrap", marginTop: 8 }}>
                {JSON.stringify(
                  {
                    copied_files: pluginSyncResult.copied_files,
                    removed_files: pluginSyncResult.removed_files,
                    preserved_files: pluginSyncResult.preserved_files,
                  },
                  null,
                  2
                )}
              </pre>
            </details>
          ) : null}
        </div>
      </Modal>
    </section>
  );
}
