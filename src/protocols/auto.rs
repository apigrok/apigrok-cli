use crate::error::{ApiGrokError, Result};
use crate::protocols::Protocol;
use crate::request::Request;
use crate::response::Response;
use crate::verbose::VerboseInfo;
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::Request as HyperRequest;
use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use std::collections::HashMap;
use std::time::Instant;

pub struct AutoClient;

impl AutoClient {
    pub fn new() -> Self {
        Self
    }

    async fn build_request(&self, req: &Request) -> Result<HyperRequest<Full<Bytes>>> {
        let uri = req.url.parse::<hyper::Uri>()
            .map_err(|e| ApiGrokError::InvalidUrl(e.to_string()))?;

        let method = req.method.parse::<hyper::Method>()
            .map_err(|e| ApiGrokError::ParseError(e.to_string()))?;

        let mut builder = HyperRequest::builder()
            .method(method)
            .uri(uri);

        // Add headers
        for (key, value) in &req.headers {
            builder = builder.header(key, value);
        }

        // Add User-Agent if not present
        if !req.headers.contains_key("user-agent") && !req.headers.contains_key("User-Agent") {
            builder = builder.header("User-Agent", "APIGrok/0.1.0");
        }

        // Build with body
        let body = if let Some(data) = &req.body {
            Full::new(Bytes::from(data.clone()))
        } else {
            Full::new(Bytes::new())
        };

        builder
            .body(body)
            .map_err(|e| ApiGrokError::ParseError(e.to_string()))
    }
}

#[async_trait::async_trait]
impl Protocol for AutoClient {
    async fn execute(&self, request: &Request) -> Result<Response> {
        let start = Instant::now();

        // Create HTTPS connector with ALPN support for both HTTP/1.1 and HTTP/2
        // Let the server decide which protocol to use via ALPN negotiation
        let https = HttpsConnectorBuilder::new()
            .with_native_roots()
            .map_err(|e| ApiGrokError::TlsError(e.to_string()))?
            .https_or_http()
            .enable_all_versions()  // Enables both HTTP/1.1 and HTTP/2
            .build();

        // Build client without forcing protocol - let ALPN negotiate
        let client = Client::builder(TokioExecutor::new())
            .build::<_, Full<Bytes>>(https);

        let mut current_request = request.clone();
        let mut redirect_count = 0;
        const MAX_REDIRECTS: u32 = 10;

        loop {
            // Display verbose request info if enabled
            if request.verbose {
                let mut header_vec = Vec::new();
                for (key, value) in &current_request.headers {
                    header_vec.push((key.clone(), value.clone()));
                }
                let verbose_info = VerboseInfo::new(
                    current_request.url.clone(),
                    current_request.method.clone(),
                    header_vec,
                );
                verbose_info.display_request();
            }

            // Display request body if verbose_body is enabled
            if request.verbose_body {
                if let Some(body) = &current_request.body {
                    VerboseInfo::display_request_body(body);
                }
            }

            // Build the request
            let hyper_req = self.build_request(&current_request).await?;

            // Execute the request
            let res = client
                .request(hyper_req)
                .await
                .map_err(|e| ApiGrokError::NetworkError(e.to_string()))?;

            let status = res.status();

            // Check if we should follow redirect
            if request.follow_redirects && status.is_redirection() && redirect_count < MAX_REDIRECTS {
                if let Some(location) = res.headers().get("location") {
                    // Display redirect response headers if verbose
                    if request.verbose {
                        let status_code = res.status().as_u16();
                        let status_text = res.status().canonical_reason().unwrap_or("Unknown").to_string();
                        let mut header_vec = Vec::new();
                        for (key, value) in res.headers() {
                            if let Ok(val_str) = value.to_str() {
                                header_vec.push((key.to_string(), val_str.to_string()));
                            }
                        }
                        let verbose_info = VerboseInfo::new(
                            current_request.url.clone(),
                            current_request.method.clone(),
                            Vec::new(),
                        );
                        verbose_info.display_response_headers(status_code, &status_text, &header_vec);
                    }

                    let location_str = location.to_str()
                        .map_err(|e| ApiGrokError::ParseError(format!("Invalid location header: {}", e)))?;

                    // Handle relative URLs
                    let new_url = if location_str.starts_with("http://") || location_str.starts_with("https://") {
                        location_str.to_string()
                    } else {
                        let base_url = url::Url::parse(&current_request.url)
                            .map_err(|e| ApiGrokError::InvalidUrl(e.to_string()))?;
                        base_url.join(location_str)
                            .map_err(|e| ApiGrokError::InvalidUrl(e.to_string()))?
                            .to_string()
                    };

                    current_request.url = new_url;
                    redirect_count += 1;

                    // For 303, change POST/PUT to GET
                    if status.as_u16() == 303 {
                        current_request.method = "GET".to_string();
                        current_request.body = None;
                    }

                    continue;
                }
            }

            let elapsed = start.elapsed().as_millis();

            // Extract response details
            let status_code = res.status().as_u16();
            let status_text = res.status().canonical_reason().unwrap_or("Unknown").to_string();
            let version = format!("{:?}", res.version());

            // Extract headers
            let mut headers = HashMap::new();
            let mut header_vec = Vec::new();
            for (key, value) in res.headers() {
                if let Ok(val_str) = value.to_str() {
                    headers.insert(key.to_string(), val_str.to_string());
                    header_vec.push((key.to_string(), val_str.to_string()));
                }
            }

            // Display verbose response headers if enabled
            if request.verbose {
                let verbose_info = VerboseInfo::new(
                    current_request.url.clone(),
                    current_request.method.clone(),
                    Vec::new(),
                );
                verbose_info.display_response_headers(status_code, &status_text, &header_vec);
            }

            // Read body
            let body_bytes = res
                .into_body()
                .collect()
                .await
                .map_err(|e| ApiGrokError::NetworkError(e.to_string()))?
                .to_bytes()
                .to_vec();

            // Display connection close if verbose
            if request.verbose {
                let verbose_info = VerboseInfo::new(
                    current_request.url.clone(),
                    current_request.method.clone(),
                    Vec::new(),
                );
                verbose_info.display_connection_close();
            }

            return Ok(Response::new(
                status_code,
                status_text,
                headers,
                body_bytes,
                version,
                elapsed,
            ));
        }
    }
}
