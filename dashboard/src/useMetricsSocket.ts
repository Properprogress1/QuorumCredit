import { useEffect, useRef, useState } from "react";
import { ProtocolMetrics } from "./analytics";

interface UseMetricsSocketResult {
  latest: ProtocolMetrics | null;
  connected: boolean;
}

/**
 * Opens a WebSocket to `url` and streams the latest metrics snapshot.
 * Reconnects automatically after `reconnectDelayMs` on unexpected close.
 */
export function useMetricsSocket(
  url: string,
  reconnectDelayMs = 3000
): UseMetricsSocketResult {
  const [latest, setLatest] = useState<ProtocolMetrics | null>(null);
  const [connected, setConnected] = useState(false);
  const wsRef = useRef<WebSocket | null>(null);
  const unmountedRef = useRef(false);

  useEffect(() => {
    unmountedRef.current = false;

    function connect() {
      if (unmountedRef.current) return;
      const ws = new WebSocket(url);
      wsRef.current = ws;

      ws.onopen = () => {
        if (!unmountedRef.current) setConnected(true);
      };

      ws.onmessage = (ev) => {
        try {
          const data: ProtocolMetrics = JSON.parse(ev.data as string);
          if (!unmountedRef.current) setLatest(data);
        } catch {
          // malformed frame — ignore
        }
      };

      ws.onclose = () => {
        if (unmountedRef.current) return;
        setConnected(false);
        setTimeout(connect, reconnectDelayMs);
      };

      ws.onerror = () => ws.close();
    }

    connect();

    return () => {
      unmountedRef.current = true;
      wsRef.current?.close();
    };
  }, [url, reconnectDelayMs]);

  return { latest, connected };
}
