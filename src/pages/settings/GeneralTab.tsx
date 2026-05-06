import { useCallback, useEffect, useImperativeHandle, useState, forwardRef } from "react";
import { LOCALES, useI18n, type Locale } from "../../i18n";
import { useTheme, type ThemePreference } from "../../theme/ThemeContext";
import type { LogLevel, GatewayHealthStatus } from "../../types/gateway_config";
import { formatError } from "../../lib/formatError";
import { LOG_LEVELS } from "../../types/gateway_config";
import { tauriApi } from "../../lib/api/tauri";
import { SaveIndicator } from "../../components/SaveIndicator";

const themeIds: ThemePreference[] = ["dark", "light", "system"];

/** Module-level state: survives SettingsPage unmount/remount so unsaved log-level changes persist across settings re-entries. */
let savedLogLevel: LogLevel | null = null;
let pendingLogLevel: LogLevel | null = null;

function ToggleSwitch({ checked, disabled, onChange, id, ariaLabel, ariaLabelledBy }: {
  checked: boolean;
  disabled?: boolean;
  onChange: (checked: boolean) => void;
  id: string;
  /** 无可见标题时供读屏使用 */
  ariaLabel?: string;
  /** 与可见标题 `id` 对应时优先于 ariaLabel */
  ariaLabelledBy?: string;
}) {
  return (
    <label className="toggle-switch" htmlFor={id}>
      <input
        id={id}
        type="checkbox"
        checked={checked}
        disabled={disabled}
        aria-label={ariaLabelledBy ? undefined : ariaLabel}
        aria-labelledby={ariaLabelledBy}
        onChange={(e) => onChange(e.target.checked)}
      />
      <span className="toggle-switch-track"><span className="toggle-switch-thumb" /></span>
    </label>
  );
}

