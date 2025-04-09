use crate::protocols::{ApiProtocol, ApiRequest, ApiResponse, Protocol};
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Client, Version};
use std::error::Error;
use std::time::Instant;
use url::Url;

pub struct HttpClient {
    pub version: HttpVersion,
}

pub enum HttpVersion {
    Http1,
    Http2,
    Http3,
}

impl HttpClient {
    fn build_client(&self, request_headers: HeaderMap) -> Result<Client, Box<dyn Error>> {
        let client = match self.version {
            HttpVersion::Http1 => Client::builder()
                .default_headers(request_headers.clone())
                .http1_only()
                .build()?,
            HttpVersion::Http2 => Client::builder()
                .default_headers(request_headers.clone())
                .http2_prior_knowledge()
                .build()?,
            HttpVersion::Http3 => unimplemented!("HTTP/3 support coming soon"),
        };

        Ok(client)
    }
}

#[async_trait]
impl ApiProtocol for HttpClient {
    async fn fetch(&self, url: &str) -> Result<(ApiRequest, ApiResponse), Box<dyn Error>> {
        let request_headers = set_default_headers(url);
        let client = self.build_client(request_headers.clone())?;

        let start = Instant::now();

        let request = client.get(url).build()?;
        let request_version = format!("{:?}", &request.version());
        let request_path = (&request.url().path()).to_string();

        let response = client.execute(request).await?;

        let response_url = response.url().clone();
        let request_duration = start.elapsed();
        let response_status = response.status().as_u16();
        let request_headers = request_headers
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let response_headers = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let version = response.version();
        let remote_ip = response.remote_addr();
        let response_body = response.bytes().await?.to_vec();

        Ok((
            ApiRequest {
                headers: Some(request_headers),
                method: "GET".to_string(),
                path: request_path,
                version: request_version,
            },
            ApiResponse {
                protocol: match self.version {
                    HttpVersion::Http1 => Protocol::Http1,
                    HttpVersion::Http2 => Protocol::Http2,
                    HttpVersion::Http3 => Protocol::Http3,
                },
                url: response_url,
                status: Some(response_status),
                headers: Some(response_headers),
                body: Some(response_body),
                version: version_to_string(version),
                remote_ip: remote_ip,
                request_duration: request_duration,
            },
        ))
    }
}

fn set_default_headers(url: &str) -> HeaderMap {
    let mut request_headers = HeaderMap::new();
    request_headers.insert(reqwest::header::ACCEPT, HeaderValue::from_static("*/*"));
    request_headers.insert(
        reqwest::header::ACCEPT_ENCODING,
        HeaderValue::from_static("gzip"),
    );
    request_headers.insert(
        reqwest::header::USER_AGENT,
        HeaderValue::from_static("apigrok/0.1.0"),
    );

    let parsed = Url::parse(url).expect("failed to parse URL");
    let host = parsed.host_str().unwrap_or_default();
    request_headers.insert(
        reqwest::header::HOST,
        HeaderValue::from_str(host).expect("invalid host header"),
    );

    return request_headers;
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

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;
    use tokio;

    #[tokio::test]
    async fn test_http1_fetch_success() {
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(GET).path("/hello");
            then.status(200)
                .header("Content-Type", "text/plain")
                .body("Hello, world!");
        });

        let client = HttpClient {
            version: HttpVersion::Http1,
        };

        let url = format!("{}/hello", &server.base_url());
        let result = client.fetch(&url).await;

        assert!(result.is_ok());

        let (request, response) = result.unwrap();

        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/hello");
        assert!(request.version.contains("HTTP"));

        assert_eq!(response.status, Some(200));
        assert_eq!(response.body.unwrap(), b"Hello, world!");
        assert_eq!(response.version, "HTTP/1.1");

        mock.assert();
    }

    #[tokio::test]
    #[ignore = "httpmock not supporting http2 properly."]
    async fn test_http2_fetch_success() {
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(GET).path("/http2-test");
            then.status(201)
                .header("X-Custom", "TestHeader")
                .body("Created!");
        });

        let client = HttpClient {
            version: HttpVersion::Http2,
        };

        let url = format!("{}/http2-test", &server.base_url());
        let result = client.fetch(&url).await;

        assert!(result.is_ok());

        let (_request, response) = result.unwrap();

        assert_eq!(response.status, Some(201));
        assert_eq!(response.body.unwrap(), b"Created!");
        assert!(
            response
                .headers
                .as_ref()
                .unwrap()
                .iter()
                .any(|(k, v)| k == "x-custom" && v == "TestHeader")
        );

        mock.assert();
    }

    #[tokio::test]
    #[ignore = "not yet implemented."]
    async fn test_http3_unimplemented() {
        let client = HttpClient {
            version: HttpVersion::Http3,
        };

        let result = client.fetch("http://localhost").await;

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("unimplemented"));
    }

    #[test]
    fn test_set_default_headers_contains_expected_headers() {
        let url = "http://example.com/test";
        let headers = set_default_headers(url);

        assert_eq!(headers.get("host").unwrap(), "example.com");
        assert_eq!(headers.get("user-agent").unwrap(), "apigrok/0.1.0");
        assert_eq!(headers.get("accept").unwrap(), "*/*");
        assert_eq!(headers.get("accept-encoding").unwrap(), "gzip");
    }

    #[test]
    fn test_version_to_string_conversion() {
        assert_eq!(version_to_string(Version::HTTP_09), "HTTP/0.9");
        assert_eq!(version_to_string(Version::HTTP_10), "HTTP/1.0");
        assert_eq!(version_to_string(Version::HTTP_11), "HTTP/1.1");
        assert_eq!(version_to_string(Version::HTTP_2), "HTTP/2");
        assert_eq!(version_to_string(Version::HTTP_3), "HTTP/3");
    }
}
