use anyhow::{Context, Result};
use clap::{Parser, Subcommand, CommandFactory};
use serde_json::Value;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use clap_complete::{generate, Generator, Shell};
use std::io;

const API_BASE_URL: &str = "https://localhost/api";
const IGNORE_SSL_CERTIFICATE_VERIFICATION: bool = true;
const HTTP_BASIC_AUTH_USER: &str = "starfish";
const HTTP_BASIC_AUTH_PASSWORD: &str = "starfish";

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Generate shell completion script
    #[arg(long = "generate", value_enum)]
    generator: Option<Shell>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Volume-related commands
    Volume {
        #[command(subcommand)]
        command: VolumeCommands,
    },
    /// Scan-related commands
    Scan {
        #[command(subcommand)]
        command: ScanCommands,
    },
}

#[derive(Subcommand)]
enum VolumeCommands {
    /// List all volumes
    List {
        /// Output full JSON response
        #[arg(long)]
        json: bool,
    },
    /// Show details of a specific volume
    Show {
        /// Name of the volume
        name: String,
        /// Output full JSON response
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum ScanCommands {
    /// List all scans
    List {
        /// Output full JSON response
        #[arg(long)]
        json: bool,
    },
    /// Show details of a specific scan
    Show {
        /// ID of the scan
        id: String,
        /// Output full JSON response
        #[arg(long)]
        json: bool,
    },
}

struct Client {
    client: reqwest::Client,
    base_url: String,
}

impl Client {
    fn new() -> Self {
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(IGNORE_SSL_CERTIFICATE_VERIFICATION)
            .default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                let auth = format!("Basic {}", STANDARD.encode(format!(
                    "{}:{}",
                    HTTP_BASIC_AUTH_USER,
                    HTTP_BASIC_AUTH_PASSWORD
                )));
                headers.insert(
                    reqwest::header::AUTHORIZATION,
                    reqwest::header::HeaderValue::from_str(&auth).unwrap(),
                );
                headers
            })
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url: API_BASE_URL.to_string(),
        }
    }

    async fn get_volumes(&self) -> Result<Value> {
        let url = format!("{}/volume/", self.base_url);
        let response = self.client
            .get(&url)
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            eprintln!("Not authorized");
            std::process::exit(1);
        }

        Ok(response.json().await?)
    }

    async fn get_volume(&self, name: &str) -> Result<Option<Value>> {
        let url = format!("{}/volume/{}", self.base_url, name);
        let response = self.client
            .get(&url)
            .send()
            .await?;
        
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            eprintln!("Not authorized");
            std::process::exit(1);
        }

        match response.status() {
            reqwest::StatusCode::NOT_FOUND => Ok(None),
            _ => Ok(Some(response.json().await?)),
        }
    }

    async fn get_scans(&self) -> Result<Value> {
        let url = format!("{}/scan/", self.base_url);
        let response = self.client
            .get(&url)
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            eprintln!("Not authorized");
            std::process::exit(1);
        }

        Ok(response.json().await?)
    }

    async fn get_scan(&self, id: &str) -> Result<Option<Value>> {
        let url = format!("{}/scan/{}", self.base_url, id);
        let response = self.client
            .get(&url)
            .send()
            .await?;
        
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            eprintln!("Not authorized");
            std::process::exit(1);
        }

        match response.status() {
            reqwest::StatusCode::NOT_FOUND => Ok(None),
            _ => Ok(Some(response.json().await?)),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Handle completion generation
    if let Some(generator) = cli.generator {
        generate_completion(generator, &mut io::stdout());
        return Ok(());
    }

    // Get command or exit with error if none provided
    let command = cli.command.ok_or_else(|| {
        anyhow::anyhow!("A subcommand is required unless using --generate")
    })?;

    // Your existing main logic
    let client = Client::new();

    match command {
        Commands::Volume { command } => match command {
            VolumeCommands::List { json } => {
                let response = client.get_volumes().await?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&response)?);
                } else {
                    let volumes = response.as_array().context("Expected array of volumes")?;
                    for volume in volumes {
                        if let Some(name) = volume["vol"].as_str() {
                            println!("{}", name);
                        }
                    }
                }
            }
            VolumeCommands::Show { name, json } => {
                match client.get_volume(&name).await? {
                    Some(volume) => {
                        if json {
                            println!("{}", serde_json::to_string_pretty(&volume)?);
                        } else {
                            let vol_name = volume["vol"].as_str().unwrap_or("");
                            let agent = volume["default_agent_address"].as_str().unwrap_or("");
                            println!("{} {}", vol_name, agent);
                        }
                    }
                    None => eprintln!("There is no volume {}", name),
                }
            }
        },
        Commands::Scan { command } => match command {
            ScanCommands::List { json } => {
                let response = client.get_scans().await?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&response)?);
                } else {
                    let scans = response["scans"].as_array().context("Expected scans array")?;
                    for scan in scans {
                        if let Some(id) = scan["id"].as_str() {
                            println!("{}", id);
                        }
                    }
                }
            }
            ScanCommands::Show { id, json } => {
                match client.get_scan(&id).await? {
                    Some(scan) => {
                        if json {
                            println!("{}", serde_json::to_string_pretty(&scan)?);
                        } else {
                            let scan_id = scan["id"].as_str().unwrap_or("");
                            let volume = scan["volume"].as_str().unwrap_or("");
                            println!("{} {}", scan_id, volume);
                        }
                    }
                    None => eprintln!("There is no scan {}", id),
                }
            }
        },
    }

    Ok(())
}

fn generate_completion<G: Generator>(gen: G, buf: &mut dyn io::Write) {
    generate(gen, &mut Cli::command(), "rsf", buf);
}
