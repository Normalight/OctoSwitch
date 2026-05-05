import type { ReactNode } from "react";
import { useMemo } from "react";
import type { MetricKpi } from "../../types";
import { formatCompactCount } from "../../lib/formatNumber";
import { useI18n } from "../../i18n";

function fmt(n: unknown, digits: number, suffix = ""): string {
  const x = typeof n === "number" ? n : Number(n);
  if (!Number.isFinite(x)) return "—";
  return `${x.toFixed(digits)}${suffix}`;
}

function totalTokensInRange(k: MetricKpi): number {
  return (
    (k.total_input_tokens ?? 0) +
    (k.total_cache_read_tokens ?? 0) +
    (k.total_output_tokens ?? 0)
  );
}

type CardDef = {
  key: string;
  label: ReactNode;
  value: (k: MetricKpi) => string;
  lead?: boolean;
};

export function KpiCards({ kpi }: { kpi: MetricKpi | null }) {
  const { t } = useI18n();

  const cardDefs = useMemo<CardDef[]>(
    () => [
      { key: "ttot", label: t("kpi.totalTokens"), value: (k) => formatCompactCount(totalTokensInRange(k)), lead: true },
      { key: "err", label: t("kpi.err"), value: (k) => fmt((k.error_rate ?? 0) * 100, 2) },
      { key: "tin", label: t("kpi.tin"), value: (k) => formatCompactCount(k.total_input_tokens ?? 0) },
      { key: "tout", label: t("kpi.tout"), value: (k) => formatCompactCount(k.total_output_tokens ?? 0) },
      { key: "cr", label: t("kpi.cacheRead"), value: (k) => formatCompactCount(k.total_cache_read_tokens ?? 0) }
    ],
    [t]
  );

  return (
    <div className="kpi-grid" aria-busy={!kpi}>
      {cardDefs.map((c) => (
        <div key={c.key} className={`card kpi-card${c.lead ? " kpi-card--lead" : ""}`}>
          <span className="kpi-card__label">{c.label}</span>
          <span className="kpi-card__value">{kpi ? c.value(kpi) : t("common.dash")}</span>
        </div>
      ))}
    </div>
  );
}
