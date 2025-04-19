mod clients;
mod color;
mod protocols;

use crate::color::request_output;
use crate::color::response_output;
use crate::protocols::ApiRequest;
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{Shell, generate};
use hyper::Method;
use protocols::{ApiProtocol, ApiResponse, Protocol};
use std::collections::HashSet;
use std::error::Error;
use std::fmt::Debug;
use std::io;

#[derive(Parser)]
#[command(name = "apigrok")]
#[command(about = "A CLI tool to explore and understand APIs", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Shortcut for GET via HTTP/1.x
    #[arg()]
    url: Option<String>,

    /// Set the verbosity level
    /// The volume of output to produce
    #[arg(
        short('v'),
        long,
        value_name = "VERBOSITY",
        value_enum,
        default_value = "normal"
    )]
    verbose: Verbosity,

    /// Specifies which verbose sections should be included
    #[arg(short('d'), long, value_enum, default_values = [ "all"])]
    verbose_detail: Vec<VerboseDetail>,
}

#[derive(Subcommand)]
enum Commands {
    /// Perform a request using HTTP/1.x
    Http {
        #[arg(value_enum)]
        method: Method,

        url: String,

        #[arg(
            long,
            help = "Attempt to upgrade to HTTP/2 over cleartext (h2c) after initial HTTP/1.x connection"
        )]
        h2c: bool,
    },

    /// Perform a request using HTTP/2
    Http2 {
        #[arg(value_enum)]
        method: Method,

        url: String,
    },

    /// Perform a gRPC request
    Grpc {
        #[arg(value_enum)]
        method: Method,

        url: String,
    },

    /// Generate autocompletion scripts
    Completion { shell: Shell },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Verbosity {
    Quiet,
    Normal,
    Verbose,
    Debug,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ValueEnum)]
enum VerboseDetail {
    /// Include all sections appropriate for the current level of verbosity
    All,

    /// Include request details appropriate for the current level of verbosity
    RequestDetails,

    /// Include response details appropriate for the current level of verbosity
    ResponseDetails,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // A[Start Request] --> B{HTTPS?}
    // B -->|Yes| C[ALPN Negotiation]
    // B -->|No| D[Try h2c Prior Knowledge]
    // C -->|h2| E[Use HTTP/2]
    // C -->|http/1.1| F[Use HTTP/1.1]
    // D -->|Success| E
    // D -->|Fail| F
    // A --> G[Check Alt-Svc/DNS for HTTP/3]
    // G -->|Supported| H[QUIC Handshake]
    // H -->|Success| I[Use HTTP/3]
    // H -->|Fail| C

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Http { method, url, h2c }) => {
            // http/1.x call

            let client: Box<dyn ApiProtocol> = Box::new(protocols::http::HttpClient {
                version: protocols::http::HttpVersion::Http1,
            });
            let (request, response) = client.execute(method, &url, h2c).await?;

            let _ = render_response(
                &request,
                &response,
                cli.verbose,
                HashSet::from_iter(cli.verbose_detail),
            )?;
        }

        Some(Commands::Http2 { method, url }) => {
            // TODO: http/2 call
            println!("Performing http2 {:?} to {}", method, url);
        }

        Some(Commands::Grpc { method, url }) => {
            // TODO: grpc call
            println!("Performing gRPC {:?} to {}", method, url);
        }

        Some(Commands::Completion { shell }) => {
            let cmd = &mut Cli::command();
            generate(shell, cmd, cmd.get_name().to_string(), &mut io::stdout());
        }

        None => {
            if let Some(url) = cli.url {
                // Default: GET via HTTP/1.1
                let client: Box<dyn ApiProtocol> = Box::new(protocols::http::HttpClient {
                    version: protocols::http::HttpVersion::Http1,
                });
                let (request, response) = client.execute(Method::GET, &url, false).await?;

                let _ = render_response(
                    &request,
                    &response,
                    cli.verbose,
                    HashSet::from_iter(cli.verbose_detail),
                );
            } else {
                eprintln!("No command or URL provided. Try `--help`.");
            }
        }
    }

    Ok(())
}

fn render_response(
    request: &ApiRequest,
    response: &ApiResponse,
    verbosity: Verbosity,
    verbose_detail: HashSet<VerboseDetail>,
) -> Result<(), Box<dyn Error>> {
    if matches!(verbosity, Verbosity::Debug | Verbosity::Verbose) {
        if verbose_detail.contains(&VerboseDetail::All)
            | verbose_detail.contains(&VerboseDetail::RequestDetails)
        {
            request_output!({
                println!("> {} {} {}", request.method, request.path, request.version);

                if let Some(header_vec) = &request.headers {
                    for (name, value) in header_vec {
                        println!("{} {}: {}", ">", name, value);
                    }
                }
            });
        }

        if verbose_detail.contains(&VerboseDetail::All)
            | verbose_detail.contains(&VerboseDetail::ResponseDetails)
        {
            response_output!({
                // TODO: Show resolved IP (requires DNS lookup)
                //let host = response..url().host_str().unwrap_or("unknown");
                let ip = response
                    .ip
                    .map(|addr| addr.to_string())
                    .unwrap_or_else(|| "unknown".to_string());

                println!("* Connected to {} ({})", "unknown", ip);
                println!("* HTTP Version: {}", response.version);
                println!("* Request took: {:?}", response.duration);

                let status = response.status.unwrap_or_else(|| 0);
                println!("< {} {} {}", response.path, response.version, status);
                if let Some(header_vec) = &response.headers {
                    for (name, value) in header_vec {
                        println!("{} {}: {}", "<", name, value);
                    }
                }

                println!("<");
            });
        }
    }

    response.render_body();

    Ok(())
}
