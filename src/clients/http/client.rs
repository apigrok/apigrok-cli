use std::time::Duration;

use hyper::{HeaderMap, Request, Response};
use hyper_util::{
    client::legacy::{Client, connect::HttpConnector},
    rt::TokioExecutor,
};
use tokio::runtime::Runtime;

pub trait BlockingHttpClient {
    fn send(&self, req: Request<Body>) -> Result<Response<Body>, hyper::Error>;
}

pub struct Body {}

#[derive(Clone, Debug)]
pub struct ClientConfiguration {
    pub base_url: Option<String>,
    pub timeout: Option<Duration>,
    pub default_headers: Option<HeaderMap>,
}

pub struct BlockingClient {
    client: Client<HttpConnector, Body>,
    rt: Runtime,
}

// impl BlockingClient {
//     pub fn new() -> Self {
//         let rt = Runtime::new().expect("Failed to create Tokio runtime");

//         let connector = HttpConnector::new();
//         let client = Client::builder(TokioExecutor::new()).build::<_, Body>(connector);

//         Self { client, rt }
//     }
// }

// impl BlockingHttpClient for BlockingClient {
//     fn send(&self, request: Request<Body>) -> Result<Response<Body>, hyper::Error> {
//         return self.rt.block_on(self.client.request(request));
//     }
// }
