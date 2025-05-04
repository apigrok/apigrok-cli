use crate::clients::{self, http};

use super::*;
use std::error::Error;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use std::time::Duration;
use std::vec;

use h2::client::{self};
use http_body_util::Empty;
use hyper::body::Bytes;
use hyper::client::conn::http2;
use hyper::header::HeaderValue;
use hyper::rt::{Read, Write};
use hyper::{HeaderMap, Request, Version, header};

use hyper_util::rt::{TokioExecutor, TokioIo};
use rustls::pki_types::ServerName;
use rustls_native_certs::load_native_certs;
use tokio::net::TcpStream;
use tokio_rustls::{TlsConnector, rustls::ClientConfig};
use url::Url;

pub struct HttpClient {
    pub version: HttpVersion,
}

pub enum HttpVersion {
    Http1,
    Http2,
    Http3,
}

// Defines a trait combination that applies equally to the TokioIo<TlsStream<TcpStream>> and TokioIo<TcpStream>
trait Streamable: Read + Write + Unpin + Send {}

// Defines a generic implementation that'll get built when we tell the compiler that we want a
// dyn pointer to Streamable for any type that implements all those traits (normal-ish I think?)
impl<T> Streamable for T where T: Read + Write + Unpin + Send {}

#[async_trait]
impl ApiProtocol for HttpClient {
    async fn execute(
        &self,
        method: Method,
        url: &str,
        h2c: bool,
    ) -> Result<(ApiRequest, ApiResponse), Box<dyn Error>> {
        let parsed_url = Url::parse(url)?;
        let scheme = parsed_url.scheme();
        let host = parsed_url.host_str().ok_or("Invalid host")?;
        let domain = parsed_url.domain().ok_or("Invalid domain")?.to_string();
        let port = parsed_url
            .port_or_known_default()
            .unwrap_or_else(|| if scheme == "https" { 443 } else { 80 });

        // 1. Open TCP connection
        let tcp = TcpStream::connect((domain.clone(), port)).await?;
        println!("Connected to {}:{} via {}", host, port, scheme);
        println!("{:?}", domain);

        println!("Local address: {}", tcp.local_addr()?);
        println!("Peer address: {}", tcp.peer_addr()?);
        println!("Socket TTL: {}", tcp.ttl()?);
        println!("Nodelay setting: {}", tcp.nodelay()?);

        let client = http::async_client::Client::builder()
            .base_url(parsed_url.clone())
            .port(port)
            .http1_only()
            .build()
            .await?;
        let request = client.get("/").build()?;
        let res = client.execute(request).await?;

        println!("Async client gave this http code: {}", res.status);

        // if h2c {
        //     http1_shizzle_with_upgrade(method, url, parsed_url, tcp).await?;
        // } else {
        //     http1_shizzle(method, url, parsed_url, tcp).await?;
        // }
        // let io: Option<Box<dyn Streamable>> = match scheme {
        //     "https" => wrap_stream_with_tls(tcp, &domain).await?,
        //     "http" => {
        //         let tokio_io = TokioIo::new(tcp);
        //         Some(Box::new(tokio_io))
        //     }
        //     _ => {
        //         println!("Unsupported scheme: {}", scheme);
        //         None
        //     }
        // };

        // if let Some(io) = io {
        //     process_stream(&parsed_url, scheme, domain, io).await?;
        // }

        Ok((
            ApiRequest {
                headers: Some(vec![]),
                method: "GET".to_string(),
                path: url.to_string(),
                version: format!("{}", version_to_string(Version::HTTP_11)),
            },
            ApiResponse {
                path: url.to_string(),
                protocol: match self.version {
                    HttpVersion::Http1 => Protocol::Http1,
                    HttpVersion::Http2 => Protocol::Http2,
                    HttpVersion::Http3 => Protocol::Http3,
                },
                status: Some(200),
                headers: Some(vec![]),
                body: Some(vec![]),
                version: format!("{}", version_to_string(Version::HTTP_11)),
                ip: Some(SocketAddr::new(
                    IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                    8080,
                )),
                duration: Duration::new(3, 0),
            },
        ))
    }
}

