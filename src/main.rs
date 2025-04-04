mod protocols;

use std::error::Error;

use clap::{CommandFactory, Parser, Subcommand};
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

        /// Output format (json, table)
        #[arg(short, long, default_value = "json")]
        format: String,
    },
    Analyze {
        input: String,

        #[arg(short, long, value_enum)]
        protocol: Option<Protocol>,
    },
    /// Generate autocompletion scripts for various shells
    Completion {
        /// The shell for which autocompletion should be generated (e.g. bash)
        shell: Shell,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Fetch {
            url,
            protocol,
            format,
        } => {
            let client = match protocol {
                Protocol::Http1 => Box::new(protocols::http::HttpClient {
                    version: protocols::http::HttpVersion::Http1,
                }) as Box<dyn ApiProtocol>,
                _ => unimplemented!("Protocol not yet implemented"),
            };

            let response = client.fetch(&url).await?;
            display_response(&response, &format)?;
        }
        Commands::Analyze {
            input: _,
            protocol: _,
        } => {}
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

fn display_response(response: &ApiResponse, format: &str) -> Result<(), Box<dyn Error>> {
    match format {
        "json" => {
            println!("{}", serde_json::to_string(&response.display_body())?);
        }
        "table" => unimplemented!("table not yet implemented, if ever"),
        _ => {
            if let Some(body) = &response.body {
                println!("{}", String::from_utf8_lossy(body));
            }
        }
    }
    Ok(())
}
