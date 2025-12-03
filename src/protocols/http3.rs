use crate::error::{ApiGrokError, Result};
use crate::protocols::Protocol;
use crate::request::Request;
use crate::response::Response;

pub struct Http3Client;

impl Http3Client {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl Protocol for Http3Client {
    async fn execute(&self, _request: &Request) -> Result<Response> {
        // HTTP/3 implementation using quinn + h3
        // This is a placeholder - full implementation would require:
        // 1. QUIC connection setup with quinn
        // 2. H3 request/response handling
        // 3. Proper certificate handling

        Err(ApiGrokError::ProtocolError(
            "HTTP/3 support is not yet implemented. Coming soon!".to_string()
        ))
    }
}
