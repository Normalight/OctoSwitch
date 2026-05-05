import type { ReactNode } from "react";
import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useI18n } from "../i18n";

/** Base z-index for modals; increments per-open so later modals always stack on top. */
const BASE_MODAL_Z = 1000;
let nextModalZ = BASE_MODAL_Z;

type Props = {
  title: string;
  open: boolean;
  onClose: () => void;
  children: ReactNode;
  footer?: ReactNode;
  headerActions?: ReactNode;
  /** 嵌套在主弹窗之上，略抬高 z-index 与视觉层次 */
  variant?: "default" | "nested";
};

export function Modal({ title, open, onClose, children, footer, headerActions, variant = "default" }: Props) {
  const { t } = useI18n();
  const [mountNode, setMountNode] = useState<HTMLElement | null>(null);
  const zIndexRef = useRef(0);

  useEffect(() => {
    setMountNode(document.body);
  }, []);

  useEffect(() => {
    if (open && mountNode) {
      nextModalZ += 1;
      zIndexRef.current = nextModalZ;
    }
  }, [open, mountNode]);

  if (!open || !mountNode) return null;
  const nested = variant === "nested";
  const baseZ = variant === "nested" ? zIndexRef.current : zIndexRef.current;

  return createPortal(
    <div
      className={`modal-backdrop${nested ? " modal-backdrop--nested" : ""}`}
      role="presentation"
      style={{ zIndex: baseZ, ...(nested ? {} : {}) }}
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        className={`modal-panel card${nested ? " modal-panel--nested" : ""}`}
        role="dialog"
        aria-modal="true"
        aria-labelledby="modal-title"
        style={{ zIndex: baseZ + 1 }}
        onClick={(e) => e.stopPropagation()}
      >
        <header className="modal-header">
          <h3 id="modal-title" className="modal-title">
            {title}
          </h3>
          <div className="modal-header__actions">
            {headerActions}
            <button type="button" className="btn btn--ghost btn--sm" onClick={onClose}>
              {t("modal.close")}
            </button>
          </div>
        </header>
        <div className="modal-body">{children}</div>
        {footer ? <footer className="modal-footer">{footer}</footer> : null}
      </div>
    </div>,
    mountNode
  );
}
