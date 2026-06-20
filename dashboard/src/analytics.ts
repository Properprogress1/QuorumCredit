export interface ProtocolMetrics {
  tvl: number;
  active_loans: number;
  total_loans: number;
  defaulted_loans: number;
  default_rate: number;
  total_yield_distributed: number;
  slash_count: number;
  fee_revenue: number;
  top_borrowers: [string, number][];
  top_vouchers: [string, number][];
  timestamp: number;
}

export interface Alert {
  kind: string;
  message: string;
}

export interface AlertThresholds {
  max_default_rate: number;
  max_tvl_drop_fraction: number;
}

export interface MetricsFilter {
  from?: number;
  to?: number;
  loan_size?: "small" | "medium" | "large";
}

export const DEFAULT_THRESHOLDS: AlertThresholds = {
  max_default_rate: 0.05,
  max_tvl_drop_fraction: 0.10,
};

/** Check client-side thresholds against a metrics snapshot. */
export function checkAlerts(
  metrics: ProtocolMetrics,
  peakTvl: number,
  thresholds: AlertThresholds = DEFAULT_THRESHOLDS
): Alert[] {
  const alerts: Alert[] = [];

  if (metrics.default_rate > thresholds.max_default_rate) {
    alerts.push({
      kind: "high_default_rate",
      message: `Default rate ${(metrics.default_rate * 100).toFixed(1)}% exceeds threshold ${(thresholds.max_default_rate * 100).toFixed(1)}%`,
    });
  }

  if (peakTvl > 0) {
    const drop = (peakTvl - metrics.tvl) / peakTvl;
    if (drop > thresholds.max_tvl_drop_fraction) {
      alerts.push({
        kind: "tvl_drop",
        message: `TVL dropped ${(drop * 100).toFixed(1)}% from peak, exceeds threshold ${(thresholds.max_tvl_drop_fraction * 100).toFixed(1)}%`,
      });
    }
  }

  return alerts;
}

/** Convert a metrics snapshot to a CSV string. */
export function metricsToCSV(rows: ProtocolMetrics[]): string {
  const header =
    "timestamp,tvl,active_loans,total_loans,defaulted_loans," +
    "default_rate,total_yield_distributed,slash_count,fee_revenue";
  const lines = rows.map(
    (r) =>
      `${r.timestamp},${r.tvl},${r.active_loans},${r.total_loans},` +
      `${r.defaulted_loans},${r.default_rate.toFixed(6)},` +
      `${r.total_yield_distributed},${r.slash_count},${r.fee_revenue}`
  );
  return [header, ...lines].join("\n");
}

/** Trigger a browser download of the given content. */
export function downloadFile(
  content: string,
  filename: string,
  mimeType: string
): void {
  const blob = new Blob([content], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}
