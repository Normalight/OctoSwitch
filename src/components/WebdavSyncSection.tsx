import { useCallback, useEffect, useRef, useState } from "react";
import { useI18n } from "../i18n";
import { tauriApi } from "../lib/api/tauri";
import { formatError } from "../lib/formatError";
import { Modal } from "./Modal";
import { CollapsibleSection } from "./CollapsibleSection";

interface Preset {
  id: string;
  baseUrl: string;
  matchPattern?: string;
}

const PRESETS: Preset[] = [
  { id: "jianguoyun", baseUrl: "https://dav.jianguoyun.com/dav/", matchPattern: "jianguoyun.com" },
  { id: "nextcloud", baseUrl: "https://your-server/remote.php/dav/files/USERNAME/", matchPattern: "remote.php/dav" },
  { id: "synology", baseUrl: "http://your-nas-ip:5005/", matchPattern: ":5005" },
  { id: "custom", baseUrl: "" },
];

function detectPreset(url: string): string {
  if (!url) return "custom";
  for (const p of PRESETS) {
    if (p.matchPattern && url.includes(p.matchPattern)) return p.id;
  }
  return "custom";
}

type ActionState = "idle" | "testing" | "saving" | "uploading" | "downloading" | "fetching";
type DialogType = "upload" | "download" | null;

const cloudIcon = (
  <svg viewBox="0 0 24 24" width={18} height={18} fill="none" stroke="currentColor" strokeWidth={1.75} strokeLinecap="round" strokeLinejoin="round">
    <path d="M18 10h-1.26A8 8 0 1 0 9 20h9a5 5 0 0 0 0-10z" />
  </svg>
);

