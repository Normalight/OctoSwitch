import { Modal } from "./Modal";
import { useI18n } from "../i18n";

type ConfirmProps = {
  title: string;
  message: string;
  open: boolean;
  onClose: () => void;
  onConfirm: () => void;
  confirmText?: string;
  confirmVariant?: "danger" | "primary";
};

export function ConfirmDialog({
  title,
  message,
  open,
  onClose,
  onConfirm,
  confirmText,
  confirmVariant = "danger"
}: ConfirmProps) {
  const { t } = useI18n();
  return (
    <Modal
      title={title}
      open={open}
      onClose={onClose}
      footer={
        <div className="panel-actions flat">
          <button
            type="button"
            className={`btn btn--${confirmVariant}`}
            onClick={() => { onConfirm(); onClose(); }}
          >
            {confirmText ?? t("common.yes")}
          </button>
          <button type="button" className="btn btn--ghost" onClick={onClose}>
            {t("common.cancel")}
          </button>
        </div>
      }
    >
      <p className="muted">{message}</p>
    </Modal>
  );
}

type ErrorDialogProps = {
  title: string;
  message: string;
  open: boolean;
  onClose: () => void;
};

export function ErrorDialog({ title, message, open, onClose }: ErrorDialogProps) {
  const { t } = useI18n();
  return (
    <Modal
      title={title}
      open={open}
      onClose={onClose}
      footer={
        <div className="panel-actions flat">
          <button type="button" className="btn btn--ghost" onClick={onClose}>
            {t("common.cancel")}
          </button>
        </div>
      }
    >
      <p className="muted">{message}</p>
    </Modal>
  );
}
