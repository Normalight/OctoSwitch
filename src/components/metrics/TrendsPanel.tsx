import { useMemo, useState } from "react";
import type { MetricPoint } from "../../types";
import { formatChartBucketLabel } from "../../lib/formatTime";
import { useTheme } from "../../theme/ThemeContext";
import { useI18n } from "../../i18n";
import {
  CartesianGrid,
  Legend,
  Line,
  LineChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis
} from "recharts";

export function TrendsPanel({
  points,
  rangeLabel
}: {
  points: MetricPoint[];
  /** 与当前 KPI / 日志一致的时间范围展示名（已翻译） */
  rangeLabel: string;
}) {
  const { t } = useI18n();
  const { resolvedTheme } = useTheme();
  const chartData = useMemo(() => (points ?? []).map(p => ({ ...p })), [points]);
  const isLight = resolvedTheme === "light";

  const [showTokens, setShowTokens] = useState(true);
  const [showInputTokens, setShowInputTokens] = useState(false);
  const [showOutputTokens, setShowOutputTokens] = useState(false);
  const [showCacheRead, setShowCacheRead] = useState(false);

  const axisProps = useMemo(
    () => ({
      stroke: isLight ? "#94a3b8" : "#475569",
      tick: { fill: isLight ? "#64748b" : "#94a3b8", fontSize: 10 },
      tickLine: { stroke: isLight ? "#94a3b8" : "#475569" }
    }),
    [isLight]
  );

  const tooltipContentStyle = useMemo(
    () => ({
      background: isLight ? "#ffffff" : "#0f172a",
      border: isLight ? "1px solid #cbd5e1" : "1px solid #334155",
      borderRadius: 8,
      color: isLight ? "#0f172a" : "#f8fafc"
    }),
    [isLight]
  );

  const gridStroke = isLight ? "#cbd5e1" : "#334155";
  const legendColor = isLight ? "#64748b" : "#94a3b8";

  const axisTickFormatter = (v: string | number) => formatChartBucketLabel(String(v));
  const yAxisTickFormatter = (v: string | number) => Number(v).toFixed(2);

  return (
    <div className="chart-box chart-box--usage">
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-end", flexWrap: "wrap", gap: "12px" }}>
        <h3 className="usage-chart-title" style={{ margin: 0 }}>
          {t("trends.titlePrefix")}
          {rangeLabel}
          {t("trends.titleSuffix")}
        </h3>

        {chartData.length > 0 && (
          <div style={{ display: "flex", gap: "8px", alignItems: "center", fontSize: "0.75rem", flexWrap: "wrap", margin: "8px 0" }}>
            {[
              {
                id: "tokens",
                label: t("trends.lineTokens"),
                color: "#c084fc",
                checked: showTokens,
                onChange: setShowTokens
              },
              {
                id: "inputTokens",
                label: t("requestLog.colIn"),
                color: "#ff7cdf",
                checked: showInputTokens,
                onChange: setShowInputTokens
              },
              {
                id: "outputTokens",
                label: t("requestLog.colOut"),
                color: "#06b6d4",
                checked: showOutputTokens,
                onChange: setShowOutputTokens
              },
              {
                id: "cacheRead",
                label: t("trends.lineCacheRead"),
                color: "#22d3ee",
                checked: showCacheRead,
                onChange: setShowCacheRead
              }
            ].map((btn) => (
              <button
                key={btn.id}
                type="button"
                onClick={() => btn.onChange(!btn.checked)}
                className="btn btn--sm"
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: "6px",
                  padding: "4px 10px",
                  borderRadius: "16px",
                  background: btn.checked ? btn.color + "15" : "transparent",
                  border: "1px solid " + (btn.checked ? btn.color : (isLight ? "#cbd5e1" : "#334155")),
                  color: btn.checked ? btn.color : legendColor,
                  fontWeight: btn.checked ? 500 : 400,
                  transition: "all 0.2s"
                }}
              >
                <div style={{
                  width: 8,
                  height: 8,
                  borderRadius: "50%",
                  background: btn.checked ? btn.color : (isLight ? "#94a3b8" : "#475569"),
                  transition: "all 0.2s"
                }} />
                {btn.label}
              </button>
            ))}
          </div>
        )}
      </div>

      {chartData.length > 0 ? (
        <p className="usage-chart-note muted" style={{ marginTop: "6px" }}>{t("trends.chartNote")}</p>
      ) : null}
      {chartData.length === 0 ? (
        <p className="muted usage-chart-empty">{t("trends.empty")}</p>
      ) : (
        <div className="recharts-host" style={{ marginTop: '24px' }}>
          <ResponsiveContainer width="100%" height={320} debounce={50}>
            <LineChart data={chartData} margin={{ top: 8, right: 12, left: 12, bottom: 24 }}>
              <CartesianGrid strokeDasharray="3 3" stroke={gridStroke} />

              <XAxis
                dataKey="bucket_time"
                interval="preserveStartEnd"
                minTickGap={28}
                tickFormatter={axisTickFormatter}
                {...axisProps}
              />
              <YAxis
                width={80}
                tickFormatter={(v) => Number(v) === 0 ? "0" : Number(v) < 1 ? Number(v).toFixed(4) : Number(v) < 10 ? Number(v).toFixed(2) : Number(v).toFixed(0)}
                {...axisProps}
              />
              <Tooltip
                contentStyle={tooltipContentStyle}
                labelStyle={{ color: legendColor }}
                labelFormatter={(label) => formatChartBucketLabel(String(label))}
                formatter={(value, name) => [
                  Number(value).toFixed(0),
                  name
                ]}
              />
              <Legend
                verticalAlign="bottom"
                align="center"
                wrapperStyle={{ fontSize: "11px", color: legendColor, paddingBottom: 4 }}
              />
              {showTokens && <Line type="monotone" name={t("trends.lineTokens")} dataKey="consumed_tokens" stroke="#c084fc" dot={false} strokeWidth={2} fill="#c084fc" fillOpacity={0.2} />}
              {showInputTokens && <Line type="monotone" name={t("requestLog.colIn")} dataKey="input_tokens" stroke="#ff7cdf" dot={false} strokeWidth={2} />}
              {showOutputTokens && <Line type="monotone" name={t("requestLog.colOut")} dataKey="output_tokens" stroke="#06b6d4" dot={false} strokeWidth={2} />}
              {showCacheRead && <Line type="monotone" name={t("trends.lineCacheRead")} dataKey="cache_read_tokens" stroke="#22d3ee" dot={false} strokeWidth={2} />}
            </LineChart>
          </ResponsiveContainer>
        </div>
      )}
    </div>
  );
}