async fn http1_shizzle(
    method: Method,
    url: &str,
    parsed_url: Url,
    tcp: TcpStream,
) -> Result<(), Box<dyn Error>> {
    let io = TokioIo::new(tcp);
    let (mut sender, conn) = hyper::client::conn::http1::handshake::<_, Empty<Bytes>>(io).await?;

    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            println!("Connection failed: {:?}", err);
        }
    });

    let req: Request<Empty<Bytes>> = Request::builder()
        .uri(url)
        .header(header::HOST, parsed_url.domain().unwrap_or_default())
        .header(hyper::header::USER_AGENT, "apigrok/0.1.0")
        .header(hyper::header::ACCEPT, "*/*")
        .header(hyper::header::ACCEPT_ENCODING, "gzip")
        .method(method)
        .body(Empty::<Bytes>::new())?;

    let response = sender.send_request(req).await?;

    println!("Status 1.x: {}", response.status());

    Ok(())
}

async fn http1_shizzle_with_upgrade(
    method: Method,
    url: &str,
    parsed_url: Url,
    tcp: TcpStream,
) -> Result<(), Box<dyn Error>> {
    let io = TokioIo::new(tcp);
    let (mut sender, conn) = hyper::client::conn::http1::handshake::<_, Empty<Bytes>>(io).await?;

    tokio::task::spawn(async move {
        if let Err(err) = conn.with_upgrades().await {
            println!("Connection failed: {:?}", err);
        }
    });

    // probing with OPTIONS request
    let req: Request<Empty<Bytes>> = Request::builder()
        .uri(url)
        .header(header::HOST, parsed_url.domain().unwrap_or_default())
        .header(hyper::header::USER_AGENT, "apigrok/0.1.0")
        .header(hyper::header::ACCEPT, "*/*")
        .header(hyper::header::ACCEPT_ENCODING, "gzip")
        .header(hyper::header::CONNECTION, "Upgrade, HTTP2-Settings")
        .header(hyper::header::UPGRADE, "h2c")
        .header("HTTP2-Settings", "")
        .method(hyper::Method::OPTIONS)
        .body(Empty::<Bytes>::new())?;

    let mut response = sender.send_request(req).await?;

    println!("Status 1.x: {}", response.status());

    if response.status() == hyper::StatusCode::SWITCHING_PROTOCOLS {
        println!("Upgrade accepted!");

        // Access the upgraded connection:
        if let Some(upgraded) = hyper::upgrade::on(&mut response).await.ok() {
            // Now you have a raw Upgraded I/O stream (impl AsyncRead + AsyncWrite)
            println!("Connection upgraded!");

            // Handle your protocol (WebSocket, h2c, etc.) here
            let io = TokioIo::new(upgraded);
            // Now upgraded can be used directly with h2
            let (mut h2_client, h2_connection) = client::handshake(io).await?;

            tokio::spawn(async move {
                if let Err(e) = h2_connection.await {
                    eprintln!("h2 connection error: {:?}", e);
                }
            });

            // probing with OPTIONS request, needs to be same as ORIGINAL upgrade request
            let req = Request::builder()
                .uri(url)
                .header(header::HOST, parsed_url.domain().unwrap_or_default())
                .header(hyper::header::USER_AGENT, "apigrok/0.1.0")
                .header(hyper::header::ACCEPT, "*/*")
                .header(hyper::header::ACCEPT_ENCODING, "gzip")
                .version(hyper::http::Version::HTTP_2)
                .method(hyper::Method::OPTIONS)
                .body(())?;

            let (response_future, _) = h2_client.send_request(req, true)?;
            let response = response_future.await?;

            println!("Status h2c: {}", response.status());

            // user intended request over h2c
            let req = Request::builder()
                .uri(url)
                .header(header::HOST, parsed_url.domain().unwrap_or_default())
                .header(hyper::header::USER_AGENT, "apigrok/0.1.0")
                .header(hyper::header::ACCEPT, "*/*")
                .header(hyper::header::ACCEPT_ENCODING, "gzip")
                .version(hyper::http::Version::HTTP_2)
                .method(method)
                .body(())?;

            let (response_future, _) = h2_client.send_request(req, true)?;
            let response = response_future.await?;

            println!("Status h2c: {}", response.status());
        } else {
            eprintln!("Upgrade failed");
        }
    }

    Ok(())
}

