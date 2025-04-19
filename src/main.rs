mod clients;
mod protocols;

use std::error::Error;

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{Shell, generate};
use hyper::Method;
use protocols::{ApiProtocol, ApiResponse, Protocol};
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
    #[arg(short = 'v', long, value_enum, default_value_t = Verbosity::Normal, global = true)]
    verbosity: Verbosity,
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Http { method, url }) => {
            // TODO: http/1.x call
            println!("Performing http1.x {:?} to {}", method, url);
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
                let response = client.fetch(&url).await?;

                render_response(&response, cli.verbosity)?;
            } else {
                eprintln!("No command or URL provided. Try `--help`.");
            }
        }
    }

    Ok(())
}

fn render_response(response: &ApiResponse, verbosity: Verbosity) -> Result<(), Box<dyn Error>> {
    if matches!(verbosity, Verbosity::Debug | Verbosity::Verbose) {
        // TODO: Show resolved IP (requires DNS lookup)
        //let host = response..url().host_str().unwrap_or("unknown");
        let ip = response
            .ip
            .map(|addr| addr.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        println!("* Connected to {} ({})", "unknown", ip);
        println!("* HTTP Version: {}", response.version);
        println!("* Request took: {:?}", response.duration);

        println!("> GET {} {}", response.path, response.version);
        // TODO:
        // for (key, value) in response.request.headers {
        //     println!("> {}: {:?}", key, value);
        // }
        println!(">");

        let status = response.status.unwrap_or_else(|| 0);
        println!("< {} {}", response.version, status);
        if let Some(header_vec) = &response.headers {
            for (name, value) in header_vec {
                println!("{} {}: {}", "<", name, value);
            }
        }

        println!("<");
    }

    response.render_body();

    Ok(())
}
