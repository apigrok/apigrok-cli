pub mod grpc;
pub mod http;
pub mod websockets;

use async_trait::async_trait;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};

#[derive(Debug, Clone, ValueEnum, Serialize, Deserialize)]
pub enum Protocol {
    Http1,
    Http2,
    Http3,
    Grpc,
    Websockets,
}

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Protocol::Http1 => write!(f, "HTTP/1.1"),
            Protocol::Http2 => write!(f, "HTTP/2"),
            Protocol::Http3 => write!(f, "HTTP/3"),
            Protocol::Grpc => write!(f, "gRPC"),
            Protocol::Websockets => write!(f, "WebSocket"),
        }
    }
}

#[async_trait]
pub trait ApiProtocol {
    async fn fetch(&self, url: &str) -> Result<ApiResponse, Box<dyn std::error::Error>>;
    async fn analyze(
        &self,
        response: &ApiResponse,
    ) -> Result<AnalysisResult, Box<dyn std::error::Error>>;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse {
    pub protocol: Protocol,
    pub status: Option<u16>,
    pub headers: Option<Vec<(String, String)>>,
    pub body: Option<Vec<u8>>,
    pub metadata: Option<serde_json::Value>,
    pub duration: std::time::Duration,
}

pub struct AnalysisResult {}
