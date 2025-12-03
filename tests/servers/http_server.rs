// Simple HTTP test server supporting HTTP/1.1 and HTTP/2
// Run with: cargo run --bin http_test_server

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use hyper_util::server::conn::auto;
use std::convert::Infallible;
use std::net::SocketAddr;
use tokio::net::TcpListener;

async fn handle_request(req: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    let mut response = Response::new(Full::new(Bytes::new()));

    match (req.method(), req.uri().path()) {
        // GET /get - Simple GET endpoint
        (&Method::GET, "/get") => {
            let body = serde_json::json!({
                "method": "GET",
                "path": "/get",
                "headers": req.headers().iter()
                    .map(|(k, v)| (k.as_str(), v.to_str().unwrap_or("")))
                    .collect::<Vec<_>>(),
                "args": {}
            });
            *response.body_mut() = Full::new(Bytes::from(serde_json::to_string_pretty(&body).unwrap()));
            response.headers_mut().insert(
                hyper::header::CONTENT_TYPE,
                "application/json".parse().unwrap(),
            );
        }

        // POST /post - Echo JSON body
        (&Method::POST, "/post") => {
            let whole_body = req.into_body().collect().await.unwrap().to_bytes();
            let body = serde_json::json!({
                "method": "POST",
                "path": "/post",
                "data": String::from_utf8_lossy(&whole_body).to_string(),
                "json": serde_json::from_slice::<serde_json::Value>(&whole_body).ok()
            });
            *response.body_mut() = Full::new(Bytes::from(serde_json::to_string_pretty(&body).unwrap()));
            response.headers_mut().insert(
                hyper::header::CONTENT_TYPE,
                "application/json".parse().unwrap(),
            );
        }

        // GET /headers - Return request headers
        (&Method::GET, "/headers") => {
            let headers: std::collections::HashMap<_, _> = req.headers().iter()
                .map(|(k, v)| (k.as_str(), v.to_str().unwrap_or("")))
                .collect();
            let body = serde_json::json!({
                "headers": headers
            });
            *response.body_mut() = Full::new(Bytes::from(serde_json::to_string_pretty(&body).unwrap()));
            response.headers_mut().insert(
                hyper::header::CONTENT_TYPE,
                "application/json".parse().unwrap(),
            );
        }

        // GET /status/:code - Return specific status code
        (&Method::GET, path) if path.starts_with("/status/") => {
            let code = path.strip_prefix("/status/").unwrap();
            if let Ok(status_code) = code.parse::<u16>() {
                *response.status_mut() = StatusCode::from_u16(status_code).unwrap();
            }
        }

        // GET /health - Health check
        (&Method::GET, "/health") => {
            let body = serde_json::json!({"status": "ok"});
            *response.body_mut() = Full::new(Bytes::from(serde_json::to_string(&body).unwrap()));
            response.headers_mut().insert(
                hyper::header::CONTENT_TYPE,
                "application/json".parse().unwrap(),
            );
        }

        // 404 for everything else
        _ => {
            *response.status_mut() = StatusCode::NOT_FOUND;
            let body = serde_json::json!({"error": "Not found"});
            *response.body_mut() = Full::new(Bytes::from(serde_json::to_string(&body).unwrap()));
        }
    }

    Ok(response)
}

#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await.unwrap();

    println!("HTTP test server listening on http://{}", addr);
    println!("Supports both HTTP/1.1 and HTTP/2 via ALPN");
    println!("\nEndpoints:");
    println!("  GET  /get");
    println!("  POST /post");
    println!("  GET  /headers");
    println!("  GET  /status/:code");
    println!("  GET  /health");

    loop {
        let (stream, _) = listener.accept().await.unwrap();
        let io = TokioIo::new(stream);

        tokio::task::spawn(async move {
            if let Err(err) = auto::Builder::new(hyper_util::rt::TokioExecutor::new())
                .serve_connection(io, service_fn(handle_request))
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}
