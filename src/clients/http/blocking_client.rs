use std::{sync::Arc, time::Duration};

use http_body_util::Empty;
use hyper::{
    Request, Response,
    body::{Bytes, Incoming},
    client::conn,
    header::{self, HeaderMap},
};

use hyper_util::rt::TokioIo;
use tokio::{net::TcpStream, runtime::Runtime, sync::Mutex};
use url::Url;

use crate::protocols::ApiRequest;

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
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let rt = Runtime::new().expect("Failed to create Tokio runtime");

        let sender = rt.block_on(async {
            let tcp = TcpStream::connect((domain, port)).await?;
            let io = TokioIo::new(tcp);

            let (sender, conn) = conn::http1::handshake::<_, Empty<Bytes>>(io).await?;

            tokio::spawn(async move {
                if let Err(err) = conn.await {
                    eprintln!("Connection task failed: {:?}", err);
                }
            });

            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(sender)
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

#[test]
fn test_blocking_client() -> Result<(), Box<dyn std::error::Error>> {
    let parsed_url = Url::parse("http://myrstack.tech")?;
    let scheme = parsed_url.scheme();
    let host = parsed_url.host_str().ok_or("Invalid host")?;
    let domain = parsed_url.domain().ok_or("Invalid domain")?.to_string();
    let port = parsed_url
        .port_or_known_default()
        .unwrap_or_else(|| if scheme == "https" { 443 } else { 80 });

    let config = ClientConfiguration {
        base_url: Some(parsed_url.to_string()),
        timeout: Some(Duration::from_secs(10)),
        default_headers: Some(HeaderMap::new()),
    };

    let client = BlockingClient::new(&domain, port, config).unwrap();
    let req: Request<Empty<Bytes>> = Request::builder()
        .uri(parsed_url.as_str())
        .header(header::HOST, host)
        .header(hyper::header::USER_AGENT, "apigrok/0.1.0")
        .header(hyper::header::ACCEPT, "*/*")
        .header(hyper::header::ACCEPT_ENCODING, "gzip")
        .method(hyper::Method::GET)
        .body(Empty::<Bytes>::new())?;

    let res: Response<Incoming> = client.send(req).unwrap();
    assert_eq!(res.status(), 301);

    Ok(())
}
