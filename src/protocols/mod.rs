pub mod grpc;
pub mod http;
pub mod websockets;

use async_trait::async_trait;
use base64::{Engine, engine::general_purpose};
use clap::ValueEnum;
use encoding_rs::{Encoding, UTF_8};
use mime::Mime;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
use std::net::SocketAddr;
use url::Url;

#[derive(Debug, Clone, ValueEnum, Serialize, Deserialize)]
pub enum Protocol {
    Http1,
    Http2,
    Http3,
    Grpc,
    Websockets,
}

#[async_trait]
pub trait ApiProtocol {
    async fn fetch(
        &self,
        url: &str,
    ) -> Result<(ApiRequest, ApiResponse), Box<dyn std::error::Error>>;
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse {
    #[serde_as(as = "DisplayFromStr")]
    pub url: Url,
    pub protocol: Protocol,
    pub status: Option<u16>,
    pub headers: Option<Vec<(String, String)>>,
    pub body: Option<Vec<u8>>,
    pub version: String,
    pub remote_ip: Option<SocketAddr>,
    pub request_duration: std::time::Duration,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiRequest {
    pub headers: Option<Vec<(String, String)>>,
    pub method: String,
    pub path: String,
    pub version: String,
}

impl ApiResponse {
    pub fn render_body(&self) {
        let (mime, charset) = self.parse_content_type();

        let data = match &self.body {
            Some(d) => d,
            None => todo!(),
        };

        if Self::is_text_based(&mime) {
            self.render_text_content(data, &mime, charset)
        } else {
            self.render_binary_content(data, &mime)
        }
    }

    fn render_text_content(&self, data: &[u8], mime: &Mime, charset: Option<&'static Encoding>) {
        let encoding = charset.unwrap_or(UTF_8);
        let (text, _actual_encoding, had_errors) = encoding.decode(data);
        had_errors.then(|| eprintln!("⚠️  Decoding had errors for encoding: {:?}", encoding));

        match mime {
            _ if mime.type_() == mime::APPLICATION && mime.subtype() == mime::JSON => {
                println!("{}", text.into_owned())
            }
            _ if mime.type_() == mime::TEXT && mime.subtype() == mime::HTML => {
                println!("{}", text.into_owned())
            }
            _ => println!("⚠️  Couldn't figure out what the heck this is..."),
        }
    }

    fn render_binary_content(&self, data: &[u8], mime: &Mime) {
        let _ = general_purpose::STANDARD.encode(data);
        match mime.subtype().as_str() {
            "png" | "jpeg" | "gif" => todo!("throw some binary output"),
            "pdf" => todo!("throw some binary output"),
            _ => println!("⚠️  Couldn't figure out what the heck this is..."),
        }
    }

    fn parse_content_type(&self) -> (Mime, Option<&'static Encoding>) {
        let header_value = match self.headers.as_ref().and_then(|headers| {
            headers
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case("content-type"))
                .map(|(_, v)| v.as_str())
        }) {
            Some(v) => v,
            None => return (mime::TEXT_PLAIN, None),
        };

        let mime = header_value
            .split(';')
            .next()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(mime::TEXT_PLAIN);

        let charset = header_value
            .split(';')
            .find(|s| s.trim().starts_with("charset="))
            .and_then(|s| s.split('=').nth(1))
            .and_then(|s| Encoding::for_label(s.trim().as_bytes()));

        (mime, charset)
    }

    fn is_text_based(mime: &Mime) -> bool {
        let subtype = mime.subtype().as_str();
        match (mime.type_(), subtype) {
            (mime::TEXT, _) => true,
            (mime::APPLICATION, "json") => true,
            (mime::APPLICATION, "xml") => true,
            (mime::APPLICATION, "javascript") => true,
            (mime::APPLICATION, "x-www-form-urlencoded") => true,
            (mime::APPLICATION, s) => s.ends_with("+json") || s.ends_with("+xml"),
            _ => false,
        }
    }
}
