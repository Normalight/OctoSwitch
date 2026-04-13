import { useEffect, useMemo, useState } from "react";
import { Modal } from "../components/Modal";
import { useI18n } from "../i18n";
import { tauriApi } from "../lib/api/tauri";
import { useModelGroups } from "../hooks/useModelGroups";
import { useModels } from "../hooks/useModels";
import type { LocalSkillsStatus, ModelBinding, TaskRoutePreference } from "../types";

type ModalState =
  | { open: false }
  | {
      open: true;
      mode: "create" | "edit";
      current?: TaskRoutePreference;
    };

const EMPTY_FORM = {
  task_kind: "",
  target_group: "",
  target_member: "",
  prompt_template: "",
  is_enabled: true,
};

function renderSkillBadges(skills: LocalSkillsStatus["source_skills"], missingLabel: string) {
  if (skills.length === 0) {
    return <span className="skills-path-badge skills-path-badge--empty">—</span>;
  }

  return skills.map((skill) => (
    <span
      key={skill.path}
      className={`skills-path-badge${skill.has_skill_md ? "" : " skills-path-badge--warn"}`}
      title={skill.path}
    >
      {skill.name}
      {skill.has_skill_md ? null : (
        <span className="skills-path-badge__meta">{missingLabel}</span>
      )}
    </span>
  ));
}

