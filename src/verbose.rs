use colored::*;
use std::net::ToSocketAddrs;

pub struct VerboseInfo {
    pub url: String,
    pub method: String,
    pub headers: Vec<(String, String)>,
}

impl VerboseInfo {
    pub fn new(url: String, method: String, headers: Vec<(String, String)>) -> Self {
        Self { url, method, headers }
    }

    pub fn display_request(&self) {
        // Parse URL to get host and port
        if let Ok(parsed_url) = url::Url::parse(&self.url) {
            if let Some(host) = parsed_url.host_str() {
                let port = parsed_url.port_or_known_default().unwrap_or(80);
                let scheme = parsed_url.scheme();

                // DNS resolution
                eprintln!("{}", format!("* Host {}:{} was resolved.", host, port).bright_black());

                // Try to resolve DNS
                let socket_addr = format!("{}:{}", host, port);
                match socket_addr.to_socket_addrs() {
                    Ok(addrs) => {
                        let mut ipv4_addrs = Vec::new();
                        let mut ipv6_addrs = Vec::new();

                        for addr in addrs {
                            if addr.is_ipv4() {
                                ipv4_addrs.push(addr.ip().to_string());
                            } else {
                                ipv6_addrs.push(addr.ip().to_string());
                            }
                        }

                        if ipv6_addrs.is_empty() {
                            eprintln!("{}", "* IPv6: (none)".bright_black());
                        } else {
                            eprintln!("{}", format!("* IPv6: {}", ipv6_addrs.join(", ")).bright_black());
                        }

                        if !ipv4_addrs.is_empty() {
                            eprintln!("{}", format!("* IPv4: {}", ipv4_addrs.join(", ")).bright_black());
                            let first_ip = &ipv4_addrs[0];
                            eprintln!("{}", format!("*   Trying {}:{}...", first_ip, port).bright_black());
                            eprintln!("{}", format!("* Connected to {} ({}:{}) using {}", host, first_ip, port, scheme.to_uppercase()).bright_black());
                        } else if !ipv6_addrs.is_empty() {
                            let first_ip = &ipv6_addrs[0];
                            eprintln!("{}", format!("*   Trying {}:{}...", first_ip, port).bright_black());
                            eprintln!("{}", format!("* Connected to {} ({}:{}) using {}", host, first_ip, port, scheme.to_uppercase()).bright_black());
                        }
                    }
                    Err(_) => {
                        eprintln!("{}", "* IPv6: (none)".bright_black());
                        eprintln!("{}", "* IPv4: (none)".bright_black());
                        eprintln!("{}", format!("*   Trying {}:{}...", host, port).bright_black());
                        eprintln!("{}", format!("* Connected to {}:{} using {}", host, port, scheme.to_uppercase()).bright_black());
                    }
                }

                // Request line
                let path = if parsed_url.path().is_empty() { "/" } else { parsed_url.path() };
                let query = parsed_url.query().map(|q| format!("?{}", q)).unwrap_or_default();
                eprintln!("{}", format!("> {} {}{} HTTP/1.1", self.method, path, query).cyan());
                eprintln!("{}", format!("> Host: {}", host).cyan());

                // Request headers
                for (key, value) in &self.headers {
                    eprintln!("{}", format!("> {}: {}", key, value).cyan());
                }

                eprintln!("{}", "> ".cyan());
                eprintln!("{}", "* Request completely sent off".bright_black());
            }
        }
    }

    pub fn display_response_headers(&self, status: u16, status_text: &str, headers: &[(String, String)]) {
        eprintln!("{}", format!("< HTTP/1.1 {} {}", status, status_text).magenta());
        for (key, value) in headers {
            eprintln!("{}", format!("< {}: {}", key, value).magenta());
        }
        eprintln!("{}", "< ".magenta());
    }

    pub fn display_connection_close(&self) {
        if let Ok(parsed_url) = url::Url::parse(&self.url) {
            if let Some(host) = parsed_url.host_str() {
                eprintln!("{}", format!("* Connection to {} left intact", host).bright_black());
            }
        }
    }

    pub fn display_request_body(body: &[u8]) {
        if body.is_empty() {
            return;
        }

        eprintln!();
        eprintln!("{}", "* Request body:".bright_black());

        // Try to display as text if it's valid UTF-8
        if let Ok(text) = std::str::from_utf8(body) {
            // Check if it's JSON
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
                if let Ok(pretty) = serde_json::to_string_pretty(&json) {
                    for line in pretty.lines() {
                        eprintln!("{}", format!("  {}", line).cyan());
                    }
                } else {
                    for line in text.lines() {
                        eprintln!("{}", format!("  {}", line).cyan());
                    }
                }
            } else {
                // Plain text
                for line in text.lines() {
                    eprintln!("{}", format!("  {}", line).cyan());
                }
            }
        } else {
            // Binary data - show hex dump
            eprintln!("{}", format!("  [Binary data: {} bytes]", body.len()).yellow());

            // Show first 256 bytes as hex
            let display_len = std::cmp::min(body.len(), 256);
            for (i, chunk) in body[..display_len].chunks(16).enumerate() {
                let hex: String = chunk.iter()
                    .map(|b| format!("{:02x}", b))
                    .collect::<Vec<_>>()
                    .join(" ");
                eprintln!("{}", format!("  {:04x}: {}", i * 16, hex).yellow());
            }

            if body.len() > 256 {
                eprintln!("{}", format!("  ... ({} more bytes)", body.len() - 256).yellow());
            }
        }
        eprintln!();
    }
}
