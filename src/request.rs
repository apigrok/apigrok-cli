use crate::cli::Cli;
use crate::error::Result;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Request {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub timeout: u64,
    pub follow_redirects: bool,
    pub insecure: bool,
    pub proto: Option<String>,
    pub reflection: bool,
    pub stream_input: Option<String>,
    pub list_services: bool,
    pub verbose: bool,
    pub verbose_body: bool,
}

impl Request {
    pub fn from_cli(cli: &Cli) -> Result<Self> {
        let url = Self::normalize_url(cli.get_url())?;
        let mut headers = cli.get_headers_map();

        let body = if let Some(json_data) = cli.get_json() {
            headers.insert("Content-Type".to_string(), "application/json".to_string());
            Some(json_data.as_bytes().to_vec())
        } else if let Some(data) = cli.get_data() {
            if !headers.contains_key("Content-Type") {
                headers.insert("Content-Type".to_string(), "application/x-www-form-urlencoded".to_string());
            }
            Some(data.as_bytes().to_vec())
        } else {
            None
        };

        Ok(Request {
            method: cli.get_method().as_str().to_string(),
            url,
            headers,
            body,
            timeout: cli.timeout,
            follow_redirects: cli.is_follow_redirects(),
            insecure: cli.is_insecure(),
            proto: cli.get_proto().cloned(),
            reflection: cli.is_reflection(),
            stream_input: cli.get_stream_input().cloned(),
            list_services: cli.is_list_services(),
            verbose: cli.verbose,
            verbose_body: cli.verbose_body,
        })
    }

    fn normalize_url(url: &str) -> Result<String> {
        let url = if !url.starts_with("http://") &&
                     !url.starts_with("https://") &&
                     !url.starts_with("ws://") &&
                     !url.starts_with("wss://") &&
                     !url.starts_with("grpc://") {
            format!("https://{}", url)
        } else {
            url.to_string()
        };

        // Validate URL (grpc:// is valid for our purposes)
        url::Url::parse(&url)?;
        Ok(url)
    }

    pub fn method_supports_body(&self) -> bool {
        matches!(self.method.as_str(), "POST" | "PUT" | "PATCH")
    }
}
