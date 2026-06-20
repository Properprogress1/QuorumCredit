import React from "react";
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, act } from "@testing-library/react";
import AnalyticsDashboard from "../AnalyticsDashboard";
import { ProtocolMetrics } from "../analytics";

// ---------------------------------------------------------------------------
// Mock WebSocket
// ---------------------------------------------------------------------------

interface MockWSInstance {
  onopen: (() => void) | null;
  onmessage: ((ev: { data: string }) => void) | null;
  onclose: (() => void) | null;
  onerror: (() => void) | null;
  close: () => void;
  send: () => void;
}

let mockWSInstance: MockWSInstance | null = null;

class MockWebSocket implements MockWSInstance {
  onopen: (() => void) | null = null;
  onmessage: ((ev: { data: string }) => void) | null = null;
  onclose: (() => void) | null = null;
  onerror: (() => void) | null = null;
  close = vi.fn();
  send = vi.fn();

  constructor(public url: string) {
    mockWSInstance = this;
  }
}

// Mock global fetch
const mockFetch = vi.fn();

const defaultMetrics: ProtocolMetrics = {
  tvl: 5_000_000_000,
  active_loans: 3,
  total_loans: 5,
  defaulted_loans: 1,
  default_rate: 0.2,
  total_yield_distributed: 100_000_000,
  slash_count: 1,
  fee_revenue: 25_000,
  top_borrowers: [["GABC1234", 3_000_000_000]],
  top_vouchers: [["GVOU5678", 1_000_000_000]],
  timestamp: 1000,
};

beforeEach(() => {
  vi.stubGlobal("WebSocket", MockWebSocket);
  vi.stubGlobal("fetch", mockFetch);
  mockFetch.mockResolvedValue({
    ok: true,
    json: async () => ({ metrics: defaultMetrics, alerts: [] }),
  });
});

afterEach(() => {
  vi.restoreAllMocks();
  mockWSInstance = null;
});

function renderDashboard() {
  return render(
    <AnalyticsDashboard
      apiBase="http://localhost:3000"
      wsUrl="ws://localhost:3000/api/admin/metrics/ws"
      token="test-token"
    />
  );
}

// Test 1: Dashboard renders without crashing
it("renders the dashboard heading", () => {
  renderDashboard();
  expect(screen.getByText(/QuorumCredit Admin Dashboard/i)).toBeInTheDocument();
});

// Test 2: Shows "Disconnected" initially
it("shows disconnected status before WS opens", () => {
  renderDashboard();
  expect(screen.getByLabelText(/WebSocket disconnected/i)).toBeInTheDocument();
});

// Test 3: Shows "Live" after WS connects
it("shows live status once WebSocket opens", async () => {
  renderDashboard();
  await act(async () => {
    mockWSInstance?.onopen?.();
  });
  expect(screen.getByLabelText(/WebSocket connected/i)).toBeInTheDocument();
});

// Test 4: KPI cards render when WS pushes a metrics update
it("renders KPI cards when WS pushes metrics", async () => {
  renderDashboard();
  await act(async () => {
    mockWSInstance?.onopen?.();
    mockWSInstance?.onmessage?.({ data: JSON.stringify(defaultMetrics) });
  });
  expect(screen.getByText("TVL (XLM)")).toBeInTheDocument();
  expect(screen.getByText("Active Loans")).toBeInTheDocument();
});

// Test 5: TVL displayed in XLM (not stroops)
it("displays TVL in XLM", async () => {
  renderDashboard();
  await act(async () => {
    mockWSInstance?.onmessage?.({ data: JSON.stringify(defaultMetrics) });
  });
  // 5_000_000_000 stroops = 500.00 XLM
  expect(screen.getByText("500.00")).toBeInTheDocument();
});

// Test 6: Default rate alert renders when rate > threshold
it("shows alert when default rate exceeds threshold", async () => {
  const highDefault = { ...defaultMetrics, default_rate: 0.06 };
  renderDashboard();
  await act(async () => {
    mockWSInstance?.onmessage?.({ data: JSON.stringify(highDefault) });
  });
  expect(screen.getAllByRole("alert").some((el) => /default/i.test(el.textContent ?? ""))).toBe(true);
});

// Test 7: No alert rendered when default rate is within threshold
it("shows no default rate alert when rate is acceptable", async () => {
  const okMetrics = { ...defaultMetrics, default_rate: 0.03 };
  renderDashboard();
  await act(async () => {
    mockWSInstance?.onmessage?.({ data: JSON.stringify(okMetrics) });
  });
  const alerts = screen.queryAllByRole("alert");
  expect(alerts.every((el) => !/default rate/i.test(el.textContent ?? ""))).toBe(true);
});

// Test 8: Malformed WS message does not crash
it("handles malformed WS message gracefully", async () => {
  renderDashboard();
  await act(async () => {
    mockWSInstance?.onmessage?.({ data: "not-json" });
  });
  expect(screen.getByText(/QuorumCredit Admin Dashboard/i)).toBeInTheDocument();
});

// Test 9: Export CSV button is present (disabled with no data)
it("export CSV button is rendered", () => {
  renderDashboard();
  expect(screen.getByText("Export CSV")).toBeInTheDocument();
});

// Test 10: Export JSON button is present
it("export JSON button is rendered", () => {
  renderDashboard();
  expect(screen.getByText("Export JSON")).toBeInTheDocument();
});

// Test 11: Filters section is accessible
it("renders filters section", () => {
  renderDashboard();
  expect(screen.getByLabelText("Filters")).toBeInTheDocument();
});

// Test 12: Alerts section is accessible when no data
it("alerts section not present before data arrives", () => {
  renderDashboard();
  expect(screen.queryByLabelText("Alerts")).not.toBeInTheDocument();
});
