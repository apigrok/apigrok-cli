mod protocols;

use std::collections::HashSet;
use std::error::Error;

use crate::protocols::ApiRequest;
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

        /// The network protocol to be used to connect to the remote service
        #[arg(short, long, value_enum, default_value_t = Protocol::Http1)]
        protocol: Protocol,

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
    let cli = Cli::parse();

    match cli.command {
        Commands::Fetch {
            url,
            protocol,
            verbose,
            verbose_detail,
        } => {
            let client = match protocol {
                Protocol::Http1 => Box::new(protocols::http::HttpClient {
                    version: protocols::http::HttpVersion::Http1,
                }) as Box<dyn ApiProtocol>,
                _ => unimplemented!("Protocol not yet implemented"),
            };

            let (request, response) = client.fetch(&url).await?;

            render_response(
                &request,
                &response,
                verbose,
                HashSet::from_iter(verbose_detail),
            )?;
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
            if let Some(headers) = &request.headers {
                let remote_ip = response
                    .remote_ip
                    .map(|addr| addr.to_string())
                    .unwrap_or_else(|| "unknown".to_string());

                if let Some((_, host_value)) = headers
                    .iter()
                    .find(|(key, _)| key.eq_ignore_ascii_case(reqwest::header::HOST.as_str()))
                {
                    println!("* Connected to {} ({})", host_value, remote_ip);
                }
            }
            println!("* using {}", request.version);
            println!("* request took: {:?}", response.request_duration);

            println!("> {} {} {}", request.method, request.path, request.version);

            if let Some(header_vec) = &request.headers {
                for (name, value) in header_vec {
                    println!("{} {}: {}", ">", name, value);
                }
            }
            println!(">")
        }

        if verbose_detail.contains(&VerboseDetail::All)
            | verbose_detail.contains(&VerboseDetail::ResponseDetails)
        {
            let status = response.status.unwrap_or_else(|| 0);
            println!("< {} {}", response.version, status);
            if let Some(header_vec) = &response.headers {
                for (name, value) in header_vec {
                    println!("{} {}: {}", "<", name, value);
                }
            }

            println!("<");
        }
    }

    response.render_body();

    Ok(())
}
