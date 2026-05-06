import { useI18n } from "../i18n";

type Props = {
  message: string | null;
  onDismiss: () => void;
};

/** Inline banner when list-data Tauri invokes fail (replacing silent empty lists). */
export function LoadErrorBanner({ message, onDismiss }: Props) {
  const { t } = useI18n();
  if (!message) return null;
  return (
    <div className="error-banner" role="alert">
      <div className="load-error-banner__row">
        <div className="load-error-banner__body">
          <strong>{t("common.listLoadFailed")}</strong>
          <p className="muted load-error-banner__detail">{message}</p>
        </div>
        <button type="button" className="btn btn--ghost btn--sm" onClick={onDismiss}>
          {t("modal.close")}
        </button>
      </div>
    </div>
  );
}
