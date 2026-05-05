import { useState } from "react";
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

export function RequestLogDrawer({ logs }: { logs: RequestLog[] }) {
  const { t } = useI18n();
  const [isExpanded, setIsExpanded] = useState(() => sessionExpanded);

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
        <div className="usage-log-wrap">
          <table>
            <thead>
              <tr>
                <th>{t("requestLog.colTime")}</th>
                <th>{t("requestLog.colProviderModel")}</th>
                <th>{t("requestLog.colStatus")}</th>
                <th>{t("requestLog.colLatency")}</th>
                <th>{t("requestLog.colIn")}</th>
                <th>{t("requestLog.colOut")}</th>
              </tr>
            </thead>
            <tbody>
              {logs.map((log) => (
                <tr key={log.id}>
                  <td className="cell-time cell-nowrap">{formatCompactDateTime(log.created_at)}</td>
                  <td className="usage-log-provider-model">{providerModelText(log)}</td>
                  <td>{log.status_code}</td>
                  <td className="cell-nowrap">{formatLatency(log.latency_ms)}</td>
                  <td className="cell-num" title={`${log.input_tokens} + ${log.cache_read_tokens}(cached)`}>
                    {formatCompactCount(log.input_tokens)}
                    {log.cache_read_tokens > 0 ? ` + ${formatCompactCount(log.cache_read_tokens)}(cached)` : ''}
                  </td>
                  <td className="cell-num" title={String(log.output_tokens)}>
                    {formatCompactCount(log.output_tokens)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
