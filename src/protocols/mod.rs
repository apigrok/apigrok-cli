pub mod auto;
pub mod http1;
pub mod http2;
pub mod http3;
pub mod websocket;
pub mod grpc;

use crate::error::Result;
use crate::request::Request;
use crate::response::Response;

#[async_trait::async_trait]
pub trait Protocol {
    async fn execute(&self, request: &Request) -> Result<Response>;
}
