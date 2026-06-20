import { describe, it, expect } from "vitest";
import {
  checkAlerts,
  metricsToCSV,
  DEFAULT_THRESHOLDS,
  ProtocolMetrics,
  AlertThresholds,
} from "../analytics";

const baseMetrics = (): ProtocolMetrics => ({
  tvl: 10_000_000_000,
  active_loans: 5,
  total_loans: 10,
  defaulted_loans: 1,
  default_rate: 0.1,
  total_yield_distributed: 200_000_000,
  slash_count: 1,
  fee_revenue: 50_000,
  top_borrowers: [],
  top_vouchers: [],
  timestamp: 1000,
});

// --- checkAlerts ---

describe("checkAlerts", () => {
  // Test 1
  it("returns no alerts when all within threshold", () => {
    const m = { ...baseMetrics(), default_rate: 0.03, tvl: 9_500_000_000 };
    expect(checkAlerts(m, 10_000_000_000)).toHaveLength(0);
  });

  // Test 2
  it("fires high_default_rate when default_rate > 5%", () => {
    const m = { ...baseMetrics(), default_rate: 0.06 };
    const alerts = checkAlerts(m, 0);
    expect(alerts.some((a) => a.kind === "high_default_rate")).toBe(true);
  });

  // Test 3
  it("does not fire high_default_rate at exactly the threshold", () => {
    const m = { ...baseMetrics(), default_rate: 0.05 };
    const alerts = checkAlerts(m, 0);
    expect(alerts.some((a) => a.kind === "high_default_rate")).toBe(false);
  });

  // Test 4
  it("fires tvl_drop when TVL drops > 10% from peak", () => {
    const m = { ...baseMetrics(), tvl: 8_000_000_000 };
    const alerts = checkAlerts(m, 10_000_000_000);
    expect(alerts.some((a) => a.kind === "tvl_drop")).toBe(true);
  });

  // Test 5
  it("does not fire tvl_drop when peak_tvl is 0", () => {
    const m = { ...baseMetrics(), tvl: 0 };
    const alerts = checkAlerts(m, 0);
    expect(alerts.some((a) => a.kind === "tvl_drop")).toBe(false);
  });

  // Test 6
  it("uses custom thresholds", () => {
    const thresholds: AlertThresholds = { max_default_rate: 0.01, max_tvl_drop_fraction: 0.50 };
    const m = { ...baseMetrics(), default_rate: 0.02 };
    const alerts = checkAlerts(m, 0, thresholds);
    expect(alerts.some((a) => a.kind === "high_default_rate")).toBe(true);
  });

  // Test 7
  it("alert message contains percentage values", () => {
    const m = { ...baseMetrics(), default_rate: 0.07 };
    const alerts = checkAlerts(m, 0);
    const alert = alerts.find((a) => a.kind === "high_default_rate")!;
    expect(alert.message).toContain("7.0%");
    expect(alert.message).toContain("5.0%");
  });

  // Test 8
  it("can fire multiple alerts simultaneously", () => {
    const m = { ...baseMetrics(), default_rate: 0.08, tvl: 5_000_000_000 };
    const alerts = checkAlerts(m, 10_000_000_000);
    expect(alerts.length).toBe(2);
  });
});

// --- metricsToCSV ---

describe("metricsToCSV", () => {
  // Test 9
  it("returns only header when given empty array", () => {
    const csv = metricsToCSV([]);
    const lines = csv.split("\n").filter(Boolean);
    expect(lines).toHaveLength(1);
  });

  // Test 10
  it("CSV header contains required columns", () => {
    const csv = metricsToCSV([]);
    expect(csv).toContain("timestamp");
    expect(csv).toContain("tvl");
    expect(csv).toContain("default_rate");
    expect(csv).toContain("fee_revenue");
  });

  // Test 11
  it("produces one data row per metrics snapshot", () => {
    const csv = metricsToCSV([baseMetrics(), baseMetrics()]);
    const lines = csv.split("\n").filter(Boolean);
    expect(lines).toHaveLength(3); // header + 2 rows
  });

  // Test 12
  it("data row starts with the timestamp value", () => {
    const csv = metricsToCSV([baseMetrics()]);
    const [, dataRow] = csv.split("\n");
    expect(dataRow.startsWith("1000,")).toBe(true);
  });

  // Test 13
  it("default_rate is formatted to 6 decimal places", () => {
    const m = { ...baseMetrics(), default_rate: 0.1 };
    const csv = metricsToCSV([m]);
    expect(csv).toContain("0.100000");
  });

  // Test 14
  it("fee_revenue value appears in the data row", () => {
    const csv = metricsToCSV([baseMetrics()]);
    expect(csv).toContain("50000");
  });
});

// --- default thresholds ---

describe("DEFAULT_THRESHOLDS", () => {
  // Test 15
  it("default max_default_rate is 5%", () => {
    expect(DEFAULT_THRESHOLDS.max_default_rate).toBe(0.05);
  });

  // Test 16
  it("default max_tvl_drop_fraction is 10%", () => {
    expect(DEFAULT_THRESHOLDS.max_tvl_drop_fraction).toBe(0.10);
  });
});

// --- edge cases ---

describe("edge cases", () => {
  // Test 17
  it("default_rate of exactly 0 produces no default alert", () => {
    const m = { ...baseMetrics(), default_rate: 0 };
    expect(checkAlerts(m, 0).some((a) => a.kind === "high_default_rate")).toBe(false);
  });

  // Test 18
  it("metricsToCSV handles large stroops values without scientific notation", () => {
    const m = { ...baseMetrics(), tvl: 1_000_000_000_000 };
    const csv = metricsToCSV([m]);
    expect(csv).toContain("1000000000000");
    expect(csv).not.toContain("e+");
  });
});
