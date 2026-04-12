import { Suspense, lazy, useEffect, useRef, useState } from "react";
import { useI18n } from "./i18n";
const ProvidersPage = lazy(async () => {
  const mod = await import("./pages/ProvidersPage");
  return { default: mod.ProvidersPage };
});
const ModelsPage = lazy(async () => {
  const mod = await import("./pages/ModelsPage");
  return { default: mod.ModelsPage };
});
const UsagePage = lazy(async () => {
  const mod = await import("./pages/UsagePage");
  return { default: mod.UsagePage };
});
const SettingsPage = lazy(async () => {
  const mod = await import("./pages/SettingsPage");
  return { default: mod.SettingsPage };
});

type Tab = "providers" | "models" | "usage" | "settings";

export function App() {
  const { t } = useI18n();
  const [tab, setTab] = useState<Tab>("providers");
  const lastMainTabRef = useRef<Tab>("providers");
  const goMain = (next: Tab) => {
    if (next !== "settings") {
      lastMainTabRef.current = next;
    }
    setTab(next);
  };

  const openSettings = () => {
    if (tab !== "settings") {
      lastMainTabRef.current = tab;
    }
    setTab("settings");
  };

  const exitSettings = () => {
    setTab(lastMainTabRef.current);
  };

  useEffect(() => {
    const prefetch = () => {
      void import("./pages/ModelsPage");
      void import("./pages/UsagePage");
      void import("./pages/SettingsPage");
    };
    const ric = window.requestIdleCallback;
    if (typeof ric === "function") {
      const id = ric(prefetch, { timeout: 1200 });
      return () => window.cancelIdleCallback(id);
    }
    const timer = window.setTimeout(prefetch, 800);
    return () => window.clearTimeout(timer);
  }, []);

  return (
    <main className="app app-shell">
      <div className="app-inner">
        {tab !== "settings" ? (
          <header className="app-header">
            <div className="app-header-row">
              <div style={{ flex: 1 }}></div>

              <nav className="nav-tabs" aria-label={t("app.navLabel")}>
                <button
                  type="button"
                  className={`nav-tab ${tab === "providers" ? "is-active" : ""}`}
                  onClick={() => goMain("providers")}
                >
                  {t("app.providers")}
                </button>
                <button
                  type="button"
                  className={`nav-tab ${tab === "models" ? "is-active" : ""}`}
                  onClick={() => goMain("models")}
                >
                  {t("app.groups")}
                </button>
                <button
                  type="button"
                  className={`nav-tab ${tab === "usage" ? "is-active" : ""}`}
                  onClick={() => goMain("usage")}
                >
                  {t("app.usage")}
                </button>
              </nav>

              <div className="app-header-actions" style={{ flex: 1 }}>
                <button
                  type="button"
                  className={`btn btn--sm btn--icon app-header-settings-btn`}
                  onClick={openSettings}
                  aria-label={t("app.settings")}
                >
                  <svg
                    className="app-settings-gear"
                    viewBox="0 0 24 24"
                    width={20}
                    height={20}
                    fill="none"
                    stroke="currentColor"
                    strokeWidth={1.5}
                    aria-hidden
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      d="M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.074.04.147.083.22.127.324.196.72.257 1.075.124l1.217-.456a1.125 1.125 0 011.37.49l1.296 2.247a1.125 1.125 0 01-.26 1.431l-1.003.827c-.293.24-.438.613-.431.992a6.759 6.759 0 010 .255c-.007.378.138.75.43.99l1.005.828c.424.35.534.954.26 1.43l-1.298 2.247a1.125 1.125 0 01-1.369.491l-1.217-.456c-.355-.133-.75-.072-1.076.124a6.57 6.57 0 01-.22.128c-.331.183-.581.495-.644.869l-.213 1.281c-.09.543-.56.941-1.11.941h-2.594c-.55 0-1.02-.398-1.11-.94l-.213-1.281c-.062-.374-.312-.686-.644-.87a6.52 6.52 0 01-.22-.127c-.325-.196-.72-.257-1.076-.124l-1.217.456a1.125 1.125 0 01-1.369-.49l-1.297-2.247a1.125 1.125 0 01.26-1.431l1.004-.827c.292-.24.437-.613.43-.992a6.932 6.932 0 010-.255c.007-.378-.138-.75-.43-.99l-1.004-.828a1.125 1.125 0 01-.26-1.43l1.297-2.247a1.125 1.125 0 011.37-.491l1.216.456c.356.133.751.072 1.076-.124.072-.044.146-.087.22-.128.332-.183.582-.495.644-.869l.214-1.281z"
                    />
                    <path strokeLinecap="round" strokeLinejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                  </svg>
                </button>
              </div>
            </div>
          </header>
        ) : null}

        <div className="app-main">
          <Suspense fallback={<p className="muted">{t("common.loading")}</p>}>
          {tab === "providers" ? <ProvidersPage /> : null}
          {tab === "models" ? <ModelsPage /> : null}
          {tab === "usage" ? <UsagePage /> : null}
          {tab === "settings" ? <SettingsPage onExit={exitSettings} /> : null}
          </Suspense>
        </div>
      </div>
    </main>
  );
}