export function SkillsPage() {
  const { t } = useI18n();
  const { groups } = useModelGroups();
  const { models } = useModels();
  const [preferences, setPreferences] = useState<TaskRoutePreference[]>([]);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [pathMsg, setPathMsg] = useState("");
  const [pathsOpen, setPathsOpen] = useState(false);
  const [modal, setModal] = useState<ModalState>({ open: false });
  const [form, setForm] = useState(EMPTY_FORM);
  const [skillsSourcePath, setSkillsSourcePath] = useState("");
  const [pluginNamespace, setPluginNamespace] = useState("octoswitch");
  const [pluginDistPath, setPluginDistPath] = useState("");
  const [pluginBuildMsg, setPluginBuildMsg] = useState("");
  const [marketBuildMsg, setMarketBuildMsg] = useState("");
  const [localSkills, setLocalSkills] = useState<LocalSkillsStatus | null>(null);

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

  const loadPathsAndScan = async () => {
    try {
      const cfg = await tauriApi.getGatewayConfig();
      setSkillsSourcePath(cfg.skills_source_path ?? "");
      setPluginNamespace(cfg.plugin_namespace ?? "octoswitch");
      setPluginDistPath(cfg.plugin_dist_path ?? "");
      setLocalSkills(
        await tauriApi.inspectLocalSkillsPaths(
          cfg.skills_source_path ?? "",
          cfg.skills_source_path ?? ""
        )
      );
      setPathMsg("");
    } catch (e) {
      setPathMsg(String(e));
    }
  };

  useEffect(() => {
    void loadPreferences();
    void loadPathsAndScan();
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

  const saveSkillPaths = async () => {
    setBusy(true);
    try {
      const current = await tauriApi.getGatewayConfig();
      await tauriApi.updateGatewayConfig({
        ...current,
        skills_source_path: skillsSourcePath.trim(),
      });
      setLocalSkills(
        await tauriApi.inspectLocalSkillsPaths(
          skillsSourcePath.trim(),
          skillsSourcePath.trim()
        )
      );
      setPathMsg("");
    } catch (e) {
      setPathMsg(String(e));
    } finally {
      setBusy(false);
    }
  };

  const copyToCcSwitch = async () => {
    setBusy(true);
    try {
      const status = await tauriApi.quickInstallRepoSkillsToCcSwitch();
      setSkillsSourcePath(status.source_path);
      const current = await tauriApi.getGatewayConfig();
      await tauriApi.updateGatewayConfig({
        ...current,
        skills_source_path: status.source_path,
      });
      setLocalSkills(
        await tauriApi.inspectLocalSkillsPaths(
          status.source_path,
          status.source_path
        )
      );
      setPathMsg(t("skills.copyDoneHint"));
    } catch (e) {
      setPathMsg(String(e));
    } finally {
      setBusy(false);
    }
  };

  const buildPluginDist = async () => {
    setBusy(true);
    setPluginBuildMsg("");
    try {
      const result = await tauriApi.buildPluginDist();
      setPluginBuildMsg(`Built plugin dist at ${result.output_path} (${result.files.length} files)`);
      const cfg = await tauriApi.getGatewayConfig();
      setPluginNamespace(cfg.plugin_namespace ?? "octoswitch");
      setPluginDistPath(cfg.plugin_dist_path ?? "");
    } catch (e) {
      setPluginBuildMsg(String(e));
    } finally {
      setBusy(false);
    }
  };

  const buildMarketplaceDist = async () => {
    setBusy(true);
    setMarketBuildMsg("");
    try {
      const result = await tauriApi.buildMarketplaceDist();
      setMarketBuildMsg(`Built marketplace dist at ${result.output_path} (${result.files.length} files)`);
      const cfg = await tauriApi.getGatewayConfig();
      setPluginDistPath(cfg.plugin_dist_path ?? "");
    } catch (e) {
      setMarketBuildMsg(String(e));
    } finally {
      setBusy(false);
    }
  };

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
            className="btn btn--accent-soft btn--sm"
            onClick={() => setPathsOpen(true)}
            disabled={busy}
          >
            {t("skills.pathsButton")}
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

      <div className="skills-grid">
        <article className="skills-card card card--compact">
          <div className="skills-card__head">
            <div>
              <h3>Plugin Dist</h3>
              <p className="form-hint muted">Build distributable plugin artifacts from the project-local skills.</p>
            </div>
            <span className="routing-debug-badge routing-debug-badge--active">/{pluginNamespace}:*</span>
          </div>
          <p className="form-hint muted">Output path: {pluginDistPath || "—"}</p>
          <p className="form-hint muted">Exportable commands: delegate, show-routing, route-activate, task-route</p>
          {pluginBuildMsg ? <p className="form-hint muted">{pluginBuildMsg}</p> : null}
          {marketBuildMsg ? <p className="form-hint muted">{marketBuildMsg}</p> : null}
          <div className="settings-section-actions">
            <button type="button" className="btn btn--primary btn--sm" onClick={() => void buildPluginDist()} disabled={busy}>
              Build Plugin Dist
            </button>
            <button type="button" className="btn btn--ghost btn--sm" onClick={() => void buildMarketplaceDist()} disabled={busy}>
              Build Marketplace Dist
            </button>
          </div>
        </article>
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
                {preference.is_enabled ? "Disable" : "Enable"}
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
        title={t("skills.pathsModalTitle")}
        open={pathsOpen}
        onClose={() => setPathsOpen(false)}
        footer={
          <div className="skills-path-modal__footer">
            <div className="skills-path-modal__footer-start">
              <button type="button" className="btn btn--ghost btn--sm" onClick={() => void loadPathsAndScan()} disabled={busy}>
                {t("common.refresh")}
              </button>
            </div>
            <div className="skills-path-modal__footer-actions">
              <button type="button" className="btn btn--primary" onClick={() => void saveSkillPaths()} disabled={busy}>
                {t("common.save")}
              </button>
              <button type="button" className="btn btn--ghost" onClick={() => setPathsOpen(false)}>
                {t("common.cancel")}
              </button>
            </div>
          </div>
        }
      >
        <div className="settings-tab-stack skills-path-modal">
          <section className="skills-path-panel">
            <div className="skills-path-panel__head">
              <h3>{t("skills.pathConfigTitle")}</h3>
              <p className="form-hint muted">{t("skills.pathConfigLead")}</p>
            </div>
            <label className="routing-debug-select">
              <span>{t("skills.sourcePath")}</span>
              <input
                value={skillsSourcePath}
                onChange={(e) => setSkillsSourcePath(e.target.value)}
                placeholder="C:\\Users\\you\\.cc-switch\\skills"
              />
              <p className="form-hint muted">{t("skills.sourcePathHint")}</p>
            </label>
          </section>

          <section className="skills-path-callout">
            <strong>{t("skills.copyWorkflowTitle")}</strong>
            <p className="form-hint muted">{t("skills.copyWorkflowHint")}</p>
            <ol className="skills-path-callout__steps">
              <li>{t("skills.copyWorkflowStep1")}</li>
              <li>{t("skills.copyWorkflowStep2")}</li>
            </ol>
            <div className="skills-path-callout__actions">
              <button type="button" className="btn btn--accent-soft btn--sm" onClick={() => void copyToCcSwitch()} disabled={busy}>
                {t("skills.copyToCcSwitch")}
              </button>
            </div>
          </section>

          {pathMsg ? <p className="form-error">{pathMsg}</p> : null}
          {localSkills ? (
            <section className="skills-path-panel">
              <div className="skills-path-panel__head">
                <strong>{t("skills.sourceList")}</strong>
                <p className="form-hint muted">
                  {t("skills.detectedCount", { count: localSkills.source_skills.length })}
                </p>
              </div>
              <div className="skills-path-status">
                <div className="skills-path-column">
                <p className="skills-path-column__path" title={localSkills.source_path}>
                  {localSkills.source_path || "—"}
                </p>
                <p className="form-hint muted">
                  {localSkills.source_exists ? t("skills.pathExists") : t("skills.pathMissing")}
                </p>
                <div className="skills-path-badges">
                  {renderSkillBadges(localSkills.source_skills, t("skills.missingSkillMd"))}
                </div>
              </div>
              </div>
            </section>
          ) : null}
        </div>
      </Modal>
    </section>
  );
}
