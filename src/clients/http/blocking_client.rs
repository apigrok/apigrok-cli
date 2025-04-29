use std::{collections::HashMap, sync::Arc, time::Duration};

use http_body_util::Empty;
use hyper::{
    HeaderMap,
    body::{Bytes, Incoming},
    client::conn,
    header::{ACCEPT, ACCEPT_ENCODING, HOST, HeaderName, HeaderValue, USER_AGENT},
};

use hyper_util::rt::TokioIo;
use tokio::{net::TcpStream, runtime::Runtime, sync::Mutex};
use url::Url;

use crate::clients::http::ClientConfiguration;

use super::{request::Request, response::Response};

pub struct Client {
    sender: Arc<Mutex<conn::http1::SendRequest<Empty<Bytes>>>>,
    rt: Runtime,
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

    pub fn execute(&self, request: Request) -> Result<Response, Box<dyn std::error::Error>> {
        let method = request.method.clone();
        let url = request.url.clone();

        let sender = Arc::clone(&self.sender);
        let mut http_req = build_http_request(request, self.config.clone())?; // map your internal Request to hyper::Request

        let response = self.rt.block_on(async {
            let mut locked = sender.lock().await;
            let resp = locked.send_request(http_req).await?;
            build_http_response(resp)
        })?;

        println!("Executing {:?} {:?}", method, url);

        Ok(response)
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
        .header(USER_AGENT, default_user_agent.clone())
        .header(HOST, default_host);

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

    pub fn build(self) -> Result<Client, Box<dyn std::error::Error>> {
        let base_url = self.base_url.ok_or("Missing base_url")?;
        let port = self.port;

        let rt = Runtime::new()?;
        let host = base_url.host_str().ok_or("Invalid host")?.to_string();

        let sender = rt.block_on(async {
            let tcp = TcpStream::connect((host, port)).await?;
            let io = TokioIo::new(tcp);

            let (sender, connection) = conn::http1::handshake(io).await?;
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    eprintln!("connection failed: {:?}", e);
                }
            });

            Ok::<_, Box<dyn std::error::Error>>(sender)
        })?;

        Ok(Client {
            sender: Arc::new(Mutex::new(sender)),
            rt,
            config: ClientConfiguration {
                timeout: self.timeout,
                headers: Some(self.headers),
                base_url,
                port,
            },
        })
    }
}

#[test]
fn test_blocking_client() -> Result<(), Box<dyn std::error::Error>> {
    let parsed_url = Url::parse("http://myrstack.tech")?;
    let scheme = parsed_url.scheme();
    let port = parsed_url
        .port_or_known_default()
        .unwrap_or_else(|| if scheme == "https" { 443 } else { 80 });

    let client = Client::builder()
        .base_url(parsed_url)
        .port(port)
        .header(ACCEPT, HeaderValue::from_static("*/*"))
        .build()?;
    let request = client.get("/").build()?;

    let res = client.execute(request).unwrap();

    assert_eq!(res.status, 301);

    Ok(())
}
