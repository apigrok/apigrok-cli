use super::*;
use reqwest::Client;
use std::time::Instant;

pub struct HttpClient {
    pub version: HttpVersion,
}

pub enum HttpVersion {
    Http1,
    Http2,
    Http3,
}

#[async_trait]
impl ApiProtocol for HttpClient {
    async fn fetch(&self, url: &str) -> Result<ApiResponse, Box<dyn Error>> {
        let client = match self.version {
            HttpVersion::Http1 => Client::builder().http1_only().build()?,
            HttpVersion::Http2 => Client::builder().http2_prior_knowledge().build()?,
            HttpVersion::Http3 => {
                unimplemented!("HTTP/3 support coming soon")
            }
        };

        let start = Instant::now();
        let response = client.get(url).send().await?;
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

        let body = response.bytes().await?.to_vec();

        Ok(ApiResponse {
            path: path,
            protocol: match self.version {
                HttpVersion::Http1 => Protocol::Http1,
                HttpVersion::Http2 => Protocol::Http2,
                HttpVersion::Http3 => Protocol::Http3,
            },
            status: Some(status),
            headers: Some(headers),
            body: Some(body),
            metadata: None,
            version: format!("{}", version_to_string(version)),
            ip: ip,
            duration: duration,
        })
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
