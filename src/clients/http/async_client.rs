use std::{sync::Arc, time::Duration};

use http_body_util::Empty;
use hyper::{
    HeaderMap,
    body::{Bytes, Incoming},
    client::conn,
    header::{HeaderName, HeaderValue},
};

use hyper_tls::HttpsConnector;
use hyper_util::rt::{TokioExecutor, TokioIo};
use rustls::{
    ClientConfig, RootCertStore,
    pki_types::{DnsName, ServerName},
};
use rustls_native_certs::load_native_certs;
use tokio::{net::TcpStream, sync::Mutex};
use tokio_rustls::{TlsConnector, client::TlsStream};
use url::Url;

use crate::clients::http::ClientConfiguration;

use super::{request::Request, response::Response};

pub trait AsyncHttpClient {
    async fn send(&self, req: Request) -> Result<Response, hyper::Error>;
}

pub enum SendRequestClient {
    Http1(conn::http1::SendRequest<Empty<Bytes>>),
    Http2(conn::http2::SendRequest<Empty<Bytes>>),
}

pub struct Client {
    sender: Arc<Mutex<SendRequestClient>>,
    config: ClientConfiguration,
}

pub struct ClientBuilder {
    http1_only: bool,
    base_url: Option<Url>,
    port: u16,
    timeout: Duration,
    headers: HeaderMap,
}

impl Client {
    pub fn builder() -> ClientBuilder {
        ClientBuilder {
            http1_only: false, // default
            base_url: None,
            port: 80,
            timeout: Duration::from_secs(10),
            headers: HeaderMap::new(),
        }
    }

    pub fn get(&self, path: &str) -> RequestBuilder {
        let full_url = join_base_and_path(self.config.base_url.as_str(), path);

        RequestBuilder {
            url: full_url,
            method: hyper::Method::GET,
        }
    }

    pub async fn execute(&self, request: Request) -> Result<Response, Box<dyn std::error::Error>> {
        let method = request.method.clone();
        let url = request.url.clone();

        let http_req = build_http_request(request, self.config.clone())?; // map your internal Request to hyper::Request

        let mut sender = self.sender.lock().await;

        let hyper_resp = match &mut *sender {
            SendRequestClient::Http1(s) => s.send_request(http_req).await?,
            SendRequestClient::Http2(s) => s.send_request(http_req).await?,
        };
        let resp = build_http_response(hyper_resp)?;

        println!("Executing {:?} {:?}", method, url);

        Ok(resp)
    }
}

fn join_base_and_path(base: &str, path: &str) -> String {
    format!(
        "{}/{}",
        base.trim_end_matches('/'),
        path.trim_start_matches('/')
    )
}

fn build_http_request(
    request: Request,
    config: ClientConfiguration,
) -> Result<hyper::Request<Empty<Bytes>>, Box<dyn std::error::Error>> {
    let original_headers = request.headers;

    let mut builder = hyper::Request::builder()
        .uri(&request.url)
        .method(&request.method);

    if let Some(headers) = original_headers {
        for (key, value) in headers.iter() {
            builder = builder.header(key, value.clone());
        }
    }

    // add default agent header
    let default_user_agent = format!("apigrok/{}", env!("CARGO_PKG_VERSION"));
    // add default host header
    let host = config.base_url.host_str().expect("URL must have a host");
    let default_host = format!("{}:{}", host, config.port);
    builder = builder
        .header(hyper::header::USER_AGENT, default_user_agent.clone())
        .header(hyper::header::HOST, default_host);

    Ok(builder.body(Empty::new())?)
}

fn build_http_response(
    res: hyper::Response<Incoming>,
) -> Result<Response, Box<dyn std::error::Error>> {
    let builder = Response {
        status: res.status(),
    };

    Ok(builder)
}

pub struct RequestBuilder {
    url: String,
    method: hyper::Method,
}

impl RequestBuilder {
    pub fn build(self) -> Result<Request, Box<dyn std::error::Error>> {
        Ok(Request {
            url: self.url,
            method: self.method,
            headers: None,
        })
    }
}

