pub mod grpc;
pub mod http;
pub mod websockets;

use async_trait::async_trait;
use base64::{Engine, engine::general_purpose};
use clap::ValueEnum;
use reqwest::Version;
use serde::{Deserialize, Serialize};
use std::{error::Error, net::SocketAddr};

#[derive(Debug, Clone, ValueEnum, Serialize, Deserialize)]
pub enum Protocol {
    Http1,
    Http2,
    Http3,
    Grpc,
    Websockets,
}

#[async_trait]
pub trait ApiProtocol {
    async fn fetch(&self, url: &str) -> Result<ApiResponse, Box<dyn std::error::Error>>;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse {
    pub path: String,
    pub protocol: Protocol,
    pub status: Option<u16>,
    pub headers: Option<Vec<(String, String)>>,
    pub body: Option<Vec<u8>>,
    pub metadata: Option<serde_json::Value>,
    pub version: String,
    pub ip: Option<SocketAddr>,
    pub duration: std::time::Duration,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum ResponseBody {
    Text(String),
    Binary(String), // base64 encoded
    Json(serde_json::Value),
    None,
}

impl ApiResponse {
    pub fn display_body(&self) -> ResponseBody {
        match &self.body {
            Some(data) => {
                if let Ok(text) = String::from_utf8(data.clone()) {
                    if text.trim().starts_with('{') || text.trim().starts_with('[') {
                        if let Ok(json) = serde_json::from_str(&text) {
                            return ResponseBody::Json(json);
                        }
                    }
                    return ResponseBody::Text(text);
                }
                ResponseBody::Binary(general_purpose::STANDARD.encode(data))
            }
            None => ResponseBody::None,
        }
    }
}
