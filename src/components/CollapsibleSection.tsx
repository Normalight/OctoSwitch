import { useState, useCallback, type ReactNode } from "react";

interface Props {
  id: string;
  icon: ReactNode;
  title: string;
  badge?: ReactNode;
  summaryCollapsed?: ReactNode;
  defaultOpen?: boolean;
  children: ReactNode;
}

export function CollapsibleSection({
  id,
  icon,
  title,
  badge,
  summaryCollapsed,
  defaultOpen = true,
  children,
}: Props) {
  const [open, setOpen] = useState(defaultOpen);

  const toggle = useCallback(() => setOpen((o) => !o), []);

  return (
    <div className={`settings-section settings-section--card card card--compact collapsible-section ${open ? "is-open" : "is-collapsed"}`}>
      <button
        type="button"
        className="collapsible-header"
        onClick={toggle}
        aria-expanded={open}
        aria-controls={`collapsible-body-${id}`}
      >
        <span className="settings-section-icon" aria-hidden>
          {icon}
        </span>
        <h3 className="settings-section__title">{title}</h3>
        {badge}
        {!open && summaryCollapsed ? (
          <span className="collapsible-summary">{summaryCollapsed}</span>
        ) : null}
        <span className="collapsible-chevron" aria-hidden>
          <svg viewBox="0 0 24 24" width={16} height={16} fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
            <polyline points="6 9 12 15 18 9" />
          </svg>
        </span>
      </button>
      <div
        id={`collapsible-body-${id}`}
        className="collapsible-body"
        role="region"
        aria-labelledby={`collapsible-heading-${id}`}
      >
        <div className="collapsible-body-inner">
          {children}
        </div>
      </div>
    </div>
  );
}
