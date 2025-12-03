mod cli;
mod error;
mod protocols;
mod request;
mod response;
mod verbose;

use clap::Parser;
use cli::{Cli, Protocol as ProtocolType};
use colored::*;
use error::Result;
use protocols::{
    Protocol,
    auto::AutoClient,
    http1::Http1Client,
    http2::Http2Client,
    http3::Http3Client,
    websocket::WebSocketClient,
    grpc::GrpcClient,
};
use request::Request;

#[tokio::main]
async fn main() {
    // Initialize rustls crypto provider
    let _ = rustls::crypto::ring::default_provider().install_default();

    if let Err(e) = run().await {
        eprintln!("{} {}", "Error:".red().bold(), e);
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();
    let request = Request::from_cli(&cli)?;

    // Determine which protocol to use
    let protocol = cli.determine_protocol();

    // Select the appropriate client
    let client: Box<dyn Protocol> = match protocol {
        ProtocolType::Auto => {
            // Auto-detect based on URL
            if request.url.starts_with("ws://") || request.url.starts_with("wss://") {
                Box::new(WebSocketClient::new())
            } else {
                // Use ALPN to negotiate between HTTP/1.1 and HTTP/2
                Box::new(AutoClient::new())
            }
        }
        ProtocolType::Http1 => Box::new(Http1Client::new()),
        ProtocolType::Http2 => Box::new(Http2Client::new()),
        ProtocolType::Http3 => Box::new(Http3Client::new()),
        ProtocolType::Websocket => Box::new(WebSocketClient::new()),
        ProtocolType::Grpc => Box::new(GrpcClient::new()),
    };

    // Execute the request
    let response = client.execute(&request).await?;

    // Display the response
    response.display(cli.verbose, &cli.output_format);

    // Exit with appropriate code
    if response.is_success() {
        Ok(())
    } else {
        std::process::exit(1)
    }
}
