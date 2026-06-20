use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Broadcast channel capacity; old messages are dropped when full.
const CHANNEL_CAPACITY: usize = 128;

#[derive(Clone)]
pub struct MetricsBroadcaster {
    tx: broadcast::Sender<String>,
}

impl MetricsBroadcaster {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(CHANNEL_CAPACITY);
        Self { tx }
    }

    /// Publish a metrics JSON payload to all connected WebSocket clients.
    pub fn publish(&self, payload: Value) {
        // Ignore send errors — no subscribers is fine.
        let _ = self.tx.send(payload.to_string());
    }

    /// Subscribe to the metrics broadcast channel.
    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.tx.subscribe()
    }
}

/// WebSocket upgrade handler. Each connected client receives broadcast metrics
/// pushes in real time. Client-to-server messages are ignored.
pub async fn ws_handler(socket: WebSocket, broadcaster: Arc<MetricsBroadcaster>) {
    let (mut sink, mut stream) = socket.split();
    let mut rx = broadcaster.subscribe();

    // Forward broadcasts to the WebSocket client.
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sink.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // Drain inbound messages (ping/pong/close) so the connection is handled cleanly.
    while let Some(msg) = stream.next().await {
        if let Ok(Message::Close(_)) = msg {
            break;
        }
    }

    send_task.abort();
}