export function WebdavSyncSection() {
  const { t } = useI18n();

  const [form, setForm] = useState({ baseUrl: "", username: "", password: "", remoteRoot: "octoswitch-sync" });
  const [presetId, setPresetId] = useState("custom");
  const [dirty, setDirty] = useState(false);
  const [configured, setConfigured] = useState(false);
  const [action, setAction] = useState<ActionState>("idle");
  const [message, setMessage] = useState<{ type: "ok" | "err"; text: string } | null>(null);
  const [dialogType, setDialogType] = useState<DialogType>(null);
  const savedTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    let cancelled = false;
    tauriApi.webdavGetSettings().then((s) => {
      if (cancelled) return;
      setForm({ baseUrl: s.baseUrl, username: s.username, password: "", remoteRoot: s.remoteRoot });
      setConfigured(s.isConfigured);
      setPresetId(detectPreset(s.baseUrl));
    }).catch(() => {});
    return () => { cancelled = true; };
  }, []);

  useEffect(() => {
    return () => { if (savedTimer.current) clearTimeout(savedTimer.current); };
  }, []);

  const updateField = useCallback((field: keyof typeof form, value: string) => {
    setForm((prev) => ({ ...prev, [field]: value }));
    setDirty(true);
    setMessage(null);
  }, []);

  const handlePresetChange = useCallback((id: string) => {
    setPresetId(id);
    const preset = PRESETS.find((p) => p.id === id);
    if (preset?.baseUrl) {
      setForm((prev) => ({ ...prev, baseUrl: preset.baseUrl }));
      setDirty(true);
      setMessage(null);
    }
  }, []);

  const handleTest = useCallback(async () => {
    setAction("testing");
    setMessage(null);
    try {
      await tauriApi.webdavTestConnection({
        base_url: form.baseUrl, username: form.username,
        password: form.password, remote_root: form.remoteRoot,
      });
      setMessage({ type: "ok", text: t("settings.webdavTestOk") });
    } catch (e) {
      setMessage({ type: "err", text: formatError(e) });
    } finally {
      setAction("idle");
    }
  }, [form, t]);

  const handleSave = useCallback(async () => {
    setAction("saving");
    setMessage(null);
    try {
      await tauriApi.webdavSaveSettings({
        base_url: form.baseUrl, username: form.username,
        password: form.password, remote_root: form.remoteRoot,
      });
      setDirty(false);
      setConfigured(true);
      setMessage({ type: "ok", text: t("settings.webdavSaved") });
      if (savedTimer.current) clearTimeout(savedTimer.current);
      savedTimer.current = setTimeout(() => setMessage(null), 2000);
    } catch (e) {
      setMessage({ type: "err", text: formatError(e) });
      setAction("idle");
    }
  }, [form, t]);

  const handleUploadClick = useCallback(async () => {
    if (dirty) {
      setMessage({ type: "err", text: t("settings.webdavUnsavedChanges") });
      return;
    }
    setDialogType("upload");
  }, [dirty, t]);

  const handleUploadConfirm = useCallback(async () => {
    setDialogType(null);
    setAction("uploading");
    setMessage(null);
    try {
      await tauriApi.webdavUpload();
      setMessage({ type: "ok", text: t("settings.webdavUploadOk") });
    } catch (e) {
      setMessage({ type: "err", text: formatError(e) });
    } finally {
      setAction("idle");
    }
  }, [t]);

  const handleDownloadClick = useCallback(async () => {
    if (dirty) {
      setMessage({ type: "err", text: t("settings.webdavUnsavedChanges") });
      return;
    }
    setDialogType("download");
  }, [dirty, t]);

  const handleDownloadConfirm = useCallback(async () => {
    setDialogType(null);
    setAction("downloading");
    setMessage(null);
    try {
      await tauriApi.webdavDownload();
      setMessage({ type: "ok", text: t("settings.webdavDownloadOk") });
      setTimeout(() => window.location.reload(), 800);
    } catch (e) {
      setMessage({ type: "err", text: formatError(e) });
    } finally {
      setAction("idle");
    }
  }, [t]);

  const busy = action !== "idle";
  const remotePath = `/${form.remoteRoot || "octoswitch-sync"}/octoswitch-config.json`;

  const presetHintKey: Record<string, string> = {
    jianguoyun: "settings.webdavJianguoyunHint",
    nextcloud: "settings.webdavNextcloudHint",
    synology: "settings.webdavSynologyHint",
  };

  const collapsedSummary = configured
    ? <span className="collapsible-summary-tag">{form.username || form.baseUrl}</span>
    : <span className="collapsible-summary-tag collapsible-summary-tag--muted">{t("settings.webdavSaveBeforeSync")}</span>;

  return (
    <>
      <CollapsibleSection
        id="webdav"
        icon={cloudIcon}
        title={t("settings.webdavTitle")}
        defaultOpen={!configured}
        summaryCollapsed={collapsedSummary}
      >
        <p className="form-hint muted" style={{ marginBottom: 10 }}>{t("settings.webdavDesc")}</p>

        <div className="settings-gateway-config-form">
          <div className="settings-config-row settings-config-row--single">
            <label>
              {t("settings.webdavPresetLabel")}
              <select
                value={presetId}
                onChange={(e) => handlePresetChange(e.target.value)}
                disabled={busy}
                className="settings-lang-select"
                style={{ width: "100%", maxWidth: "100%" }}
              >
                {PRESETS.map((p) => (
                  <option key={p.id} value={p.id}>
                    {t(`settings.webdavPreset${p.id.charAt(0).toUpperCase()}${p.id.slice(1)}`)}
                  </option>
                ))}
              </select>
            </label>
          </div>

          <div className="settings-config-row settings-config-row--single">
            <label>
              {t("settings.webdavBaseUrl")}
              <input
                value={form.baseUrl}
                onChange={(e) => updateField("baseUrl", e.target.value)}
                placeholder={t("settings.webdavBaseUrlPlaceholder")}
                disabled={busy}
              />
            </label>
          </div>

          <div className="settings-config-row">
            <label>
              {t("settings.webdavUsername")}
              <input
                value={form.username}
                onChange={(e) => updateField("username", e.target.value)}
                placeholder={t("settings.webdavUsernamePlaceholder")}
                disabled={busy}
              />
            </label>
            <label>
              {t("settings.webdavPassword")}
              <input
                type="password"
                value={form.password}
                onChange={(e) => updateField("password", e.target.value)}
                placeholder={t("settings.webdavPasswordPlaceholder")}
                disabled={busy}
                autoComplete="off"
              />
            </label>
          </div>

          <div className="settings-config-row settings-config-row--single">
            <label>
              {t("settings.webdavRemoteRoot")}
              <input
                value={form.remoteRoot}
                onChange={(e) => updateField("remoteRoot", e.target.value)}
                placeholder="octoswitch-sync"
                disabled={busy}
              />
              <span className="form-hint muted">{t("settings.webdavRemoteRootDefault")}</span>
            </label>
          </div>

          {presetHintKey[presetId] ? (
            <p className="form-hint muted" style={{ maxWidth: "42rem" }}>
              {t(presetHintKey[presetId])}
            </p>
          ) : null}

          {message ? (
            <p className={message.type === "ok" ? "form-hint muted" : "form-error form-error--tight"}>
              {message.text}
            </p>
          ) : null}

          <div className="settings-behavior-item settings-behavior-item--divider">
            <span />
            <div className="collapsible-actions">
              <button type="button" className="btn btn--ghost btn--sm" disabled={busy || !form.baseUrl} onClick={() => void handleTest()}>
                {action === "testing" ? t("settings.webdavTesting") : t("settings.webdavTest")}
              </button>
              <button type="button" className="btn btn--primary btn--sm" disabled={busy || !dirty} onClick={() => void handleSave()}>
                {action === "saving" ? t("settings.webdavSaving") : t("settings.webdavSave")}
              </button>
              {dirty && action === "idle" && <span className="collapsible-dirty-dot" />}
            </div>
          </div>

          <div className="settings-behavior-item">
            <span className="settings-behavior-label muted">{t("settings.webdavRemoteRoot")}</span>
            <div className="collapsible-actions">
              <button type="button" className="btn btn--ghost btn--sm" disabled={busy || !configured} onClick={() => void handleUploadClick()}>
                {action === "uploading" ? t("settings.webdavUploading") : t("settings.webdavUpload")}
              </button>
              <button type="button" className="btn btn--ghost btn--sm" disabled={busy || !configured} onClick={() => void handleDownloadClick()}>
                {action === "downloading" ? t("settings.webdavDownloading") : t("settings.webdavDownload")}
              </button>
            </div>
          </div>

          {!configured && (
            <p className="form-hint muted">{t("settings.webdavSaveBeforeSync")}</p>
          )}
        </div>
      </CollapsibleSection>

      <Modal
        title={t("settings.webdavConfirmUploadTitle")}
        open={dialogType === "upload"}
        onClose={() => setDialogType(null)}
        footer={
          <div className="panel-actions flat">
            <button type="button" className="btn btn--primary" disabled={busy} onClick={() => void handleUploadConfirm()}>
              {t("settings.webdavUpload")}
            </button>
            <button type="button" className="btn btn--ghost" onClick={() => setDialogType(null)}>
              {t("common.cancel")}
            </button>
          </div>
        }
      >
        <p className="muted">{t("settings.webdavConfirmUploadBody")}</p>
        <p className="form-hint muted" style={{ marginTop: 8 }}>
          {t("settings.webdavConfirmUploadTarget")}
          <code style={{ marginLeft: 4 }}>{remotePath}</code>
        </p>
      </Modal>

      <Modal
        title={t("settings.webdavConfirmDownloadTitle")}
        open={dialogType === "download"}
        onClose={() => setDialogType(null)}
        footer={
          <div className="panel-actions flat">
            <button type="button" className="btn btn--primary" disabled={busy} onClick={() => void handleDownloadConfirm()}>
              {t("settings.webdavDownload")}
            </button>
            <button type="button" className="btn btn--ghost" onClick={() => setDialogType(null)}>
              {t("common.cancel")}
            </button>
          </div>
        }
      >
        <p className="muted">{t("settings.webdavConfirmDownloadBody")}</p>
      </Modal>
    </>
  );
}
