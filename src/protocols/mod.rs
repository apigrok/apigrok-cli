pub mod api_response;
pub mod grpc;
pub mod http;
pub mod websockets;

use async_trait::async_trait;
use clap::ValueEnum;
use hyper::Method;
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
    // TODO: refactor to a builder pattern
    async fn execute(
        &self,
        method: Method,
        url: &str,
        h2c: bool,
    ) -> Result<(ApiRequest, ApiResponse), Box<dyn Error>>;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse {
    pub path: String,
    pub protocol: Protocol,
    pub status: Option<u16>,
    pub headers: Option<Vec<(String, String)>>,
    pub body: Option<Vec<u8>>,
    pub version: String,
    pub ip: Option<SocketAddr>,
    pub duration: std::time::Duration,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiRequest {
    pub headers: Option<Vec<(String, String)>>,
    pub method: String,
    pub path: String,
    pub version: String,
}
