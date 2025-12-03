use clap::{Parser, Subcommand, ValueEnum};
use std::collections::HashMap;

#[derive(Debug, Clone, ValueEnum)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
}

impl HttpMethod {
    pub fn as_str(&self) -> &str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPTIONS",
        }
    }
}

#[derive(Debug, Clone, ValueEnum)]
pub enum Protocol {
    Auto,
    Http1,
    Http2,
    Http3,
    Websocket,
    Grpc,
}

#[derive(Parser, Debug)]
#[command(name = "apigrok")]
#[command(about = "A powerful multi-protocol API CLI tool", long_about = None)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Verbose output (show connection metadata)
    #[arg(short = 'v', long = "verbose", global = true)]
    pub verbose: bool,

    /// Show request body being sent
    #[arg(short = 'V', long = "verbose-body", global = true)]
    pub verbose_body: bool,

    /// Request timeout in seconds
    #[arg(short = 't', long = "timeout", default_value = "30", global = true)]
    pub timeout: u64,

    /// Output format (auto, json, plain)
    #[arg(short = 'o', long = "output", default_value = "auto", global = true)]
    pub output_format: String,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Make HTTP/HTTPS requests
    #[command(alias = "h")]
    Http {
        /// Target URL
        url: String,

        /// HTTP method to use
        #[arg(value_enum, default_value = "get")]
        method: HttpMethod,

        /// Request headers (can be specified multiple times: -H "Key: Value")
        #[arg(short = 'H', long = "header", value_parser = parse_header)]
        headers: Vec<(String, String)>,

        /// Request body data
        #[arg(short = 'd', long = "data")]
        data: Option<String>,

        /// JSON request body
        #[arg(short = 'j', long = "json")]
        json: Option<String>,

        /// Force HTTP/1.1
        #[arg(long, conflicts_with_all = ["http2", "http3"])]
        http1: bool,

        /// Force HTTP/2
        #[arg(long, conflicts_with_all = ["http1", "http3"])]
        http2: bool,

        /// Force HTTP/3
        #[arg(long, conflicts_with_all = ["http1", "http2"])]
        http3: bool,

        /// Follow redirects
        #[arg(short = 'L', long = "location")]
        follow_redirects: bool,

        /// Use insecure connection (skip TLS verification)
        #[arg(long)]
        insecure: bool,
    },

    /// Make gRPC requests
    #[command(alias = "g")]
    Grpc {
        /// Target URL (format: grpc://host:port/service.Name/Method)
        url: String,

        /// JSON request body
        #[arg(short = 'j', long = "json")]
        json: Option<String>,

        /// Proto file for message schema
        #[arg(long = "proto", conflicts_with = "reflection")]
        proto: Option<String>,

        /// Use server reflection instead of proto file
        #[arg(long, conflicts_with = "proto")]
        reflection: bool,

        /// Input file for client streaming (one JSON message per line)
        #[arg(long = "stream-input")]
        stream_input: Option<String>,

        /// List available services via reflection
        #[arg(long)]
        list_services: bool,

        /// Use insecure connection (no TLS)
        #[arg(long)]
        insecure: bool,

        /// Request headers (can be specified multiple times: -H "Key: Value")
        #[arg(short = 'H', long = "header", value_parser = parse_header)]
        headers: Vec<(String, String)>,
    },

    /// Connect to WebSocket endpoints
    #[command(alias = "ws")]
    Websocket {
        /// Target URL (format: ws://host:port or wss://host:port)
        url: String,

        /// JSON message to send
        #[arg(short = 'j', long = "json")]
        json: Option<String>,

        /// Request headers (can be specified multiple times: -H "Key: Value")
        #[arg(short = 'H', long = "header", value_parser = parse_header)]
        headers: Vec<(String, String)>,
    },
}

impl Cli {
    pub fn get_url(&self) -> &str {
        match &self.command {
            Commands::Http { url, .. } => url,
            Commands::Grpc { url, .. } => url,
            Commands::Websocket { url, .. } => url,
        }
    }

    pub fn get_headers_map(&self) -> HashMap<String, String> {
        let headers = match &self.command {
            Commands::Http { headers, .. } => headers,
            Commands::Grpc { headers, .. } => headers,
            Commands::Websocket { headers, .. } => headers,
        };
        headers.iter().cloned().collect()
    }

    pub fn determine_protocol(&self) -> Protocol {
        match &self.command {
            Commands::Http { http1, http2, http3, .. } => {
                if *http1 {
                    Protocol::Http1
                } else if *http2 {
                    Protocol::Http2
                } else if *http3 {
                    Protocol::Http3
                } else {
                    Protocol::Auto
                }
            }
            Commands::Grpc { .. } => Protocol::Grpc,
            Commands::Websocket { .. } => Protocol::Websocket,
        }
    }

    pub fn get_method(&self) -> HttpMethod {
        match &self.command {
            Commands::Http { method, .. } => method.clone(),
            _ => HttpMethod::Get, // Default for non-HTTP
        }
    }

    pub fn get_data(&self) -> Option<&String> {
        match &self.command {
            Commands::Http { data, .. } => data.as_ref(),
            _ => None,
        }
    }

    pub fn get_json(&self) -> Option<&String> {
        match &self.command {
            Commands::Http { json, .. } => json.as_ref(),
            Commands::Grpc { json, .. } => json.as_ref(),
            Commands::Websocket { json, .. } => json.as_ref(),
        }
    }

    pub fn is_insecure(&self) -> bool {
        match &self.command {
            Commands::Http { insecure, .. } => *insecure,
            Commands::Grpc { insecure, .. } => *insecure,
            _ => false,
        }
    }

    pub fn is_follow_redirects(&self) -> bool {
        match &self.command {
            Commands::Http { follow_redirects, .. } => *follow_redirects,
            _ => false,
        }
    }

    pub fn get_proto(&self) -> Option<&String> {
        match &self.command {
            Commands::Grpc { proto, .. } => proto.as_ref(),
            _ => None,
        }
    }

    pub fn is_reflection(&self) -> bool {
        match &self.command {
            Commands::Grpc { reflection, .. } => *reflection,
            _ => false,
        }
    }

    pub fn get_stream_input(&self) -> Option<&String> {
        match &self.command {
            Commands::Grpc { stream_input, .. } => stream_input.as_ref(),
            _ => None,
        }
    }

    pub fn is_list_services(&self) -> bool {
        match &self.command {
            Commands::Grpc { list_services, .. } => *list_services,
            _ => false,
        }
    }
}

fn parse_header(s: &str) -> Result<(String, String), String> {
    let parts: Vec<&str> = s.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid header format: {}", s));
    }
    Ok((parts[0].trim().to_string(), parts[1].trim().to_string()))
}
