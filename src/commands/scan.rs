use anyhow::{Context, Result};
use clap::Subcommand;
use crate::client::Client;

#[derive(Subcommand)]
pub enum ScanCommands {
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

pub async fn handle_scan_command(client: &Client, command: ScanCommands) -> Result<()> {
    match command {
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
    }
    Ok(())
} 