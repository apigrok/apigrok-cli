mod protocols;

use std::error::Error;

use clap::{Parser, Subcommand};
use protocols::{ApiProtocol, ApiResponse, Protocol};

#[derive(Parser)]
#[command(name = "apigrok")]
#[command(about = "A CLI tool to explore and understand APIs", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Fetch {
        url: String,

        #[arg(short, long, value_enum, default_value_t = Protocol::Http1)]
        protocol: Protocol,

        /// Output format (json, table)
        #[arg(short, long, default_value = "table")]
        format: String,
    },
    Analyze {
        input: String,

        #[arg(short, long, value_enum)]
        protocol: Option<Protocol>,
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
    }

    Ok(())
}

fn display_response(response: &ApiResponse, format: &str) -> Result<(), Box<dyn Error>> {
    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(response)?);
        }
        "table" => {
            print_response_table(response);
        }
        _ => {
            if let Some(body) = &response.body {
                println!("{}", String::from_utf8_lossy(body));
            }
        }
    }
    Ok(())
}

fn print_response_table(response: &ApiResponse) {
    use prettytable::{Table, row};

    let mut table = Table::new();
    table.add_row(row!["Metric", "Value"]);
    table.add_row(row!["Protocol", response.protocol]);
    table.add_row(row!["Status", response.status.unwrap_or(0)]);
    table.add_row(row!["Duration", format!("{:?}", response.duration)]);

    table.add_row(row![b->"Headers"]);

    if let Some(headers) = &response.headers {
        for (i, (name, value)) in headers.iter().enumerate() {
            if i < 5 {
                table.add_row(row![name, value]);
            } else if i == 5 {
                table.add_row(row![Fy->format!("... and {} more", headers.len() - 5)]);
            }
        }
    }

    table.printstd();
}
