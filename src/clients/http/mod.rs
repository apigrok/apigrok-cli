use std::time::Duration;

use http_body_util::Empty;
use hyper::{
    HeaderMap, Request, Response,
    body::{Bytes, Incoming},
};

mod async_client;
mod blocking_client;
mod request;
mod response;

#[derive(Clone, Debug)]
pub struct ClientConfiguration {
    pub timeout: Duration,
    pub base_url: url::Url,
    pub port: u16,
    pub headers: Option<HeaderMap>,
}
