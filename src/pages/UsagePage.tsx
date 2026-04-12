import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { KpiCards } from "../components/metrics/KpiCards";
import { RequestLogDrawer, type RequestLog } from "../components/metrics/RequestLogDrawer";
import { TrendsPanel } from "../components/metrics/TrendsPanel";
import { useI18n } from "../i18n";
import { tauriApi, type UsageWindowKey } from "../lib/api/tauri";
import type { MetricKpi, MetricPoint } from "../types";

function errMsg(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

/** 用量页打开时的自动拉取间隔（毫秒） */
const USAGE_AUTO_REFRESH_MS = 10_000;
/** 同参数短时间复用结果，减少快速重复触发的开销 */
const USAGE_CACHE_TTL_MS = 3_000;

function pad2(n: number): string {
  return String(n).padStart(2, "0");
}

function toLocalDatetimeValue(d: Date): string {
  return `${d.getFullYear()}-${pad2(d.getMonth() + 1)}-${pad2(d.getDate())}T${pad2(d.getHours())}:${pad2(
    d.getMinutes()
  )}`;
}

function localDatetimeInputToIsoUtc(v: string): string | null {
  if (!v.trim()) return null;
  const ms = new Date(v).getTime();
  if (Number.isNaN(ms)) return null;
  return new Date(ms).toISOString();
}

export function UsagePage() {
  const { t } = useI18n();
  const [kpi, setKpi] = useState<MetricKpi | null>(null);
  const [points, setPoints] = useState<MetricPoint[]>([]);
  const [logs, setLogs] = useState<RequestLog[]>([]);
  const [kpiError, setKpiError] = useState<string | null>(null);
  const [seriesError, setSeriesError] = useState<string | null>(null);
  const [logsError, setLogsError] = useState<string | null>(null);
  const [customRangeError, setCustomRangeError] = useState<string | null>(null);
  const [windowKey, setWindowKey] = useState<UsageWindowKey>("1h");
  const [customStartLocal, setCustomStartLocal] = useState("");
  const [customEndLocal, setCustomEndLocal] = useState("");
  const skipNextAutoLoad = useRef(false);
  const loadingRef = useRef(false);
  const requestSeqRef = useRef(0);
  const cacheRef = useRef<{
    key: string;
    at: number;
    kpi: MetricKpi;
    points: MetricPoint[];
    logs: RequestLog[];
  } | null>(null);

  const load = useCallback(async () => {
    if (loadingRef.current) return;
    loadingRef.current = true;
    const seq = ++requestSeqRef.current;

    setKpiError(null);
    setSeriesError(null);
    setLogsError(null);
    setCustomRangeError(null);

    let customStartIso: string | null = null;
    let customEndIso: string | null = null;
    if (windowKey === "custom") {
      customStartIso = localDatetimeInputToIsoUtc(customStartLocal);
      customEndIso = localDatetimeInputToIsoUtc(customEndLocal);
      if (!customStartIso || !customEndIso) {
        setCustomRangeError(t("usage.customInvalid"));
        loadingRef.current = false;
        return;
      }
      if (new Date(customStartIso) >= new Date(customEndIso)) {
        setCustomRangeError(t("usage.customOrder"));
        loadingRef.current = false;
        return;
      }
    }

    const cacheKey = JSON.stringify({ windowKey, customStartIso, customEndIso });
    const cached = cacheRef.current;
    if (cached && cached.key === cacheKey && Date.now() - cached.at < USAGE_CACHE_TTL_MS) {
      setKpi(cached.kpi);
      setPoints(cached.points);
      setLogs(cached.logs);
      loadingRef.current = false;
      return;
    }

    const [kpiRes, seriesRes, logsRes] = await Promise.allSettled([
      tauriApi.getMetricsKpi(windowKey, customStartIso, customEndIso),
      tauriApi.getMetricsSeries(windowKey, customStartIso, customEndIso),
      tauriApi.getRequestLogs(windowKey, customStartIso, customEndIso)
    ]);

    if (seq !== requestSeqRef.current) {
      loadingRef.current = false;
      return;
    }

    if (kpiRes.status === "fulfilled") {
      setKpi(kpiRes.value);
    } else {
      setKpi(null);
      setKpiError(errMsg(kpiRes.reason));
    }

    if (seriesRes.status === "fulfilled") {
      setPoints(seriesRes.value);
    } else {
      setPoints([]);
      setSeriesError(errMsg(seriesRes.reason));
    }

    if (logsRes.status === "fulfilled") {
      setLogs(logsRes.value);
    } else {
      setLogs([]);
      setLogsError(errMsg(logsRes.reason));
    }

    if (
      kpiRes.status === "fulfilled" &&
      seriesRes.status === "fulfilled" &&
      logsRes.status === "fulfilled"
    ) {
      cacheRef.current = {
        key: cacheKey,
        at: Date.now(),
        kpi: kpiRes.value,
        points: seriesRes.value,
        logs: logsRes.value,
      };
    }

    loadingRef.current = false;
  }, [windowKey, customStartLocal, customEndLocal, t]);

  useEffect(() => {
    if (skipNextAutoLoad.current) {
      skipNextAutoLoad.current = false;
      return;
    }
    void load();
  }, [load]);

  useEffect(() => {
    const tick = () => {
      if (typeof document !== "undefined" && document.visibilityState !== "visible") {
        return;
      }
      void load();
    };
    const id = window.setInterval(tick, USAGE_AUTO_REFRESH_MS);
    const onVisible = () => {
      if (document.visibilityState !== "visible") return;
      // 延后到下一帧再拉数，避免与窗口恢复时的首帧绘制抢主线程
      requestAnimationFrame(() => {
        requestAnimationFrame(() => {
          void load();
        });
      });
    };
    document.addEventListener("visibilitychange", onVisible);
    return () => {
      window.clearInterval(id);
      document.removeEventListener("visibilitychange", onVisible);
    };
  }, [load]);

  const trendsRangeLabel = useMemo(() => {
    switch (windowKey) {
      case "5m":
        return t("trends.win5m");
      case "1h":
        return t("trends.win1h");
      case "24h":
        return t("trends.win24h");
      case "30d":
        return t("trends.win30d");
      case "custom":
        return t("trends.winCustom");
      default:
        return windowKey;
    }
  }, [windowKey, t]);

  return (
    <section className="usage-page page-resource">
      <header className="usage-page__head">
        <div className="usage-page__head-row usage-page__head-row--title">
          <h2 className="page-title usage-page__title">{t("usage.title")}</h2>
          <div className="usage-page__toolbar" role="toolbar" aria-label={t("usage.windowLabel")}>
            <label className="usage-control">
              <span className="sr-only">{t("usage.windowLabel")}</span>
              <select
                className="usage-select"
                value={windowKey}
                aria-label={t("usage.windowLabel")}
                onChange={(ev) => {
                  const v = ev.target.value as UsageWindowKey;
                  if (v === "custom") {
                    const end = new Date();
                    const start = new Date(end.getTime() - 24 * 3600 * 1000);
                    setCustomStartLocal(toLocalDatetimeValue(start));
                    setCustomEndLocal(toLocalDatetimeValue(end));
                    skipNextAutoLoad.current = true;
                  }
                  setWindowKey(v);
                }}
              >
                <option value="5m">{t("usage.win5m")}</option>
                <option value="1h">{t("usage.win1h")}</option>
                <option value="24h">{t("usage.win24h")}</option>
                <option value="30d">{t("usage.win30d")}</option>
                <option value="custom">{t("usage.winCustom")}</option>
              </select>
            </label>
            <button
              type="button"
              className="btn btn--sm btn--icon btn--ghost usage-refresh-btn"
              title={t("common.refresh")}
              onClick={() => load()}
            >
              <svg viewBox="0 0 24 24" width="16" height="16" stroke="currentColor" strokeWidth="2" fill="none" strokeLinecap="round" strokeLinejoin="round">
                <polyline points="23 4 23 10 17 10" />
                <polyline points="1 20 1 14 7 14" />
                <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15" />
              </svg>
            </button>
          </div>
        </div>
        <p className="page-lead muted usage-page__lead">{t("usage.lead")}</p>
        {windowKey === "custom" ? (
          <div className="usage-custom-row" role="group" aria-label={t("usage.winCustom")}>
            <label className="usage-control">
              <span className="sr-only">{t("usage.customStart")}</span>
              <input
                type="datetime-local"
                className="usage-datetime"
                value={customStartLocal}
                onChange={(ev) => setCustomStartLocal(ev.target.value)}
                aria-label={t("usage.customStart")}
              />
            </label>
            <span className="usage-custom-sep" aria-hidden="true">
              —
            </span>
            <label className="usage-control">
              <span className="sr-only">{t("usage.customEnd")}</span>
              <input
                type="datetime-local"
                className="usage-datetime"
                value={customEndLocal}
                onChange={(ev) => setCustomEndLocal(ev.target.value)}
                aria-label={t("usage.customEnd")}
              />
            </label>
            <p className="muted usage-custom-hint">{t("usage.customHint")}</p>
          </div>
        ) : null}
      </header>
      {customRangeError ? (
        <p className="error-banner" role="alert">
          {customRangeError}
        </p>
      ) : null}
      {kpiError ? (
        <p className="error-banner" role="alert">
          {t("usage.kpiLoadErr")}
          {kpiError}
        </p>
      ) : null}
      {seriesError ? (
        <p className="error-banner" role="alert">
          {t("usage.seriesLoadErr")}
          {seriesError}
        </p>
      ) : null}
      {logsError ? (
        <p className="error-banner" role="alert">
          {t("usage.logsLoadErr")}
          {logsError}
        </p>
      ) : null}
      <KpiCards kpi={kpi} />
      <TrendsPanel points={points} rangeLabel={trendsRangeLabel} />
      <RequestLogDrawer logs={logs} />
    </section>
  );
}
