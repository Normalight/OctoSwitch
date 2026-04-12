import { useState } from "react";
import { ConfirmDialog } from "../../components/Dialogs";
import { Modal } from "../../components/Modal";
import { ConfigImportExport } from "../../components/ConfigImportExport";
import { CONFIG_IMPORTED } from "../../lib/constants";
import { useI18n } from "../../i18n";
import type { ImportReport } from "../../types";
import { tauriApi } from "../../lib/api/tauri";

export function DataTab() {
  const { t } = useI18n();
  const [clearing, setClearing] = useState(false);
  const [clearConfirmOpen, setClearConfirmOpen] = useState(false);
  const [clearErr, setClearErr] = useState<string | null>(null);
  const [importingCc, setImportingCc] = useState(false);
  const [importCcConfirmOpen, setImportCcConfirmOpen] = useState(false);
  const [importCcErr, setImportCcErr] = useState<string | null>(null);
  const [importReport, setImportReport] = useState<ImportReport | null>(null);
  const [showReportDetail, setShowReportDetail] = useState(false);

  const handleClearData = async () => setClearConfirmOpen(true);

  const executeClearData = async () => {
    setClearConfirmOpen(false);
    setClearing(true);
    try {
      await tauriApi.clearAllData();
      setTimeout(() => window.location.reload(), 800);
    } catch (e) {
      setClearErr(String(e));
    } finally {
      setClearing(false);
    }
  };

  const handleImportCcSwitch = () => setImportCcConfirmOpen(true);

  const executeImportCcSwitch = async () => {
    setImportCcConfirmOpen(false);
    setImportingCc(true);
    setImportCcErr(null);
    setImportReport(null);
    try {
      const report = await tauriApi.importCcSwitchProviders();
      setImportReport(report);
      window.dispatchEvent(new CustomEvent(CONFIG_IMPORTED));
      setTimeout(() => window.dispatchEvent(new CustomEvent(CONFIG_IMPORTED)), 500);
    } catch (e) {
      setImportCcErr(String(e));
    } finally {
      setImportingCc(false);
    }
  };

  return (
    <div className="settings-tab-stack">
      <section className="settings-section settings-section--card card card--compact" aria-labelledby="settings-data-config-heading">
        <div className="settings-section-head">
          <span className="settings-section-icon" aria-hidden>
            <svg viewBox="0 0 24 24" width={18} height={18} fill="none" stroke="currentColor" strokeWidth={1.75} strokeLinecap="round" strokeLinejoin="round">
              <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
              <polyline points="14 2 14 8 20 8" />
              <line x1="12" y1="18" x2="12" y2="12" />
              <line x1="9" y1="15" x2="15" y2="15" />
            </svg>
          </span>
          <h3 id="settings-data-config-heading" className="settings-section__title">
            {t("settings.configFileTitle")}
          </h3>
        </div>
        <p className="form-hint muted" style={{ margin: "0 0 12px" }}>{t("settings.dataBlurb")}</p>
        <div className="settings-section-stack">
          <ConfigImportExport />
        </div>
      </section>

      <section className="settings-section settings-section--card card card--compact" aria-labelledby="settings-data-cc-heading">
        <div className="settings-section-head">
          <span className="settings-section-icon" aria-hidden>
            <svg viewBox="0 0 24 24" width={18} height={18} fill="none" stroke="currentColor" strokeWidth={1.75} strokeLinecap="round" strokeLinejoin="round">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
              <polyline points="7 10 12 15 17 10" />
              <line x1="12" y1="15" x2="12" y2="3" />
            </svg>
          </span>
          <h3 id="settings-data-cc-heading" className="settings-section__title">
            {t("settings.importCcSwitchTitle")}
          </h3>
        </div>
        <p className="form-hint muted" style={{ margin: "0 0 12px" }}>{t("settings.importCcSwitchDesc")}</p>
        <div className="settings-section-actions">
          <button type="button" className="btn btn--primary btn--sm" disabled={importingCc} onClick={() => void handleImportCcSwitch()}>
            {importingCc ? t("common.loading") : t("settings.importCcSwitch")}
          </button>
        </div>
        {importReport ? (
          <div className="settings-import-result">
            <p className="form-hint muted">
              {t("settings.importDetailSummary", {
                imported: importReport.providers_imported,
                skipped: importReport.providers_skipped,
                modelsBound: importReport.models_bound,
                modelsSkipped: importReport.models_skipped,
              })}
            </p>
            <button type="button" className="btn btn--ghost btn--sm" onClick={() => setShowReportDetail(true)}>
              {t("settings.importDetailView")}
            </button>
          </div>
        ) : null}
      </section>

      <section className="settings-section settings-section--card settings-section--note card card--compact" aria-labelledby="settings-data-privacy-heading">
        <div className="settings-section-head">
          <span className="settings-section-icon" aria-hidden>
            <svg viewBox="0 0 24 24" width={18} height={18} fill="none" stroke="currentColor" strokeWidth={1.75} strokeLinecap="round" strokeLinejoin="round">
              <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
            </svg>
          </span>
          <h3 id="settings-data-privacy-heading" className="settings-section__title">
            {t("settings.privacyTitle")}
          </h3>
        </div>
        <ul className="settings-list settings-list--tight">
          <li>{t("settings.privacy1")}</li>
          <li>{t("settings.privacy2")}</li>
          <li>{t("settings.privacy3")}</li>
        </ul>
      </section>

      <section className="settings-section settings-section--card settings-section--danger card card--compact" aria-labelledby="settings-data-clear-heading">
        <div className="settings-section-head">
          <span className="settings-section-icon settings-section-icon--danger" aria-hidden>
            <svg viewBox="0 0 24 24" width={18} height={18} fill="none" stroke="currentColor" strokeWidth={1.75} strokeLinecap="round" strokeLinejoin="round">
              <polyline points="3 6 5 6 21 6" />
              <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
              <line x1="10" y1="11" x2="10" y2="17" />
              <line x1="14" y1="11" x2="14" y2="17" />
            </svg>
          </span>
          <h3 id="settings-data-clear-heading" className="settings-section__title settings-section__title--danger">
            {t("settings.clearData")}
          </h3>
        </div>
        <p className="form-hint muted" style={{ margin: "0 0 10px" }}>
          {t("settings.clearDataConfirm")}
        </p>
        <div className="settings-section-actions">
          <button type="button" className="btn btn--danger btn--sm" disabled={clearing} onClick={() => void handleClearData()}>
            {clearing ? t("common.loading") : t("settings.clearData")}
          </button>
        </div>
      </section>

      <Modal
        title={t("settings.clearData")}
        open={clearConfirmOpen}
        onClose={() => setClearConfirmOpen(false)}
        footer={
          <div className="panel-actions flat">
            <button type="button" className="btn btn--danger" disabled={clearing} onClick={() => void executeClearData()}>
              {t("common.delete")}
            </button>
            <button type="button" className="btn btn--ghost" disabled={clearing} onClick={() => setClearConfirmOpen(false)}>
              {t("common.cancel")}
            </button>
          </div>
        }
      >
        <p className="muted">{t("settings.clearDataConfirm")}</p>
      </Modal>

      <Modal
        title={t("settings.clearDataFailed")}
        open={clearErr !== null}
        onClose={() => setClearErr(null)}
        footer={
          <div className="panel-actions flat">
            <button type="button" className="btn btn--ghost" onClick={() => setClearErr(null)}>
              {t("common.cancel")}
            </button>
          </div>
        }
      >
        <p className="muted">{clearErr ?? ""}</p>
      </Modal>

      <ConfirmDialog
        title={t("settings.importCcSwitchConfirm")}
        message={t("settings.importCcSwitchConfirmDesc")}
        open={importCcConfirmOpen}
        onClose={() => setImportCcConfirmOpen(false)}
        onConfirm={() => void executeImportCcSwitch()}
        confirmVariant="primary"
      />

      <Modal
        title={t("settings.importCcSwitchFailed")}
        open={importCcErr !== null}
        onClose={() => setImportCcErr(null)}
        footer={
          <div className="panel-actions flat">
            <button type="button" className="btn btn--ghost" onClick={() => setImportCcErr(null)}>
              {t("common.cancel")}
            </button>
          </div>
        }
      >
        <p className="muted">{importCcErr ?? ""}</p>
      </Modal>

      <Modal
        title={t("settings.importDetailTitle")}
        open={showReportDetail}
        onClose={() => setShowReportDetail(false)}
        footer={
          <div className="panel-actions flat">
            <button type="button" className="btn btn--ghost" onClick={() => setShowReportDetail(false)}>
              {t("common.cancel")}
            </button>
          </div>
        }
      >
        <div className="import-report settings-import-detail">
          {importReport?.details.map((d, i) => (
            <div key={i} className={`import-report__item ${d.status === "imported" ? "is-ok" : "is-skip"}`}>
              <div className="import-report__row">
                <strong>{d.cc_name}</strong>
                <span className={`import-report__badge ${d.status === "imported" ? "badge-ok" : "badge-skip"}`}>
                  {d.status === "imported" ? t("settings.importDetailImported") : t("settings.importDetailSkipped")}
                </span>
              </div>
              {d.models_imported.length > 0 ? (
                <p className="import-report__meta muted">
                  {t("settings.importDetailModels")}: {d.models_imported.join(", ")}
                </p>
              ) : null}
              {d.reason ? (
                <p className="import-report__meta import-report__meta--warn muted">
                  {t("settings.importDetailReason")}: {d.reason}
                </p>
              ) : null}
            </div>
          ))}
        </div>
      </Modal>
    </div>
  );
}
