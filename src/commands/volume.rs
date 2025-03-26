use anyhow::{Context, Result};
use clap::Subcommand;
use crate::client::Client;

#[derive(Subcommand)]
pub enum VolumeCommands {
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

pub async fn handle_volume_command(client: &Client, command: VolumeCommands) -> Result<()> {
    match command {
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
    }
    Ok(())
} 