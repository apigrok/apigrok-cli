use super::*;
use std::error::Error;
use std::sync::Arc;
use std::vec;

use http_body_util::Empty;
use hyper::body::Bytes;
use hyper::client::conn::http2;
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

#[async_trait]
impl ApiProtocol for HttpClient {
    async fn fetch(&self, url: &str) -> Result<ApiResponse, Box<dyn Error>> {
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

        let server_name = ServerName::try_from(domain.clone())?;

        let io = match scheme {
            "https" => {
                // 2. Wrap with TLS using ALP
                let mut root_store = rustls::RootCertStore::empty();
                for cert in load_native_certs().expect("Could not load platform certificates") {
                    root_store.add(cert)?;
                }

                let mut tls_config =
                    ClientConfig::builder_with_protocol_versions(&[&rustls::version::TLS13])
                        .with_root_certificates(root_store)
                        .with_no_client_auth();
                // Configure ALPN protocols (order matters!)
                tls_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()]; // Prefer HTTP/2

                let connector = TlsConnector::from(Arc::new(tls_config));
                let mut tls = connector.connect(server_name, tcp).await?;

                let (_, client_connection) = tls.get_mut();
                // let foo = client_connection.peer_certificates();
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

                let (mut sender, conn) = http2::Builder::new(TokioExecutor::new())
                    .initial_stream_window_size(65535)
                    .initial_connection_window_size(1_048_576)
                    .max_frame_size(16_384) // Common server-friendly size
                    .handshake(TokioIo::new(tls))
                    .await?;

                // 5. Spawn connection driver
                tokio::spawn(async move {
                    if let Err(err) = conn.await {
                        eprintln!("Connection failed: {:?}", err);
                    }
                });

                // 6. Send HTTP/2 request
                let req: Request<Empty<Bytes>> = Request::builder()
                    .uri(format!("{}://{}", scheme, domain.clone()))
                    .header(header::HOST, parsed_url.domain().unwrap_or_default())
                    .header(header::USER_AGENT, "my-client/0.1.0")
                    .header(header::ACCEPT, "*/*")
                    .body(Empty::new())?;

                let res = sender.send_request(req).await?;

                println!("Status: {}", res.status())
            }
            "http" => {
                let (mut sender, conn) = http2::Builder::new(TokioExecutor::new())
                    .initial_stream_window_size(65535)
                    .initial_connection_window_size(1_048_576)
                    .max_frame_size(16_384)
                    .handshake(TokioIo::new(tcp))
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

                println!("Status: {}", res.status())
            }
            _ => {
                println!("Unsupported scheme: {}", scheme);
            }
        };

        // // Parse our URL...
        // let url = url.parse::<hyper::Uri>()?;

        // // Get the host and the port
        // let host = url.host().expect("uri has no host");
        // let port = url.port_u16().unwrap_or(80);

        // let address = format!("{}:{}", host, port);

        // // Open a TCP connection to the remote host
        // let stream1 = TcpStream::connect(address.clone()).await?;

        // // Use an adapter to access something implementing `tokio::io` traits as if they implement
        // // `hyper::rt` IO traits.
        // let io1 = TokioIo::new(stream1);

        // // Create the Hyper client
        // let (mut sender1, conn1) = hyper::client::conn::http1::handshake(io1).await?;

        // let stream2 = TcpStream::connect(address).await?;
        // let io2 = TokioIo::new(stream2);
        // let exec = TokioExecutor::new(); // used to spawn internal tasks
        // let (mut sender2, conn2) =
        //     hyper::client::conn::http2::handshake::<_, _, http_body_util::Empty<Bytes>>(exec, io2)
        //         .await?;

        // // Spawn a task to poll the connection, driving the HTTP state
        // tokio::task::spawn(async move {
        //     if let Err(err) = conn1.await {
        //         println!("Connection failed: {:?}", err);
        //     }
        // });

        // // Spawn a task to poll the connection, driving the HTTP state
        // tokio::task::spawn(async move {
        //     if let Err(err) = conn2.await {
        //         println!("Connection failed: {:?}", err);
        //     }
        // });

        // let authority = url.authority().unwrap().clone();

        // // Create an HTTP request with an empty body and a HOST header
        // let req: Request<Empty<Bytes>> = Request::builder()
        //     .uri(url)
        //     .header(hyper::header::HOST, authority.as_str())
        //     .header(hyper::header::USER_AGENT, "apigrok/0.1.0")
        //     .header(hyper::header::ACCEPT, "*/*")
        //     .header(hyper::header::ACCEPT_ENCODING, "gzip")
        //     .body(Empty::<Bytes>::new())?;

        // let ret = sender2.send_request(req.clone()).await?;

        // println!("GAUYYY: {:?}", ret.headers());

        // Await the response...
        // let mut res = sender1.send_request(req).await?;

        // println!("Response status: {}", res.status());
        // println!("Response headers: {:?}", res.headers());
        // println!("Response version: {:?}", res.version());
        // println!("Response extensions: {:?}", res.extensions());

        // // Stream the body, writing each frame to stdout as it arrives
        // while let Some(next) = res.frame().await {
        //     let frame = next?;
        //     if let Some(chunk) = frame.data_ref() {
        //         io::stdout().write_all(chunk).await?;
        //     }
        // }

        // let client = match self.version {
        //     HttpVersion::Http1 => Client::new(),
        // };

        // let start = Instant::now();

        // let uri = Uri::from_str(url)?;
        // let req = Request::builder()
        //     .method("GET")
        //     .uri(uri.clone())
        //     .header("HOST", "localhost")
        //     .header("ACCEPT", "application/json");

        // Send the request
        // let res = client.request(req).await?;

        // let response = client.get(url).send().await?;
        // let path = response.url().path().to_string();
        // let duration = start.elapsed();
        // let status = response.status().as_u16();
        // let headers = response
        //     .headers()
        //     .iter()
        //     .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        //     .collect();
        // let version = response.version();
        // let ip = response.remote_addr();

        // let body = response.bytes().await?.to_vec();

        // Ok(ApiResponse {
        //     path: path,
        //     protocol: match self.version {
        //         HttpVersion::Http1 => Protocol::Http1,
        //         // HttpVersion::Http2 => Protocol::Http2,
        //         // HttpVersion::Http3 => Protocol::Http3,
        //     },
        //     status: Some(status),
        //     headers: Some(headers),
        //     body: Some(body),
        //     version: format!("{}", version_to_string(version)),
        //     ip: ip,
        //     duration: duration,
        // })
        todo!();
    }
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
