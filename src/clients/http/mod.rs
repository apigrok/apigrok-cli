use std::time::Duration;

use hyper::HeaderMap;

pub mod client;
mod request;
mod response;

#[allow(unused)]
#[derive(Clone, Debug)]
pub struct ClientConfiguration {
    pub timeout: Duration,
    pub base_url: url::Url,
    pub port: u16,
    pub headers: Option<HeaderMap>,
}
