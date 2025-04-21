use std::time::Duration;

use http_body_util::Empty;
use hyper::{
    HeaderMap, Request, Response,
    body::{Bytes, Incoming},
};

pub trait BlockingHttpClient {
    fn send(&self, req: Request<Empty<Bytes>>) -> Result<Response<Incoming>, hyper::Error>;
}

pub trait AsyncHttpClient {
    async fn send(&self, req: Request<Empty<Bytes>>) -> Result<Response<Incoming>, hyper::Error>;
}

#[derive(Clone, Debug)]
pub struct ClientConfiguration {
    pub base_url: Option<String>,
    pub timeout: Option<Duration>,
    pub default_headers: Option<HeaderMap>,
}
