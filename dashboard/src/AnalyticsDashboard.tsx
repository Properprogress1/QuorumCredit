import React, { useCallback, useEffect, useRef, useState } from "react";
import {
  LineChart,
  Line,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Legend,
} from "recharts";
import {
  Alert,
  AlertThresholds,
  DEFAULT_THRESHOLDS,
  MetricsFilter,
  ProtocolMetrics,
  checkAlerts,
  downloadFile,
  metricsToCSV,
} from "./analytics";
import { useMetricsSocket } from "./useMetricsSocket";

const XLM = 10_000_000;
const fmt = (stroops: number) => (stroops / XLM).toFixed(2);

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

interface AnalyticsDashboardProps {
  apiBase: string;
  wsUrl: string;
  token: string;
  thresholds?: AlertThresholds;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

const AnalyticsDashboard: React.FC<AnalyticsDashboardProps> = ({
  apiBase,
  wsUrl,
  token,
  thresholds = DEFAULT_THRESHOLDS,
}) => {
  const [history, setHistory] = useState<ProtocolMetrics[]>([]);
  const [alerts, setAlerts] = useState<Alert[]>([]);
  const [filter, setFilter] = useState<MetricsFilter>({});
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const peakTvlRef = useRef(0);

  const { latest, connected } = useMetricsSocket(wsUrl);

  // Apply incoming WS snapshot
  useEffect(() => {
    if (!latest) return;
    if (latest.tvl > peakTvlRef.current) peakTvlRef.current = latest.tvl;
    setHistory((h) => [...h.slice(-99), latest]);
    setAlerts(checkAlerts(latest, peakTvlRef.current, thresholds));
  }, [latest, thresholds]);

  // Fetch on demand / filter change
  const fetchMetrics = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch(`${apiBase}/api/admin/metrics`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify({
          loans: [],
          vouches: [],
          slash_count: 0,
          fee_revenue: 0,
          filter,
        }),
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const { metrics }: { metrics: ProtocolMetrics; alerts: Alert[] } =
        await res.json();
      if (metrics.tvl > peakTvlRef.current) peakTvlRef.current = metrics.tvl;
      setHistory((h) => [...h.slice(-99), metrics]);
      setAlerts(checkAlerts(metrics, peakTvlRef.current, thresholds));
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Unknown error");
    } finally {
      setLoading(false);
    }
  }, [apiBase, token, filter, thresholds]);

  const handleExportCSV = () => {
    downloadFile(metricsToCSV(history), "metrics.csv", "text/csv");
  };

  const handleExportJSON = () => {
    downloadFile(JSON.stringify(history, null, 2), "metrics.json", "application/json");
  };

  const current = history.at(-1);

  return (
    <div style={{ fontFamily: "system-ui", padding: 24, maxWidth: 1200, margin: "0 auto" }}>
      {/* Header */}
      <header style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 24 }}>
        <h1 style={{ margin: 0 }}>QuorumCredit Admin Dashboard</h1>
        <span
          aria-label={connected ? "WebSocket connected" : "WebSocket disconnected"}
          style={{
            width: 10,
            height: 10,
            borderRadius: "50%",
            background: connected ? "#22c55e" : "#ef4444",
            display: "inline-block",
          }}
        />
        <span style={{ fontSize: 12, color: "#6b7280" }}>
          {connected ? "Live" : "Disconnected"}
        </span>
      </header>

      {/* Alerts */}
      {alerts.length > 0 && (
        <section aria-label="Alerts" style={{ marginBottom: 16 }}>
          {alerts.map((a) => (
            <div
              key={a.kind}
              role="alert"
              style={{
                background: "#fef2f2",
                border: "1px solid #fca5a5",
                borderRadius: 6,
                padding: "8px 12px",
                marginBottom: 8,
                color: "#991b1b",
              }}
            >
              ⚠ {a.message}
            </div>
          ))}
        </section>
      )}

      {/* Filters */}
      <section aria-label="Filters" style={{ display: "flex", gap: 12, marginBottom: 24, flexWrap: "wrap" }}>
        <label>
          From:{" "}
          <input
            type="date"
            onChange={(e) =>
              setFilter((f) => ({
                ...f,
                from: e.target.value
                  ? Math.floor(new Date(e.target.value).getTime() / 1000)
                  : undefined,
              }))
            }
          />
        </label>
        <label>
          To:{" "}
          <input
            type="date"
            onChange={(e) =>
              setFilter((f) => ({
                ...f,
                to: e.target.value
                  ? Math.floor(new Date(e.target.value).getTime() / 1000)
                  : undefined,
              }))
            }
          />
        </label>
        <label>
          Loan size:{" "}
          <select
            onChange={(e) =>
              setFilter((f) => ({
                ...f,
                loan_size: (e.target.value as MetricsFilter["loan_size"]) || undefined,
              }))
            }
          >
            <option value="">All</option>
            <option value="small">Small (&lt;0.1 XLM)</option>
            <option value="medium">Medium (0.1–10 XLM)</option>
            <option value="large">Large (&gt;10 XLM)</option>
          </select>
        </label>
        <button onClick={fetchMetrics} disabled={loading}>
          {loading ? "Loading…" : "Fetch"}
        </button>
      </section>

      {error && (
        <p role="alert" style={{ color: "#dc2626" }}>
          Error: {error}
        </p>
      )}

      {/* KPI Cards */}
      {current && (
        <section
          aria-label="KPI metrics"
          style={{
            display: "grid",
            gridTemplateColumns: "repeat(auto-fill, minmax(180px, 1fr))",
            gap: 12,
            marginBottom: 32,
          }}
        >
          <KpiCard label="TVL (XLM)" value={fmt(current.tvl)} />
          <KpiCard label="Active Loans" value={current.active_loans} />
          <KpiCard label="Total Loans" value={current.total_loans} />
          <KpiCard
            label="Default Rate"
            value={`${(current.default_rate * 100).toFixed(1)}%`}
            highlight={current.default_rate > thresholds.max_default_rate}
          />
          <KpiCard label="Yield Distributed (XLM)" value={fmt(current.total_yield_distributed)} />
          <KpiCard label="Slash Events" value={current.slash_count} />
          <KpiCard label="Fee Revenue (XLM)" value={fmt(current.fee_revenue)} />
        </section>
      )}

      {/* TVL over time chart */}
      {history.length > 1 && (
        <section style={{ marginBottom: 32 }}>
          <h2>TVL Over Time</h2>
          <ResponsiveContainer width="100%" height={240}>
            <LineChart data={history} aria-label="TVL chart">
              <CartesianGrid strokeDasharray="3 3" />
              <XAxis dataKey="timestamp" hide />
              <YAxis tickFormatter={(v: number) => fmt(v)} />
              <Tooltip formatter={(v: number) => `${fmt(v)} XLM`} />
              <Legend />
              <Line type="monotone" dataKey="tvl" name="TVL" stroke="#6366f1" dot={false} />
            </LineChart>
          </ResponsiveContainer>
        </section>
      )}

      {/* Default rate chart */}
      {history.length > 1 && (
        <section style={{ marginBottom: 32 }}>
          <h2>Default Rate Over Time</h2>
          <ResponsiveContainer width="100%" height={200}>
            <LineChart data={history} aria-label="Default rate chart">
              <CartesianGrid strokeDasharray="3 3" />
              <XAxis dataKey="timestamp" hide />
              <YAxis tickFormatter={(v: number) => `${(v * 100).toFixed(1)}%`} />
              <Tooltip formatter={(v: number) => `${(v * 100).toFixed(1)}%`} />
              <Line type="monotone" dataKey="default_rate" name="Default Rate" stroke="#ef4444" dot={false} />
            </LineChart>
          </ResponsiveContainer>
        </section>
      )}

      {/* Top borrowers */}
      {current && current.top_borrowers.length > 0 && (
        <section style={{ marginBottom: 32 }}>
          <h2>Top Borrowers</h2>
          <ResponsiveContainer width="100%" height={220}>
            <BarChart
              data={current.top_borrowers.map(([addr, amt]) => ({
                addr: addr.slice(0, 8) + "…",
                amount: amt / XLM,
              }))}
              aria-label="Top borrowers chart"
            >
              <CartesianGrid strokeDasharray="3 3" />
              <XAxis dataKey="addr" />
              <YAxis />
              <Tooltip formatter={(v: number) => `${v.toFixed(2)} XLM`} />
              <Bar dataKey="amount" name="Borrowed (XLM)" fill="#6366f1" />
            </BarChart>
          </ResponsiveContainer>
        </section>
      )}

      {/* Top vouchers */}
      {current && current.top_vouchers.length > 0 && (
        <section style={{ marginBottom: 32 }}>
          <h2>Top Vouchers</h2>
          <ResponsiveContainer width="100%" height={220}>
            <BarChart
              data={current.top_vouchers.map(([addr, stake]) => ({
                addr: addr.slice(0, 8) + "…",
                stake: stake / XLM,
              }))}
              aria-label="Top vouchers chart"
            >
              <CartesianGrid strokeDasharray="3 3" />
              <XAxis dataKey="addr" />
              <YAxis />
              <Tooltip formatter={(v: number) => `${v.toFixed(2)} XLM`} />
              <Bar dataKey="stake" name="Staked (XLM)" fill="#22c55e" />
            </BarChart>
          </ResponsiveContainer>
        </section>
      )}

      {/* Export */}
      <section aria-label="Export" style={{ display: "flex", gap: 8 }}>
        <button onClick={handleExportCSV} disabled={history.length === 0}>
          Export CSV
        </button>
        <button onClick={handleExportJSON} disabled={history.length === 0}>
          Export JSON
        </button>
      </section>
    </div>
  );
};

// ---------------------------------------------------------------------------
// KPI card sub-component
// ---------------------------------------------------------------------------

const KpiCard: React.FC<{
  label: string;
  value: string | number;
  highlight?: boolean;
}> = ({ label, value, highlight }) => (
  <div
    style={{
      padding: "12px 16px",
      background: highlight ? "#fef2f2" : "#f8fafc",
      border: `1px solid ${highlight ? "#fca5a5" : "#e2e8f0"}`,
      borderRadius: 8,
    }}
  >
    <div style={{ fontSize: 12, color: "#6b7280", marginBottom: 4 }}>{label}</div>
    <div style={{ fontSize: 20, fontWeight: 700, color: highlight ? "#dc2626" : "#0f172a" }}>
      {value}
    </div>
  </div>
);

export default AnalyticsDashboard;
