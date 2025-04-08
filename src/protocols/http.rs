use super::*;
use http_body_util::{BodyExt, Empty};
use hyper::Request;
use hyper::body::Bytes;
use hyper_util::rt::TokioIo;
use std::time::Instant;
use tokio::io::{self, AsyncWriteExt};
use tokio::net::TcpStream;

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
        // Parse our URL...
        let url = url.parse::<hyper::Uri>()?;

        // Get the host and the port
        let host = url.host().expect("uri has no host");
        let port = url.port_u16().unwrap_or(80);

        let address = format!("{}:{}", host, port);

        // Open a TCP connection to the remote host
        let stream = TcpStream::connect(address).await?;

        // Use an adapter to access something implementing `tokio::io` traits as if they implement
        // `hyper::rt` IO traits.
        let io = TokioIo::new(stream);

        // Create the Hyper client
        let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;

        // Spawn a task to poll the connection, driving the HTTP state
        tokio::task::spawn(async move {
            if let Err(err) = conn.await {
                println!("Connection failed: {:?}", err);
            }
        });

        let authority = url.authority().unwrap().clone();

        // Create an HTTP request with an empty body and a HOST header
        let req: Request<Empty<Bytes>> = Request::builder()
            .uri(url)
            .header(hyper::header::HOST, authority.as_str())
            .header(hyper::header::USER_AGENT, "apigrok/0.1.0")
            .header(hyper::header::ACCEPT, "*/*")
            .header(hyper::header::ACCEPT_ENCODING, "gzip")
            .body(Empty::<Bytes>::new())?;

        // Await the response...
        let mut res = sender.send_request(req).await?;

        println!("Response status: {}", res.status());
        println!("Response headers: {:?}", res.headers());
        println!("Response version: {:?}", res.version());
        println!("Response extensions: {:?}", res.extensions());

        // Stream the body, writing each frame to stdout as it arrives
        while let Some(next) = res.frame().await {
            let frame = next?;
            if let Some(chunk) = frame.data_ref() {
                io::stdout().write_all(chunk).await?;
            }
        }

        // let client = match self.version {
        //     HttpVersion::Http1 => Client::new(),
        // };

        let start = Instant::now();

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
