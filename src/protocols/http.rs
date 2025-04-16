use super::*;
use crate::protocols::{ApiProtocol, ApiRequest, ApiResponse, Protocol};
use async_trait::async_trait;
use reqwest::{Client, Version};
use std::error::Error;
use std::error::Error;
use std::sync::Arc;
use std::time::Instant;
use std::vec;

use http_body_util::Empty;
use hyper::body::Bytes;
use hyper::client::conn::http2;
use hyper::rt::{Read, Write};
use hyper::{Request, header};
use hyper_util::client::legacy::connect::HttpConnector;
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
    //Http2,
    //Http3,
}

// Defines a trait combination that applies equally to the TokioIo<TlsStream<TcpStream>> and TokioIo<TcpStream>
trait Streamable: Read + Write + Unpin + Send {}

// Defines a generic implementation that'll get built when we tell the compiler that we want a
// dyn pointer to Streamable for any type that implements all those traits (normal-ish I think?)
impl<T> Streamable for T where T: Read + Write + Unpin + Send {}

#[async_trait]
impl ApiProtocol for HttpClient {
    async fn fetch(&self, url: &str) -> Result<(ApiRequest, ApiResponse), Box<dyn Error>> {
        let client = match self.version {
            HttpVersion::Http1 => Client::builder().http1_only().build()?,
            // HttpVersion::Http2 => Client::builder().http2_prior_knowledge().build()?,
            // HttpVersion::Http3 => {
            //     unimplemented!("HTTP/3 support coming soon")
            // }
        };

        let start = Instant::now();

        let request = client.get(url).build()?;
        let request_headers: Vec<(String, String)> = request
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        let request_version = format!("{:?}", &request.version());
        let request_path = (&request.url().path()).to_string();

        let response = client.execute(request).await?;

        let path = response.url().path().to_string();
        let duration = start.elapsed();
        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let version = response.version();
        let ip = response.remote_addr();

        let mut config = ClientConfig::builder_with_protocol_versions(&[&rustls::version::TLS13])
            .with_root_certificates(root_store)
            .with_no_client_auth();

        Ok((
            ApiRequest {
                headers: Some(request_headers),
                method: "GET".to_string(),
                path: request_path,
                version: request_version,
            },
            ApiResponse {
                path: path,
                protocol: match self.version {
                    HttpVersion::Http1 => Protocol::Http1,
                    // HttpVersion::Http2 => Protocol::Http2,
                    // HttpVersion::Http3 => Protocol::Http3,
                },
                status: Some(status),
                headers: Some(headers),
                body: Some(body),
                version: format!("{}", version_to_string(version)),
                ip: ip,
                duration: duration,
            },
        ))
    }
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
