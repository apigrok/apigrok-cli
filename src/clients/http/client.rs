use std::{sync::Arc, time::Duration};

use http_body_util::Empty;
use hyper::{
    Request, Response,
    body::{Bytes, Incoming},
    client::conn,
    header::HeaderMap,
};

use hyper_util::rt::TokioIo;
use tokio::{net::TcpStream, runtime::Runtime, sync::Mutex};

pub trait BlockingHttpClient {
    fn send(&self, req: Request<Empty<Bytes>>) -> Result<Response<Incoming>, hyper::Error>;
}

#[derive(Clone, Debug)]
pub struct ClientConfiguration {
    pub base_url: Option<String>,
    pub timeout: Option<Duration>,
    pub default_headers: Option<HeaderMap>,
}

pub struct BlockingClient {
    sender: Arc<Mutex<conn::http1::SendRequest<Empty<Bytes>>>>,
    rt: Runtime,
    config: ClientConfiguration,
}

impl BlockingClient {
    pub fn new(
        domain: &str,
        port: u16,
        config: ClientConfiguration,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let rt = Runtime::new().expect("Failed to create Tokio runtime");

        let domain = domain.to_string(); // Clone for move into async block

        let sender = rt.block_on(async {
            let addr = format!("{}:{}", domain, port);
            let tcp = TcpStream::connect(&addr).await?;
            let io = TokioIo::new(tcp);

            let (sender, conn) = conn::http1::handshake::<_, Empty<Bytes>>(io).await?;

            tokio::spawn(async move {
                if let Err(err) = conn.await {
                    eprintln!("Connection task failed: {:?}", err);
                }
            });

            Ok::<_, Box<dyn std::error::Error>>(sender)
        })?;

        Ok(Self {
            sender: Arc::new(Mutex::new(sender)),
            rt,
            config,
        })
    }
}

impl BlockingHttpClient for BlockingClient {
    fn send(&self, req: Request<Empty<Bytes>>) -> Result<Response<Incoming>, hyper::Error> {
        let sender = Arc::clone(&self.sender);

        self.rt.block_on(async {
            let mut sender = sender.lock().await;
            sender.send_request(req).await
        })
    }
}
