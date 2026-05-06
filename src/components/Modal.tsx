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
  /** 隐藏右上角关闭按钮 */
  noClose?: boolean;
};

export function Modal({ title, open, onClose, children, footer, headerActions, variant = "default", noClose = false }: Props) {
  const { t } = useI18n();
  const [mountNode, setMountNode] = useState<HTMLElement | null>(null);
  const zIndexRef = useRef(0);
  const prevOpen = useRef(false);

  useEffect(() => {
    setMountNode(document.body);
  }, []);

  // Assign z-index synchronously during render when modal transitions closed→open,
  // so the portal always renders with the correct value (refs don't trigger re-renders).
  if (open && !prevOpen.current) {
    nextModalZ += 1;
    zIndexRef.current = nextModalZ;
  }
  prevOpen.current = open;

  if (!open || !mountNode) return null;
  const nested = variant === "nested";
  const baseZ = zIndexRef.current + (nested ? 50 : 0);

  return createPortal(
    <div
      className={`modal-backdrop${nested ? " modal-backdrop--nested" : ""}`}
      role="presentation"
      style={{ zIndex: baseZ }}
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
            {!noClose ? (
            <button type="button" className="btn btn--ghost btn--sm" onClick={onClose}>
              {t("modal.close")}
            </button>
            ) : null}
          </div>
        </header>
        <div className="modal-body">{children}</div>
        {footer ? <footer className="modal-footer">{footer}</footer> : null}
      </div>
    </div>,
    mountNode
  );
}
