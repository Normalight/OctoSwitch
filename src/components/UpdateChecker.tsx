import { useState, useEffect, useCallback } from "react";
import { useI18n } from "../i18n";
import { tauriApi } from "../lib/api/tauri";

export function UpdateChecker() {
  const { t } = useI18n();
  const [update, setUpdate] = useState<UpdateState>({ status: "idle" });
  const [checking, setChecking] = useState(false);

  const doCheck = useCallback(async () => {
    setChecking(true);
    try {
      const result = await tauriApi.checkForUpdate();
      setUpdate({
        status: "checked",
        currentVersion: result.current_version,
        latestVersion: result.latest_version,
        hasUpdate: result.has_update,
        releaseNotes: result.release_notes,
        releaseUrl: result.release_url,
        isIgnored: result.is_ignored,
      });
    } catch (e) {
      setUpdate({ status: "error", message: String(e) });
    } finally {
      setChecking(false);
    }
  }, []);

  useEffect(() => {
    void doCheck();
  }, [doCheck]);

  const handleIgnore = async () => {
    if (update.status !== "checked") return;
    try {
      await tauriApi.ignoreUpdateVersion(update.latestVersion);
      setUpdate({ ...update, hasUpdate: false, isIgnored: true });
    } catch {
      // silently fail
    }
  };

  const handleUnignore = async () => {
    try {
      await tauriApi.clearIgnoredUpdateVersion();
      if (update.status === "checked") {
        setUpdate({ ...update, hasUpdate: true, isIgnored: false });
      }
    } catch {
      // silently fail
    }
  };

  const handleDownload = () => {
    if (update.status === "checked" && update.releaseUrl) {
      window.open(update.releaseUrl, "_blank", "noopener,noreferrer");
    }
  };

  if (checking) {
    return (
      <div className="update-checker update-checker--center">
        <span className="update-checker__spinner" aria-label={t("settings.checkingUpdate")}>
          <svg viewBox="0 0 24 24" width={20} height={20} fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round">
            <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83" />
          </svg>
        </span>
        <span className="muted">{t("settings.checkingUpdate")}</span>
      </div>
    );
  }

  if (update.status === "idle") {
    return (
      <div className="update-checker">
        <button
          type="button"
          className="btn btn--ghost btn--sm"
          onClick={() => void doCheck()}
        >
          {t("settings.checkUpdate")}
        </button>
      </div>
    );
  }

  if (update.status === "error") {
    return (
      <div className="update-checker update-checker--error">
        <span className="muted">{t("settings.updateCheckError")}: {update.message}</span>
        <button
          type="button"
          className="btn btn--ghost btn--sm"
          onClick={() => void doCheck()}
        >
          {t("settings.checkUpdate")}
        </button>
      </div>
    );
  }

  // checked
  return (
    <div className="update-checker">
      <div className="update-checker__result">
        <div className="update-checker__versions">
          <span className="update-checker__version">
            {t("settings.currentVersion")}: <strong>{update.currentVersion}</strong>
          </span>
          <span className="update-checker__divider" aria-hidden>·</span>
          <span className="update-checker__version">
            {t("settings.latestVersion")}: <strong>{update.latestVersion}</strong>
          </span>
        </div>

        {update.hasUpdate ? (
          <>
            <div className="update-checker__badge update-checker__badge--new">
              {t("settings.updateAvailable")}
            </div>
            {update.releaseNotes && (
              <div className="update-checker__notes">
                <strong>{t("settings.releaseNotes")}</strong>
                <pre className="update-checker__notes-text">{update.releaseNotes}</pre>
              </div>
            )}
            <div className="update-checker__actions">
              <button
                type="button"
                className="btn btn--primary btn--sm"
                onClick={handleDownload}
              >
                {t("settings.downloadUpdate")}
              </button>
              <button
                type="button"
                className="btn btn--ghost btn--sm"
                onClick={handleIgnore}
              >
                {t("settings.ignoreVersion")}
              </button>
            </div>
          </>
        ) : update.isIgnored ? (
          <div className="update-checker__ignored">
            <span className="muted">
              {t("settings.updateUpToDate")}
              {" — "}
              <em>({t("settings.latestVersion")}: {update.latestVersion} {t("common.dash")} {t("settings.ignoreVersion")})</em>
            </span>
            <button
              type="button"
              className="btn btn--ghost btn--sm btn--xs"
              onClick={handleUnignore}
            >
              {t("settings.clearIgnoredVersion")}
            </button>
          </div>
        ) : (
          <div className="update-checker__uptodate">
            <span className="update-checker__badge update-checker__badge--ok">
              {t("settings.updateUpToDate")}
            </span>
          </div>
        )}
      </div>
    </div>
  );
}

type UpdateState =
  | { status: "idle" }
  | { status: "checking" }
  | {
      status: "checked";
      currentVersion: string;
      latestVersion: string;
      hasUpdate: boolean;
      releaseNotes: string;
      releaseUrl: string;
      isIgnored: boolean;
    }
  | { status: "error"; message: string };
