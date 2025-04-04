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
        #[arg(short, long, default_value = "json")]
        format: String,

        #[arg(short('v'), long("verbose"))]
        verbose: bool,
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
            verbose,
        } => {
            let client = match protocol {
                Protocol::Http1 => Box::new(protocols::http::HttpClient {
                    version: protocols::http::HttpVersion::Http1,
                }) as Box<dyn ApiProtocol>,
                _ => unimplemented!("Protocol not yet implemented"),
            };

            let response = client.fetch(&url).await?;

            display_response(&response, &format, verbose)?;
        }
    }

    Ok(())
}

fn display_response(
    response: &ApiResponse,
    format: &str,
    verbose: bool,
) -> Result<(), Box<dyn Error>> {
    if verbose {
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