impl ClientBuilder {
    pub fn http1_only(mut self) -> Self {
        self.http1_only = true;
        self
    }
    pub fn base_url(mut self, url: impl Into<url::Url>) -> Self {
        self.base_url = Some(url.into());
        self
    }
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
    pub fn header(mut self, key: impl Into<HeaderName>, value: impl Into<HeaderValue>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    pub async fn build(self) -> Result<Client, Box<dyn std::error::Error>> {
        let base_url = self.base_url.ok_or("Missing base_url")?;
        let port = self.port;
        let host = base_url.host_str().ok_or("Invalid host")?.to_string();
        let domain = base_url.domain().ok_or("Invalid domain")?.to_string();
        let scheme = base_url.scheme().to_string();
        let addr = format!("{}:{}", host, port);

        // G: Check Alt-Svc/DNS for HTTP/3 (stubbed)
        let http3_supported = false;
        if http3_supported {
            let quic_success = false;
            if quic_success {
                println!("[I] Use HTTP/3");
                unimplemented!("HTTP/3 not implemented in this example");
            }
        }

        let sender = match scheme.as_str() {
            "https" => {
                // HTTPS with ALPN negotiation
                let mut root_store = rustls::RootCertStore::empty();
                for cert in load_native_certs().expect("Could not load platform certificates") {
                    root_store.add(cert)?;
                }

                let mut tls_config =
                    ClientConfig::builder_with_protocol_versions(&[&rustls::version::TLS13])
                        .with_root_certificates(root_store)
                        .with_no_client_auth();
                // Configure ALPN protocols (order matters!)
                tls_config.alpn_protocols = vec![
                    b"h2".to_vec(),
                    b"http/1.3".to_vec(),
                    b"http/1.2".to_vec(),
                    b"http/1.1".to_vec(),
                ]; // Prefer HTTP/2

                let connector = TlsConnector::from(Arc::new(tls_config));
                let tcp = TcpStream::connect(&addr).await?;

                let server_name = ServerName::try_from(domain.clone())?;
                let tls = connector.connect(server_name, tcp).await?;
                let negotiated = tls.get_ref().1.alpn_protocol().map(|proto| proto.to_vec());
                let io = TokioIo::new(tls);

                match negotiated.as_deref() {
                    Some(b"h2") => {
                        let (sender, conn) = conn::http2::Builder::new(TokioExecutor::new())
                            .initial_stream_window_size(65535)
                            .initial_connection_window_size(1_048_576)
                            .max_frame_size(16_384)
                            .handshake(io)
                            .await?;
                        tokio::spawn(async move {
                            if let Err(e) = conn.await {
                                eprintln!("HTTP/2 connection error: {:?}", e);
                            }
                        });
                        SendRequestClient::Http2(sender)
                    }
                    _ => {
                        let (sender, conn) = conn::http1::handshake(io).await?;
                        tokio::spawn(async move {
                            if let Err(e) = conn.await {
                                eprintln!("HTTP/1.1 connection error: {:?}", e);
                            }
                        });
                        SendRequestClient::Http1(sender)
                    }
                }
            }
            "http" => {
                // TODO: Try h2c with prior knowledge and upgrade support
                let tcp = TcpStream::connect(&addr).await?;
                let io = TokioIo::new(tcp);
                let (sender, conn) = conn::http1::handshake(io).await?;
                tokio::spawn(async move {
                    if let Err(e) = conn.await {
                        eprintln!("HTTP/1.1 connection error: {:?}", e);
                    }
                });
                SendRequestClient::Http1(sender)
            }
            _ => return Err("Unsupported scheme".into()),
        };

        Ok(Client {
            sender: Arc::new(Mutex::new(sender)),
            config: ClientConfiguration {
                timeout: self.timeout,
                headers: Some(self.headers),
                base_url,
                port,
            },
        })
    }
}

#[tokio::test]
async fn test_async_client() -> Result<(), Box<dyn std::error::Error>> {
    let parsed_url = Url::parse("http://myrstack.tech")?;
    let scheme = parsed_url.scheme();
    let port = parsed_url
        .port_or_known_default()
        .unwrap_or_else(|| if scheme == "https" { 443 } else { 80 });

    let client = Client::builder()
        .base_url(parsed_url)
        .port(port)
        .http1_only()
        .build()
        .await?;

    let request = client.get("/").build()?;

    let res = client.execute(request).await?;

    assert_eq!(res.status, 301);

    Ok(())
}

#[tokio::test]
async fn test_async_client_http2() -> Result<(), Box<dyn std::error::Error>> {
    let parsed_url = Url::parse("https://myrstack.tech")?;
    let scheme = parsed_url.scheme();
    let port = parsed_url
        .port_or_known_default()
        .unwrap_or_else(|| if scheme == "https" { 443 } else { 80 });

    let client = Client::builder()
        .base_url(parsed_url)
        .port(port)
        .build()
        .await?;

    let request = client.get("/").build()?;

    let res = client.execute(request).await?;

    assert_eq!(res.status, 200);

    Ok(())
}
