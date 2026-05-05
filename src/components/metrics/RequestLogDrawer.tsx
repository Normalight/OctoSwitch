import { useRef, useState } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { formatCompactCount } from "../../lib/formatNumber";
import { formatCompactDateTime } from "../../lib/formatTime";
import { useI18n } from "../../i18n";

// Session-level state: persists across tab switches, resets on app restart.
let sessionExpanded = false;

export interface RequestLog {
  id: string;
  group_name: string;
  model_name: string;
  provider_name: string;
  latency_ms: number;
  input_tokens: number;
  output_tokens: number;
  cache_read_tokens: number;
  status_code: number;
  created_at: string;
}

function formatLatency(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  const totalSec = Math.round(ms / 1000);
  const days = Math.floor(totalSec / 86400);
  const hours = Math.floor((totalSec % 86400) / 3600);
  const minutes = Math.floor((totalSec % 3600) / 60);
  const seconds = totalSec % 60;
  let result = "";
  if (days > 0) result += `${days}d`;
  if (hours > 0) result += `${hours}h`;
  if (minutes > 0) result += `${minutes}m`;
  if (seconds > 0) result += `${seconds}s`;
  return result || "0ms";
}

/** CSS Grid column template matching the existing table column layout. */
const GRID_COLS = "162px 1fr 66px 90px 74px 74px";

export function RequestLogDrawer({ logs }: { logs: RequestLog[] }) {
  const { t } = useI18n();
  const [isExpanded, setIsExpanded] = useState(() => sessionExpanded);
  const scrollRef = useRef<HTMLDivElement>(null);

  const virtualizer = useVirtualizer({
    count: isExpanded ? logs.length : 0,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => 36,
    overscan: 10,
  });

  const providerModelText = (log: RequestLog) =>
    [log.group_name?.trim(), log.provider_name?.trim(), log.model_name?.trim()]
      .filter((v) => Boolean(v))
      .join(" · ");

  return (
    <div className="usage-log-section">
      <h3
        style={{ cursor: "pointer", display: "flex", alignItems: "center", gap: "8px", userSelect: "none" }}
        onClick={() => {
          sessionExpanded = !sessionExpanded;
          setIsExpanded(sessionExpanded);
        }}
      >
        <svg
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          style={{ transform: isExpanded ? "rotate(90deg)" : "none", transition: "transform 0.2s" }}
        >
          <path d="M9 18l6-6-6-6" />
        </svg>
        {t("requestLog.title")}
      </h3>
      {isExpanded && (
        <div className="usage-log-wrap" ref={scrollRef}>
          <div
            className="usage-log-grid-head"
            style={{
              display: "grid",
              gridTemplateColumns: GRID_COLS,
              position: "sticky",
              top: 0,
              zIndex: 1,
              background: "var(--bg)",
              borderBottom: "1px solid var(--border)",
              padding: "8px 10px 6px",
              fontWeight: 600,
              fontSize: "0.8rem",
              fontFamily: "var(--font-usage-log, inherit)",
            }}
          >
            <div>{t("requestLog.colTime")}</div>
            <div>{t("requestLog.colProviderModel")}</div>
            <div>{t("requestLog.colStatus")}</div>
            <div>{t("requestLog.colLatency")}</div>
            <div style={{ textAlign: "right" }}>{t("requestLog.colIn")}</div>
            <div style={{ textAlign: "right" }}>{t("requestLog.colOut")}</div>
          </div>
          <div
            style={{
              height: `${virtualizer.getTotalSize()}px`,
              position: "relative",
              width: "100%",
            }}
          >
            {virtualizer.getVirtualItems().map((vItem) => {
              const log = logs[vItem.index];
              return (
                <div
                  key={log.id}
                  data-index={vItem.index}
                  style={{
                    position: "absolute",
                    top: 0,
                    left: 0,
                    width: "100%",
                    transform: `translateY(${vItem.start}px)`,
                    display: "grid",
                    gridTemplateColumns: GRID_COLS,
                    alignItems: "center",
                    padding: "6px 10px",
                    fontFamily: "var(--font-usage-log, inherit)",
                    fontSize: "0.875rem",
                    lineHeight: "1.45",
                    borderBottom: "1px solid color-mix(in srgb, var(--border) 60%, transparent)",
                  }}
                >
                  <div className="cell-time cell-nowrap">{formatCompactDateTime(log.created_at)}</div>
                  <div className="usage-log-provider-model">{providerModelText(log)}</div>
                  <div>{log.status_code}</div>
                  <div className="cell-nowrap">{formatLatency(log.latency_ms)}</div>
                  <div className="cell-num" style={{ textAlign: "right" }} title={`${log.input_tokens}`}>
                    {formatCompactCount(log.input_tokens + (log.cache_read_tokens ?? 0))}
                  </div>
                  <div className="cell-num" style={{ textAlign: "right" }} title={String(log.output_tokens)}>
                    {formatCompactCount(log.output_tokens)}
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}
