import pkg from "../../../package.json";
import { useI18n } from "../../i18n";

export function AboutTab() {
  const { t } = useI18n();

  return (
    <div className="settings-tab-stack">
      <div className="settings-section settings-section--card card card--compact">
        <div className="settings-section-head">
          <span className="settings-section-icon" aria-hidden>
            <svg viewBox="0 0 24 24" width={18} height={18} fill="none" stroke="currentColor" strokeWidth={1.75} strokeLinecap="round" strokeLinejoin="round">
              <circle cx="12" cy="12" r="10" />
              <line x1="12" y1="16" x2="12" y2="12" />
              <line x1="12" y1="8" x2="12.01" y2="8" />
            </svg>
          </span>
          <h3 className="settings-section__title">{t("settings.aboutTitle")}</h3>
        </div>
        <dl className="settings-about settings-about--compact settings-about--stretch">
          <div>
            <dt>{t("settings.aboutVersion")}</dt>
            <dd>{pkg.version}</dd>
          </div>
          <div className="settings-about__note">
            <dt>{t("settings.aboutNote")}</dt>
            <dd className="muted settings-about__desc">{t("settings.aboutDesc")}</dd>
          </div>
        </dl>
      </div>
    </div>
  );
}
