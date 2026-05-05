import { useMemo, useState } from "react";
import type { MetricPoint } from "../../types";
import { formatChartBucketLabel } from "../../lib/formatTime";
import { formatCompactCount } from "../../lib/formatNumber";
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

function CustomTooltip({
  active, payload, label,
  isLight, t
}: {
  active?: boolean;
  payload?: Array<{ payload: Record<string, unknown> }>;
  label?: string;
  isLight: boolean;
  t: (key: string) => string;
}) {
  if (!active || !payload || payload.length === 0) return null;
  const p = payload[0].payload as Record<string, unknown>;
  const models = (p._models as Array<{
    group_name: string;
    input_tokens: number;
    output_tokens: number;
    cache_read_tokens: number;
  }>) ?? [];

  const bg = isLight ? "#ffffff" : "#0f172a";
  const border = isLight ? "#cbd5e1" : "#334155";
  const text = isLight ? "#0f172a" : "#f8fafc";
  const muted = isLight ? "#64748b" : "#94a3b8";

  return (
    <div style={{
      background: bg,
      border: `1px solid ${border}`,
      borderRadius: 8,
      padding: "10px 14px",
      color: text,
      fontSize: "0.8rem",
      lineHeight: 1.6,
      minWidth: 200
    }}>
      <div style={{ fontWeight: 600, marginBottom: 6, fontSize: "0.82rem", color: muted }}>
        {formatChartBucketLabel(String(label))}
      </div>
      {models.length === 0 ? (
        <div style={{ color: muted }}>
          {t("trends.lineTokens")}: {formatCompactCount(Number(p.consumed_tokens) || 0)}
        </div>
      ) : (
        models.map((m, i) => {
          const total = (m.input_tokens ?? 0) + (m.cache_read_tokens ?? 0) + (m.output_tokens ?? 0);
          const cachePct = total > 0
            ? Math.round(((m.cache_read_tokens ?? 0) / total) * 100)
            : 0;
          return (
            <div key={i} style={{ marginBottom: i < models.length - 1 ? 8 : 0 }}>
              <div style={{ fontWeight: 600 }}>
                {m.group_name || t("common.unknown")}: {formatCompactCount(total)}
              </div>
              <div style={{ marginLeft: 8, color: muted, fontSize: "0.76rem" }}>
                <div>
                  Input: {formatCompactCount(m.input_tokens)}
                  {cachePct > 0 ? ` (${cachePct}% ${t("trends.cached")})` : ""}
                </div>
                <div>Output: {formatCompactCount(m.output_tokens)}</div>
              </div>
            </div>
          );
        })
      )}
    </div>
  );
}

export function TrendsPanel({
  points,
  rangeLabel
}: {
  points: MetricPoint[];
  rangeLabel: string;
}) {
  const { t } = useI18n();
  const { resolvedTheme } = useTheme();
  const isLight = resolvedTheme === "light";

  const chartData = useMemo(() => {
    // Group points by bucket_time, collect per-model breakdown
    const byBucket = new Map<string, {
      input_tokens: number;
      output_tokens: number;
      cache_read_tokens: number;
      consumed_tokens: number;
      models: Array<{
        group_name: string;
        input_tokens: number;
        output_tokens: number;
        cache_read_tokens: number;
      }>;
    }>();
    for (const p of points) {
      let entry = byBucket.get(p.bucket_time);
      if (!entry) {
        entry = { input_tokens: 0, output_tokens: 0, cache_read_tokens: 0, consumed_tokens: 0, models: [] };
        byBucket.set(p.bucket_time, entry);
      }
      entry.input_tokens += p.input_tokens;
      entry.output_tokens += p.output_tokens;
      entry.cache_read_tokens += p.cache_read_tokens;
      entry.consumed_tokens += p.consumed_tokens;
      if (p.group_name) {
        entry.models.push({
          group_name: p.group_name,
          input_tokens: p.input_tokens,
          output_tokens: p.output_tokens,
          cache_read_tokens: p.cache_read_tokens,
        });
      }
    }
    return Array.from(byBucket.entries()).map(([bucket_time, v]) => ({
      bucket_time,
      input_tokens: v.input_tokens,
      output_tokens: v.output_tokens,
      cache_read_tokens: v.cache_read_tokens,
      consumed_tokens: v.consumed_tokens,
      _models: v.models,
    }));
  }, [points]);

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
                content={<CustomTooltip isLight={isLight} t={t} />}
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
