import { useMemo } from "react";
import type { MetricPoint } from "../../types";
import { formatChartBucketLabel } from "../../lib/formatTime";
import { formatCompactCount } from "../../lib/formatNumber";
import { useTheme } from "../../theme/ThemeContext";
import { useI18n } from "../../i18n";
import {
  CartesianGrid,
  Line,
  LineChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis
} from "recharts";

type ModelBreakdown = {
  group_name: string;
  provider_name: string;
  model_name: string;
  input_tokens: number;
  output_tokens: number;
  cache_read_tokens: number;
};

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
  const models = (p._models as ModelBreakdown[]) ?? [];
  const consumedTotal = Number(p.consumed_tokens) || 0;

  const bg = isLight ? "#ffffff" : "#0f172a";
  const border = isLight ? "#cbd5e1" : "#334155";
  const text = isLight ? "#0f172a" : "#f8fafc";
  const muted = isLight ? "#64748b" : "#94a3b8";
  const accent = isLight ? "#0891b2" : "#22d3ee";

  // Group by group_name → provider_name → model_name
  const hierarchy = new Map<string, Map<string, Map<string, ModelBreakdown>>>();
  for (const m of models) {
    const gn = m.group_name?.trim() || t("common.unknown");
    const pn = m.provider_name?.trim() || t("common.unknown");
    const mn = m.model_name?.trim() || t("common.unknown");
    if (!hierarchy.has(gn)) hierarchy.set(gn, new Map());
    if (!hierarchy.get(gn)!.has(pn)) hierarchy.get(gn)!.set(pn, new Map());
    const existing = hierarchy.get(gn)!.get(pn)!.get(mn);
    if (existing) {
      existing.input_tokens += m.input_tokens;
      existing.output_tokens += m.output_tokens;
      existing.cache_read_tokens += m.cache_read_tokens;
    } else {
      hierarchy.get(gn)!.get(pn)!.set(mn, { ...m });
    }
  }

  return (
    <div style={{
      background: bg,
      border: `1px solid ${border}`,
      borderRadius: 8,
      padding: "10px 14px",
      color: text,
      fontSize: "0.8rem",
      lineHeight: 1.6,
      minWidth: 240,
      maxWidth: 360
    }}>
      <div style={{ fontWeight: 600, marginBottom: 6, fontSize: "0.82rem", color: muted }}>
        {formatChartBucketLabel(String(label))}
      </div>
      {models.length === 0 ? (
        <div style={{ color: muted }}>
          {t("trends.lineTokens")}: {formatCompactCount(consumedTotal)}
        </div>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          {Array.from(hierarchy.entries()).map(([groupName, providers]) => (
            <div key={groupName}>
              <div style={{ fontWeight: 600, color: accent, fontSize: "0.78rem" }}>
                {groupName}
              </div>
              {Array.from(providers.entries()).map(([providerName, models]) => (
                <div key={`${groupName}/${providerName}`} style={{ marginLeft: 8 }}>
                  <div style={{ fontWeight: 500, color: muted, fontSize: "0.73rem", marginTop: 2 }}>
                    {providerName}
                  </div>
                  {Array.from(models.entries()).map(([modelName, m]) => {
                    const cachePct = (m.input_tokens + m.cache_read_tokens) > 0
                      ? Math.round((m.cache_read_tokens / (m.input_tokens + m.cache_read_tokens)) * 100)
                      : 0;
                    return (
                      <div key={`${groupName}/${providerName}/${modelName}`} style={{ marginLeft: 12, marginTop: 3, fontSize: "0.72rem" }}>
                        <div style={{ color: text }}>
                          {modelName}: {formatCompactCount(m.input_tokens + m.cache_read_tokens + m.output_tokens)}
                        </div>
                        <div style={{ color: muted }}>
                          Input: {formatCompactCount(m.input_tokens)}
                          {m.cache_read_tokens > 0 ? ` + ${formatCompactCount(m.cache_read_tokens)}` : ""}
                          {cachePct > 0 ? ` (${cachePct}% ${t("trends.cached")})` : ""}
                          {" "}· Output: {formatCompactCount(m.output_tokens)}
                        </div>
                      </div>
                    );
                  })}
                </div>
              ))}
            </div>
          ))}
        </div>
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
    const byBucket = new Map<string, {
      consumed_tokens: number;
      models: ModelBreakdown[];
    }>();
    for (const p of points) {
      let entry = byBucket.get(p.bucket_time);
      if (!entry) {
        entry = { consumed_tokens: 0, models: [] };
        byBucket.set(p.bucket_time, entry);
      }
      entry.consumed_tokens += p.consumed_tokens;
      entry.models.push({
        group_name: p.group_name,
        provider_name: p.provider_name,
        model_name: p.model_name,
        input_tokens: p.input_tokens,
        output_tokens: p.output_tokens,
        cache_read_tokens: p.cache_read_tokens,
      });
    }
    return Array.from(byBucket.entries()).map(([bucket_time, v]) => ({
      bucket_time,
      consumed_tokens: v.consumed_tokens,
      _models: v.models,
    }));
  }, [points]);

  const axisProps = useMemo(
    () => ({
      stroke: isLight ? "#94a3b8" : "#475569",
      tick: { fill: isLight ? "#64748b" : "#94a3b8", fontSize: 10 },
      tickLine: { stroke: isLight ? "#94a3b8" : "#475569" }
    }),
    [isLight]
  );

  const gridStroke = isLight ? "#cbd5e1" : "#334155";
  const axisTickFormatter = (v: string | number) => formatChartBucketLabel(String(v));

  const hasData = chartData.length > 0;

  return (
    <div className="chart-box chart-box--usage">
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-end", flexWrap: "wrap", gap: "12px" }}>
        <h3 className="usage-chart-title" style={{ margin: 0 }}>
          {t("trends.titlePrefix")}
          {rangeLabel}
          {t("trends.titleSuffix")}
        </h3>
      </div>

      <div className="recharts-host" style={{ marginTop: '24px', minHeight: 320 }}>
        <ResponsiveContainer width="100%" height={320}>
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
              tickFormatter={(v) => Number(v) === 0 ? "0" : Number(v) < 1000 ? String(v) : formatCompactCount(Number(v))}
              {...axisProps}
            />
            <Tooltip
              content={<CustomTooltip isLight={isLight} t={t} />}
            />
            {hasData && (
              <Line
                type="monotone"
                name={t("trends.lineTokens")}
                dataKey="consumed_tokens"
                stroke="#c084fc"
                dot={false}
                strokeWidth={2}
                fill="#c084fc"
                fillOpacity={0.2}
              />
            )}
          </LineChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}