export const GeneralTab = forwardRef<{ resetLogLevel: () => void }, {}>((_props, ref) => {
  const { t, locale, setLocale } = useI18n();
  const { preference, setPreference } = useTheme();
  const [gwHost, setGwHost] = useState("127.0.0.1");
  const [gwPort, setGwPort] = useState("8787");
  const [closeToTray, setCloseToTray] = useState(false);
  const [autoStart, setAutoStart] = useState(false);
  const [silentAutoStart, setSilentAutoStart] = useState(false);
  const [debugMode, setDebugMode] = useState(false);
  const [autoUpdateCheck, setAutoUpdateCheck] = useState(true);
  const [skillsEnabled, setSkillsEnabled] = useState(false);
  const [logLevel, setLogLevel] = useState<LogLevel>("info");
  const [allowGroupMemberModelPath, setAllowGroupMemberModelPath] = useState(true);
  const [gwSaving, setGwSaving] = useState(false);
  const [behaviorSaving, setBehaviorSaving] = useState(false);
  const [gwMsg, setGwMsg] = useState<{ text: string; type: "ok" | "err" } | null>(null);
  const [gwShowSaved, setGwShowSaved] = useState(false);
  const [logLevelSaving, setLogLevelSaving] = useState(false);
  const [logLevelShowSaved, setLogLevelShowSaved] = useState(false);

  // Auto-reset saved indicators after 1.5s
  useEffect(() => {
    if (!gwShowSaved) return;
    const t = setTimeout(() => setGwShowSaved(false), 1500);
    return () => clearTimeout(t);
  }, [gwShowSaved]);
  useEffect(() => {
    if (!logLevelShowSaved) return;
    const t = setTimeout(() => setLogLevelShowSaved(false), 1500);
    return () => clearTimeout(t);
  }, [logLevelShowSaved]);
  const [configLoaded, setConfigLoaded] = useState(false);
  const [gwHealth, setGwHealth] = useState<GatewayHealthStatus | null>(null);
  const [healthChecking, setHealthChecking] = useState(false);
  const [healthError, setHealthError] = useState<string | null>(null);
  const [gwRestarting, setGwRestarting] = useState(false);

  const loadGatewayConfig = useCallback(async () => {
    try {
      const cfg = await tauriApi.getGatewayConfig();
      setGwHost(cfg.host);
      setGwPort(String(cfg.port));
      setCloseToTray(cfg.close_to_tray);
      setAutoStart(cfg.auto_start);
      setSilentAutoStart(cfg.silent_autostart ?? false);
      setDebugMode(cfg.debug_mode ?? false);
      setAutoUpdateCheck(cfg.auto_update_check ?? true);
      setSkillsEnabled(cfg.skills_enabled ?? false);
      setAllowGroupMemberModelPath(cfg.allow_group_member_model_path ?? true);
      const persistedLevel = cfg.log_level || "info" as LogLevel;
      savedLogLevel = persistedLevel;
      const initialLevel = pendingLogLevel ?? persistedLevel;
      setLogLevel(initialLevel);
    } catch {
      savedLogLevel = "info";
      setLogLevel(pendingLogLevel ?? "info");
    } finally {
      setConfigLoaded(true);
    }
  }, []);

  useEffect(() => {
    void loadGatewayConfig();
  }, [loadGatewayConfig]);

  const checkGwHealth = useCallback(async () => {
    setHealthChecking(true);
    setHealthError(null);
    try {
      const status = await tauriApi.checkGatewayHealth();
      setGwHealth(status);
    } catch (e) {
      setGwHealth(null);
      setHealthError(formatError(e));
    } finally {
      setHealthChecking(false);
    }
  }, []);

  useEffect(() => {
    void checkGwHealth();
  }, [checkGwHealth]);

  const saveGatewayConfig = async () => {
    const port = parseInt(gwPort, 10);
    if (!gwHost.trim() || !Number.isFinite(port) || port < 1 || port > 65535) {
      setGwMsg({ text: t("common.validationFailed"), type: "err" });
      return;
    }
    setGwSaving(true);
    setGwMsg(null);
    try {
      const current = await tauriApi.getGatewayConfig();
      const willRestart =
        current.host.trim() !== gwHost.trim() || Number(current.port) !== port;
      await tauriApi.updateGatewayConfig({
        ...current,
        host: gwHost.trim(),
        port,
        close_to_tray: closeToTray,
        auto_start: autoStart,
        silent_autostart: silentAutoStart,
        debug_mode: debugMode,
        skills_enabled: skillsEnabled,
        allow_group_member_model_path: allowGroupMemberModelPath,
        log_level: logLevel,
      });
      if (willRestart) {
        setGwMsg({ text: t("settings.gatewayRestarted"), type: "ok" });
        setGwHealth(null);
        setTimeout(() => {
          void checkGwHealth();
        }, 3000);
      } else {
        setGwShowSaved(true);
      }
    } catch (e) {
      setGwMsg({ text: t("settings.gatewaySaveFailed"), type: "err" });
    } finally {
      setGwSaving(false);
    }
  };

  const saveCloseToTray = async (checked: boolean) => {
    setBehaviorSaving(true);
    try {
      const current = await tauriApi.getGatewayConfig();
      await tauriApi.updateGatewayConfig({ ...current, close_to_tray: checked });
      setCloseToTray(checked);
    } catch {
      void loadGatewayConfig();
    } finally {
      setBehaviorSaving(false);
    }
  };

  const saveAutoStart = async (checked: boolean) => {
    setBehaviorSaving(true);
    try {
      const current = await tauriApi.getGatewayConfig();
      const nextSilent = checked ? (current.silent_autostart ?? false) : false;
      await tauriApi.updateGatewayConfig({ ...current, auto_start: checked, silent_autostart: nextSilent });
      setAutoStart(checked);
      if (!checked) setSilentAutoStart(false);
      else setSilentAutoStart(nextSilent);
    } catch {
      void loadGatewayConfig();
    } finally {
      setBehaviorSaving(false);
    }
  };

  const saveAllowGroupMemberModelPath = async (checked: boolean) => {
    setBehaviorSaving(true);
    setGwMsg(null);
    try {
      const current = await tauriApi.getGatewayConfig();
      await tauriApi.updateGatewayConfig({ ...current, allow_group_member_model_path: checked });
      setAllowGroupMemberModelPath(checked);
      setGwShowSaved(true);
    } catch {
      setGwMsg({ text: t("settings.gatewaySaveFailed"), type: "err" });
      void loadGatewayConfig();
    } finally {
      setBehaviorSaving(false);
    }
  };

  const saveSilentAutoStart = async (checked: boolean) => {
    setBehaviorSaving(true);
    try {
      const current = await tauriApi.getGatewayConfig();
      await tauriApi.updateGatewayConfig({ ...current, silent_autostart: checked });
      setSilentAutoStart(checked);
    } catch {
      void loadGatewayConfig();
    } finally {
      setBehaviorSaving(false);
    }
  };

  const saveDebugMode = async (checked: boolean) => {
    setBehaviorSaving(true);
    try {
      const current = await tauriApi.getGatewayConfig();
      await tauriApi.updateGatewayConfig({ ...current, debug_mode: checked });
      setDebugMode(checked);
    } catch {
      void loadGatewayConfig();
    } finally {
      setBehaviorSaving(false);
    }
  };

  const saveAutoUpdateCheck = async (checked: boolean) => {
    setBehaviorSaving(true);
    try {
      const current = await tauriApi.getGatewayConfig();
      await tauriApi.updateGatewayConfig({ ...current, auto_update_check: checked });
      setAutoUpdateCheck(checked);
    } catch {
      void loadGatewayConfig();
    } finally {
      setBehaviorSaving(false);
    }
  };

  const saveSkillsEnabled = async (checked: boolean) => {
    setBehaviorSaving(true);
    try {
      const current = await tauriApi.getGatewayConfig();
      await tauriApi.updateGatewayConfig({ ...current, skills_enabled: checked });
      setSkillsEnabled(checked);
    } catch {
      void loadGatewayConfig();
    } finally {
      setBehaviorSaving(false);
    }
  };

  const handleLogLevelChange = (level: LogLevel) => {
    setLogLevel(level);
    pendingLogLevel = level;
  };

  const saveLogLevel = async () => {
    setLogLevelSaving(true);
    try {
      const current = await tauriApi.getGatewayConfig();
      await tauriApi.updateGatewayConfig({ ...current, log_level: logLevel });
      savedLogLevel = logLevel;
      pendingLogLevel = null;
      setLogLevelShowSaved(true);
    } catch {
      setLogLevel(savedLogLevel ?? "info");
      pendingLogLevel = null;
    } finally {
      setLogLevelSaving(false);
    }
  };

  /** Expose reset for when leaving settings */
  const resetLogLevel = () => {
    if (pendingLogLevel !== null) {
      setLogLevel(savedLogLevel ?? "info");
      pendingLogLevel = null;
    }
  };

  const restartGateway = async () => {
    setGwRestarting(true);
    setGwHealth(null);
    try {
      await tauriApi.restartGateway();
      // Give the gateway time to bind, then re-check with retries
      let ok = false;
      for (let i = 0; i < 5; i++) {
        await new Promise(r => setTimeout(r, 600));
        try {
          const status = await tauriApi.checkGatewayHealth();
          if (status.is_running) {
            setGwHealth(status);
            ok = true;
            break;
          }
        } catch { /* retry */ }
      }
      if (!ok) {
        setGwHealth(null);
        setHealthError(t("settings.gatewayHealthNotRunning"));
      }
      setGwMsg({ text: t("settings.gatewayRestarted"), type: "ok" });
    } catch (e) {
      setGwMsg({ text: t("settings.gatewaySaveFailed"), type: "err" });
    } finally {
      setGwRestarting(false);
    }
  };

  const isLogLevelDirty = configLoaded && pendingLogLevel !== null;

  useImperativeHandle(ref, () => ({ resetLogLevel }));

  const themeTitle: Record<ThemePreference, string> = {
    dark: t("theme.darkTitle"),
    light: t("theme.lightTitle"),
    system: t("theme.systemTitle"),
  };

  const localeLabel: Record<Locale, string> = {
    "zh-CN": t("settings.langZh"),
    en: t("settings.langEn"),
  };

  return (
    <div className="settings-tab-stack">
      {/* Gateway — config + status in one card */}
        <div className="settings-section settings-section--card card card--compact">
          <div className="settings-section-head">
            <span className="settings-section-icon" aria-hidden>
              <svg viewBox="0 0 24 24" width={18} height={18} fill="none" stroke="currentColor" strokeWidth={1.75} strokeLinecap="round" strokeLinejoin="round">
                <circle cx="12" cy="12" r="3" />
                <path d="M12 1v4M12 19v4M4.22 4.22l2.83 2.83M16.95 16.95l2.83 2.83M1 12h4M19 12h4M4.22 19.78l2.83-2.83M16.95 7.05l2.83-2.83" />
              </svg>
            </span>
            <h3 className="settings-section__title">{t("settings.gatewayConfig")}</h3>
            {gwHealth ? (
              <div className={`gateway-status-inline ${gwHealth.is_running ? "gateway-status--ok" : "gateway-status--err"}`}>
                <span className="gateway-status-dot" />
                <span className="gateway-status-text">
                  {gwHealth.is_running ? t("settings.gatewayHealthRunning") : t("settings.gatewayHealthNotRunning")}
                </span>
                {!gwHealth.is_running ? (
                  <button
                    type="button"
                    className="btn btn--primary btn--sm"
                    disabled={gwRestarting}
                    onClick={() => void restartGateway()}
                    style={{ marginLeft: "8px" }}
                  >
                    {gwRestarting ? t("settings.restartingGateway") : t("settings.restartGateway")}
                  </button>
                ) : null}
              </div>
            ) : healthError ? (
              <div className="gateway-status-inline gateway-status--err">
                <span className="gateway-status-dot" />
                <span className="gateway-status-text">{t("settings.gatewayHealthCheckFailed")}</span>
                <button
                  type="button"
                  className="btn btn--primary btn--sm"
                  disabled={gwRestarting}
                  onClick={() => void restartGateway()}
                  style={{ marginLeft: "8px" }}
                >
                  {gwRestarting ? t("settings.restartingGateway") : t("settings.restartGateway")}
                </button>
              </div>
            ) : null}
          </div>
          {healthError ? (
            <p className="form-error form-error--tight">{healthError}</p>
          ) : null}
          <div className="settings-gateway-config-form">
            <div className="settings-config-row">
              <label>
                {t("settings.gatewayHost")}
                <input value={gwHost} onChange={(e) => setGwHost(e.target.value)} placeholder={t("settings.gatewayHostPlaceholder")} disabled={gwSaving} />
              </label>
              <label>
                {t("settings.gatewayPort")}
                <input type="number" min={1} max={65535} value={gwPort} onChange={(e) => setGwPort(e.target.value)} placeholder={t("settings.gatewayPortPlaceholder")} disabled={gwSaving} />
              </label>
            </div>
            <div className="settings-behavior-item settings-behavior-item--divider">
              <div>
                <span className="settings-behavior-label" id="label-allow-group-member-path">{t("settings.allowGroupMemberModelPath")}</span>
                <p className="form-hint muted" style={{ margin: "4px 0 0", maxWidth: "42rem" }}>{t("settings.allowGroupMemberModelPathHint")}</p>
              </div>
              <ToggleSwitch
                id="toggle-allow-group-member-path"
                checked={allowGroupMemberModelPath}
                disabled={behaviorSaving || !configLoaded}
                onChange={(v) => void saveAllowGroupMemberModelPath(v)}
                ariaLabelledBy="label-allow-group-member-path"
              />
            </div>
            {gwMsg && gwMsg.type === "err" ? <p className="form-error">{gwMsg.text}</p> : null}
            {gwMsg && gwMsg.type === "ok" ? <p className="form-hint muted">{gwMsg.text}</p> : null}
            <span className="settings-save-row">
              <button type="button" className="btn btn--primary btn--sm" disabled={gwSaving} onClick={() => void saveGatewayConfig()}>
                {gwShowSaved ? (
                  <span className="save-indicator">
                    <svg viewBox="0 0 24 24" width={18} height={18} fill="none" stroke="currentColor" strokeWidth={2.5} strokeLinecap="round" strokeLinejoin="round">
                      <polyline points="20 6 9 17 4 12" />
                    </svg>
                    {t("common.saved")}
                  </span>
                ) : t("common.save")}
              </button>
            </span>
          </div>
        </div>

        {/* Preferences — language + appearance */}
        <div className="settings-preferences-row">
          <div className="settings-section settings-section--card card card--compact">
            <div className="settings-section-head">
              <span className="settings-section-icon" aria-hidden>
                <svg viewBox="0 0 24 24" width={18} height={18} fill="none" stroke="currentColor" strokeWidth={1.75} strokeLinecap="round" strokeLinejoin="round">
                  <path d="M5 8l6 6M4 14l6-6 2-3M2 5h12M7 2h1" />
                  <path d="M22 22l-5-10-5 10M14 18h6" />
                </svg>
              </span>
              <h3 className="settings-section__title">{t("settings.languageTitle")}</h3>
            </div>
            <select className="settings-lang-select" value={locale} aria-label={t("settings.languageAria")} onChange={(e) => setLocale(e.target.value as Locale)}>
              {LOCALES.map((id) => (<option key={id} value={id}>{localeLabel[id]}</option>))}
            </select>
          </div>

          <div className="settings-section settings-section--card card card--compact">
            <div className="settings-section-head">
              <span className="settings-section-icon" aria-hidden>
                <svg viewBox="0 0 24 24" width={18} height={18} fill="none" stroke="currentColor" strokeWidth={1.75} strokeLinecap="round" strokeLinejoin="round">
                  <circle cx="12" cy="12" r="5" />
                  <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42" />
                </svg>
              </span>
              <h3 className="settings-section__title">{t("settings.appearanceTitle")}</h3>
            </div>
            <div className="settings-theme-row" role="radiogroup" aria-label={t("settings.appearanceAria")}>
              {themeIds.map((id) => (
                <button key={id} type="button" role="radio" aria-checked={preference === id} className={`settings-theme-btn ${preference === id ? "is-active" : ""}`} onClick={() => setPreference(id)}>
                  {themeTitle[id]}
                </button>
              ))}
            </div>
          </div>
        </div>

        {/* Behavior */}
        <div className="settings-section settings-section--card card card--compact">
          <div className="settings-section-head">
            <span className="settings-section-icon" aria-hidden>
              <svg viewBox="0 0 24 24" width={18} height={18} fill="none" stroke="currentColor" strokeWidth={1.75} strokeLinecap="round" strokeLinejoin="round">
                <rect x="2" y="3" width="20" height="14" rx="2" /><path d="M8 21h8M12 17v4" />
              </svg>
            </span>
            <h3 className="settings-section__title">{t("settings.behaviorTitle")}</h3>
          </div>
          <div className="settings-behavior-list">
            {configLoaded ? (
              <>
                <div className="settings-behavior-item">
                  <span className="settings-behavior-label">{t("settings.closeToTray")}</span>
                  <ToggleSwitch id="toggle-close-tray" checked={closeToTray} disabled={behaviorSaving} onChange={(v) => void saveCloseToTray(v)} />
                </div>
                <div className="settings-behavior-item">
                  <span className="settings-behavior-label">{t("settings.autoStart")}</span>
                  <ToggleSwitch id="toggle-auto-start" checked={autoStart} disabled={behaviorSaving} onChange={(v) => void saveAutoStart(v)} />
                </div>
                {autoStart ? (
                  <div className="settings-behavior-item">
                    <span className="settings-behavior-label">{t("settings.silentAutostart")}</span>
                    <ToggleSwitch
                      id="toggle-silent-autostart"
                      checked={silentAutoStart}
                      disabled={behaviorSaving}
                      onChange={(v) => void saveSilentAutoStart(v)}
                    />
                  </div>
                ) : null}
                <div className="settings-behavior-item">
                  <span className="settings-behavior-label">{t("settings.debugMode")}</span>
                  <ToggleSwitch
                    id="toggle-debug-mode"
                    checked={debugMode}
                    disabled={behaviorSaving}
                    onChange={(v) => void saveDebugMode(v)}
                  />
                </div>
                <div className="settings-behavior-item">
                  <span className="settings-behavior-label">{t("settings.startupUpdateCheckHint")}</span>
                  <ToggleSwitch
                    id="toggle-auto-update-check"
                    checked={autoUpdateCheck}
                    disabled={behaviorSaving}
                    onChange={(v) => void saveAutoUpdateCheck(v)}
                  />
                </div>
                <div className="settings-behavior-item">
                  <span className="settings-behavior-label">{t("settings.enableSkills")}</span>
                  <ToggleSwitch
                    id="toggle-skills-enabled"
                    checked={skillsEnabled}
                    disabled={behaviorSaving}
                    onChange={(v) => void saveSkillsEnabled(v)}
                  />
                </div>
              </>
            ) : (
              <>
                {[0, 1, 2].map((i) => (
                  <div key={i} className="settings-behavior-item" style={{ opacity: 0.3, pointerEvents: "none" }}>
                    <span className="settings-behavior-label">&nbsp;</span>
                    <span className="toggle-switch" style={{ pointerEvents: "none" }}>
                      <span className="toggle-switch-track" style={{ opacity: 0.3 }}>
                        <span className="toggle-switch-thumb" />
                      </span>
                    </span>
                  </div>
                ))}
              </>
            )}
          </div>
        </div>

        {/* Logging */}
        <div className="settings-section settings-section--card card card--compact">
          <div className="settings-section-head">
            <span className="settings-section-icon" aria-hidden>
              <svg viewBox="0 0 24 24" width={18} height={18} fill="none" stroke="currentColor" strokeWidth={1.75} strokeLinecap="round" strokeLinejoin="round">
                <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" /><polyline points="14,2 14,8 20,8" /><line x1="16" y1="13" x2="8" y2="13" /><line x1="16" y1="17" x2="8" y2="17" /><polyline points="10,9 9,9 8,9" />
              </svg>
            </span>
            <h3 className="settings-section__title">{t("settings.logLevelTitle")}</h3>
          </div>
          <div className="settings-loglevel-row">
            <select className="settings-lang-select settings-loglevel-select" value={logLevel} aria-label={t("settings.logLevelTitle")} onChange={(e) => handleLogLevelChange(e.target.value as LogLevel)}>
              {LOG_LEVELS.map((id) => (<option key={id} value={id}>{t(`settings.logLevel_${id}`)}</option>))}
            </select>
            <button
              type="button"
              className={`btn settings-loglevel-save ${isLogLevelDirty ? "btn--primary" : "btn--ghost"}`}
              disabled={!isLogLevelDirty || logLevelSaving}
              onClick={() => void saveLogLevel()}
            >
              {logLevelSaving ? t("common.loading") : logLevelShowSaved ? (
                <span className="save-indicator">
                  <svg viewBox="0 0 24 24" width={18} height={18} fill="none" stroke="currentColor" strokeWidth={2.5} strokeLinecap="round" strokeLinejoin="round">
                    <polyline points="20 6 9 17 4 12" />
                  </svg>
                  {t("common.saved")}
                </span>
              ) : t("common.save")}
            </button>
          </div>
          <p className="form-hint muted" style={{ marginTop: "4px" }}>{t("settings.logLevelHint")}</p>
        </div>
    </div>
  );
});
GeneralTab.displayName = "GeneralTab";
