mod protocols;

use std::error::Error;

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{Shell, generate};
use protocols::{ApiProtocol, ApiResponse, Protocol};
use std::io;

#[derive(Parser)]
#[command(name = "apigrok")]
#[command(about = "A CLI tool to explore and understand APIs", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Fetch the content available at a specified url
    Fetch {
        url: String,

        #[arg(short, long, value_enum, default_value_t = Protocol::Http1)]
        protocol: Protocol,

        #[arg(short('v'), long, value_enum, default_value_t = Verbosity::Normal)]
        verbosity: Verbosity,
    },
    /// Generate autocompletion scripts for various shells
    Completion {
        /// The shell for which autocompletion should be generated (e.g. bash)
        shell: Shell,
    },
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
        Commands::Fetch {
            url,
            protocol,
            verbosity,
        } => {
            let client = match protocol {
                Protocol::Http1 => Box::new(protocols::http::HttpClient {
                    version: protocols::http::HttpVersion::Http1,
                }) as Box<dyn ApiProtocol>,
                _ => unimplemented!("Protocol not yet implemented"),
            };

            let response = client.fetch(&url).await?;

            render_response(&response, verbosity)?;
        }
        Commands::Completion { shell } => {
            let command = &mut Cli::command();
            generate(
                shell,
                command,
                command.get_name().to_string(),
                &mut io::stdout(),
            );
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
