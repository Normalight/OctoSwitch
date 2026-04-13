import { useState, useEffect, useCallback, useRef } from "react";
import { useI18n } from "../i18n";
import { tauriApi } from "../lib/api/tauri";
import { listen } from "@tauri-apps/api/event";
import type { UnlistenFn } from "@tauri-apps/api/event";

export function UpdateChecker() {
  const { t } = useI18n();
  const [update, setUpdate] = useState<UpdateState>({ status: "idle" });
  const [checking, setChecking] = useState(false);
  const checkedRef = useRef<CheckedState | null>(null);
  const downloadingRef = useRef(false);

  const doCheck = useCallback(async () => {
    setChecking(true);
    try {
      const result = await tauriApi.checkForUpdate();
      const checked: CheckedState = {
        status: "checked",
        currentVersion: result.current_version,
        latestVersion: result.latest_version,
        hasUpdate: result.has_update,
        releaseNotes: result.release_notes,
        releaseUrl: result.release_url,
        isIgnored: result.is_ignored,
        installerUrl: (result as Record<string, unknown>).installer_url as string | null ?? null,
      };
      checkedRef.current = checked;
      setUpdate(checked);
    } catch (e) {
      setUpdate({ status: "error", message: String(e) });
    } finally {
      setChecking(false);
    }
  }, []);

  useEffect(() => {
    void doCheck();
  }, [doCheck]);

  // Listen for download progress events from the backend
  useEffect(() => {
    const unsubs: Promise<UnlistenFn>[] = [];

    unsubs.push(
      listen("update-download-progress", (event) => {
        const p = event.payload as Record<string, unknown>;
        setUpdate({
          status: "downloading",
          progress: (p.progress as number) ?? 0,
          downloadedBytes: (p.downloaded_bytes as number) ?? 0,
          totalBytes: (p.total_bytes as number) ?? 0,
        });
      })
    );

    unsubs.push(
      listen("update-download-complete", () => {
        setUpdate({ status: "installing" });
      })
    );

    unsubs.push(
      listen("update-download-error", (event) => {
        const p = event.payload as Record<string, unknown>;
        setUpdate({
          status: "error",
          message: (p.message as string) ?? "Download failed",
        });
      })
    );

    return () => {
      unsubs.forEach((u) => void u.then((fn) => fn()));
    };
  }, []);

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

  const handleDownload = async () => {
    if (downloadingRef.current) return;
    downloadingRef.current = true;
    try {
      if (update.status === "checked" && update.installerUrl) {
        try {
          await tauriApi.downloadAndInstallUpdate();
        } catch (e) {
          setUpdate({ status: "error", message: String(e) });
        }
      } else if (update.status === "checked" && update.releaseUrl) {
        // Fallback: no installer asset, open release page in browser via Tauri opener
        try {
          await tauriApi.openExternalUrl(update.releaseUrl);
        } catch {
          // silently fail if opener is unavailable
        }
      }
    } finally {
      downloadingRef.current = false;
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

  // Installing state
  if (update.status === "installing") {
    return (
      <div className="update-checker update-checker--center">
        <span className="update-checker__spinner" aria-label={t("settings.installingUpdate")}>
          <svg viewBox="0 0 24 24" width={20} height={20} fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round">
            <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83" />
          </svg>
        </span>
        <span className="muted">{t("settings.installingUpdate")}</span>
      </div>
    );
  }

  if (update.status !== "checked" && update.status !== "downloading") {
    return null;
  }

  // Common data for checked/downloading states
  const base = update.status === "downloading"
    ? checkedRef.current
    : update;

  if (!base) {
    return null;
  }

  return (
    <div className="update-checker">
      <div className="update-checker__result">
        <div className="update-checker__versions">
          <span className="update-checker__version">
            {t("settings.currentVersion")}: <strong>{base.currentVersion}</strong>
          </span>
          <span className="update-checker__divider" aria-hidden>·</span>
          <span className="update-checker__version">
            {t("settings.latestVersion")}: <strong>{base.latestVersion}</strong>
          </span>
        </div>

        {/* Downloading progress */}
        {update.status === "downloading" && (
          <>
            <div className="update-checker__badge update-checker__badge--new">
              {t("settings.downloadingUpdate")}
            </div>
            <div className="update-checker__progress-bar">
              <div
                className="update-checker__progress-fill"
                style={{ width: `${update.progress}%` }}
              />
            </div>
            <div className="muted" style={{ fontSize: "0.78rem" }}>
              {update.progress}% ({formatBytes(update.downloadedBytes)} / {formatBytes(update.totalBytes)})
            </div>
          </>
        )}

        {base.hasUpdate && update.status !== "downloading" ? (
          <>
            <div className="update-checker__badge update-checker__badge--new">
              {t("settings.updateAvailable")}
            </div>
            {base.releaseNotes && (
              <div className="update-checker__notes">
                <strong>{t("settings.releaseNotes")}</strong>
                <pre className="update-checker__notes-text">{base.releaseNotes}</pre>
              </div>
            )}
            <div className="update-checker__actions">
              <button
                type="button"
                className="btn btn--primary btn--sm"
                onClick={handleDownload}
              >
                {t("settings.downloadUpdateInline")}
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
        ) : base.isIgnored && update.status !== "downloading" ? (
          <div className="update-checker__ignored">
            <span className="muted">
              {t("settings.updateUpToDate")}
              {" — "}
              <em>({t("settings.latestVersion")}: {base.latestVersion} {t("common.dash")} {t("settings.ignoreVersion")})</em>
            </span>
            <button
              type="button"
              className="btn btn--ghost btn--sm btn--xs"
              onClick={handleUnignore}
            >
              {t("settings.clearIgnoredVersion")}
            </button>
          </div>
        ) : update.status !== "downloading" ? (
          <div className="update-checker__uptodate">
            <span className="update-checker__badge update-checker__badge--ok">
              {t("settings.updateUpToDate")}
            </span>
          </div>
        ) : null}
      </div>
    </div>
  );
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
}

type CheckedState = {
  status: "checked";
  currentVersion: string;
  latestVersion: string;
  hasUpdate: boolean;
  releaseNotes: string;
  releaseUrl: string;
  isIgnored: boolean;
  installerUrl: string | null;
};

type UpdateState =
  | { status: "idle" }
  | { status: "checking" }
  | CheckedState
  | {
      status: "downloading";
      progress: number;
      downloadedBytes: number;
      totalBytes: number;
    }
  | { status: "installing" }
  | { status: "error"; message: string };
