import { useRef, useState } from "react";
import { GeneralTab } from "./settings/GeneralTab";
import { DataTab } from "./settings/DataTab";
import { AboutTab } from "./settings/AboutTab";
import { useI18n } from "../i18n";

type SettingsSubTab = "general" | "data" | "about";

type Props = {
  onExit: () => void;
};

export function SettingsPage({ onExit }: Props) {
  const { t } = useI18n();
  const [subTab, setSubTab] = useState<SettingsSubTab>("general");
  const generalRef = useRef<{ resetLogLevel: () => void } | null>(null);

  const exitSettings = () => {
    generalRef.current?.resetLogLevel();
    onExit();
  };

  const tabs: Array<[SettingsSubTab, string]> = [
    ["general", t("settings.tabGeneral")],
    ["data", t("settings.tabData")],
    ["about", t("settings.tabAbout")],
  ];

  return (
    <section className="page-resource settings-page">
      <header className="app-header">
        <div className="app-header-row">
          <div style={{ flex: 1 }}>
            <button type="button" className="settings-back" onClick={exitSettings} aria-label={t("settings.backMain")}>
              <svg className="settings-back__svg" viewBox="0 0 24 24" width={18} height={18} aria-hidden>
                <path d="M15 18l-6-6 6-6" fill="none" stroke="currentColor" strokeWidth="1.75" strokeLinecap="round" strokeLinejoin="round" />
              </svg>
            </button>
          </div>

          <nav className="nav-tabs" aria-label={t("settings.title")}>
            {tabs.map(([id, label]) => (
              <button key={id} type="button" className={`nav-tab ${subTab === id ? "is-active" : ""}`} onClick={() => setSubTab(id)}>
                {label}
              </button>
            ))}
          </nav>

          <div style={{ flex: 1 }}></div>
        </div>
      </header>

      {subTab === "general" ? <GeneralTab ref={generalRef} /> : null}
      {subTab === "data" ? <DataTab /> : null}
      {subTab === "about" ? <AboutTab /> : null}
    </section>
  );
}