// Wrap with TLS using ALP
async fn wrap_stream_with_tls(
    tcp: TcpStream,
    domain: &String,
) -> Result<Option<Box<dyn Streamable>>, Box<dyn Error>> {
    let server_name = ServerName::try_from(domain.clone())?;

    let mut root_store = rustls::RootCertStore::empty();
    for cert in load_native_certs().expect("Could not load platform certificates") {
        root_store.add(cert)?;
    }

    let mut tls_config = ClientConfig::builder_with_protocol_versions(&[&rustls::version::TLS13])
        .with_root_certificates(root_store)
        .with_no_client_auth();
    // Configure ALPN protocols (order matters!)
    tls_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()]; // Prefer HTTP/2

    let connector = TlsConnector::from(Arc::new(tls_config));
    let mut tls = connector.connect(server_name, tcp).await?;

    let (_, client_connection) = tls.get_mut();
    match client_connection.peer_certificates() {
        Some(certs) => {
            for der_cert in certs {
                // These are DER-encoded bytes (https://datatracker.ietf.org/doc/html/rfc5280)
                let raw_cert = &der_cert.to_vec();
                let (_, decoded_cert) = x509_parser::parse_x509_certificate(raw_cert)?;

                println!("Certificate Version: {}", &decoded_cert.version);
                println!("Certificate Issuer: {}", &decoded_cert.issuer);
                println!("Certificate Subject: {}", &decoded_cert.subject);
                let cert_validity = &decoded_cert.validity;
                let start = &cert_validity.not_before;
                let end = &cert_validity.not_after;
                println!("Certificate Validity: From {} until {}", start, end);
                println!("...");
            }
        }
        None => {}
    }

    let (_, session) = tls.get_ref();
    if session.alpn_protocol() != Some(b"h2") {
        return Err("Server didn't negotiate HTTP/2".into());
    }

    let tokio_io = TokioIo::new(tls);
    Ok(Some(Box::new(tokio_io)))
}

async fn process_stream(
    parsed_url: &Url,
    scheme: &str,
    domain: String,
    io: Box<dyn Streamable>,
) -> Result<(), Box<dyn Error>> {
    let (mut sender, conn) = http2::Builder::new(TokioExecutor::new())
        .initial_stream_window_size(65535)
        .initial_connection_window_size(1_048_576)
        .max_frame_size(16_384)
        .handshake(io)
        .await?;

    tokio::spawn(async move {
        if let Err(err) = conn.await {
            eprintln!("Connection failed: {:?}", err);
        }
    });

    println!("{}", format!("{}://{}", scheme, domain.clone()));

    let req: Request<Empty<Bytes>> = Request::builder()
        .uri(format!("{}://{}", scheme, domain.clone()))
        .header(header::HOST, parsed_url.domain().unwrap_or_default())
        .header(header::USER_AGENT, "my-client/0.1.0")
        .header(header::ACCEPT, "*/*")
        .body(Empty::new())?;

    let res = sender.send_request(req).await?;

    println!("Status: {}", res.status());
    Ok(())
}

fn version_to_string(version: Version) -> String {
    match version {
        Version::HTTP_09 => "HTTP/0.9",
        Version::HTTP_10 => "HTTP/1.0",
        Version::HTTP_11 => "HTTP/1.1",
        Version::HTTP_2 => "HTTP/2",
        Version::HTTP_3 => "HTTP/3",
        _ => "UNKNOWN_HTTP_VERSION",
    }
    .to_string()
}
