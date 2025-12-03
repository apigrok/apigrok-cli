use std::fmt;

#[derive(Debug)]
pub enum ApiGrokError {
    InvalidUrl(String),
    NetworkError(String),
    ProtocolError(String),
    ParseError(String),
    TimeoutError,
    TlsError(String),
    IoError(std::io::Error),
    Other(String),
}

impl fmt::Display for ApiGrokError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiGrokError::InvalidUrl(msg) => write!(f, "Invalid URL: {}", msg),
            ApiGrokError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            ApiGrokError::ProtocolError(msg) => write!(f, "Protocol error: {}", msg),
            ApiGrokError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            ApiGrokError::TimeoutError => write!(f, "Request timed out"),
            ApiGrokError::TlsError(msg) => write!(f, "TLS error: {}", msg),
            ApiGrokError::IoError(err) => write!(f, "I/O error: {}", err),
            ApiGrokError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for ApiGrokError {}

impl From<std::io::Error> for ApiGrokError {
    fn from(err: std::io::Error) -> Self {
        ApiGrokError::IoError(err)
    }
}

impl From<hyper::Error> for ApiGrokError {
    fn from(err: hyper::Error) -> Self {
        ApiGrokError::NetworkError(err.to_string())
    }
}

impl From<url::ParseError> for ApiGrokError {
    fn from(err: url::ParseError) -> Self {
        ApiGrokError::InvalidUrl(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, ApiGrokError>;
