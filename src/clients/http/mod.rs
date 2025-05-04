use std::time::Duration;

use hyper::HeaderMap;

pub mod async_client;
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
