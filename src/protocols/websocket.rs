use crate::error::{ApiGrokError, Result};
use crate::protocols::Protocol;
use crate::request::Request;
use crate::response::Response;
use futures::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use std::collections::HashMap;
use std::time::Instant;

pub struct WebSocketClient;

impl WebSocketClient {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl Protocol for WebSocketClient {
    async fn execute(&self, request: &Request) -> Result<Response> {
        let start = Instant::now();

        // Convert http/https to ws/wss
        let ws_url = request.url
            .replace("http://", "ws://")
            .replace("https://", "wss://");

        // Connect to WebSocket
        let (mut ws_stream, _) = connect_async(&ws_url)
            .await
            .map_err(|e| ApiGrokError::NetworkError(format!("WebSocket connection failed: {}", e)))?;

        // Send message if body is present
        if let Some(body) = &request.body {
            let message = String::from_utf8_lossy(body).to_string();
            ws_stream
                .send(Message::Text(message))
                .await
                .map_err(|e| ApiGrokError::NetworkError(format!("Failed to send message: {}", e)))?;
        }

        // Read response
        let mut responses = Vec::new();
        let timeout = tokio::time::Duration::from_secs(request.timeout);

        match tokio::time::timeout(timeout, ws_stream.next()).await {
            Ok(Some(Ok(msg))) => {
                responses.push(msg.to_string());
            }
            Ok(Some(Err(e))) => {
                return Err(ApiGrokError::NetworkError(format!("WebSocket error: {}", e)));
            }
            Ok(None) => {}
            Err(_) => {
                return Err(ApiGrokError::TimeoutError);
            }
        }

        let elapsed = start.elapsed().as_millis();

        // Close the connection gracefully
        let _ = ws_stream.close(None).await;

        let body = if responses.is_empty() {
            b"WebSocket connection established and closed successfully".to_vec()
        } else {
            responses.join("\n").into_bytes()
        };

        Ok(Response::new(
            101,
            "Switching Protocols".to_string(),
            HashMap::new(),
            body,
            "WebSocket".to_string(),
            elapsed,
        ))
    }
}
