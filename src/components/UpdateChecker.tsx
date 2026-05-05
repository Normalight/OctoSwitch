import { useState, useEffect, useCallback, useRef } from "react";
import { useI18n } from "../i18n";
import { tauriApi } from "../lib/api/tauri";
import { listen } from "@tauri-apps/api/event";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { ConfirmDialog } from "./Dialogs";

const CHECK_CACHE_MS = 60_000;
let lastCheckedAt = 0;
let lastCheckedResult: CheckedState | null = null;
let isDownloadActive = false;

export function UpdateChecker() {
  const { t } = useI18n();
  const [update, setUpdate] = useState<UpdateState>({ status: "idle" });
  const [checking, setChecking] = useState(false);
  const [fallbackDialogOpen, setFallbackDialogOpen] = useState(false);
  const [fallbackUrl, setFallbackUrl] = useState("");
  const checkedRef = useRef<CheckedState | null>(null);
  const downloadingRef = useRef(false);

  const doCheck = useCallback(async (force = false) => {
    if (!force && isDownloadActive) {
      setUpdate({ status: "preparing" });
      return;
    }
    if (!force && lastCheckedResult && Date.now() - lastCheckedAt < CHECK_CACHE_MS) {
      checkedRef.current = lastCheckedResult;
      setUpdate(lastCheckedResult);
      setChecking(false);
      return;
    }

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
      lastCheckedAt = Date.now();
      lastCheckedResult = checked;
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
        lastCheckedAt = Date.now();
      })
    );

    unsubs.push(
      listen("update-download-complete", () => {
        lastCheckedAt = Date.now();
        setUpdate({ status: "installing" });
      })
    );

    unsubs.push(
      listen("update-download-error", (event) => {
        const p = event.payload as Record<string, unknown>;
        lastCheckedAt = 0;
        setUpdate({
          status: "error",
          message: (p.message as string) ?? "Download failed",
        });
      })
    );

    unsubs.push(
      listen("update-installer-launching", () => {
        lastCheckedAt = Date.now();
        setUpdate({ status: "launching" });
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
    isDownloadActive = true;
    // IMMEDIATE feedback: transition to "preparing" before any backend response
    setUpdate({ status: "preparing" });
    // Cache checked state so re-entering About tab doesn't re-check
    if (update.status === "checked") {
      lastCheckedAt = Date.now();
      lastCheckedResult = { ...update };
    }
    try {
      if (update.status === "checked" && update.installerUrl) {
        try {
          await tauriApi.downloadAndInstallUpdate();
        } catch (e) {
          isDownloadActive = false;
          lastCheckedAt = 0;
          // Show fallback dialog: open in browser or close
          setFallbackUrl(update.releaseUrl || "");
          setFallbackDialogOpen(true);
        }
      } else if (update.status === "checked" && update.releaseUrl) {
        try {
          await tauriApi.openExternalUrl(update.releaseUrl);
        } catch {
          // silently fail
        }
        isDownloadActive = false;
      }
    } finally {
      downloadingRef.current = false;
    }
  };

  const handleFallbackConfirm = async () => {
    setFallbackDialogOpen(false);
    if (fallbackUrl) {
      try {
        await tauriApi.openExternalUrl(fallbackUrl);
      } catch {
        // silently fail
      }
    }
  };

  // ── Checking ──
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

  // ── Idle ──
  if (update.status === "idle") {
    return (
      <div className="update-checker">
        <button
          type="button"
          className="btn btn--ghost btn--sm"
          onClick={() => void doCheck(true)}
          disabled={checking}
        >
          <svg viewBox="0 0 24 24" width={14} height={14} fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
            <polyline points="23 4 23 10 17 10" />
            <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10" />
          </svg>
          {t("settings.checkUpdate")}
        </button>
      </div>
    );
  }

  // ── Preparing (immediate feedback on download click) ──
  if (update.status === "preparing") {
    return (
      <div className="update-checker update-checker--center">
        <span className="update-checker__spinner" aria-label={t("settings.preparingDownload")}>
          <svg viewBox="0 0 24 24" width={20} height={20} fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round">
            <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83" />
          </svg>
        </span>
        <span className="muted">{t("settings.preparingDownload")}</span>
      </div>
    );
  }

  // ── Error ──
  if (update.status === "error") {
    return (
      <div className="update-checker update-checker--error">
        <span className="muted">{t("settings.updateCheckError")}: {update.message}</span>
        <button
          type="button"
          className="btn btn--ghost btn--sm"
          onClick={() => void doCheck(true)}
        >
          {t("settings.checkUpdate")}
        </button>
      </div>
    );
  }

  // ── Launching installer ──
  if (update.status === "launching") {
    return (
      <div className="update-checker update-checker--launching">
        <div className="update-checker__phase">
          <span className="update-checker__phase-icon update-checker__phase-icon--done">
            <svg viewBox="0 0 24 24" width={16} height={16} fill="none" stroke="currentColor" strokeWidth={2.5} strokeLinecap="round" strokeLinejoin="round">
              <polyline points="20 6 9 17 4 12" />
            </svg>
          </span>
          <span className="muted">{t("settings.downloadComplete")}</span>
        </div>
        <div className="update-checker__phase">
          <span className="update-checker__spinner" aria-label={t("settings.launchingInstaller")}>
            <svg viewBox="0 0 24 24" width={16} height={16} fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round">
              <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83" />
            </svg>
          </span>
          <span className="muted">{t("settings.launchingInstaller")}</span>
        </div>
        <div className="update-checker__restart-notice">
          {t("settings.restartNotice")}
        </div>
      </div>
    );
  }

  // ── Installing ──
  if (update.status === "installing") {
    return (
      <div className="update-checker update-checker--installing">
        <div className="update-checker__phase">
          <span className="update-checker__phase-icon update-checker__phase-icon--done">
            <svg viewBox="0 0 24 24" width={16} height={16} fill="none" stroke="currentColor" strokeWidth={2.5} strokeLinecap="round" strokeLinejoin="round">
              <polyline points="20 6 9 17 4 12" />
            </svg>
          </span>
          <span className="muted">{t("settings.downloadComplete")}</span>
        </div>
        <div className="update-checker__phase">
          <span className="update-checker__spinner update-checker__spinner--sm" aria-label={t("settings.installingUpdate")}>
            <svg viewBox="0 0 24 24" width={16} height={16} fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round">
              <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83" />
            </svg>
          </span>
          <span className="muted">{t("settings.installingUpdate")}</span>
        </div>
        <div className="update-checker__restart-notice">
          {t("settings.restartNotice")}
        </div>
      </div>
    );
  }

  // ── Checked / Downloading ──
  if (update.status !== "checked" && update.status !== "downloading") {
    return null;
  }

  const base = update.status === "downloading"
    ? checkedRef.current
    : update;

  if (!base) {
    return null;
  }

  const isDownloading = update.status === "downloading";

  return (
    <>
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
            <button
              type="button"
              className="update-checker__refresh-btn"
              title={t("settings.checkUpdate")}
              onClick={() => void doCheck(true)}
              disabled={checking}
            >
              <svg viewBox="0 0 24 24" width={14} height={14} fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
                <polyline points="23 4 23 10 17 10" />
                <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10" />
              </svg>
            </button>
          </div>

          {isDownloading && (
            <div className="update-checker__download-status">
              <div className="update-checker__badge update-checker__badge--new update-checker__badge--pulse">
                {t("settings.downloadingUpdate")}
              </div>
              <div className="update-checker__progress-bar">
                <div
                  className="update-checker__progress-fill"
                  style={{ width: `${update.progress}%` }}
                />
              </div>
              <div className="update-checker__progress-stats">
                <span>{update.progress}%</span>
                <span className="update-checker__progress-size">
                  {formatBytes(update.downloadedBytes)} / {formatBytes(update.totalBytes)}
                </span>
              </div>
            </div>
          )}

          {base.hasUpdate && !isDownloading ? (
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
          ) : base.isIgnored && !isDownloading ? (
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
          ) : !isDownloading ? (
            <div className="update-checker__uptodate">
              <span className="update-checker__badge update-checker__badge--ok">
                {t("settings.updateUpToDate")}
              </span>
            </div>
          ) : null}
        </div>
      </div>
      <ConfirmDialog
        title={t("settings.downloadFailedTitle")}
        message={t("settings.downloadFailedFallback")}
        open={fallbackDialogOpen}
        onClose={() => setFallbackDialogOpen(false)}
        onConfirm={handleFallbackConfirm}
        confirmText={t("settings.openInBrowser")}
        confirmVariant="primary"
      />
    </>
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
  | { status: "preparing" }
  | CheckedState
  | {
      status: "downloading";
      progress: number;
      downloadedBytes: number;
      totalBytes: number;
    }
  | { status: "installing" }
  | { status: "launching" }
  | { status: "error"; message: string };
