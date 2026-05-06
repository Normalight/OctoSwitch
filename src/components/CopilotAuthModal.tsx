import { useEffect, useRef, useState } from "react";
import { ConfirmDialog, ErrorDialog } from "./Dialogs";
import { Modal } from "./Modal";
import { useI18n } from "../i18n";
import { tauriApi } from "../lib/api/tauri";
import { formatError } from "../lib/formatError";
import type { CopilotAccountStatus, CopilotStatus, DeviceCodeResponse } from "../types";

type Props = {
  open: boolean;
  onClose: () => void;
  onStatusChange: () => void;
  existingCopilotAccounts: CopilotAccountStatus[];
  pendingDeviceCode: DeviceCodeResponse | null;
  onPendingDeviceCodeChange: (dc: DeviceCodeResponse | null) => void;
};

export function CopilotAuthModal({
  open,
  onClose,
  onStatusChange,
  existingCopilotAccounts,
  pendingDeviceCode,
  onPendingDeviceCodeChange,
}: Props) {
  const { t } = useI18n();
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<CopilotStatus | null>(null);
  const [polling, setPolling] = useState(false);
  const [copyHint, setCopyHint] = useState<string | null>(null);
  const [errorOpen, setErrorOpen] = useState(false);
  const [errorMsg, setErrorMsg] = useState({ title: "", message: "" });
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [revokingAccountId, setRevokingAccountId] = useState<number | null>(null);
  const pollTimerRef = useRef<number | null>(null);
  const expiryTimerRef = useRef<number | null>(null);
  const activeDeviceCodeRef = useRef<string | null>(null);
  const cancelledRef = useRef(false);

  // Cancel polling on close, but preserve pendingDeviceCode
  useEffect(() => {
    if (!open) {
      cancelledRef.current = true;
      if (pollTimerRef.current != null) {
        window.clearTimeout(pollTimerRef.current);
        pollTimerRef.current = null;
      }
      setPolling(false);
      setCopyHint(null);
      return;
    }
    cancelledRef.current = false;
    void loadStatus();
    return () => {
      if (pollTimerRef.current != null) window.clearTimeout(pollTimerRef.current);
      if (expiryTimerRef.current != null) window.clearTimeout(expiryTimerRef.current);
    };
  }, [open]);

  const loadStatus = async () => {
    try {
      const s = await tauriApi.getCopilotStatus();
      setStatus(s);
    } catch {
      // ignore
    }
  };

  const setExpiryTimeout = (expiresIn: number) => {
    if (expiryTimerRef.current != null) window.clearTimeout(expiryTimerRef.current);
    expiryTimerRef.current = window.setTimeout(() => {
      onPendingDeviceCodeChange(null);
      setPolling(false);
      activeDeviceCodeRef.current = null;
      localStorage.removeItem("copilot_pending_dc");
    }, expiresIn * 1000);
  };

  const handleStart = async () => {
    setBusy(true);
    try {
      const dc = await tauriApi.startCopilotAuth();
      const providerId = `copilot_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
      onPendingDeviceCodeChange(dc);
      localStorage.setItem("copilot_pending_dc", JSON.stringify(dc));
      setPolling(true);
      setExpiryTimeout(dc.expires_in);
      activeDeviceCodeRef.current = dc.device_code;

      try {
        await tauriApi.openExternalUrl(dc.verification_uri);
      } catch {
        try {
          window.open(dc.verification_uri, "_blank", "noopener,noreferrer");
        } catch {
          // 用户仍可手动点击下方链接打开
        }
      }

      const pollOnce = async () => {
        try {
          const result = await tauriApi.completeCopilotAuth(dc.device_code, providerId);
          if (result.pending) {
            if (cancelledRef.current) return;
            pollTimerRef.current = window.setTimeout(pollOnce, Math.max(1, dc.interval) * 1000);
            return;
          }

          setStatus(result);
          if (result.authenticated) {
            onPendingDeviceCodeChange(null);
            localStorage.removeItem("copilot_pending_dc");
            setPolling(false);
            activeDeviceCodeRef.current = null;
            if (expiryTimerRef.current != null) {
              window.clearTimeout(expiryTimerRef.current);
              expiryTimerRef.current = null;
            }
            onStatusChange();
          }
          setPolling(false);
        } catch (e) {
          setPolling(false);
          if (cancelledRef.current) return;
          if (!activeDeviceCodeRef.current) return;
          onPendingDeviceCodeChange(null);
          activeDeviceCodeRef.current = null;
          localStorage.removeItem("copilot_pending_dc");
          setErrorMsg({ title: t("copilot.authFailed"), message: formatError(e) });
          setErrorOpen(true);
        }
      };

      pollTimerRef.current = window.setTimeout(pollOnce, Math.max(1, dc.interval) * 1000);
    } catch (e) {
      setErrorMsg({ title: t("copilot.authFailed"), message: formatError(e) });
      setErrorOpen(true);
      setPolling(false);
    } finally {
      setBusy(false);
    }
  };

  const handleCopyDeviceCode = async (e: React.MouseEvent) => {
    e.stopPropagation();
    const dc = pendingDeviceCode;
    if (!dc?.user_code) return;
    try {
      await navigator.clipboard.writeText(dc.user_code);
      setCopyHint(t("copilot.copyOk"));
      setTimeout(() => setCopyHint(null), 1500);
    } catch {
      setCopyHint(t("copilot.copyFail"));
      setTimeout(() => setCopyHint(null), 1500);
    }
  };

  const handleRevoke = async (accountId: number) => {
    setRevokingAccountId(accountId);
    setConfirmOpen(true);
  };

  const confirmRevoke = async () => {
    const accountId = revokingAccountId;
    setConfirmOpen(false);
    if (!accountId) return;
    setBusy(true);
    try {
      await tauriApi.removeCopilotAccount(accountId);
      onStatusChange();
      setStatus(null);
    } catch (e) {
      setErrorMsg({ title: t("copilot.revokeFailed"), message: formatError(e) });
      setErrorOpen(true);
    } finally {
      setBusy(false);
      setRevokingAccountId(null);
    }
  };

  const isAuthenticated = status?.authenticated ?? false;
  const hasExistingAccounts = existingCopilotAccounts.length > 0;

  return (
    <>
    <Modal
      title={t("copilot.title")}
      open={open}
      onClose={onClose}
      footer={
        hasExistingAccounts ? null : (
          <div className="panel-actions flat">
            {isAuthenticated ? (
              <button
                type="button"
                className="btn btn--danger"
                disabled={busy}
                onClick={() => void handleRevoke(0)}
              >
                {t("copilot.revoke")}
              </button>
            ) : polling ? (
              <button
                type="button"
                className="btn"
                disabled={true}
              >
                {t("copilot.polling")}
              </button>
            ) : (
              <button
                type="button"
                className="btn btn--primary"
                disabled={busy}
                onClick={() => void handleStart()}
              >
                {t("copilot.start")}
              </button>
            )}
          </div>
        )
      }
    >
      <div className="form-stack">
        {/* Existing Copilot accounts - always shown if any exist */}
        {hasExistingAccounts && (
          <div className="copilot-existing-accounts">
            <p className="muted copilot-existing-accounts-lead">
              {t("copilot.existingAccounts")}
            </p>
            {existingCopilotAccounts.map((acc) => (
              <div key={acc.id} className="copilot-account-row">
                <div className="copilot-account-info">
                  <span className={`copilot-status-dot ${acc.authenticated ? "active" : "inactive"}`} />
                  <span className="copilot-account-name">{acc.github_login}</span>
                  <span className="copilot-account-type muted">({acc.account_type})</span>
                </div>
                <button
                  type="button"
                  className="btn btn--sm btn--outline"
                  disabled={busy}
                  onClick={() => void handleRevoke(acc.id)}
                >
                  {t("copilot.revoke")}
                </button>
              </div>
            ))}
          </div>
        )}

        {/* Auth status or pending device code */}
        {isAuthenticated && !hasExistingAccounts ? (
          <>
            <p>{t("copilot.authenticated")}</p>
            {status?.account_type && (
              <p className="muted">
                {t("copilot.accountType")}: {status.account_type}
              </p>
            )}
            {status?.account_login && (
              <p className="muted">
                {t("copilot.accountLogin")}: {status.account_login}
              </p>
            )}
            {status?.token_expires_at && (
              <p className="muted">
                {t("copilot.expiresAt")}: {new Date(Number(status.token_expires_at) * 1000).toLocaleString()}
              </p>
            )}
          </>
        ) : pendingDeviceCode ? (
          <>
            <p>{t("copilot.openBrowser")}</p>
            <div className="copilot-device-code">
              <code>{pendingDeviceCode.user_code}</code>
              <button
                type="button"
                className="copilot-copy-code-btn"
                title={t("copilot.copyCode")}
                aria-label={t("copilot.copyCode")}
                onClick={handleCopyDeviceCode}
              >
                <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
                  <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
                </svg>
              </button>
            </div>
            {copyHint ? <p className="muted copilot-copy-hint">{copyHint}</p> : null}
            <p className="muted">
              {t("copilot.verificationUrl")}:{" "}
              <span className="copilot-verify-link-row">
                <a href={pendingDeviceCode.verification_uri} target="_blank" rel="noreferrer">
                  {pendingDeviceCode.verification_uri}
                </a>
              </span>
            </p>
            {polling && <p className="muted">{t("copilot.waiting")}</p>}
          </>
        ) : !hasExistingAccounts ? (
          <p>{t("copilot.description")}</p>
        ) : null}
      </div>
    </Modal>

    <ConfirmDialog
      title={t("copilot.revokeConfirm")}
      message={t("copilot.revokeConfirm")}
      open={confirmOpen}
      onClose={() => setConfirmOpen(false)}
      onConfirm={() => void confirmRevoke()}
      confirmText={t("copilot.revoke")}
      confirmVariant="danger"
    />

    <ErrorDialog
      title={errorMsg.title}
      message={errorMsg.message}
      open={errorOpen}
      onClose={() => setErrorOpen(false)}
    />
    </>
  );
}
